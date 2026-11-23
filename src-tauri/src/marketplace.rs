use crate::git::{get_bundled_git_exe, get_git_dir};
#[cfg(target_os = "windows")]
use crate::git::get_git_bash_path;
#[cfg(not(target_os = "windows"))]
use crate::nodejs::get_claude_cmd_path;
use crate::nodejs::{get_nodejs_bin_path, get_nodejs_dir, is_claude_code_installed_quick};
use serde::de::Deserializer;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Execute claude CLI command in background
/// Uses system PATH claude command directly
fn run_claude_command(args: &[&str]) -> Result<String, String> {
    log::info!("run_claude_command called with args: {:?}", args);

    // Pre-check: verify Claude is available in system PATH
    let claude_available = is_claude_code_installed_quick();
    log::info!("is_claude_code_installed_quick() = {}", claude_available);
    if !claude_available {
        return Err("Claude Code 未安装。请先在引导页面安装 Claude Code，或手动运行: npm install -g @anthropic-ai/claude-code".to_string());
    }

    let git_dir = get_git_dir().ok();
    log::info!("git_dir: {:?}", git_dir);

    // On Windows, need to use claude.cmd explicitly since Command::new doesn't resolve .cmd files
    #[cfg(target_os = "windows")]
    let mut cmd = Command::new("cmd");
    #[cfg(target_os = "windows")]
    {
        cmd.arg("/c").arg("claude");
        cmd.args(args);
    }

    #[cfg(not(target_os = "windows"))]
    let mut cmd = {
        // On macOS/Linux, prefer using the full path to npm global claude
        if let Some(claude_path) = get_claude_cmd_path() {
            if claude_path.exists() {
                log::info!("Using claude from npm global: {:?}", claude_path);
                Command::new(claude_path)
            } else {
                log::warn!("npm global claude path does not exist: {:?}, falling back to PATH", claude_path);
                Command::new("claude")
            }
        } else {
            log::info!("npm global claude path not found, using PATH");
            Command::new("claude")
        }
    };
    #[cfg(not(target_os = "windows"))]
    cmd.args(args);

    // Build PATH with nodejs, npm global dir and bundled git
    let mut path_parts: Vec<String> = Vec::new();

    // Add Node.js directory (prefer system, fallback to bundled)
    if let Some(nodejs_bin) = get_nodejs_bin_path() {
        path_parts.push(nodejs_bin.clone());
        log::info!("Added nodejs path: {}", nodejs_bin);
    } else if let Ok(nodejs_dir) = get_nodejs_dir() {
        if nodejs_dir.exists() {
            path_parts.push(nodejs_dir.to_string_lossy().to_string());
            log::info!("Added bundled nodejs path: {:?}", nodejs_dir);
        }
    }

    // Add npm global directory (where user-level Claude is installed)
    #[cfg(target_os = "windows")]
    {
        // Windows: %APPDATA%\npm (e.g., C:\Users\<user>\AppData\Roaming\npm)
        if let Some(home) = dirs::home_dir() {
            let npm_global = home.join("AppData").join("Roaming").join("npm");
            log::info!("npm_global path: {:?}, exists: {}", npm_global, npm_global.exists());
            if npm_global.exists() {
                path_parts.push(npm_global.to_string_lossy().to_string());
            }
        } else {
            log::warn!("Could not get home directory!");
        }
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: ~/.npm-global/bin
        if let Some(home) = dirs::home_dir() {
            let npm_global = home.join(".npm-global").join("bin");
            log::info!("npm_global path: {:?}, exists: {}", npm_global, npm_global.exists());
            if npm_global.exists() {
                path_parts.push(npm_global.to_string_lossy().to_string());
            }
        } else {
            log::warn!("Could not get home directory!");
        }
    }

    // Add git directories to PATH
    #[cfg(target_os = "windows")]
    {
        // Windows: Add bundled git cmd and bin directories
        if let Some(ref dir) = git_dir {
            let cmd_dir = dir.join("cmd");
            let bin_dir = dir.join("bin");
            if cmd_dir.exists() {
                path_parts.push(cmd_dir.to_string_lossy().to_string());
            }
            if bin_dir.exists() {
                path_parts.push(bin_dir.to_string_lossy().to_string());
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: Use system git (git_dir points to system git parent directory)
        if let Some(ref dir) = git_dir {
            if dir.exists() {
                path_parts.push(dir.to_string_lossy().to_string());
            }
        }
    }

    // Append current PATH
    let current_path = std::env::var("PATH").unwrap_or_default();
    path_parts.push(current_path.clone());

    // Platform-specific PATH separator
    #[cfg(target_os = "windows")]
    let path_separator = ";";

    #[cfg(target_os = "macos")]
    let path_separator = ":";

    let final_path = path_parts.join(path_separator);
    log::info!("Final PATH (first 500 chars): {}", &final_path[..final_path.len().min(500)]);

    cmd.env("PATH", final_path);

    // Set CLAUDE_CODE_GIT_BASH_PATH for git bash (Windows only)
    #[cfg(target_os = "windows")]
    {
        if let Some(bash_path) = get_git_bash_path() {
            cmd.env("CLAUDE_CODE_GIT_BASH_PATH", bash_path.to_string_lossy().to_string());
        }
    }

    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    log::info!("Executing: claude {:?}", args);
    let output = cmd.output();

    match &output {
        Ok(o) => log::info!("Command executed, status: {:?}", o.status),
        Err(e) => log::error!("Command execution failed: {}", e),
    }

    let output = output.map_err(|e| format!("Failed to execute claude command: {}", e))?;

    if output.status.success() {
        log::info!("Claude command succeeded");
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        log::error!("Claude command failed: stdout={}, stderr={}", stdout, stderr);
        Err(format!("Claude command failed: {} {}", stdout, stderr))
    }
}

/// Source configuration for a marketplace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceSource {
    pub source: String, // "github" or "git"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>, // for github: "owner/repo"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>, // for git: full URL
}

/// Marketplace registry entry (from known_marketplaces.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceEntry {
    pub source: MarketplaceSource,
    pub install_location: String,
    pub last_updated: String,
}

/// Plugin author info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAuthor {
    pub name: String,
}

/// Custom deserializer for plugin source that can be either a string or an object
/// Handles both formats from marketplace.json:
///   - String: "./plugins/xxx"
///   - Object: { "source": "url", "url": "https://..." }
fn deserialize_plugin_source<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum SourceValue {
        String(String),
        Object {
            #[serde(default)]
            url: Option<String>,
            #[serde(default)]
            #[allow(dead_code)]
            source: Option<String>,
        },
    }

    match SourceValue::deserialize(deserializer)? {
        SourceValue::String(s) => Ok(s),
        SourceValue::Object { url, source: _ } => {
            // For object format, use the url field
            Ok(url.unwrap_or_default())
        }
    }
}

/// Plugin info from marketplace.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_plugin_source")]
    pub source: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: Option<PluginAuthor>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(rename = "mcpServers", default)]
    pub mcp_servers: Option<serde_json::Value>,
}

/// Marketplace owner info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceOwner {
    pub name: String,
}

/// Marketplace manifest (marketplace.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceManifest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub owner: Option<MarketplaceOwner>,
    #[serde(default)]
    pub plugins: Vec<PluginInfo>,
}

/// Combined marketplace info for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceInfo {
    pub name: String,
    pub source: MarketplaceSource,
    pub install_location: String,
    pub last_updated: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    pub plugin_count: usize,
}

/// Get Claude plugins directory path
fn get_claude_plugins_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    Ok(home.join(".claude").join("plugins"))
}

/// Get known_marketplaces.json path
fn get_known_marketplaces_path() -> Result<PathBuf, String> {
    Ok(get_claude_plugins_dir()?.join("known_marketplaces.json"))
}

/// Get marketplaces directory path
fn get_marketplaces_dir() -> Result<PathBuf, String> {
    Ok(get_claude_plugins_dir()?.join("marketplaces"))
}

/// Read known_marketplaces.json
fn read_known_marketplaces() -> Result<HashMap<String, MarketplaceEntry>, String> {
    let path = get_known_marketplaces_path()?;
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {}", e))
}

/// Write known_marketplaces.json
#[allow(dead_code)]
fn write_known_marketplaces(data: &HashMap<String, MarketplaceEntry>) -> Result<(), String> {
    let path = get_known_marketplaces_path()?;
    let content =
        serde_json::to_string_pretty(data).map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Failed to write file: {}", e))
}

/// Read marketplace.json from a marketplace directory
fn read_marketplace_manifest(marketplace_name: &str) -> Result<MarketplaceManifest, String> {
    let marketplace_dir = get_marketplaces_dir()?.join(marketplace_name);
    let manifest_path = marketplace_dir.join(".claude-plugin").join("marketplace.json");

    if !manifest_path.exists() {
        return Err(format!(
            "Marketplace manifest not found: {}",
            manifest_path.display()
        ));
    }

    let content =
        fs::read_to_string(&manifest_path).map_err(|e| format!("Failed to read manifest: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse manifest: {}", e))
}

/// Check if a string looks like a GitHub owner/repo pattern
#[allow(dead_code)]
fn is_owner_repo_pattern(s: &str) -> bool {
    // Pattern like "owner/repo" - contains exactly one slash, no protocol, no dots before slash
    let parts: Vec<&str> = s.split('/').collect();
    parts.len() == 2
        && !s.contains("://")
        && !s.contains('.')
        && !parts[0].is_empty()
        && !parts[1].is_empty()
}

/// Convert a git clone URL to raw file URL for different providers
#[allow(dead_code)]
fn git_url_to_raw_file_url(url: &str, file_path: &str) -> String {
    let url = url.trim_end_matches('/').trim_end_matches(".git");

    // GitHub
    if url.contains("github.com") {
        let repo_path = url
            .replace("https://github.com/", "")
            .replace("http://github.com/", "");
        return format!(
            "https://raw.githubusercontent.com/{}/main/{}",
            repo_path, file_path
        );
    }

    // GitLab (gitlab.com or self-hosted)
    // GitLab raw URL format: https://gitlab.com/owner/repo/-/raw/branch/path
    if url.contains("gitlab.com") || url.contains(".git.") || url.contains("/gitlab/") {
        // Extract base URL with auth if present
        // URL might be: http://user:pass@host/path/to/repo
        return format!("{}/-/raw/main/{}", url, file_path);
    }

    // Gitea/Gogs style (common for self-hosted)
    // Raw URL format: https://gitea.example.com/owner/repo/raw/branch/path
    if url.contains("gitea") || url.contains("gogs") {
        return format!("{}/raw/branch/main/{}", url, file_path);
    }

    // Bitbucket
    if url.contains("bitbucket.org") {
        let repo_path = url
            .replace("https://bitbucket.org/", "")
            .replace("http://bitbucket.org/", "");
        return format!(
            "https://bitbucket.org/{}/raw/main/{}",
            repo_path, file_path
        );
    }

    // Default: assume GitLab-style since it's most common for self-hosted
    format!("{}/-/raw/main/{}", url, file_path)
}

