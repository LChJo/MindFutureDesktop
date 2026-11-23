use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::AppHandle;

/// Build a PATH for npm subprocesses: prepend bundled nodejs dir, keep the
/// inherited PATH so npm lifecycle scripts can still find `/bin/sh`, git, etc.
/// Falls back to a conservative Unix default if the parent has no PATH (e.g.,
/// macOS apps launched from the Dock sometimes get an empty PATH).
fn npm_child_path(nodejs_dir: &Path) -> String {
    let sep = if cfg!(windows) { ";" } else { ":" };
    let nodejs_str = nodejs_dir.to_string_lossy().to_string();
    match std::env::var("PATH") {
        Ok(existing) if !existing.is_empty() => {
            format!("{}{}{}", nodejs_str, sep, existing)
        }
        _ => {
            #[cfg(target_family = "unix")]
            {
                format!(
                    "{}:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin",
                    nodejs_str
                )
            }
            #[cfg(not(target_family = "unix"))]
            {
                nodejs_str
            }
        }
    }
}

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodejsStatus {
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub source: Option<String>, // "system" or "bundled"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClaudeCodeStatus {
    pub installed: bool,
    pub version: Option<String>,
    pub error: Option<String>, // Error details if detection failed
}

/// Get the nodejs directory path (~/.MoontonClaudeDesktop/nodejs)
pub fn get_nodejs_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir()
        .ok_or_else(|| "Failed to get home directory".to_string())?;
    Ok(home.join(".MoontonClaudeDesktop").join("nodejs"))
}

/// Get the bundled resources directory path (platform-aware)
/// Windows: {exe_dir}/resources
/// macOS: {exe_dir}/../Resources/resources (inside .app bundle)
pub fn get_bundled_resources_dir() -> Option<PathBuf> {
    let exe_path = std::env::current_exe().ok()?;
    let exe_dir = exe_path.parent()?;

    #[cfg(target_os = "macos")]
    {
        // macOS: .app/Contents/MacOS/exe -> .app/Contents/Resources/resources
        let resources_dir = exe_dir.parent()?.join("Resources").join("resources");
        if resources_dir.exists() {
            log::info!("Found bundled resources at macOS app bundle path: {:?}", resources_dir);
            return Some(resources_dir);
        }
        // Fallback to exe_dir for development or non-.app scenarios
        let fallback = exe_dir.join("resources");
        if fallback.exists() {
            log::info!("Found bundled resources at fallback path: {:?}", fallback);
            return Some(fallback);
        }
        log::warn!("Bundled resources not found at {:?} or {:?}", resources_dir, fallback);
        None
    }

    #[cfg(not(target_os = "macos"))]
    {
        let resources_dir = exe_dir.join("resources");
        if resources_dir.exists() {
            log::info!("Found bundled resources at: {:?}", resources_dir);
            Some(resources_dir)
        } else {
            log::warn!("Bundled resources not found at: {:?}", resources_dir);
            None
        }
    }
}

/// Parse Node.js version string (e.g., "22.19.0") into comparable tuple
#[cfg(target_os = "macos")]
fn parse_node_version(version: &str) -> Option<(u32, u32, u32)> {
    let version = version.trim().trim_start_matches('v');
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 2 {
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let patch = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0);
        Some((major, minor, patch))
    } else {
        None
    }
}

