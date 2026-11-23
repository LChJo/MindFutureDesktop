use crate::nodejs::{get_bundled_resources_dir, get_nodejs_path};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tauri::AppHandle;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Strip Windows extended-length path prefix (\\?\) for Node.js compatibility
#[cfg(target_os = "windows")]
fn strip_unc_prefix(path: &PathBuf) -> PathBuf {
    let path_str = path.to_string_lossy();
    if path_str.starts_with(r"\\?\") {
        PathBuf::from(&path_str[4..])
    } else {
        path.clone()
    }
}

#[cfg(not(target_os = "windows"))]
fn strip_unc_prefix(path: &PathBuf) -> PathBuf {
    path.clone()
}

/// LLM Router service status
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LlmRouterStatus {
    pub running: bool,
    pub port: Option<u16>,
    pub error: Option<String>,
}

/// Get the embedded CCR cli.cjs path
/// Development: src-tauri/resources/ccr/cli.cjs
/// Production (Windows): {exe_dir}/resources/ccr/cli.cjs
/// Production (macOS): {exe_dir}/../Resources/resources/ccr/cli.cjs
fn get_embedded_ccr_path<R: tauri::Runtime>(_app_handle: &AppHandle<R>) -> Option<PathBuf> {
    // Priority 1: Production path using platform-aware resource resolution
    if let Some(resources_dir) = get_bundled_resources_dir() {
        let prod_path = resources_dir.join("ccr").join("cli.cjs");
        if prod_path.exists() {
            log::info!("Found CCR at bundled resources: {:?}", prod_path);
            return Some(prod_path);
        }
    }

    // Priority 2: Development fallback - relative to src-tauri
    let dev_path = PathBuf::from("resources").join("ccr").join("cli.cjs");
    if dev_path.exists() {
        log::info!("Found CCR at dev path: {:?}", dev_path);
        return Some(dev_path);
    }

    log::warn!("CCR cli.cjs not found in any location");
    None
}

/// Get the CCR config directory path (~/.MoontonClaudeDesktop/ccr-config)
fn get_ccr_config_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".MoontonClaudeDesktop").join("ccr-config"))
}

/// Initialize CCR config and fill in missing details for configured providers.
/// Also fills in missing provider names and api_key from auth_token parameter
/// thinking_level: Controls the reasoning effort for GPT-5.2 (none/low/medium/high)
fn ensure_ccr_config(auth_token: Option<String>, thinking_level: Option<String>) -> Result<(), String> {
    let config_dir =
        get_ccr_config_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;
    let config_path = config_dir.join("config.json");

    // Create directory if it doesn't exist
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create CCR config dir: {}", e))?;
        log::info!("Created CCR config directory: {:?}", config_dir);
    }

    // Use provided auth_token, fallback to environment variable
    let auth_token = auth_token.or_else(|| std::env::var("ANTHROPIC_AUTH_TOKEN").ok());

    // Check if config exists
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read CCR config: {}", e))?;

        if let Ok(mut config) = serde_json::from_str::<serde_json::Value>(&content) {
            let mut config_modified = false;

            // Check and fix Providers array
            if let Some(providers) = config.get_mut("Providers").and_then(|p| p.as_array_mut()) {
                for (index, provider) in providers.iter_mut().enumerate() {
                    if let Some(provider_obj) = provider.as_object_mut() {
                        // Check if "name" field is missing or empty
                        let needs_name = match provider_obj.get("name") {
                            None => true,
                            Some(v) => v.as_str().map(|s| s.is_empty()).unwrap_or(true),
                        };

                        if needs_name {
                            // Generate a default name based on index
                            let default_name = format!("provider_{}", index + 1);
                            provider_obj.insert(
                                "name".to_string(),
                                serde_json::Value::String(default_name.clone()),
                            );
                            log::info!("Added missing provider name: {}", default_name);
                            config_modified = true;
                        }

                        // Check if "api_key" field is missing or empty
                        let needs_api_key = match provider_obj.get("api_key") {
                            None => true,
                            Some(v) => v.as_str().map(|s| s.is_empty()).unwrap_or(true),
                        };

                        if needs_api_key {
                            if let Some(ref token) = auth_token {
                                provider_obj.insert(
                                    "api_key".to_string(),
                                    serde_json::Value::String(token.clone()),
                                );
                                log::info!(
                                    "Filled api_key from ANTHROPIC_AUTH_TOKEN for provider: {}",
                                    provider_obj
                                        .get("name")
                                        .and_then(|n| n.as_str())
                                        .unwrap_or("unknown")
                                );
                                config_modified = true;
                            }
                        }

                        // Update transformer with reasoning effort if thinking_level is provided
                        if let Some(ref level) = thinking_level {
                            let new_transformer = serde_json::json!({
                                "use": [
                                    "stripreasoning",
                                    ["reasoning", { "effort": level }]
                                ]
                            });
                            provider_obj.insert("transformer".to_string(), new_transformer);
                            log::info!(
                                "Updated transformer with reasoning effort: {} for provider: {}",
                                level,
                                provider_obj
                                    .get("name")
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("unknown")
                            );
                            config_modified = true;
                        }
                    }
                }
            }

            // Save modified config if changes were made
            if config_modified {
                fs::write(
                    &config_path,
                    serde_json::to_string_pretty(&config).unwrap(),
                )
                .map_err(|e| format!("Failed to write CCR config: {}", e))?;
                log::info!("Updated CCR config with missing fields");
            }

            return Ok(());
        }
    }

    // Start with no providers until the user configures one in the CCR dashboard.
    let default_config = serde_json::json!({
        "PORT": 3456,
        "HOST": "127.0.0.1",
        "LOG": false,
        "LOG_LEVEL": "debug",
        "API_TIMEOUT_MS": "600000",
        "Router": {
            "longContextThreshold": 60000
        },
        "Providers": []
    });

    fs::write(
        &config_path,
        serde_json::to_string_pretty(&default_config).unwrap(),
    )
    .map_err(|e| format!("Failed to write CCR config: {}", e))?;

    log::info!("Created initial CCR config at {:?}", config_path);
    Ok(())
}