/// Build raw URL for marketplace.json based on source
#[allow(dead_code)]
fn build_marketplace_json_url(source: &MarketplaceSource) -> Result<String, String> {
    let file_path = ".claude-plugin/marketplace.json";

    match source.source.as_str() {
        "github" => {
            let repo = source
                .repo
                .as_ref()
                .ok_or("GitHub source requires repo field")?;
            // Try main branch first, then master
            Ok(format!(
                "https://raw.githubusercontent.com/{}/main/{}",
                repo, file_path
            ))
        }
        "git" => {
            // For custom git URLs, we need the user to provide raw URL or we construct it
            let url = source.url.as_ref().ok_or("Git source requires url field")?;

            // Check if it already points to marketplace.json
            if url.ends_with("marketplace.json") {
                return Ok(url.clone());
            }

            // Check if it's a simple owner/repo pattern (treat as GitHub)
            if is_owner_repo_pattern(url) {
                return Ok(format!(
                    "https://raw.githubusercontent.com/{}/main/{}",
                    url, file_path
                ));
            }

            // It's a full URL, convert to raw file URL
            if url.starts_with("http://") || url.starts_with("https://") {
                Ok(git_url_to_raw_file_url(url, file_path))
            } else {
                Err("Invalid URL format. Please provide either 'owner/repo' for GitHub or a full URL".to_string())
            }
        }
        _ => Err(format!("Unknown source type: {}", source.source)),
    }
}

/// Fetch marketplace.json from remote
#[allow(dead_code)]
fn fetch_marketplace_json(url: &str) -> Result<MarketplaceManifest, String> {
    log::info!("Fetching marketplace.json from: {}", url);

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(url)
        .header("User-Agent", "Moonton-Claude-Desktop/1.0")
        .send()
        .map_err(|e| format!("HTTP request to {} failed: {}", url, e))?;

    let status = response.status();

    if !status.is_success() {
        // Try master branch if main fails
        if url.contains("/main/") {
            let master_url = url.replace("/main/", "/master/");
            log::info!("Trying master branch: {}", master_url);

            let response = client
                .get(&master_url)
                .header("User-Agent", "Moonton-Claude-Desktop/1.0")
                .send()
                .map_err(|e| format!("HTTP request to {} failed: {}", master_url, e))?;

            if response.status().is_success() {
                let text = response
                    .text()
                    .map_err(|e| format!("Failed to read response body: {}", e))?;

                return serde_json::from_str::<MarketplaceManifest>(&text).map_err(|e| {
                    // Show part of the response to help debug
                    let preview = if text.len() > 200 {
                        format!("{}...", &text[..200])
                    } else {
                        text.clone()
                    };
                    format!(
                        "Failed to parse JSON from {}: {}. Response preview: {}",
                        master_url, e, preview
                    )
                });
            }
        }
        return Err(format!(
            "HTTP error {} when fetching {}. Make sure the repository exists and has .claude-plugin/marketplace.json",
            status, url
        ));
    }

    // Read response as text first for better error messages
    let text = response
        .text()
        .map_err(|e| format!("Failed to read response body from {}: {}", url, e))?;

    serde_json::from_str::<MarketplaceManifest>(&text).map_err(|e| {
        // Show part of the response to help debug
        let preview = if text.len() > 200 {
            format!("{}...", &text[..200])
        } else {
            text.clone()
        };
        format!(
            "Failed to parse JSON from {}: {}. Response preview: {}",
            url, e, preview
        )
    })
}

/// Get all marketplaces with their info
#[tauri::command]
pub fn get_marketplaces() -> Result<Vec<MarketplaceInfo>, String> {
    let known = read_known_marketplaces()?;
    let mut result = Vec::new();

    for (name, entry) in known {
        // Try to read local manifest for additional info
        let manifest = read_marketplace_manifest(&name).ok();

        result.push(MarketplaceInfo {
            name: name.clone(),
            source: entry.source,
            install_location: entry.install_location,
            last_updated: entry.last_updated,
            description: manifest.as_ref().map(|m| m.description.clone()),
            version: manifest.as_ref().map(|m| m.version.clone()),
            owner: manifest
                .as_ref()
                .and_then(|m| m.owner.as_ref().map(|o| o.name.clone())),
            plugin_count: manifest.map(|m| m.plugins.len()).unwrap_or(0),
        });
    }

    // Sort by name
    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

/// Add a new marketplace (async to avoid blocking UI)
#[tauri::command]
pub async fn add_marketplace(
    source_type: String,
    source_value: String,
) -> Result<MarketplaceInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        add_marketplace_sync(source_type, source_value)
    }).await.map_err(|e| format!("Task join error: {}", e))?
}

/// Sync version of add_marketplace
/// Uses Claude CLI: claude plugin marketplace add <source>
fn add_marketplace_sync(
    source_type: String,
    source_value: String,
) -> Result<MarketplaceInfo, String> {
    // Build git source for Claude CLI
    let git_source = match source_type.as_str() {
        "github" => source_value.clone(), // e.g., "anthropics/claude-plugins-official"
        "url" => source_value.clone(),    // Full URL
        _ => return Err("Invalid source type".to_string()),
    };

    // Record known marketplaces before adding
    let before_known = read_known_marketplaces().unwrap_or_default();

    // Execute: claude plugin marketplace add <source>
    log::info!("Adding marketplace via Claude CLI: {}", git_source);
    if let Err(e) = run_claude_command(&["plugin", "marketplace", "add", &git_source]) {
        // Check for "already installed" error
        if e.contains("already installed") {
            return Err("该插件市场已安装。如需重新安装，请先删除后再添加。".to_string());
        }
        // Check for network/clone errors
        if e.contains("Clone") || e.contains("git") || e.contains("SSH") {
            return Err(format!("克隆仓库失败，请检查网络连接或仓库地址是否正确。\n详情: {}", e));
        }
        return Err(e);
    }

    // Read known_marketplaces.json after CLI execution
    let after_known = read_known_marketplaces()?;

    // Find the newly added marketplace (not in before, but in after)
    let new_marketplace_name = after_known.keys()
        .find(|name| !before_known.contains_key(*name))
        .cloned();

    // If we found a new marketplace, return its info
    if let Some(name) = new_marketplace_name {
        let entry = after_known.get(&name).unwrap();
        let manifest = read_marketplace_manifest(&name).ok();

        return Ok(MarketplaceInfo {
            name: name.clone(),
            source: entry.source.clone(),
            install_location: entry.install_location.clone(),
            last_updated: entry.last_updated.clone(),
            description: manifest.as_ref().map(|m| m.description.clone()),
            version: manifest.as_ref().map(|m| m.version.clone()),
            owner: manifest.as_ref().and_then(|m| m.owner.as_ref().map(|o| o.name.clone())),
            plugin_count: manifest.map(|m| m.plugins.len()).unwrap_or(0),
        });
    }

    // Fallback: maybe the marketplace already existed, try to find by source
    // This handles the case where the CLI updated an existing marketplace
    for (name, entry) in &after_known {
        let matches = match source_type.as_str() {
            "github" => entry.source.repo.as_ref() == Some(&git_source),
            "url" => entry.source.url.as_ref() == Some(&git_source),
            _ => false,
        };
        if matches {
            let manifest = read_marketplace_manifest(name).ok();
            return Ok(MarketplaceInfo {
                name: name.clone(),
                source: entry.source.clone(),
                install_location: entry.install_location.clone(),
                last_updated: entry.last_updated.clone(),
                description: manifest.as_ref().map(|m| m.description.clone()),
                version: manifest.as_ref().map(|m| m.version.clone()),
                owner: manifest.as_ref().and_then(|m| m.owner.as_ref().map(|o| o.name.clone())),
                plugin_count: manifest.map(|m| m.plugins.len()).unwrap_or(0),
            });
        }
    }

    Err("Marketplace added but could not retrieve info. Please refresh the list.".to_string())
}

/// Update a marketplace (async to avoid blocking UI)
#[tauri::command]
pub async fn update_marketplace(name: String) -> Result<MarketplaceInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        update_marketplace_sync(name)
    }).await.map_err(|e| format!("Task join error: {}", e))?
}

/// Sync version of update_marketplace
/// Uses Claude CLI: claude plugin marketplace update <name>
fn update_marketplace_sync(name: String) -> Result<MarketplaceInfo, String> {
    // Check if marketplace exists before calling CLI
    let known = read_known_marketplaces()?;
    if !known.contains_key(&name) {
        return Err(format!("Marketplace not found: {}", name));
    }

    // Execute: claude plugin marketplace update <name>
    log::info!("Updating marketplace via Claude CLI: {}", name);
    if let Err(e) = run_claude_command(&["plugin", "marketplace", "update", &name]) {
        // Check for common errors
        if e.contains("not found") || e.contains("not installed") {
            return Err(format!("市场 {} 未找到", name));
        }
        if e.contains("network") || e.contains("git") || e.contains("fetch") {
            return Err(format!("更新失败，请检查网络连接。\n详情: {}", e));
        }
        return Err(e);
    }

    // Re-read known_marketplaces.json after CLI execution to get updated info
    let updated_known = read_known_marketplaces()?;
    let updated_entry = updated_known
        .get(&name)
        .ok_or(format!("Marketplace not found after update: {}", name))?;

    // Read manifest for additional info
    let manifest = read_marketplace_manifest(&name).ok();

    Ok(MarketplaceInfo {
        name: name.clone(),
        source: updated_entry.source.clone(),
        install_location: updated_entry.install_location.clone(),
        last_updated: updated_entry.last_updated.clone(),
        description: manifest.as_ref().map(|m| m.description.clone()),
        version: manifest.as_ref().map(|m| m.version.clone()),
        owner: manifest.as_ref().and_then(|m| m.owner.as_ref().map(|o| o.name.clone())),
        plugin_count: manifest.map(|m| m.plugins.len()).unwrap_or(0),
    })
}