/// Check for system-installed Node.js (in PATH)
/// Returns (version, path) if found
pub fn check_system_nodejs() -> Option<(String, String)> {
    #[cfg(target_os = "windows")]
    {
        // Try to run 'node --version' from system PATH
        let mut cmd = Command::new("node");
        cmd.args(["--version"]);
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = cmd.output().ok()?;

        if !output.status.success() {
            return None;
        }

        let version = String::from_utf8_lossy(&output.stdout)
            .trim()
            .trim_start_matches('v')
            .to_string();

        if version.is_empty() {
            return None;
        }

        // Try to get node path using where command
        let mut where_cmd = Command::new("where");
        where_cmd.arg("node");
        where_cmd.creation_flags(CREATE_NO_WINDOW);

        if let Ok(where_output) = where_cmd.output() {
            if where_output.status.success() {
                let path = String::from_utf8_lossy(&where_output.stdout)
                    .lines()
                    .next()
                    .map(|p| {
                        // Get parent directory of node.exe
                        PathBuf::from(p.trim())
                            .parent()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|| p.trim().to_string())
                    })
                    .unwrap_or_default();

                if !path.is_empty() {
                    return Some((version, path));
                }
            }
        }

        // Fallback: just return version without path
        Some((version, "System PATH".to_string()))
    }

    #[cfg(target_os = "macos")]
    {
        use std::collections::HashSet;

        let mut candidates: HashSet<String> = HashSet::new();

        // 1. Add known standard paths
        candidates.insert("/usr/bin/node".to_string());
        candidates.insert("/usr/local/bin/node".to_string());
        candidates.insert("/opt/homebrew/bin/node".to_string());

        // 2. Scan all nvm versions
        if let Some(home_dir) = dirs::home_dir() {
            let nvm_dir = home_dir.join(".nvm/versions/node");
            if nvm_dir.exists() {
                if let Ok(entries) = fs::read_dir(&nvm_dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let node_bin = entry.path().join("bin/node");
                        if node_bin.exists() {
                            candidates.insert(node_bin.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        // 3. Use 'which -a' to get all node paths in PATH
        if let Ok(output) = Command::new("which").args(["-a", "node"]).output() {
            if output.status.success() {
                for line in String::from_utf8_lossy(&output.stdout).lines() {
                    let path = line.trim();
                    if !path.is_empty() {
                        candidates.insert(path.to_string());
                    }
                }
            }
        }

        // 4. Collect all valid Node.js versions
        let mut found_nodes: Vec<((u32, u32, u32), String, String)> = Vec::new();

        for node_path_str in candidates {
            let node_path = PathBuf::from(&node_path_str);
            if !node_path.exists() {
                continue;
            }

            let mut cmd = Command::new(&node_path);
            cmd.args(["--version"]);

            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    let version = String::from_utf8_lossy(&output.stdout)
                        .trim()
                        .trim_start_matches('v')
                        .to_string();

                    if let Some(version_tuple) = parse_node_version(&version) {
                        let bin_dir = node_path
                            .parent()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|| node_path_str.clone());

                        log::info!("Found Node.js v{} at {}", version, bin_dir);
                        found_nodes.push((version_tuple, version, bin_dir));
                    }
                }
            }
        }

        // 5. Sort by version number and select the highest
        found_nodes.sort_by(|a, b| b.0.cmp(&a.0)); // Descending order

        if let Some((_, version, path)) = found_nodes.into_iter().next() {
            log::info!("Selected Node.js v{} at {} (highest version)", version, path);
            return Some((version, path));
        }

        log::info!("Node.js not found in any known location");
        None
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        // Try to run 'node --version' from system PATH
        let mut cmd = Command::new("node");
        cmd.args(["--version"]);

        let output = cmd.output().ok()?;

        if !output.status.success() {
            return None;
        }

        let version = String::from_utf8_lossy(&output.stdout)
            .trim()
            .trim_start_matches('v')
            .to_string();

        if version.is_empty() {
            return None;
        }

        Some((version, "System PATH".to_string()))
    }
}

/// Check if Node.js is installed in AppData
pub fn is_nodejs_installed() -> bool {
    if let Ok(nodejs_dir) = get_nodejs_dir() {
        #[cfg(target_os = "windows")]
        let node_binary = nodejs_dir.join("node.exe");

        #[cfg(target_os = "macos")]
        let node_binary = nodejs_dir.join("node");

        node_binary.exists()
    } else {
        false
    }
}

/// Get the installed Node.js version from version.txt
pub fn get_nodejs_version() -> Option<String> {
    if let Ok(nodejs_dir) = get_nodejs_dir() {
        let version_file = nodejs_dir.join("version.txt");
        if version_file.exists() {
            fs::read_to_string(version_file).ok().map(|v| v.trim().to_string())
        } else {
            None
        }
    } else {
        None
    }
}

/// Get bundled Node.js version from resource directory
fn get_bundled_nodejs_version<R: tauri::Runtime>(_app_handle: &AppHandle<R>) -> Option<String> {
    // Platform-specific resource subdirectory
    #[cfg(target_os = "windows")]
    let resource_subdir = "nodejs-windows";

    #[cfg(target_os = "macos")]
    let resource_subdir = "nodejs-macos";

    // Use platform-aware resource path resolution
    let resources_dir = get_bundled_resources_dir()?;
    let resource_path = resources_dir.join(resource_subdir).join("version.txt");

    log::info!("Looking for bundled Node.js version at: {:?}", resource_path);

    fs::read_to_string(&resource_path)
        .ok()
        .map(|v| v.trim().to_string())
}

/// Check if extraction is needed (no existing node or version mismatch)
fn needs_extraction<R: tauri::Runtime>(app_handle: &AppHandle<R>) -> bool {
    let nodejs_dir = match get_nodejs_dir() {
        Ok(dir) => dir,
        Err(_) => return true, // Can't get directory, need extraction
    };

    #[cfg(target_os = "windows")]
    let node_binary = nodejs_dir.join("node.exe");

    #[cfg(target_os = "macos")]
    let node_binary = nodejs_dir.join("node");

    // If node binary doesn't exist, need extraction
    if !node_binary.exists() {
        log::info!("Node.js binary not found, extraction needed");
        return true;
    }

    // Check npm directory exists (required for npm to work)
    let npm_dir = nodejs_dir.join("node_modules").join("npm");
    if !npm_dir.exists() {
        log::info!("npm directory not found, extraction needed");
        return true;
    }

    // Compare versions
    let installed_version = get_nodejs_version();
    let bundled_version = get_bundled_nodejs_version(app_handle);

    match (installed_version, bundled_version) {
        (Some(installed), Some(bundled)) => {
            if installed != bundled {
                log::info!(
                    "Version mismatch: installed={}, bundled={}. Extraction needed.",
                    installed,
                    bundled
                );
                true
            } else {
                log::info!("Node.js v{} already installed, skipping extraction", installed);
                false
            }
        }
        (None, Some(_)) => {
            log::info!("No installed version found, extraction needed");
            true
        }
        (Some(installed), None) => {
            log::info!(
                "No bundled version info, keeping installed v{}",
                installed
            );
            false // Keep existing if we can't determine bundled version
        }
        (None, None) => {
            log::info!("No version info available, extraction needed");
            true
        }
    }
}

/// Extract Node.js files from bundled resources to AppData
pub fn extract_nodejs<R: tauri::Runtime>(app_handle: &AppHandle<R>) -> Result<(), String> {
    log::info!("Starting Node.js extraction check...");

    // Check if extraction is needed
    if !needs_extraction(app_handle) {
        return Ok(());
    }

    log::info!("Proceeding with Node.js extraction...");

    let nodejs_dir = get_nodejs_dir().map_err(|e| {
        log::error!("Failed to get nodejs directory: {}", e);
        e
    })?;

    // Clean up existing directory to ensure fresh extraction
    if nodejs_dir.exists() {
        log::info!("Removing old Node.js directory: {:?}", nodejs_dir);
        if let Err(e) = fs::remove_dir_all(&nodejs_dir) {
            log::warn!("Failed to remove old Node.js directory: {}. Will attempt overwrite.", e);
        }
    }

    // Create nodejs directory
    fs::create_dir_all(&nodejs_dir).map_err(|e| {
        let err = format!("Failed to create nodejs directory {:?}: {}", nodejs_dir, e);
        log::error!("{}", err);
        err
    })?;

    // Platform-specific resource subdirectory
    #[cfg(target_os = "windows")]
    let resource_subdir = "nodejs-windows";

    #[cfg(target_os = "macos")]
    let resource_subdir = "nodejs-macos";

    // Get the resource path from bundled resources (platform-aware)
    let resources_dir = get_bundled_resources_dir().ok_or_else(|| {
        let err = "Failed to get bundled resources directory".to_string();
        log::error!("{}", err);
        err
    })?;
    let resource_path = resources_dir.join(resource_subdir);

    if !resource_path.exists() {
        let err = format!("Node.js resource path does not exist: {:?}", resource_path);
        log::error!("{}", err);
        return Err(err);
    }

    log::info!("Extracting Node.js from: {:?} to: {:?}", resource_path, nodejs_dir);

    // Copy all files recursively
    copy_dir_recursive(&resource_path, &nodejs_dir).map_err(|e| {
        log::error!("Failed to copy Node.js files: {}", e);
        e
    })?;

    // On macOS, set executable permissions for binaries
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::PermissionsExt;

        let executables = vec!["node", "npm", "npx"];
        for exe in executables {
            let exe_path = nodejs_dir.join(exe);
            if exe_path.exists() {
                let mut perms = fs::metadata(&exe_path)
                    .map_err(|e| format!("Failed to get metadata for {}: {}", exe, e))?
                    .permissions();
                perms.set_mode(0o755); // rwxr-xr-x
                fs::set_permissions(&exe_path, perms)
                    .map_err(|e| format!("Failed to set permissions for {}: {}", exe, e))?;
                log::info!("Set executable permissions for: {:?}", exe_path);
            } else {
                // Not all bundled layouts have top-level npm/npx shims; that's OK.
                log::debug!("Executable shim not found (ok): {:?}", exe_path);
            }
        }
    }

    log::info!("Node.js extracted successfully to: {:?}", nodejs_dir);
    Ok(())
}