/// Check if the LLM Router service is running by checking port 3456
#[tauri::command]
pub fn check_llm_router_status() -> LlmRouterStatus {
    log::info!("Checking LLM Router status...");

    use std::net::TcpStream;
    use std::time::Duration;

    let port = 3456u16;
    let address = format!("127.0.0.1:{}", port);

    match TcpStream::connect_timeout(&address.parse().unwrap(), Duration::from_millis(500)) {
        Ok(_) => {
            log::info!("LLM Router is running on port {}", port);
            LlmRouterStatus {
                running: true,
                port: Some(port),
                error: None,
            }
        }
        Err(_) => {
            log::info!("LLM Router is not running (port {} not responding)", port);
            LlmRouterStatus {
                running: false,
                port: None,
                error: None,
            }
        }
    }
}

/// Start the LLM Router service using embedded CCR
#[tauri::command]
pub async fn start_llm_router(
    app_handle: AppHandle,
    auth_token: Option<String>,
    thinking_level: Option<String>,
) -> Result<(), String> {
    log::info!("Starting LLM Router service with embedded CCR...");
    log::info!("Auth token provided: {}", auth_token.is_some());
    log::info!("Thinking level: {:?}", thinking_level);

    let app_handle_clone = app_handle.clone();

    // Run in blocking thread since it's a process operation
    tauri::async_runtime::spawn_blocking(move || {
        // Ensure CCR config exists before starting.
        if let Err(e) = ensure_ccr_config(auth_token, thinking_level) {
            log::warn!("Failed to ensure CCR config: {}", e);
            // Continue anyway, user might have their own config
        }

        // Get Node.js path
        let node_path = get_nodejs_path().ok_or_else(|| {
            "Node.js not found. Please ensure Node.js is installed.".to_string()
        })?;

        // Get embedded CCR path
        let ccr_path_raw = get_embedded_ccr_path(&app_handle_clone).ok_or_else(|| {
            "Embedded CCR not found. This may be a build issue.".to_string()
        })?;

        // Strip UNC prefix for Node.js compatibility on Windows
        let ccr_path = strip_unc_prefix(&ccr_path_raw);

        // Get CCR directory for working directory (needed for WASM file)
        let ccr_dir = ccr_path.parent().ok_or_else(|| {
            "Failed to get CCR directory".to_string()
        })?;

        log::info!(
            "Starting CCR with Node.js: {:?}, CCR: {:?}, Working dir: {:?}",
            node_path,
            ccr_path,
            ccr_dir
        );

        // Use spawn() instead of output() because CCR start runs a server in the foreground
        // We need to start it as a background process
        #[cfg(target_os = "windows")]
        {
            use std::process::Stdio;

            let mut cmd = Command::new(&node_path);
            cmd.arg(&ccr_path);
            cmd.arg("start");
            cmd.current_dir(ccr_dir);
            cmd.creation_flags(CREATE_NO_WINDOW);
            cmd.stdout(Stdio::null());
            cmd.stderr(Stdio::null());
            cmd.stdin(Stdio::null());

            match cmd.spawn() {
                Ok(_child) => {
                    log::info!("LLM Router process spawned successfully");
                    Ok(())
                }
                Err(e) => {
                    let error = format!("Failed to spawn CCR start: {}", e);
                    log::error!("{}", error);
                    Err(error)
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            use std::process::Stdio;

            let mut cmd = Command::new(&node_path);
            cmd.arg(&ccr_path);
            cmd.arg("start");
            cmd.current_dir(ccr_dir);
            cmd.stdout(Stdio::null());
            cmd.stderr(Stdio::null());
            cmd.stdin(Stdio::null());

            match cmd.spawn() {
                Ok(_child) => {
                    log::info!("LLM Router process spawned successfully");
                    Ok(())
                }
                Err(e) => {
                    let error = format!("Failed to spawn CCR start: {}", e);
                    log::error!("{}", error);
                    Err(error)
                }
            }
        }
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Get the CCR bin directory path for terminal PATH injection
/// Returns the directory containing ccr.cmd (Windows) or ccr (macOS)
pub fn get_ccr_bin_path() -> Option<String> {
    // Try production path using platform-aware resource resolution
    if let Some(resources_dir) = get_bundled_resources_dir() {
        let ccr_dir = resources_dir.join("ccr");

        #[cfg(target_os = "windows")]
        let ccr_cmd = ccr_dir.join("ccr.cmd");

        #[cfg(not(target_os = "windows"))]
        let ccr_cmd = ccr_dir.join("ccr");

        if ccr_cmd.exists() {
            let path_str = ccr_dir.to_string_lossy().to_string();
            log::info!("Found CCR bin at bundled resources: {}", path_str);
            return Some(path_str);
        }
    }

    // Try development path relative to src-tauri
    let dev_ccr_dir = std::path::PathBuf::from("resources").join("ccr");

    #[cfg(target_os = "windows")]
    let dev_ccr_cmd = dev_ccr_dir.join("ccr.cmd");

    #[cfg(not(target_os = "windows"))]
    let dev_ccr_cmd = dev_ccr_dir.join("ccr");

    if dev_ccr_cmd.exists() {
        // Get absolute path for development
        if let Ok(abs_path) = std::fs::canonicalize(&dev_ccr_dir) {
            let path_str = abs_path.to_string_lossy().to_string();
            // Strip UNC prefix on Windows
            #[cfg(target_os = "windows")]
            let path_str = if path_str.starts_with(r"\\?\") {
                path_str[4..].to_string()
            } else {
                path_str
            };

            log::info!("Found CCR bin at dev path: {}", path_str);
            return Some(path_str);
        }
    }

    log::warn!("CCR bin directory not found");
    None
}

/// Get the mcd-cli bin directory path for terminal PATH injection
/// Returns the directory containing mcd.cmd (Windows) or mcd (macOS)
pub fn get_mcd_bin_path() -> Option<String> {
    // Try production path using platform-aware resource resolution
    if let Some(resources_dir) = get_bundled_resources_dir() {
        let mcd_dir = resources_dir.join("mcd-cli");

        #[cfg(target_os = "windows")]
        let mcd_cmd = mcd_dir.join("mcd.cmd");

        #[cfg(not(target_os = "windows"))]
        let mcd_cmd = mcd_dir.join("mcd");

        if mcd_cmd.exists() {
            let path_str = mcd_dir.to_string_lossy().to_string();
            log::info!("Found mcd bin at bundled resources: {}", path_str);
            return Some(path_str);
        }
    }

    // Try development path relative to src-tauri
    let dev_mcd_dir = std::path::PathBuf::from("resources").join("mcd-cli");

    #[cfg(target_os = "windows")]
    let dev_mcd_cmd = dev_mcd_dir.join("mcd.cmd");

    #[cfg(not(target_os = "windows"))]
    let dev_mcd_cmd = dev_mcd_dir.join("mcd");

    if dev_mcd_cmd.exists() {
        // Get absolute path for development
        if let Ok(abs_path) = std::fs::canonicalize(&dev_mcd_dir) {
            let path_str = abs_path.to_string_lossy().to_string();
            // Strip UNC prefix on Windows
            #[cfg(target_os = "windows")]
            let path_str = if path_str.starts_with(r"\\?\") {
                path_str[4..].to_string()
            } else {
                path_str
            };

            log::info!("Found mcd bin at dev path: {}", path_str);
            return Some(path_str);
        }
    }

    log::warn!("mcd bin directory not found");
    None
}

/// Get CCR debug info for troubleshooting
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CcrDebugInfo {
    pub node_path: Option<String>,
    pub ccr_path: Option<String>,
    pub ccr_bin_path: Option<String>,
    pub config_path: Option<String>,
    pub config_content: Option<String>,
    pub anthropic_model_env: Option<String>,
    pub anthropic_base_url_env: Option<String>,
}

#[tauri::command]
pub fn get_ccr_debug_info(app_handle: AppHandle) -> CcrDebugInfo {
    log::info!("Getting CCR debug info...");

    let node_path = get_nodejs_path().map(|p| p.to_string_lossy().to_string());
    let ccr_path = get_embedded_ccr_path(&app_handle).map(|p| p.to_string_lossy().to_string());
    let ccr_bin_path = get_ccr_bin_path();

    let config_dir = get_ccr_config_dir();
    let config_path = config_dir.as_ref().map(|d| d.join("config.json"));

    let config_content = config_path.as_ref().and_then(|p| {
        if p.exists() {
            fs::read_to_string(p).ok()
        } else {
            Some(format!("Config file not found at: {:?}", p))
        }
    });

    let config_path_str = config_path.map(|p| p.to_string_lossy().to_string());

    // Get environment variables
    let anthropic_model_env = std::env::var("ANTHROPIC_MODEL").ok();
    let anthropic_base_url_env = std::env::var("ANTHROPIC_BASE_URL").ok();

    let debug_info = CcrDebugInfo {
        node_path,
        ccr_path,
        ccr_bin_path,
        config_path: config_path_str,
        config_content,
        anthropic_model_env,
        anthropic_base_url_env,
    };

    log::info!("CCR Debug Info: {:?}", debug_info);
    debug_info
}

/// Stop the LLM Router service using embedded CCR
#[tauri::command]
pub async fn stop_llm_router(app_handle: AppHandle) -> Result<(), String> {
    log::info!("Stopping LLM Router service with embedded CCR...");

    let app_handle_clone = app_handle.clone();

    // Run in blocking thread since it's a process operation
    tauri::async_runtime::spawn_blocking(move || {
        // Get Node.js path
        let node_path = get_nodejs_path().ok_or_else(|| {
            "Node.js not found. Please ensure Node.js is installed.".to_string()
        })?;

        // Get embedded CCR path
        let ccr_path_raw = get_embedded_ccr_path(&app_handle_clone).ok_or_else(|| {
            "Embedded CCR not found. This may be a build issue.".to_string()
        })?;

        // Strip UNC prefix for Node.js compatibility on Windows
        let ccr_path = strip_unc_prefix(&ccr_path_raw);

        // Get CCR directory for working directory
        let ccr_dir = ccr_path.parent().ok_or_else(|| {
            "Failed to get CCR directory".to_string()
        })?;

        log::info!(
            "Stopping CCR with Node.js: {:?}, CCR: {:?}",
            node_path,
            ccr_path
        );

        #[cfg(target_os = "windows")]
        {
            let mut cmd = Command::new(&node_path);
            cmd.arg(&ccr_path);
            cmd.arg("stop");
            cmd.current_dir(ccr_dir);
            cmd.creation_flags(CREATE_NO_WINDOW);

            match cmd.output() {
                Ok(output) => {
                    if output.status.success() {
                        log::info!("LLM Router stopped successfully");
                        Ok(())
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let error = format!(
                            "Failed to stop LLM Router: {} {}",
                            stderr, stdout
                        );
                        log::error!("{}", error);
                        Err(error)
                    }
                }
                Err(e) => {
                    let error = format!("Failed to run CCR stop: {}", e);
                    log::error!("{}", error);
                    Err(error)
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let mut cmd = Command::new(&node_path);
            cmd.arg(&ccr_path);
            cmd.arg("stop");
            cmd.current_dir(ccr_dir);

            match cmd.output() {
                Ok(output) => {
                    if output.status.success() {
                        log::info!("LLM Router stopped successfully");
                        Ok(())
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let error = format!(
                            "Failed to stop LLM Router: {} {}",
                            stderr, stdout
                        );
                        log::error!("{}", error);
                        Err(error)
                    }
                }
                Err(e) => {
                    let error = format!("Failed to run CCR stop: {}", e);
                    log::error!("{}", error);
                    Err(error)
                }
            }
        }
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Restart the LLM Router service
/// When skip_config_update is true, preserves user's config changes (e.g. after CCR web UI save)
/// When false, applies auth_token and thinking_level from settings store
#[tauri::command]
pub async fn restart_llm_router(
    app_handle: AppHandle,
    auth_token: Option<String>,
    thinking_level: Option<String>,
    skip_config_update: Option<bool>,
) -> Result<(), String> {
    log::info!(
        "Restarting LLM Router service (skip_config_update: {:?})...",
        skip_config_update
    );

    let app_handle_clone = app_handle.clone();

    tauri::async_runtime::spawn_blocking(move || {
        // Resolve paths
        let node_path = get_nodejs_path().ok_or_else(|| {
            "Node.js not found. Please ensure Node.js is installed.".to_string()
        })?;

        let ccr_path_raw = get_embedded_ccr_path(&app_handle_clone).ok_or_else(|| {
            "Embedded CCR not found. This may be a build issue.".to_string()
        })?;

        let ccr_path = strip_unc_prefix(&ccr_path_raw);

        let ccr_dir = ccr_path.parent().ok_or_else(|| {
            "Failed to get CCR directory".to_string()
        })?;

        // Step 1: Stop (best-effort — process may already be dead from /api/restart)
        log::info!("Restart: stopping CCR...");
        {
            #[cfg(target_os = "windows")]
            let mut cmd = {
                let mut c = Command::new(&node_path);
                c.arg(&ccr_path).arg("stop").current_dir(ccr_dir);
                c.creation_flags(CREATE_NO_WINDOW);
                c
            };

            #[cfg(not(target_os = "windows"))]
            let mut cmd = {
                let mut c = Command::new(&node_path);
                c.arg(&ccr_path).arg("stop").current_dir(ccr_dir);
                c
            };

            match cmd.output() {
                Ok(output) => {
                    log::info!(
                        "Restart: stop result: exit={}, stdout={}, stderr={}",
                        output.status,
                        String::from_utf8_lossy(&output.stdout).trim(),
                        String::from_utf8_lossy(&output.stderr).trim()
                    );
                }
                Err(e) => {
                    log::warn!("Restart: stop command failed (process may already be dead): {}", e);
                }
            }
        }

        // Step 2: Sleep to let port 3456 release
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Step 3: Optionally update config
        if skip_config_update != Some(true) {
            if let Err(e) = ensure_ccr_config(auth_token, thinking_level) {
                log::warn!("Restart: failed to update CCR config: {}", e);
            }
        }

        // Step 4: Start
        log::info!("Restart: starting CCR...");
        {
            use std::process::Stdio;

            #[cfg(target_os = "windows")]
            {
                let mut cmd = Command::new(&node_path);
                cmd.arg(&ccr_path).arg("start").current_dir(ccr_dir);
                cmd.creation_flags(CREATE_NO_WINDOW);
                cmd.stdout(Stdio::null());
                cmd.stderr(Stdio::null());
                cmd.stdin(Stdio::null());

                cmd.spawn()
                    .map_err(|e| format!("Failed to spawn CCR start: {}", e))?;
            }

            #[cfg(not(target_os = "windows"))]
            {
                let mut cmd = Command::new(&node_path);
                cmd.arg(&ccr_path).arg("start").current_dir(ccr_dir);
                cmd.stdout(Stdio::null());
                cmd.stderr(Stdio::null());
                cmd.stdin(Stdio::null());

                cmd.spawn()
                    .map_err(|e| format!("Failed to spawn CCR start: {}", e))?;
            }
        }

        log::info!("Restart: CCR process spawned successfully");
        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}