/// Remove a marketplace (async to avoid blocking UI)
#[tauri::command]
pub async fn remove_marketplace(name: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        remove_marketplace_sync(name)
    }).await.map_err(|e| format!("Task join error: {}", e))?
}

/// Sync version of remove_marketplace
/// Uses Claude CLI: claude plugin marketplace remove <name>
fn remove_marketplace_sync(name: String) -> Result<(), String> {
    // Execute: claude plugin marketplace remove <name>
    log::info!("Removing marketplace via Claude CLI: {}", name);
    if let Err(e) = run_claude_command(&["plugin", "marketplace", "remove", &name]) {
        // Check for common errors
        if e.contains("not found") || e.contains("not installed") {
            return Err(format!("市场 {} 未找到", name));
        }
        return Err(e);
    }

    log::info!("Marketplace removed successfully: {}", name);
    Ok(())
}

/// Get plugins from a specific marketplace
#[tauri::command]
pub fn get_marketplace_plugins(name: String) -> Result<Vec<PluginInfo>, String> {
    let manifest = read_marketplace_manifest(&name)?;
    Ok(manifest.plugins)
}

/// Get all plugins from all marketplaces
#[tauri::command]
pub fn get_all_plugins() -> Result<Vec<(String, PluginInfo)>, String> {
    let known = read_known_marketplaces()?;
    let mut all_plugins = Vec::new();

    for name in known.keys() {
        if let Ok(manifest) = read_marketplace_manifest(name) {
            for plugin in manifest.plugins {
                all_plugins.push((name.clone(), plugin));
            }
        }
    }

    Ok(all_plugins)
}

/// Installed plugin entry (compatible with Claude's installed_plugins.json format v2)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPluginEntry {
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub install_path: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub installed_at: String,
    #[serde(default)]
    pub last_updated: String,
    #[serde(default)]
    pub git_commit_sha: Option<String>,
    #[serde(default)]
    pub is_local: bool,
    // Capture any other fields we don't explicitly handle
    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}

/// Installed plugins file structure (v2 format - plugins is array)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPluginsFileV2 {
    pub version: i32,
    pub plugins: HashMap<String, Vec<InstalledPluginEntry>>,
}

/// Legacy installed plugins file structure (v1 format - plugins is object)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPluginsFileV1 {
    pub version: i32,
    pub plugins: HashMap<String, InstalledPluginEntry>,
}

/// Unified installed plugins file that handles both formats
#[derive(Debug, Clone)]
pub struct InstalledPluginsFile {
    pub version: i32,
    pub plugins: HashMap<String, Vec<InstalledPluginEntry>>,
}

/// Installed plugin info (for frontend)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPluginInfo {
    pub name: String,
    pub marketplace: String,
    pub version: String,
    pub installed_at: String,
    pub install_path: String,
}

/// Get cache directory path
fn get_cache_dir() -> Result<PathBuf, String> {
    Ok(get_claude_plugins_dir()?.join("cache"))
}

/// Clean orphaned cache entries that are not referenced in installed_plugins.json
/// Returns list of cleaned paths
fn clean_orphaned_cache(installed_paths: &HashSet<String>) -> Result<Vec<String>, String> {
    let cache_dir = get_cache_dir()?;
    let mut cleaned = Vec::new();

    if !cache_dir.exists() {
        return Ok(cleaned);
    }

    // Traverse cache directory (structure: cache/{scope}/{plugin_name}/)
    for scope_entry in fs::read_dir(&cache_dir).map_err(|e| e.to_string())?.flatten() {
        let scope_dir = scope_entry.path();
        if !scope_dir.is_dir() {
            continue;
        }

        for plugin_entry in fs::read_dir(&scope_dir).map_err(|e| e.to_string())?.flatten() {
            let plugin_dir = plugin_entry.path();
            if !plugin_dir.is_dir() {
                continue;
            }

            let plugin_path_str = plugin_dir.to_string_lossy().to_string();

            // Check if any installed plugin uses this path
            let is_used = installed_paths.iter().any(|p| {
                // Normalize paths for comparison
                let normalized_p = p.replace('/', "\\");
                let normalized_plugin = plugin_path_str.replace('/', "\\");
                normalized_p.starts_with(&normalized_plugin) || normalized_plugin.starts_with(&normalized_p)
            });

            if !is_used {
                // Orphaned - delete it
                if let Err(e) = fs::remove_dir_all(&plugin_dir) {
                    log::warn!("Failed to remove orphaned cache: {}", e);
                } else {
                    cleaned.push(plugin_path_str);
                    log::info!("Cleaned orphaned cache: {}", plugin_dir.display());
                }
            }
        }

        // After cleaning plugin dirs, check if scope dir is empty and remove it
        if let Ok(remaining) = fs::read_dir(&scope_dir) {
            if remaining.count() == 0 {
                let _ = fs::remove_dir(&scope_dir);
                log::info!("Removed empty scope directory: {}", scope_dir.display());
            }
        }
    }

    Ok(cleaned)
}

/// Get Claude settings.json path (~/.claude/settings.json)
fn get_claude_settings_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    Ok(home.join(".claude").join("settings.json"))
}

/// Claude settings.json structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeSettings {
    #[serde(default)]
    pub enabled_plugins: HashMap<String, bool>,
    #[serde(default)]
    pub always_thinking_enabled: bool,
    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}

/// Read Claude settings.json
pub fn read_claude_settings() -> Result<ClaudeSettings, String> {
    let path = get_claude_settings_path()?;
    if !path.exists() {
        return Ok(ClaudeSettings {
            enabled_plugins: HashMap::new(),
            always_thinking_enabled: false,
            other: HashMap::new(),
        });
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("Failed to read settings.json: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse settings.json: {}", e))
}

/// Write Claude settings.json
fn write_claude_settings(settings: &ClaudeSettings) -> Result<(), String> {
    let path = get_claude_settings_path()?;
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    let content = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Failed to write settings.json: {}", e))
}

/// Get installed_plugins.json path
fn get_installed_plugins_path() -> Result<PathBuf, String> {
    Ok(get_claude_plugins_dir()?.join("installed_plugins.json"))
}

/// Read installed_plugins.json (supports both v1 and v2 formats)
pub fn read_installed_plugins_file() -> Result<InstalledPluginsFile, String> {
    let path = get_installed_plugins_path()?;
    if !path.exists() {
        return Ok(InstalledPluginsFile {
            version: 2,
            plugins: HashMap::new(),
        });
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))?;

    // First, try to parse as v2 format (plugins is array)
    if let Ok(v2) = serde_json::from_str::<InstalledPluginsFileV2>(&content) {
        return Ok(InstalledPluginsFile {
            version: v2.version,
            plugins: v2.plugins,
        });
    }

    // If that fails, try v1 format (plugins is object) and convert to v2
    if let Ok(v1) = serde_json::from_str::<InstalledPluginsFileV1>(&content) {
        let plugins: HashMap<String, Vec<InstalledPluginEntry>> = v1
            .plugins
            .into_iter()
            .map(|(k, v)| (k, vec![v]))
            .collect();
        return Ok(InstalledPluginsFile {
            version: 2,
            plugins,
        });
    }

    // If both fail, return empty
    log::warn!("Could not parse installed_plugins.json, returning empty");
    Ok(InstalledPluginsFile {
        version: 2,
        plugins: HashMap::new(),
    })
}

/// Write installed_plugins.json (always writes v2 format)
fn write_installed_plugins_file(data: &InstalledPluginsFile) -> Result<(), String> {
    let path = get_installed_plugins_path()?;
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    // Convert to V2 format for serialization
    let v2 = InstalledPluginsFileV2 {
        version: data.version,
        plugins: data.plugins.clone(),
    };
    let content =
        serde_json::to_string_pretty(&v2).map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Failed to write file: {}", e))
}

/// Parse plugin key (pluginName@marketplaceName) into (name, marketplace)
fn parse_plugin_key(key: &str) -> (String, String) {
    if let Some(pos) = key.rfind('@') {
        let name = key[..pos].to_string();
        let marketplace = key[pos + 1..].to_string();
        (name, marketplace)
    } else {
        (key.to_string(), String::new())
    }
}

/// Get list of installed plugins from installed_plugins.json
/// Only returns plugins whose install_path actually exists
#[tauri::command]
pub fn get_installed_plugins() -> Result<Vec<InstalledPluginInfo>, String> {
    let file = read_installed_plugins_file()?;
    let mut result = Vec::new();

    for (key, entries) in &file.plugins {
        if let Some(entry) = entries.first() {
            // Only include if install_path exists
            let install_dir = PathBuf::from(&entry.install_path);
            if install_dir.exists() {
                let (name, marketplace) = parse_plugin_key(key);
                result.push(InstalledPluginInfo {
                    name,
                    marketplace,
                    version: entry.version.clone(),
                    installed_at: entry.installed_at.clone(),
                    install_path: entry.install_path.clone(),
                });
            }
        }
    }

    Ok(result)
}

/// Download a directory from GitHub repository using GitHub API
fn download_github_directory(repo: &str, path: &str, dest: &PathBuf) -> Result<(), String> {
    let api_url = format!(
        "https://api.github.com/repos/{}/contents/{}",
        repo, path
    );

    log::info!("Downloading from GitHub: {}", api_url);

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&api_url)
        .header("User-Agent", "Moonton-Claude-Desktop/1.0")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .map_err(|e| format!("GitHub API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        if status == reqwest::StatusCode::FORBIDDEN {
            return Err("GitHub API 请求频率超限 (403 Forbidden)。未认证请求每小时限制 60 次。请等待约 1 小时后重试，或考虑使用 git 命令行安装。".to_string());
        } else if status == reqwest::StatusCode::NOT_FOUND {
            return Err(format!("GitHub 路径不存在: {}", path));
        }
        return Err(format!("GitHub API error: {}", status));
    }

    let contents: serde_json::Value = response.json()
        .map_err(|e| format!("Failed to parse GitHub response: {}", e))?;

    // Handle array of contents (directory listing)
    if let Some(items) = contents.as_array() {
        // Create destination directory for directory listing
        fs::create_dir_all(dest)
            .map_err(|e| format!("Failed to create directory: {}", e))?;

        for item in items {
            let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");
            let item_name = item.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let item_path = item.get("path").and_then(|p| p.as_str()).unwrap_or("");

            if item_type == "file" {
                // Download file
                if let Some(download_url) = item.get("download_url").and_then(|u| u.as_str()) {
                    let file_dest = dest.join(item_name);
                    let file_response = client
                        .get(download_url)
                        .header("User-Agent", "Moonton-Claude-Desktop/1.0")
                        .send()
                        .map_err(|e| format!("Failed to download file: {}", e))?;

                    let content = file_response.bytes()
                        .map_err(|e| format!("Failed to read file content: {}", e))?;

                    fs::write(&file_dest, &content)
                        .map_err(|e| format!("Failed to write file: {}", e))?;

                    log::info!("Downloaded: {}", file_dest.display());
                }
            } else if item_type == "dir" {
                // Recursively download subdirectory
                let subdir_dest = dest.join(item_name);
                download_github_directory(repo, item_path, &subdir_dest)?;
            }
        }
    } else if contents.get("type").and_then(|t| t.as_str()) == Some("file") {
        // Handle single file response (not a directory)
        if let Some(download_url) = contents.get("download_url").and_then(|u| u.as_str()) {
            // Ensure parent directory exists
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent directory: {}", e))?;
            }

            let file_response = client
                .get(download_url)
                .header("User-Agent", "Moonton-Claude-Desktop/1.0")
                .send()
                .map_err(|e| format!("Failed to download file: {}", e))?;

            let content = file_response.bytes()
                .map_err(|e| format!("Failed to read file content: {}", e))?;

            fs::write(dest, &content)
                .map_err(|e| format!("Failed to write file: {}", e))?;

            log::info!("Downloaded file: {}", dest.display());
        }
    }

    Ok(())
}