/// Extract Node.js asynchronously in a separate thread
#[allow(dead_code)]
pub fn extract_nodejs_async<R: tauri::Runtime + 'static>(
    app_handle: AppHandle<R>,
) -> std::thread::JoinHandle<Result<(), String>> {
    std::thread::spawn(move || {
        extract_nodejs(&app_handle)
    })
}

/// Recursively copy directory contents
fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> Result<(), String> {
    if !dst.exists() {
        fs::create_dir_all(dst)
            .map_err(|e| format!("Failed to create directory {:?}: {}", dst, e))?;
    }

    for entry in fs::read_dir(src).map_err(|e| format!("Failed to read directory: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);

        if path.is_dir() {
            copy_dir_recursive(&path, &dst_path)?;
        } else {
            fs::copy(&path, &dst_path)
                .map_err(|e| format!("Failed to copy {:?} to {:?}: {}", path, dst_path, e))?;
        }
    }

    Ok(())
}

/// Get the claude command path in npm global bin directory
pub fn get_claude_cmd_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        dirs::data_dir().map(|dir| dir.join("npm").join("claude.cmd"))
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: ~/.npm-global/bin/claude or /usr/local/bin/claude
        if let Some(home) = dirs::home_dir() {
            let npm_global = home.join(".npm-global").join("bin").join("claude");
            if npm_global.exists() {
                return Some(npm_global);
            }
        }

        // Fallback to system-wide installation
        let system_path = PathBuf::from("/usr/local/bin/claude");
        if system_path.exists() {
            Some(system_path)
        } else {
            None
        }
    }
}