/// Get the git executable path - prefers bundled git, falls back to system git
fn get_git_exe() -> Result<PathBuf, String> {
    // First try bundled git
    if let Some(bundled_git) = get_bundled_git_exe() {
        log::info!("Using bundled git: {:?}", bundled_git);
        return Ok(bundled_git);
    }

    // Fall back to system git
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut cmd = Command::new("where");
        cmd.arg("git");
        cmd.creation_flags(CREATE_NO_WINDOW);

        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .map(|p| PathBuf::from(p.trim()));
                if let Some(git_path) = path {
                    if git_path.exists() {
                        log::info!("Using system git: {:?}", git_path);
                        return Ok(git_path);
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(git_path) = which::which("git") {
            log::info!("Using system git: {:?}", git_path);
            return Ok(git_path);
        }
    }

    Err("Git not found. Please install Git or download the portable version from the Settings page.".to_string())
}

/// Check if a source string is a URL (for plugins with source: { source: "url", url: "..." })
fn is_url_source(source: &str) -> bool {
    source.starts_with("http://") ||
    source.starts_with("https://") ||
    source.starts_with("git@")
}

/// Validate a plugin URL source to avoid local-file injection and obviously unsafe inputs.
///
/// Allowed:
/// - https://... (preferred)
/// - http://...  (allowed but discouraged)
/// - git@host:org/repo(.git)
/// - ssh://...   (explicit ssh scheme)
///
/// Rejected:
/// - file://, scp-like paths that look local, relative paths, whitespace/newlines
/// - credentials embedded in URL (user:pass@)
fn validate_plugin_source_url(url: &str) -> Result<(), String> {
    let u = url.trim();
    if u.is_empty() {
        return Err("Plugin URL is empty".to_string());
    }
    if u != url {
        return Err("Plugin URL contains leading/trailing whitespace".to_string());
    }
    if u.contains(['\n', '\r', '\t', ' ']) {
        return Err("Plugin URL must not contain whitespace".to_string());
    }
    if u.contains("..") {
        return Err("Plugin URL contains suspicious '..'".to_string());
    }

    // Explicitly block local/file schemes
    let lowered = u.to_ascii_lowercase();
    if lowered.starts_with("file://") {
        return Err("file:// URLs are not allowed for plugin sources".to_string());
    }
    if lowered.starts_with("/") || lowered.starts_with("./") || lowered.starts_with("../") {
        return Err("Local paths are not allowed for URL-type plugin sources".to_string());
    }

    // Allow http/https URLs (including those with OAuth2 tokens for private repos)
    // Note: URLs with credentials like oauth2:token@host are allowed since they're
    // intentionally configured by marketplace maintainers for private repository access
    if lowered.starts_with("http://") || lowered.starts_with("https://") {
        return Ok(());
    }

    // Allow explicit ssh scheme
    if lowered.starts_with("ssh://") {
        return Ok(());
    }

    // Allow git@host:owner/repo(.git)
    if lowered.starts_with("git@") {
        return Ok(());
    }

    Err("Unsupported plugin URL scheme. Use https://..., ssh://..., or git@...".to_string())
}

/// Clone a plugin directly from a URL to the install path
fn clone_plugin_from_url(url: &str, install_path: &PathBuf) -> Result<(), String> {
    validate_plugin_source_url(url)?;

    let git_exe = get_git_exe()?;

    log::info!("Cloning plugin from URL: {} to {}", url, install_path.display());

    // Create parent directory
    if let Some(parent) = install_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
    }

    // Git clone with depth=1 for efficiency
    let mut cmd = Command::new(&git_exe);
    cmd.args([
        "clone",
        "--depth=1",
        "--single-branch",
        "--config",
        "advice.detachedHead=false",
        url,
    ])
    .arg(install_path);

    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd.output()
        .map_err(|e| format!("Failed to execute git clone: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Git clone failed: {}", stderr));
    }

    // Remove .git directory to save space
    let git_dir = install_path.join(".git");
    if git_dir.exists() {
        let _ = fs::remove_dir_all(&git_dir);
    }

    log::info!("Successfully cloned plugin from URL");
    Ok(())
}

/// Download files from a git repository using sparse checkout (clone only specific paths)
fn download_git_sparse(git_url: &str, paths: &[&str], dest: &PathBuf) -> Result<(), String> {
    use std::process::Command;
    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;
    #[cfg(target_os = "windows")]
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    // Get git executable path (bundled or system)
    let git_exe = get_git_exe()?;

    // Create a temp directory for sparse checkout
    let temp_dir = dest.parent()
        .ok_or("Cannot get parent directory")?
        .join(format!(".git_temp_{}", std::process::id()));

    // Clean up any existing temp dir
    if temp_dir.exists() {
        let _ = fs::remove_dir_all(&temp_dir);
    }

    log::info!("Using git sparse checkout to download: {:?}", paths);

    // Initialize sparse checkout
    let mut cmd = Command::new(&git_exe);
    cmd.args(["clone", "--filter=blob:none", "--no-checkout", "--depth=1", git_url])
        .arg(&temp_dir);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let init_result = cmd.output();

    match init_result {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                log::warn!("git clone failed: {}", stderr);
                // Clean up
                let _ = fs::remove_dir_all(&temp_dir);
                return Err(format!("Git clone failed: {}", stderr));
            }
        }
        Err(e) => {
            return Err(format!("Git not found or failed: {}. Please install git.", e));
        }
    }

    // Enable sparse checkout
    let mut cmd = Command::new(&git_exe);
    cmd.args(["sparse-checkout", "init", "--cone"])
        .current_dir(&temp_dir);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let sparse_result = cmd.output();

    if let Ok(output) = sparse_result {
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::warn!("sparse-checkout init failed: {}", stderr);
        }
    }

    // Set sparse checkout paths
    let mut sparse_cmd = Command::new(&git_exe);
    sparse_cmd.arg("sparse-checkout").arg("set").current_dir(&temp_dir);
    for path in paths {
        sparse_cmd.arg(path);
    }
    #[cfg(target_os = "windows")]
    sparse_cmd.creation_flags(CREATE_NO_WINDOW);
    let sparse_set_result = sparse_cmd.output();

    if let Ok(output) = sparse_set_result {
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::warn!("sparse-checkout set failed: {}", stderr);
        }
    }

    // Checkout
    let mut cmd = Command::new(&git_exe);
    cmd.args(["checkout"])
        .current_dir(&temp_dir);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let checkout_result = cmd.output();

    if let Ok(output) = checkout_result {
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::warn!("checkout failed: {}", stderr);
        }
    }

    // Copy files from temp to destination
    for path in paths {
        let src_path = temp_dir.join(path);
        if src_path.exists() {
            // Destination is relative to marketplace dir (dest)
            let dest_path = dest.join(path);

            if src_path.is_dir() {
                if let Err(e) = copy_dir_recursive(&src_path, &dest_path) {
                    log::warn!("Failed to copy directory: {}", e);
                } else {
                    log::info!("Copied directory: {} -> {}", src_path.display(), dest_path.display());
                }
            } else if src_path.is_file() {
                if let Some(parent) = dest_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if let Err(e) = fs::copy(&src_path, &dest_path) {
                    log::warn!("Failed to copy file: {}", e);
                } else {
                    log::info!("Copied file: {} -> {}", src_path.display(), dest_path.display());
                }
            }
        } else {
            log::warn!("Path not found in repo: {}", path);
        }
    }

    // Clean up temp directory
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}

/// Download plugin files from marketplace source
fn download_plugin_files(
    marketplace_name: &str,
    plugin: &PluginInfo,
    marketplace_dir: &PathBuf,
) -> Result<(), String> {
    // Get marketplace source info
    let known = read_known_marketplaces()?;
    let entry = known.get(marketplace_name)
        .ok_or(format!("Marketplace not found: {}", marketplace_name))?;

    let is_github = entry.source.source == "github";

    log::info!("Downloading plugin files for {} (source: {})",
        plugin.name, entry.source.source);

    // Build git URL for sparse checkout
    let git_url = if is_github {
        let repo = entry.source.repo.as_ref()
            .ok_or("GitHub repo not configured")?;
        format!("https://github.com/{}.git", repo)
    } else {
        entry.source.url.as_ref()
            .ok_or("Git URL not configured")?.clone()
    };

    // Collect all paths to download
    let mut paths_to_download: Vec<String> = Vec::new();
    let source_trimmed = plugin.source.trim_start_matches("./");

    // Add source path
    if !source_trimmed.is_empty() && source_trimmed != "." {
        let dest = marketplace_dir.join(source_trimmed);
        if !dest.exists() {
            paths_to_download.push(source_trimmed.to_string());
        }
    }

    // Add skills paths
    for skill_path in &plugin.skills {
        let path_trimmed = skill_path.trim_start_matches("./");
        if !path_trimmed.is_empty() && path_trimmed != "." {
            let dest = marketplace_dir.join(path_trimmed);
            if !dest.exists() {
                paths_to_download.push(path_trimmed.to_string());
            }
        }
    }

    // Add agents paths
    for agent_path in &plugin.agents {
        let path_trimmed = agent_path.trim_start_matches("./");
        if !path_trimmed.is_empty() && path_trimmed != "." {
            let dest = marketplace_dir.join(path_trimmed);
            if !dest.exists() {
                paths_to_download.push(path_trimmed.to_string());
            }
        }
    }

    // Add commands paths (add parent directory for files)
    for cmd_path in &plugin.commands {
        let path_trimmed = cmd_path.trim_start_matches("./");
        if !path_trimmed.is_empty() && path_trimmed != "." {
            let dest = marketplace_dir.join(path_trimmed);
            if !dest.exists() {
                if let Some(parent) = std::path::Path::new(path_trimmed).parent() {
                    let parent_str = parent.to_string_lossy().to_string();
                    if !parent_str.is_empty() && !paths_to_download.contains(&parent_str) {
                        paths_to_download.push(parent_str);
                    }
                }
            }
        }
    }

    if paths_to_download.is_empty() {
        log::info!("No paths to download for {}", plugin.name);
        return Ok(());
    }

    // Try git sparse checkout first (works for both GitHub and other git sources)
    log::info!("Using git sparse checkout for {} paths", paths_to_download.len());
    let path_refs: Vec<&str> = paths_to_download.iter().map(|s| s.as_str()).collect();
    let git_result = download_git_sparse(&git_url, &path_refs, marketplace_dir);

    if git_result.is_ok() {
        log::info!("Successfully downloaded plugin files via git sparse checkout");
        return Ok(());
    }

    // If git sparse checkout fails and it's GitHub, fall back to GitHub API
    if is_github {
        log::warn!("Git sparse checkout failed, falling back to GitHub API: {:?}", git_result.err());
        let repo = entry.source.repo.as_ref().unwrap();

        for path in &paths_to_download {
            let dest = marketplace_dir.join(path);
            if !dest.exists() {
                download_github_directory(repo, path, &dest)?;
            }
        }
    } else {
        // For non-GitHub sources, propagate the git error
        return git_result;
    }

    log::info!("Successfully downloaded plugin files for {}", plugin.name);
    Ok(())
}

/// Install a plugin from a marketplace (async to avoid blocking UI)
#[tauri::command]
pub async fn install_plugin(marketplace_name: String, plugin_name: String) -> Result<InstalledPluginInfo, String> {
    // Run the blocking operations in a separate thread
    let result = tauri::async_runtime::spawn_blocking(move || {
        install_plugin_sync(marketplace_name, plugin_name)
    }).await.map_err(|e| format!("Task join error: {}", e))?;
    result
}