/// Check if claude command is available (either in npm global bin or system PATH)
fn check_system_claude() -> bool {
    // First check npm global bin directory
    if let Some(claude_path) = get_claude_cmd_path() {
        if claude_path.exists() {
            return true;
        }
    }

    // Fallback: check system PATH using platform-specific command
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("where");
        cmd.arg("claude");
        cmd.creation_flags(CREATE_NO_WINDOW);

        cmd.output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[cfg(target_os = "macos")]
    {
        let mut cmd = Command::new("which");
        cmd.arg("claude");

        cmd.output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

/// Check if claude command is available
fn is_claude_command_available() -> bool {
    check_system_claude()
}

/// Get claude version by running claude --version
fn get_claude_version() -> Option<String> {
    // First try using the known path in npm global bin
    #[cfg(target_os = "windows")]
    {
        if let Some(claude_path) = get_claude_cmd_path() {
            if claude_path.exists() {
                let mut cmd = Command::new("cmd");
                cmd.args(["/c", &claude_path.to_string_lossy(), "--version"]);
                cmd.creation_flags(CREATE_NO_WINDOW);

                if let Ok(output) = cmd.output() {
                    if output.status.success() {
                        let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if !version_str.is_empty() {
                            return Some(version_str);
                        }
                    }
                }
            }
        }

        // Fallback: try system PATH
        let mut cmd = Command::new("cmd");
        cmd.args(["/c", "claude", "--version"]);
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = cmd.output().ok()?;
        if output.status.success() {
            let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !version_str.is_empty() {
                return Some(version_str);
            }
        }
        None
    }

    #[cfg(target_os = "macos")]
    {
        // First try using the known path in npm global bin directory
        if let Some(claude_path) = get_claude_cmd_path() {
            if claude_path.exists() {
                log::info!("Checking claude version at: {:?}", claude_path);
                let mut cmd = Command::new(&claude_path);
                cmd.arg("--version");

                if let Ok(output) = cmd.output() {
                    if output.status.success() {
                        let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        log::info!("Claude version output: {}", version_str);
                        if !version_str.is_empty() {
                            return Some(version_str);
                        }
                    } else {
                        log::warn!("Claude command failed with status: {:?}", output.status);
                    }
                } else {
                    log::warn!("Failed to execute claude command at: {:?}", claude_path);
                }
            } else {
                log::warn!("Claude path does not exist: {:?}", claude_path);
            }
        }

        // Fallback: try system PATH using 'which' to find the actual executable
        if let Ok(which_output) = Command::new("which").arg("claude").output() {
            if which_output.status.success() {
                let claude_path_str = String::from_utf8_lossy(&which_output.stdout).trim().to_string();
                if !claude_path_str.is_empty() {
                    log::info!("Found claude via 'which' at: {}", claude_path_str);
                    let mut cmd = Command::new(&claude_path_str);
                    cmd.arg("--version");

                    if let Ok(output) = cmd.output() {
                        if output.status.success() {
                            let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                            log::info!("Claude version from system PATH: {}", version_str);
                            if !version_str.is_empty() {
                                return Some(version_str);
                            }
                        }
                    }
                }
            }
        }

        // Last resort: direct command
        let mut cmd = Command::new("claude");
        cmd.arg("--version");

        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !version_str.is_empty() {
                    return Some(version_str);
                }
            }
        }

        None
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let mut cmd = Command::new("claude");
        cmd.arg("--version");

        let output = cmd.output().ok()?;
        if output.status.success() {
            let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if version_str.is_empty() {
                None
            } else {
                Some(version_str)
            }
        } else {
            None
        }
    }
}

/// Quick check if Claude Code is installed in system PATH
pub fn is_claude_code_installed_quick() -> bool {
    check_system_claude()
}

/// Get Claude Code installation status (full check with version)
pub fn get_claude_code_status() -> ClaudeCodeStatus {
    // Core detection: check if claude command is available
    if is_claude_command_available() {
        // Try to get version
        let version = get_claude_version();
        return ClaudeCodeStatus {
            installed: true,
            version,
            error: None,
        };
    }

    // Not installed
    ClaudeCodeStatus {
        installed: false,
        version: None,
        error: None,
    }
}

/// Get the mcd CLI resource path from bundled resources
fn get_mcd_cli_resource_path<R: tauri::Runtime>(
    _app_handle: &tauri::AppHandle<R>,
) -> Result<PathBuf, String> {
    // Priority 1: Production environment - use platform-aware resource path
    if let Some(resources_dir) = get_bundled_resources_dir() {
        let prod_path = resources_dir.join("mcd-cli");
        log::info!("Checking mcd-cli at bundled resources: {:?}", prod_path);
        if prod_path.exists() {
            log::info!("Found mcd-cli at bundled resources: {:?}", prod_path);
            return Ok(prod_path);
        }
    }

    // Priority 2: Development environment - try multiple paths
    let dev_paths = [
        PathBuf::from("src-tauri/resources/mcd-cli"), // From project root
        PathBuf::from("resources/mcd-cli"),           // From src-tauri directory
    ];

    for dev_path in &dev_paths {
        if dev_path.exists() {
            log::info!("Found mcd-cli at dev path: {:?}", dev_path);
            return Ok(dev_path.clone());
        }
    }

    Err("mcd-cli resource not found".to_string())
}

/// Install mcd CLI to npm global
pub fn install_mcd_cli<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<String, String> {
    log::info!("Starting mcd CLI installation...");

    let mcd_cli_path = get_mcd_cli_resource_path(app_handle)?;

    let (npm_path, nodejs_dir) = get_npm_path()?;

    // Get the correct npm prefix path (platform-specific)
    #[cfg(target_os = "windows")]
    let npm_prefix = dirs::data_dir()
        .ok_or_else(|| "Failed to get AppData/Roaming directory".to_string())?
        .join("npm");

    #[cfg(target_os = "macos")]
    let npm_prefix = dirs::home_dir()
        .ok_or_else(|| "Failed to get home directory".to_string())?
        .join(".npm-global");

    // Create npm prefix directory if it doesn't exist
    if !npm_prefix.exists() {
        fs::create_dir_all(&npm_prefix).map_err(|e| {
            format!("Failed to create npm prefix directory {:?}: {}", npm_prefix, e)
        })?;
    }

    log::info!("Installing mcd CLI from: {:?}", mcd_cli_path);

    // Install mcd-cli globally from local path
    let mut install_cmd = Command::new(&npm_path);

    #[cfg(target_os = "macos")]
    if let Some(npm_cli) = bundled_npm_cli_if_needed(&npm_path, &nodejs_dir) {
        install_cmd.arg(npm_cli);
    }

    install_cmd
        .args(["install", "-g", &mcd_cli_path.to_string_lossy()])
        .env("PATH", npm_child_path(&nodejs_dir))
        .env("NPM_CONFIG_PREFIX", npm_prefix.to_string_lossy().to_string());

    #[cfg(target_os = "windows")]
    install_cmd.creation_flags(CREATE_NO_WINDOW);

    let output = install_cmd.output().map_err(|e| {
        let err = format!("Failed to run npm install for mcd-cli: {}", e);
        log::error!("{}", err);
        err
    })?;

    if output.status.success() {
        log::info!("mcd CLI installed successfully");
        Ok("mcd CLI installed successfully".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let err = format!("mcd CLI installation failed: stdout={} stderr={}", stdout, stderr);
        log::error!("{}", err);
        Err(err)
    }
}

/// Get npm path - prefers system npm if available, falls back to bundled
fn get_npm_path() -> Result<(PathBuf, PathBuf), String> {
    // First, check for system Node.js/npm
    if let Some((_, node_dir)) = check_system_nodejs() {
        let node_path = PathBuf::from(&node_dir);

        #[cfg(target_os = "windows")]
        let npm_name = "npm.cmd";
        #[cfg(target_os = "macos")]
        let npm_name = "npm";

        let npm_path = node_path.join(npm_name);
        if npm_path.exists() {
            log::info!("Using system npm at: {:?}", npm_path);
            return Ok((npm_path, node_path));
        }

        // On macOS, npm might be in a different directory than node
        #[cfg(target_os = "macos")]
        {
            // Try common npm locations
            let possible_npm_paths = vec![
                PathBuf::from("/usr/local/bin/npm"),
                PathBuf::from("/opt/homebrew/bin/npm"),
            ];

            for possible_npm in possible_npm_paths {
                if possible_npm.exists() {
                    log::info!("Using system npm at: {:?}", possible_npm);
                    let npm_dir = possible_npm.parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| node_path.clone());
                    return Ok((possible_npm, npm_dir));
                }
            }

            // Try using 'which' to find npm
            if let Ok(which_output) = std::process::Command::new("which").arg("npm").output() {
                if which_output.status.success() {
                    let npm_path_str = String::from_utf8_lossy(&which_output.stdout).trim().to_string();
                    if !npm_path_str.is_empty() {
                        let npm_path = PathBuf::from(&npm_path_str);
                        if npm_path.exists() {
                            log::info!("Found system npm via 'which' at: {:?}", npm_path);
                            let npm_dir = npm_path.parent()
                                .map(|p| p.to_path_buf())
                                .unwrap_or_else(|| node_path.clone());
                            return Ok((npm_path, npm_dir));
                        }
                    }
                }
            }
        }
    }

    // Fallback to bundled Node.js
    let nodejs_dir = get_nodejs_dir().map_err(|_| {
        "npm not found. Please ensure Node.js is installed on the system or extract the bundled version.".to_string()
    })?;

    #[cfg(target_os = "windows")]
    {
        let npm_path = nodejs_dir.join("npm.cmd");
        if npm_path.exists() {
            log::info!("Using bundled npm at: {:?}", npm_path);
            return Ok((npm_path, nodejs_dir));
        }
    }

    #[cfg(target_os = "macos")]
    {
        // Prefer a real npm executable shim if present
        let npm_shim = nodejs_dir.join("npm");
        if npm_shim.exists() {
            log::info!("Using bundled npm shim at: {:?}", npm_shim);
            return Ok((npm_shim, nodejs_dir));
        }

        // Some bundled Node.js layouts don't include a top-level `npm` binary.
        // In that case, run npm via: `node <.../npm-cli.js>`.
        let npm_cli = nodejs_dir.join("lib/node_modules/npm/bin/npm-cli.js");
        let node_bin = nodejs_dir.join("node");
        if node_bin.exists() && npm_cli.exists() {
            log::info!(
                "Bundled npm shim not found; using npm-cli.js via node: {:?}",
                npm_cli
            );
            // Return `node` as the executable; call sites should pass args starting with the cli path.
            let node_bin_dir = node_bin
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| nodejs_dir.clone());
            return Ok((node_bin, node_bin_dir));
        }
    }

    Err("npm not found. Please ensure Node.js is installed on the system or extract the bundled version.".to_string())
}