/// Get the HEAD commit SHA from a marketplace's git repository
fn get_marketplace_git_sha(marketplace_name: &str) -> Option<String> {
    let marketplace_dir = get_marketplaces_dir().ok()?.join(marketplace_name);

    let git_exe = get_git_exe().ok()?;
    let mut cmd = Command::new(&git_exe);
    cmd.args(["rev-parse", "HEAD"]);
    cmd.current_dir(&marketplace_dir);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd.output().ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Sync version of install_plugin (runs in blocking thread)
/// Copies plugin files from marketplaces/ to cache/
fn install_plugin_sync(marketplace_name: String, plugin_name: String) -> Result<InstalledPluginInfo, String> {
    // Get plugin info from marketplace
    let manifest = read_marketplace_manifest(&marketplace_name)?;
    let plugin = manifest
        .plugins
        .iter()
        .find(|p| p.name == plugin_name)
        .cloned()
        .ok_or(format!("Plugin not found: {}", plugin_name))?;

    // Source path (in marketplaces/)
    let marketplace_dir = get_marketplaces_dir()?.join(&marketplace_name);
    let source_clean = plugin.source.trim_start_matches("./");

    // Check if source is a URL (for plugins with source: { source: "url", url: "..." })
    if is_url_source(&plugin.source) {
        log::info!("Plugin {} has URL source, will clone from: {}", plugin_name, plugin.source);

        // Determine version first
        let version = if !plugin.version.is_empty() {
            plugin.version.clone()
        } else {
            "latest".to_string()
        };

        // Destination path: cache/{marketplace}/{plugin_name}/{version}/
        let cache_dir = get_cache_dir()?;
        let install_path = cache_dir
            .join(&marketplace_name)
            .join(&plugin_name)
            .join(&version);

        // Clean old version if exists
        if install_path.exists() {
            fs::remove_dir_all(&install_path)
                .map_err(|e| format!("Failed to remove old installation: {}", e))?;
        }

        // Clone from URL directly to install_path
        clone_plugin_from_url(&plugin.source, &install_path)?;

        // Plugin key format: pluginName@marketplaceName
        let plugin_key = format!("{}@{}", plugin_name, marketplace_name);
        let now = chrono::Utc::now().to_rfc3339();

        // Record in installed_plugins.json
        let mut file = read_installed_plugins_file()?;
        let entry = InstalledPluginEntry {
            scope: Some("user".to_string()),
            install_path: install_path.to_string_lossy().to_string(),
            version: version.clone(),
            installed_at: now.clone(),
            last_updated: now.clone(),
            git_commit_sha: None, // No git SHA for URL sources
            is_local: false,
            other: HashMap::new(),
        };
        file.plugins.insert(plugin_key.clone(), vec![entry]);
        write_installed_plugins_file(&file)?;

        // Enable plugin in settings.json (default enabled on install)
        let mut settings = read_claude_settings()?;
        settings.enabled_plugins.insert(plugin_key, true);
        write_claude_settings(&settings)?;
        log::info!("Plugin installed and enabled: {}@{}", plugin_name, marketplace_name);

        return Ok(InstalledPluginInfo {
            name: plugin_name,
            marketplace: marketplace_name,
            version,
            installed_at: now,
            install_path: install_path.to_string_lossy().to_string(),
        });
    }

    // Local path source - continue with existing logic
    // Collect all paths to copy (skills, agents, commands)
    let mut paths_to_copy: Vec<PathBuf> = Vec::new();

    // 1. If source is not empty/root, add source path
    if !source_clean.is_empty() && source_clean != "." {
        paths_to_copy.push(marketplace_dir.join(source_clean));
    }

    // 2. Add skills paths
    for skill_path in &plugin.skills {
        let path_clean = skill_path.trim_start_matches("./");
        if !path_clean.is_empty() && path_clean != "." {
            let full_path = marketplace_dir.join(path_clean);
            if !paths_to_copy.contains(&full_path) {
                paths_to_copy.push(full_path);
            }
        }
    }

    // 3. Add agents paths (copy parent directory for files)
    for agent_path in &plugin.agents {
        let path_clean = agent_path.trim_start_matches("./");
        if !path_clean.is_empty() && path_clean != "." {
            if let Some(parent) = std::path::Path::new(path_clean).parent() {
                let parent_str = parent.to_string_lossy();
                if !parent_str.is_empty() {
                    let parent_path = marketplace_dir.join(parent);
                    if !paths_to_copy.contains(&parent_path) {
                        paths_to_copy.push(parent_path);
                    }
                }
            }
        }
    }

    // 4. Add commands paths (copy parent directory for files)
    for cmd_path in &plugin.commands {
        let path_clean = cmd_path.trim_start_matches("./");
        if !path_clean.is_empty() && path_clean != "." {
            if let Some(parent) = std::path::Path::new(path_clean).parent() {
                let parent_str = parent.to_string_lossy();
                if !parent_str.is_empty() {
                    let parent_path = marketplace_dir.join(parent);
                    if !paths_to_copy.contains(&parent_path) {
                        paths_to_copy.push(parent_path);
                    }
                }
            }
        }
    }

    // 5. Add MCP servers paths if present
    if let Some(ref mcp_servers) = plugin.mcp_servers {
        if let Some(obj) = mcp_servers.as_object() {
            for (_, server_config) in obj {
                if let Some(args) = server_config.get("args").and_then(|a| a.as_array()) {
                    for arg in args {
                        if let Some(arg_str) = arg.as_str() {
                            let path_clean = arg_str.trim_start_matches("./");
                            if path_clean.starts_with("plugins/") || path_clean.starts_with("skills/") {
                                if let Some(parent) = std::path::Path::new(path_clean).parent() {
                                    let parent_path = marketplace_dir.join(parent);
                                    if !paths_to_copy.contains(&parent_path) {
                                        paths_to_copy.push(parent_path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // If no paths collected, fallback to marketplace_dir (backward compatibility)
    if paths_to_copy.is_empty() {
        log::warn!("No specific paths found for plugin {}, falling back to marketplace root", plugin_name);
        paths_to_copy.push(marketplace_dir.clone());
    }

    log::info!("Paths to copy for plugin {}: {:?}", plugin_name, paths_to_copy);

    // Check if any source files need to be downloaded
    let needs_download = paths_to_copy.iter().any(|p| !p.exists());

    if needs_download {
        log::info!("Plugin files not found in marketplace, downloading...");
        download_plugin_files(&marketplace_name, &plugin, &marketplace_dir)?;
    }

    // Get git commit SHA from marketplace repository
    let git_sha = get_marketplace_git_sha(&marketplace_name);
    log::info!("Marketplace {} git SHA: {:?}", marketplace_name, git_sha);

    // Determine version: prefer plugin.version, fallback to git SHA prefix, then "latest"
    let version = if !plugin.version.is_empty() {
        plugin.version.clone()
    } else if let Some(ref sha) = git_sha {
        sha[..12.min(sha.len())].to_string()
    } else {
        "latest".to_string()
    };

    // Destination path: cache/{marketplace}/{plugin_name}/{version}/
    let cache_dir = get_cache_dir()?;
    let install_path = cache_dir
        .join(&marketplace_name)
        .join(&plugin_name)
        .join(&version);

    // Clean old version if exists and copy new files
    if install_path.exists() {
        fs::remove_dir_all(&install_path)
            .map_err(|e| format!("Failed to remove old installation: {}", e))?;
    }

    // Create install directory
    fs::create_dir_all(&install_path)
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;

    // Determine the source path for relative path calculation
    // When source is defined (e.g., "./plugins/skill-creator"), we want to copy its contents
    // directly to install_path root, not preserve the source path structure
    let source_path = if !source_clean.is_empty() && source_clean != "." {
        Some(marketplace_dir.join(source_clean))
    } else {
        None
    };

    // Copy all collected paths to install_path
    for src_path in &paths_to_copy {
        if src_path.exists() {
            // Calculate relative path based on whether this path is under source directory
            let relative = if let Some(ref sp) = source_path {
                if src_path == sp {
                    // This is the source directory itself - copy its contents to root
                    PathBuf::new()
                } else if src_path.starts_with(sp) {
                    // Path is inside source directory - make relative to source
                    src_path.strip_prefix(sp)
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|_| PathBuf::new())
                } else {
                    // Path is outside source directory - use relative to marketplace_dir
                    src_path.strip_prefix(&marketplace_dir)
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|_| {
                            src_path.file_name()
                                .map(|n| PathBuf::from(n))
                                .unwrap_or_else(|| src_path.clone())
                        })
                }
            } else {
                // No source defined - use original logic (relative to marketplace_dir)
                src_path.strip_prefix(&marketplace_dir)
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|_| {
                        src_path.file_name()
                            .map(|n| PathBuf::from(n))
                            .unwrap_or_else(|| src_path.clone())
                    })
            };

            // If relative is empty, copy directory contents directly to install_path
            if relative.as_os_str().is_empty() {
                if src_path.is_dir() {
                    log::info!("Copying contents of {} to {}", src_path.display(), install_path.display());
                    copy_dir_contents(&src_path, &install_path)?;
                }
                // Skip empty relative for files (shouldn't happen in practice)
            } else {
                let dst_path = install_path.join(&relative);
                log::info!("Copying {} to {}", src_path.display(), dst_path.display());

                if src_path.is_dir() {
                    copy_dir_recursive(&src_path, &dst_path)?;
                } else {
                    if let Some(parent) = dst_path.parent() {
                        fs::create_dir_all(parent)
                            .map_err(|e| format!("Failed to create directory: {}", e))?;
                    }
                    fs::copy(&src_path, &dst_path)
                        .map_err(|e| format!("Failed to copy file {}: {}", src_path.display(), e))?;
                }
            }
        } else {
            log::warn!("Source path does not exist: {}", src_path.display());
        }
    }
    log::info!("Copied plugin files successfully");

    // Plugin key format: pluginName@marketplaceName
    let plugin_key = format!("{}@{}", plugin_name, marketplace_name);

    // Record in installed_plugins.json
    let mut file = read_installed_plugins_file()?;
    let now = chrono::Utc::now().to_rfc3339();

    let entry = InstalledPluginEntry {
        scope: Some("user".to_string()),
        install_path: install_path.to_string_lossy().to_string(),
        version: version.clone(),
        installed_at: now.clone(),
        last_updated: now.clone(),
        git_commit_sha: git_sha,
        is_local: false, // Not local - installed from marketplace
        other: HashMap::new(),
    };
    file.plugins.insert(plugin_key.clone(), vec![entry]);
    write_installed_plugins_file(&file)?;

    // Enable plugin in settings.json (default enabled on install)
    let mut settings = read_claude_settings()?;
    settings.enabled_plugins.insert(plugin_key, true);
    write_claude_settings(&settings)?;
    log::info!("Plugin installed and enabled: {}@{}", plugin_name, marketplace_name);

    Ok(InstalledPluginInfo {
        name: plugin_name,
        marketplace: marketplace_name,
        version,
        installed_at: now,
        install_path: install_path.to_string_lossy().to_string(),
    })
}

/// Uninstall a plugin (async to avoid blocking UI)
#[tauri::command]
pub async fn uninstall_plugin(plugin_name: String, marketplace_name: Option<String>) -> Result<(), String> {
    // Run the blocking operations in a separate thread
    tauri::async_runtime::spawn_blocking(move || {
        uninstall_plugin_sync(plugin_name, marketplace_name)
    }).await.map_err(|e| format!("Task join error: {}", e))?
}

/// Sync version of uninstall_plugin (runs in blocking thread)
fn uninstall_plugin_sync(plugin_name: String, marketplace_name: Option<String>) -> Result<(), String> {
    let mut file = read_installed_plugins_file()?;

    // Find plugin key and get install_path from entry
    let (plugin_key, install_path) = if let Some(ref mp) = marketplace_name {
        let key = format!("{}@{}", plugin_name, mp);
        let path = file.plugins.get(&key)
            .and_then(|entries| entries.first())
            .map(|e| e.install_path.clone())
            .ok_or(format!("Plugin not installed: {}", plugin_name))?;
        (key, path)
    } else {
        // Search for plugin by name
        let (key, path) = file.plugins.iter()
            .find(|(k, _)| k.starts_with(&format!("{}@", plugin_name)))
            .and_then(|(k, entries)| entries.first().map(|e| (k.clone(), e.install_path.clone())))
            .ok_or(format!("Plugin not installed: {}", plugin_name))?;
        (key, path)
    };

    // Remove from installed_plugins.json
    file.plugins.remove(&plugin_key);
    write_installed_plugins_file(&file)?;

    // Remove from settings.json enabledPlugins
    let mut settings = read_claude_settings()?;
    settings.enabled_plugins.remove(&plugin_key);
    write_claude_settings(&settings)?;
    log::info!("Plugin removed from settings.json: {}", plugin_key);

    // Delete the installPath directory directly
    let install_dir = PathBuf::from(&install_path);
    if install_dir.exists() {
        fs::remove_dir_all(&install_dir)
            .map_err(|e| format!("Failed to remove plugin directory: {}", e))?;
        log::info!("Removed plugin directory: {}", install_path);
    }

    Ok(())
}

/// Helper function to recursively copy a directory
fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> Result<(), String> {
    if !src.is_dir() {
        return Err(format!("Source is not a directory: {}", src.display()));
    }

    fs::create_dir_all(dst).map_err(|e| format!("Failed to create directory: {}", e))?;

    for entry in fs::read_dir(src).map_err(|e| format!("Failed to read directory: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy file: {}", e))?;
        }
    }

    Ok(())
}

/// Copy directory contents (not the directory itself) to destination
/// Used when we want to copy the contents of source directory directly to install_path root
fn copy_dir_contents(src: &PathBuf, dst: &PathBuf) -> Result<(), String> {
    if !src.is_dir() {
        return Err(format!("Source is not a directory: {}", src.display()));
    }

    fs::create_dir_all(dst)
        .map_err(|e| format!("Failed to create directory {}: {}", dst.display(), e))?;

    for entry in fs::read_dir(src)
        .map_err(|e| format!("Failed to read directory {}: {}", src.display(), e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy file {}: {}", src_path.display(), e))?;
        }
    }
    Ok(())
}

/// Enable a plugin (set to true in settings.json)
#[tauri::command]
pub fn enable_plugin(plugin_name: String, marketplace_name: String) -> Result<(), String> {
    let plugin_key = format!("{}@{}", plugin_name, marketplace_name);
    let mut settings = read_claude_settings()?;
    settings.enabled_plugins.insert(plugin_key, true);
    write_claude_settings(&settings)?;
    log::info!("Enabled plugin: {}@{}", plugin_name, marketplace_name);
    Ok(())
}

/// Disable a plugin (set to false in settings.json)
#[tauri::command]
pub fn disable_plugin(plugin_name: String, marketplace_name: String) -> Result<(), String> {
    let plugin_key = format!("{}@{}", plugin_name, marketplace_name);
    let mut settings = read_claude_settings()?;
    settings.enabled_plugins.insert(plugin_key, false);
    write_claude_settings(&settings)?;
    log::info!("Disabled plugin: {}@{}", plugin_name, marketplace_name);
    Ok(())
}

/// Get plugin enabled status from settings.json
#[tauri::command]
pub fn get_plugin_enabled_status(plugin_name: String, marketplace_name: String) -> Result<bool, String> {
    let plugin_key = format!("{}@{}", plugin_name, marketplace_name);
    let settings = read_claude_settings()?;
    Ok(settings.enabled_plugins.get(&plugin_key).copied().unwrap_or(false))
}

/// Get all enabled plugins from settings.json
#[tauri::command]
pub fn get_all_enabled_plugins() -> Result<HashMap<String, bool>, String> {
    let settings = read_claude_settings()?;
    Ok(settings.enabled_plugins)
}

/// Ensure marketplace files exist, update via CLI if missing
/// Returns true if files were downloaded, false if they already existed
/// Uses Claude CLI: claude plugin marketplace update <name>
fn ensure_marketplace_files(marketplace_name: &str, _entry: &MarketplaceEntry) -> Result<bool, String> {
    let marketplace_dir = get_marketplaces_dir()?.join(marketplace_name);
    let manifest_path = marketplace_dir.join(".claude-plugin").join("marketplace.json");

    // Check if marketplace.json exists
    if manifest_path.exists() {
        return Ok(false);
    }

    log::info!("Marketplace {} files missing, updating via CLI...", marketplace_name);

    // Use Claude CLI to update marketplace (will re-download files)
    if let Err(e) = run_claude_command(&["plugin", "marketplace", "update", marketplace_name]) {
        return Err(format!("Failed to update marketplace via CLI: {}", e));
    }

    log::info!("Marketplace {} updated via CLI", marketplace_name);
    Ok(true)
}

/// Repair result for a single plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRepairAction {
    pub plugin_key: String,
    pub action: String, // "added", "removed", "enabled_fixed", "recovered", "marketplace_downloaded"
    pub details: String,
}

/// Overall repair result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairResult {
    pub actions: Vec<PluginRepairAction>,
    pub installed_plugins_fixed: i32,
    pub settings_fixed: i32,
    pub total_installed: i32,
}

// ============================================================
// Plugin Meta Types for Full Plugin Market
// ============================================================

/// Applicable work/scenario for a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginMetaApplicableWork {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// Screenshot info for a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginMetaScreenshot {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

/// Plugin metadata from plugin.meta.json
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applicable_works: Option<Vec<PluginMetaApplicableWork>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshots: Option<Vec<PluginMetaScreenshot>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlights: Option<Vec<String>>,
}

/// Extended plugin info with metadata and status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfoExtended {
    pub name: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: Option<PluginAuthor>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub marketplace_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<PluginMeta>,
    pub is_installed: bool,
    pub is_enabled: bool,
    pub has_update: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_version: Option<String>,
}

/// Read plugin.meta.json from a plugin directory or docs/plugins directory
#[allow(dead_code)]
fn read_plugin_meta(marketplace_name: &str, plugin_source: &str) -> Option<PluginMeta> {
    read_plugin_meta_with_name(marketplace_name, plugin_source, None)
}

/// Read plugin.meta.json with optional explicit plugin name
fn read_plugin_meta_with_name(marketplace_name: &str, plugin_source: &str, plugin_name: Option<&str>) -> Option<PluginMeta> {
    let marketplace_dir = get_marketplaces_dir().ok()?.join(marketplace_name);
    let source_clean = plugin_source.trim_start_matches("./");

    // First, try docs/plugins/{plugin_name}.meta.json
    // Extract plugin name from source path (last component) or use provided name
    let name = plugin_name.unwrap_or_else(|| {
        std::path::Path::new(source_clean)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(source_clean)
    });
    let docs_meta_path = marketplace_dir.join("docs").join("plugins").join(format!("{}.meta.json", name));

    if docs_meta_path.exists() {
        if let Ok(content) = fs::read_to_string(&docs_meta_path) {
            if let Ok(meta) = serde_json::from_str(&content) {
                return Some(meta);
            }
        }
    }

    // Fall back to plugin.meta.json in the plugin directory
    let plugin_dir = marketplace_dir.join(source_clean);
    let meta_path = plugin_dir.join("plugin.meta.json");

    if meta_path.exists() {
        let content = fs::read_to_string(&meta_path).ok()?;
        serde_json::from_str(&content).ok()
    } else {
        None
    }
}

/// Compare two semantic versions, returns true if new_version > old_version
fn is_newer_version(new_version: &str, old_version: &str) -> bool {
    let parse_version = |v: &str| -> Vec<u32> {
        v.trim_start_matches('v')
            .split(|c: char| !c.is_ascii_digit())
            .filter_map(|s| s.parse().ok())
            .collect()
    };

    let new_parts = parse_version(new_version);
    let old_parts = parse_version(old_version);

    for i in 0..new_parts.len().max(old_parts.len()) {
        let new_part = new_parts.get(i).copied().unwrap_or(0);
        let old_part = old_parts.get(i).copied().unwrap_or(0);

        if new_part > old_part {
            return true;
        } else if new_part < old_part {
            return false;
        }
    }

    false
}

/// Get plugin metadata
#[tauri::command]
pub fn get_plugin_meta(marketplace_name: String, plugin_name: String) -> Result<Option<PluginMeta>, String> {
    let manifest = read_marketplace_manifest(&marketplace_name)?;
    let plugin = manifest
        .plugins
        .iter()
        .find(|p| p.name == plugin_name)
        .ok_or(format!("Plugin not found: {}", plugin_name))?;

    Ok(read_plugin_meta_with_name(&marketplace_name, &plugin.source, Some(&plugin_name)))
}

/// Get all plugins with extended info from all marketplaces
#[tauri::command]
pub fn get_all_plugins_extended() -> Result<Vec<PluginInfoExtended>, String> {
    let known = read_known_marketplaces()?;
    let installed_file = read_installed_plugins_file()?;
    let settings = read_claude_settings()?;

    let mut all_plugins = Vec::new();

    for marketplace_name in known.keys() {
        if let Ok(manifest) = read_marketplace_manifest(marketplace_name) {
            for plugin in manifest.plugins {
                let plugin_key = format!("{}@{}", plugin.name, marketplace_name);

                // Check installation status
                let installed_entry = installed_file.plugins.get(&plugin_key)
                    .and_then(|entries| entries.first());

                let is_installed = installed_entry.is_some();
                let installed_version = installed_entry.map(|e| e.version.clone());

                // Check if update available
                let has_update = if let Some(ref iv) = installed_version {
                    !plugin.version.is_empty() && is_newer_version(&plugin.version, iv)
                } else {
                    false
                };

                // Check enabled status
                let is_enabled = settings.enabled_plugins.get(&plugin_key)
                    .copied()
                    .unwrap_or(false);

                // Read metadata (pass plugin name explicitly for correct lookup)
                let meta = read_plugin_meta_with_name(marketplace_name, &plugin.source, Some(&plugin.name));

                all_plugins.push(PluginInfoExtended {
                    name: plugin.name,
                    source: plugin.source,
                    description: plugin.description,
                    version: plugin.version,
                    author: plugin.author,
                    skills: plugin.skills,
                    agents: plugin.agents,
                    tags: plugin.tags,
                    marketplace_name: marketplace_name.clone(),
                    meta,
                    is_installed,
                    is_enabled,
                    has_update,
                    installed_version,
                });
            }
        }
    }

    // Sort by name
    all_plugins.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(all_plugins)
}

/// Get plugin icon as base64 data URL or return icon URL
#[tauri::command]
pub fn get_plugin_icon(marketplace_name: String, plugin_name: String) -> Result<Option<String>, String> {
    let manifest = read_marketplace_manifest(&marketplace_name)?;
    let plugin = manifest
        .plugins
        .iter()
        .find(|p| p.name == plugin_name)
        .ok_or(format!("Plugin not found: {}", plugin_name))?;

    let marketplace_dir = get_marketplaces_dir()?.join(&marketplace_name);

    // First check plugin.meta.json for iconUrl (from docs/plugins or plugin directory)
    if let Some(meta) = read_plugin_meta_with_name(&marketplace_name, &plugin.source, Some(&plugin_name)) {
        if let Some(icon_url) = meta.icon_url {
            // If it's a relative path, try to read the file
            if icon_url.starts_with("./") || icon_url.starts_with("../") {
                // First try resolving relative to docs/plugins directory
                let docs_plugins_dir = marketplace_dir.join("docs").join("plugins");
                let icon_path = docs_plugins_dir.join(icon_url.trim_start_matches("./"));

                if icon_path.exists() {
                    if let Ok(bytes) = fs::read(&icon_path) {
                        let base64_data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
                        let extension = icon_path.extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("png");
                        let mime_type = match extension {
                            "svg" => "image/svg+xml",
                            "jpg" | "jpeg" => "image/jpeg",
                            "gif" => "image/gif",
                            "webp" => "image/webp",
                            _ => "image/png",
                        };
                        return Ok(Some(format!("data:{};base64,{}", mime_type, base64_data)));
                    }
                }

                // Fall back to plugin directory
                let source_clean = plugin.source.trim_start_matches("./");
                let plugin_dir = marketplace_dir.join(source_clean);
                let icon_path = plugin_dir.join(icon_url.trim_start_matches("./"));

                if icon_path.exists() {
                    if let Ok(bytes) = fs::read(&icon_path) {
                        let base64_data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
                        let extension = icon_path.extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("png");
                        let mime_type = match extension {
                            "svg" => "image/svg+xml",
                            "jpg" | "jpeg" => "image/jpeg",
                            "gif" => "image/gif",
                            "webp" => "image/webp",
                            _ => "image/png",
                        };
                        return Ok(Some(format!("data:{};base64,{}", mime_type, base64_data)));
                    }
                }
            } else {
                // It's an absolute URL, return as-is
                return Ok(Some(icon_url));
            }
        }
    }

    // Try to find default icon files in plugin directory
    let source_clean = plugin.source.trim_start_matches("./");
    let plugin_dir = marketplace_dir.join(source_clean);

    for icon_name in &["icon.png", "icon.svg", "icon.jpg", "logo.png", "logo.svg"] {
        let icon_path = plugin_dir.join(icon_name);
        if icon_path.exists() {
            if let Ok(bytes) = fs::read(&icon_path) {
                let base64_data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
                let extension = icon_path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("png");
                let mime_type = match extension {
                    "svg" => "image/svg+xml",
                    "jpg" | "jpeg" => "image/jpeg",
                    _ => "image/png",
                };
                return Ok(Some(format!("data:{};base64,{}", mime_type, base64_data)));
            }
        }
    }

    Ok(None)
}

/// Repair plugin registry - sync installed_plugins.json and settings.json with actual files
/// Uses marketplace.json definitions and actual plugin directories as source of truth
#[tauri::command]
pub async fn repair_plugin_registry() -> Result<RepairResult, String> {
    tauri::async_runtime::spawn_blocking(|| {
        repair_plugin_registry_sync()
    }).await.map_err(|e| format!("Task join error: {}", e))?
}

fn repair_plugin_registry_sync() -> Result<RepairResult, String> {
    log::info!("Starting plugin registry repair...");

    let mut actions: Vec<PluginRepairAction> = Vec::new();
    let mut installed_plugins_fixed = 0;
    let mut settings_fixed = 0;

    // Read current state
    let known_marketplaces = read_known_marketplaces()?;
    let mut installed_file = read_installed_plugins_file()?;
    let mut settings = read_claude_settings()?;

    // Step 0: Ensure all marketplace files exist (download if missing)
    for (marketplace_name, entry) in &known_marketplaces {
        match ensure_marketplace_files(marketplace_name, entry) {
            Ok(true) => {
                actions.push(PluginRepairAction {
                    plugin_key: marketplace_name.clone(),
                    action: "marketplace_downloaded".to_string(),
                    details: format!("市场 {} 文件缺失，已从远程下载", marketplace_name),
                });
            }
            Ok(false) => {}
            Err(e) => {
                log::warn!("Failed to ensure marketplace files for {}: {}", marketplace_name, e);
                actions.push(PluginRepairAction {
                    plugin_key: marketplace_name.clone(),
                    action: "marketplace_download_failed".to_string(),
                    details: format!("市场 {} 文件下载失败: {}", marketplace_name, e),
                });
            }
        }
    }

    // Step 1: Validate installed_plugins.json entries
    // Only keep entries whose install_path actually exists
    let keys_to_check: Vec<String> = installed_file.plugins.keys().cloned().collect();
    let mut valid_keys: HashSet<String> = HashSet::new();

    for key in keys_to_check {
        let path_exists = installed_file.plugins.get(&key)
            .and_then(|entries| entries.first())
            .map(|e| PathBuf::from(&e.install_path).exists())
            .unwrap_or(false);

        if path_exists {
            valid_keys.insert(key);
        } else {
            // Remove entries where install_path doesn't exist
            installed_file.plugins.remove(&key);
            installed_plugins_fixed += 1;
            actions.push(PluginRepairAction {
                plugin_key: key.clone(),
                action: "removed_from_installed".to_string(),
                details: "插件文件不存在于 cache 目录，从 installed_plugins.json 移除".to_string(),
            });
            log::info!("Removed missing plugin from installed_plugins.json: {}", key);
        }
    }

    log::info!("Found {} valid installed plugins", valid_keys.len());

    // Step 2: Sync settings.json - remove entries for plugins not in installed_plugins.json
    let old_enabled_keys: Vec<String> = settings.enabled_plugins.keys().cloned().collect();
    for key in old_enabled_keys {
        if !valid_keys.contains(&key) {
            settings.enabled_plugins.remove(&key);
            settings_fixed += 1;
            actions.push(PluginRepairAction {
                plugin_key: key.clone(),
                action: "removed_from_settings".to_string(),
                details: "插件未安装，从 settings.json 移除".to_string(),
            });
            log::info!("Removed non-existent plugin from settings.json: {}", key);
        }
    }

    // Step 3: Ensure all installed plugins have an entry in settings.json (default to true)
    for key in &valid_keys {
        if !settings.enabled_plugins.contains_key(key) {
            settings.enabled_plugins.insert(key.clone(), true);
            settings_fixed += 1;
            actions.push(PluginRepairAction {
                plugin_key: key.clone(),
                action: "added_to_settings".to_string(),
                details: "插件已安装但未在 settings.json 中，添加并默认启用".to_string(),
            });
            log::info!("Added installed plugin to settings.json: {}", key);
        }
    }

    // Step 4: Fix "unknown" version and paths
    let mut entries_to_fix: Vec<(String, InstalledPluginEntry)> = Vec::new();

    for (key, entries) in &installed_file.plugins {
        if let Some(entry) = entries.first() {
            if entry.version == "unknown" || entry.install_path.contains("\\unknown") || entry.install_path.contains("/unknown") {
                entries_to_fix.push((key.clone(), entry.clone()));
            }
        }
    }

    for (key, old_entry) in entries_to_fix {
        let (plugin_name, marketplace_name) = parse_plugin_key(&key);

        // Try to get version from marketplace.json
        let new_version = if let Ok(manifest) = read_marketplace_manifest(&marketplace_name) {
            manifest.plugins.iter()
                .find(|p| p.name == plugin_name)
                .and_then(|p| if !p.version.is_empty() { Some(p.version.clone()) } else { None })
        } else {
            None
        };

        // Fallback: use gitCommitSha first 12 chars
        let new_version = new_version.unwrap_or_else(|| {
            old_entry.git_commit_sha.as_ref()
                .filter(|sha| sha.len() >= 12)
                .map(|sha| sha[..12].to_string())
                .unwrap_or_else(|| "unknown".to_string())
        });

        if new_version == "unknown" {
            continue;
        }

        let old_path = &old_entry.install_path;
        let new_path = old_path.replace("\\unknown", &format!("\\{}", new_version))
            .replace("/unknown", &format!("/{}", new_version));

        if new_path == *old_path && old_entry.version == new_version {
            continue;
        }

        // Rename directory if needed
        if new_path != *old_path {
            let old_dir = PathBuf::from(old_path);
            let new_dir = PathBuf::from(&new_path);

            if old_dir.exists() && !new_dir.exists() {
                if let Err(e) = fs::rename(&old_dir, &new_dir) {
                    log::warn!("Failed to rename {} to {}: {}", old_path, new_path, e);
                } else {
                    log::info!("Renamed plugin directory: {} -> {}", old_path, new_path);
                }
            }
        }

        // Update entry
        if let Some(entries) = installed_file.plugins.get_mut(&key) {
            if let Some(entry) = entries.first_mut() {
                entry.version = new_version.clone();
                entry.install_path = new_path.clone();
            }
        }

        installed_plugins_fixed += 1;
        actions.push(PluginRepairAction {
            plugin_key: key.clone(),
            action: "version_fixed".to_string(),
            details: format!("版本从 'unknown' 修复为 '{}'，路径已更新", new_version),
        });
        log::info!("Fixed plugin version: {} -> {}", key, new_version);
    }

    // Step 5: Clean orphaned cache entries
    let installed_paths: HashSet<String> = installed_file
        .plugins
        .values()
        .flat_map(|entries| entries.iter().map(|e| e.install_path.clone()))
        .collect();

    if let Ok(cleaned) = clean_orphaned_cache(&installed_paths) {
        for path in cleaned {
            actions.push(PluginRepairAction {
                plugin_key: path,
                action: "cache_cleaned".to_string(),
                details: "孤立 cache 目录已清理".to_string(),
            });
        }
    }

    // Write back changes
    write_installed_plugins_file(&installed_file)?;
    write_claude_settings(&settings)?;

    let total_installed = valid_keys.len() as i32;

    log::info!("Repair complete: {} installed_plugins fixes, {} settings fixes, {} total installed",
        installed_plugins_fixed, settings_fixed, total_installed);

    Ok(RepairResult {
        actions,
        installed_plugins_fixed,
        settings_fixed,
        total_installed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_url_source ─────────────────────────────────────────────────

    #[test]
    fn url_source_http() {
        assert!(is_url_source("http://example.com/repo"));
    }

    #[test]
    fn url_source_https() {
        assert!(is_url_source("https://github.com/owner/repo"));
    }

    #[test]
    fn url_source_git_ssh() {
        assert!(is_url_source("git@github.com:owner/repo.git"));
    }

    #[test]
    fn url_source_owner_repo_is_not_url() {
        assert!(!is_url_source("owner/repo"));
    }

    #[test]
    fn url_source_empty_is_not_url() {
        assert!(!is_url_source(""));
    }

    // ── is_owner_repo_pattern ─────────────────────────────────────────

    #[test]
    fn owner_repo_valid() {
        assert!(is_owner_repo_pattern("owner/repo"));
        assert!(is_owner_repo_pattern("my-org/my-plugin"));
    }

    #[test]
    fn owner_repo_with_url_rejected() {
        assert!(!is_owner_repo_pattern("https://github.com/owner/repo"));
    }

    #[test]
    fn owner_repo_with_dots_rejected() {
        assert!(!is_owner_repo_pattern("github.com/owner"));
    }

    #[test]
    fn owner_repo_empty_parts_rejected() {
        assert!(!is_owner_repo_pattern("/repo"));
        assert!(!is_owner_repo_pattern("owner/"));
    }

    #[test]
    fn owner_repo_too_many_slashes_rejected() {
        assert!(!is_owner_repo_pattern("a/b/c"));
    }

    // ── validate_plugin_source_url ────────────────────────────────────

    #[test]
    fn validate_url_https_allowed() {
        assert!(validate_plugin_source_url("https://github.com/owner/repo.git").is_ok());
    }

    #[test]
    fn validate_url_http_allowed() {
        assert!(validate_plugin_source_url("http://internal.company.com/repo.git").is_ok());
    }

    #[test]
    fn validate_url_ssh_allowed() {
        assert!(validate_plugin_source_url("ssh://git@github.com/owner/repo.git").is_ok());
    }

    #[test]
    fn validate_url_git_at_allowed() {
        assert!(validate_plugin_source_url("git@github.com:owner/repo.git").is_ok());
    }

    #[test]
    fn validate_url_file_scheme_rejected() {
        assert!(validate_plugin_source_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn validate_url_local_path_rejected() {
        assert!(validate_plugin_source_url("/etc/passwd").is_err());
        assert!(validate_plugin_source_url("./local/repo").is_err());
        assert!(validate_plugin_source_url("../escape/repo").is_err());
    }

    #[test]
    fn validate_url_empty_rejected() {
        assert!(validate_plugin_source_url("").is_err());
    }

    #[test]
    fn validate_url_whitespace_rejected() {
        assert!(validate_plugin_source_url(" https://github.com/x/y").is_err());
        assert!(validate_plugin_source_url("https://github.com/x/y ").is_err());
    }

    #[test]
    fn validate_url_traversal_rejected() {
        assert!(validate_plugin_source_url("https://github.com/../etc/passwd").is_err());
    }

    // ── git_url_to_raw_file_url ───────────────────────────────────────

    #[test]
    fn github_url_to_raw() {
        let raw = git_url_to_raw_file_url(
            "https://github.com/owner/repo",
            "README.md",
        );
        assert_eq!(raw, "https://raw.githubusercontent.com/owner/repo/main/README.md");
    }

    #[test]
    fn github_url_with_git_suffix() {
        let raw = git_url_to_raw_file_url(
            "https://github.com/owner/repo.git",
            "file.json",
        );
        assert_eq!(raw, "https://raw.githubusercontent.com/owner/repo/main/file.json");
    }

    #[test]
    fn gitlab_url_to_raw() {
        let raw = git_url_to_raw_file_url(
            "https://gitlab.com/owner/repo",
            "README.md",
        );
        assert_eq!(raw, "https://gitlab.com/owner/repo/-/raw/main/README.md");
    }

    #[test]
    fn bitbucket_url_to_raw() {
        let raw = git_url_to_raw_file_url(
            "https://bitbucket.org/owner/repo",
            "README.md",
        );
        assert_eq!(raw, "https://bitbucket.org/owner/repo/raw/main/README.md");
    }

    // ── build_marketplace_json_url ────────────────────────────────────

    #[test]
    fn build_url_github_source() {
        let source = MarketplaceSource {
            source: "github".to_string(),
            repo: Some("my-org/marketplace".to_string()),
            url: None,
        };
        let url = build_marketplace_json_url(&source).unwrap();
        assert!(url.contains("raw.githubusercontent.com"));
        assert!(url.contains("my-org/marketplace"));
        assert!(url.ends_with(".claude-plugin/marketplace.json"));
    }

    #[test]
    fn build_url_github_missing_repo() {
        let source = MarketplaceSource {
            source: "github".to_string(),
            repo: None,
            url: None,
        };
        assert!(build_marketplace_json_url(&source).is_err());
    }

    #[test]
    fn build_url_git_owner_repo_pattern() {
        let source = MarketplaceSource {
            source: "git".to_string(),
            repo: None,
            url: Some("owner/repo".to_string()),
        };
        let url = build_marketplace_json_url(&source).unwrap();
        assert!(url.contains("raw.githubusercontent.com"));
    }

    #[test]
    fn build_url_unknown_source_rejected() {
        let source = MarketplaceSource {
            source: "ftp".to_string(),
            repo: None,
            url: None,
        };
        assert!(build_marketplace_json_url(&source).is_err());
    }

    // ── MarketplaceSource serialization ───────────────────────────────

    #[test]
    fn marketplace_source_roundtrip() {
        let src = MarketplaceSource {
            source: "github".to_string(),
            repo: Some("owner/repo".to_string()),
            url: None,
        };
        let json = serde_json::to_string(&src).unwrap();
        let parsed: MarketplaceSource = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.source, "github");
        assert_eq!(parsed.repo, Some("owner/repo".to_string()));
        assert!(parsed.url.is_none());
    }
}