/// On macOS, if `npm_exe` is actually the bundled `node` binary, return the bundled npm-cli.js path.
#[cfg(target_os = "macos")]
fn bundled_npm_cli_if_needed(npm_exe: &PathBuf, nodejs_dir: &PathBuf) -> Option<PathBuf> {
    let npm_cli = nodejs_dir.join("lib/node_modules/npm/bin/npm-cli.js");
    if npm_exe.file_name().is_some_and(|n| n == "node") && npm_cli.exists() {
        Some(npm_cli)
    } else {
        None
    }
}


/// Install Claude Code via npm (hides console window on Windows)
pub fn install_claude_code() -> Result<String, String> {
    log::info!("Starting Claude Code installation...");

    let (npm_path, nodejs_dir) = get_npm_path()?;

    log::info!("Using npm at: {:?}", npm_path);

    // Get npm root for global packages
    let mut root_cmd = Command::new(&npm_path);

    #[cfg(target_os = "macos")]
    let npm_cli = {
        // If npm_path is actually the bundled `node` binary, run npm via npm-cli.js
        let candidate = nodejs_dir.join("lib/node_modules/npm/bin/npm-cli.js");
        if npm_path.file_name().is_some_and(|n| n == "node") && candidate.exists() {
            Some(candidate)
        } else {
            None
        }
    };

    #[cfg(not(target_os = "macos"))]
    let npm_cli: Option<PathBuf> = None;

    if let Some(cli) = &npm_cli {
        root_cmd.arg(cli);
    }

    root_cmd
        .args(["root", "-g"])
        .env("PATH", npm_child_path(&nodejs_dir));

    #[cfg(target_os = "windows")]
    root_cmd.creation_flags(CREATE_NO_WINDOW);

    let npm_root_output = root_cmd.output().map_err(|e| {
        let err = format!("Failed to get npm root: {}", e);
        log::error!("{}", err);
        err
    })?;

    let npm_root = String::from_utf8_lossy(&npm_root_output.stdout).trim().to_string();
    log::info!("npm global root: {}", npm_root);

    // Get the correct npm prefix path (platform-specific)
    #[cfg(target_os = "windows")]
    let npm_prefix = dirs::data_dir()
        .ok_or_else(|| "Failed to get AppData/Roaming directory".to_string())?
        .join("npm");

    #[cfg(target_os = "macos")]
    let npm_prefix = dirs::home_dir()
        .ok_or_else(|| "Failed to get home directory".to_string())?
        .join(".npm-global");

    // Create npm prefix directory if it doesn't exist
    if !npm_prefix.exists() {
        fs::create_dir_all(&npm_prefix).map_err(|e| {
            format!("Failed to create npm prefix directory {:?}: {}", npm_prefix, e)
        })?;
    }

    log::info!("npm prefix set to: {:?}", npm_prefix);

    // Install @anthropic-ai/claude-code globally
    log::info!("Running: npm install -g @anthropic-ai/claude-code");
    let mut install_cmd = Command::new(&npm_path);

    if let Some(cli) = &npm_cli {
        install_cmd.arg(cli);
    }

    install_cmd
        .args(["install", "-g", "@anthropic-ai/claude-code"])
        .env("PATH", npm_child_path(&nodejs_dir))
        .env("NPM_CONFIG_PREFIX", npm_prefix.to_string_lossy().to_string());

    #[cfg(target_os = "windows")]
    install_cmd.creation_flags(CREATE_NO_WINDOW);

    let output = install_cmd.output().map_err(|e| {
        let err = format!("Failed to run npm install: {}", e);
        log::error!("{}", err);
        err
    })?;

    if output.status.success() {
        log::info!("Claude Code installed successfully");

        // Add nodejs, npm global bin, and git to user PATH if missing
        #[cfg(target_os = "windows")]
        {
            use crate::git::get_git_dir;

            let mut paths_to_add = Vec::new();

            // nodejs path: ~/.MoontonClaudeDesktop/nodejs
            let nodejs_path = nodejs_dir.to_string_lossy().to_string();
            if !is_path_in_user_env(&nodejs_path) {
                paths_to_add.push(nodejs_path);
            }

            // npm/claude path: %APPDATA%\npm
            let npm_bin_path = npm_prefix.to_string_lossy().to_string();
            if !is_path_in_user_env(&npm_bin_path) {
                paths_to_add.push(npm_bin_path);
            }

            // git path: ~/.MoontonClaudeDesktop/git/cmd
            if let Ok(git_dir) = get_git_dir() {
                let git_cmd_path = git_dir.join("cmd").to_string_lossy().to_string();
                if !is_path_in_user_env(&git_cmd_path) {
                    paths_to_add.push(git_cmd_path);
                }
            }

            if !paths_to_add.is_empty() {
                if let Err(e) = add_missing_paths_to_user_env(&paths_to_add) {
                    log::warn!("Failed to add to user PATH: {}", e);
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            // On macOS, write paths to shell profile instead of registry.
            // npm-global/bin is prepended (priority); bundled nodejs is appended
            // (fallback) so any system nodejs takes precedence.
            if let Err(e) = add_to_shell_profile(&nodejs_dir, &npm_prefix) {
                log::warn!("Failed to update shell profile: {}", e);
            }
        }

        Ok("Claude Code installed successfully".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let err = format!("Claude Code installation failed: stdout={} stderr={}", stdout, stderr);
        log::error!("{}", err);
        Err(err)
    }
}

// Tauri commands

#[tauri::command]
pub fn check_nodejs_status() -> NodejsStatus {
    // First, check for system-installed Node.js
    if let Some((version, path)) = check_system_nodejs() {
        return NodejsStatus {
            installed: true,
            version: Some(version),
            path: Some(path),
            source: Some("system".to_string()),
        };
    }

    // Fallback: check bundled Node.js in AppData
    let installed = is_nodejs_installed();
    let version = if installed { get_nodejs_version() } else { None };
    let path = if installed {
        get_nodejs_dir().ok().map(|p| p.to_string_lossy().to_string())
    } else {
        None
    };

    NodejsStatus {
        installed,
        version,
        path,
        source: if installed { Some("bundled".to_string()) } else { None },
    }
}

#[tauri::command]
pub fn extract_nodejs_cmd(app_handle: AppHandle) -> Result<(), String> {
    extract_nodejs(&app_handle)
}

/// Quick check if Claude Code is installed (instant, no process spawn)
/// Use this for startup checks to avoid lag
#[tauri::command]
pub fn check_claude_code_installed_quick() -> bool {
    is_claude_code_installed_quick()
}

/// Full Claude Code status check with version (may be slower)
/// Use this when you need version information
#[tauri::command]
pub fn check_claude_code_status() -> ClaudeCodeStatus {
    get_claude_code_status()
}

#[tauri::command]
pub fn install_claude_code_cmd<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
) -> Result<String, String> {
    install_claude_code()?;

    // Also install mcd CLI (don't fail if this doesn't work)
    if let Err(e) = install_mcd_cli(&app_handle) {
        log::warn!("Failed to install mcd CLI: {}", e);
        // Don't block the main flow - mcd is optional
    }

    Ok("Claude Code installed successfully".to_string())
}

/// Install mcd CLI standalone command
#[tauri::command]
pub fn install_mcd_cli_cmd<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
) -> Result<String, String> {
    install_mcd_cli(&app_handle)
}

/// Get the nodejs bin path for terminal PATH injection
/// Prefers system Node.js if available, falls back to bundled
pub fn get_nodejs_bin_path() -> Option<String> {
    // First, check for system Node.js
    if let Some((_, path)) = check_system_nodejs() {
        log::info!("Using system Node.js bin path: {}", path);
        return Some(path);
    }

    // Fallback to bundled Node.js
    get_nodejs_dir().ok().and_then(|dir| {
        #[cfg(target_os = "windows")]
        let node_binary = dir.join("node.exe");

        #[cfg(target_os = "macos")]
        let node_binary = dir.join("node");

        if node_binary.exists() {
            log::info!("Using bundled Node.js bin path: {:?}", dir);
            Some(dir.to_string_lossy().to_string())
        } else {
            None
        }
    })
}

/// Get the full path to the node executable (cross-platform)
/// Prefers system Node.js, falls back to bundled
/// This is used by NodePtyManager, LlmRouter, CoworksSidecar, etc.
pub fn get_nodejs_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let node_name = "node.exe";
    #[cfg(not(target_os = "windows"))]
    let node_name = "node";

    // First, check for system Node.js (preferred)
    if let Some((_, bin_dir)) = check_system_nodejs() {
        let node_exe = PathBuf::from(&bin_dir).join(node_name);
        if node_exe.exists() {
            log::info!("Using system Node.js: {:?}", node_exe);
            return Some(node_exe);
        }
    }

    // Fallback to bundled nodejs in AppData
    if let Ok(dir) = get_nodejs_dir() {
        let node_exe = dir.join(node_name);
        if node_exe.exists() {
            log::info!("Using bundled Node.js: {:?}", node_exe);
            return Some(node_exe);
        }
    }

    // Try production path (platform-aware resource path)
    if let Some(resources_dir) = get_bundled_resources_dir() {
        #[cfg(target_os = "windows")]
        let prod_node = resources_dir.join("nodejs-windows").join(node_name);
        #[cfg(target_os = "macos")]
        let prod_node = resources_dir.join("nodejs-macos").join(node_name);
        #[cfg(target_os = "linux")]
        let prod_node = resources_dir.join("nodejs-linux").join(node_name);

        if prod_node.exists() {
            log::info!("Using production Node.js: {:?}", prod_node);
            return Some(prod_node);
        }
    }

    log::warn!("Node.js not found in any location");
    None
}

/// Get the npm global bin path for terminal PATH injection
/// This is where npm installs global packages (e.g., claude-code)
pub fn get_npm_global_bin_path() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        // Windows: %APPDATA%\npm
        dirs::data_dir().map(|dir| dir.join("npm").to_string_lossy().to_string())
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: ~/.npm-global/bin
        dirs::home_dir().map(|dir| dir.join(".npm-global").join("bin").to_string_lossy().to_string())
    }
}

/// Check if a path exists in user PATH environment variable (case-insensitive)
#[cfg(target_os = "windows")]
fn is_path_in_user_env(path: &str) -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(env) = hkcu.open_subkey("Environment") {
        if let Ok(current_path) = env.get_value::<String, _>("Path") {
            return current_path.split(';')
                .any(|p| p.eq_ignore_ascii_case(path));
        }
    }
    false
}

/// Add paths to user PATH environment variable if not already present
#[cfg(target_os = "windows")]
fn add_missing_paths_to_user_env(paths: &[String]) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .map_err(|e| format!("Failed to open registry: {}", e))?;

    let current_path: String = env.get_value("Path").unwrap_or_default();

    // Log original PATH before modification for troubleshooting and recovery
    log::info!("Original user PATH before modification: {}", current_path);

    let mut path_parts: Vec<String> = current_path.split(';')
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let mut added = false;
    for path in paths {
        if !path_parts.iter().any(|p| p.eq_ignore_ascii_case(path)) {
            path_parts.push(path.clone());
            log::info!("Adding to user PATH: {}", path);
            added = true;
        }
    }

    if added {
        let new_path = path_parts.join(";");
        env.set_value("Path", &new_path)
            .map_err(|e| format!("Failed to set PATH: {}", e))?;
        log::info!("User PATH updated successfully");
    }

    Ok(())
}

/// Add Node.js / npm paths to shell profile on macOS (updates ~/.zshrc or ~/.bash_profile).
///
/// `nodejs_dir` is appended to PATH (not prepended), so that any system nodejs the
/// user installs later — whether to system PATH or via brew/nvm at any time — is
/// resolved first.  Bundled nodejs is only used as a last resort, which preserves
/// the `claude` shebang (`#!/usr/bin/env node`) for users without system nodejs.
///
/// `npm_prefix/bin` is prepended (highest priority) so the npm-installed `claude`
/// command is always found before any system one.
#[cfg(target_os = "macos")]
fn add_to_shell_profile(nodejs_dir: &PathBuf, npm_prefix: &PathBuf) -> Result<(), String> {
    use std::io::Write;

    let home = dirs::home_dir().ok_or("Cannot get home directory")?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());

    let profile_file = if shell.contains("zsh") {
        home.join(".zshrc")
    } else if shell.contains("bash") {
        home.join(".bash_profile")
    } else {
        home.join(".zshrc")
    };

    let nodejs_dir_str = nodejs_dir.to_string_lossy().to_string();
    let npm_bin_str = npm_prefix.join("bin").to_string_lossy().to_string();
    let npm_prefix_str = npm_prefix.to_string_lossy().to_string();

    let mut needs_nodejs = true;
    let mut needs_npm = true;
    let mut needs_npm_prefix = true;

    if profile_file.exists() {
        if let Ok(content) = fs::read_to_string(&profile_file) {
            log::info!(
                "Original shell profile ({}) before modification:\n{}",
                profile_file.display(),
                content
            );

            if content.contains(&nodejs_dir_str) {
                needs_nodejs = false;
            }
            if content.contains(&npm_bin_str) {
                needs_npm = false;
            }
            // Match the full assignment, not bare substring, to avoid false positives
            // from comments or `unset NPM_CONFIG_PREFIX` lines.
            if content.contains("export NPM_CONFIG_PREFIX=") {
                needs_npm_prefix = false;
            }
        }
    }

    if needs_nodejs || needs_npm || needs_npm_prefix {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&profile_file)
            .map_err(|e| format!("Failed to open shell profile: {}", e))?;

        writeln!(file, "\n# Moonton Claude Desktop - Node.js and npm paths")
            .map_err(|e| format!("Failed to write to profile: {}", e))?;

        // Prepend npm global bin so `claude` / `mcd` always come from npm-global.
        if needs_npm {
            writeln!(file, "export PATH=\"{}:$PATH\"", npm_bin_str)
                .map_err(|e| format!("Failed to write npm path: {}", e))?;
            log::info!("Added npm global bin path (prepend) to {:?}", profile_file);
        }

        // Pin npm global prefix so manual `npm install -g` always lands in the same
        // directory that PATH already contains.  Without this, the bundled nodejs
        // (whose binary lives at nodejs/node rather than nodejs/bin/node) computes
        // its default prefix as ~/.MoontonClaudeDesktop, installing binaries to
        // ~/.MoontonClaudeDesktop/bin which is not in PATH.
        if needs_npm_prefix {
            writeln!(file, "export NPM_CONFIG_PREFIX=\"{}\"", npm_prefix_str)
                .map_err(|e| format!("Failed to write NPM_CONFIG_PREFIX: {}", e))?;
            log::info!("Added NPM_CONFIG_PREFIX={} to {:?}", npm_prefix_str, profile_file);
        }

        // Append bundled nodejs (lowest priority) so any system nodejs the user
        // already has — or installs later — is preferred.  Bundled nodejs is the
        // fallback for the `claude` shebang on systems without any other nodejs.
        if needs_nodejs {
            writeln!(file, "export PATH=\"$PATH:{}\"", nodejs_dir_str)
                .map_err(|e| format!("Failed to write nodejs path: {}", e))?;
            log::info!("Added Node.js path (append) to {:?}", profile_file);
        }
    }

    Ok(())
}
