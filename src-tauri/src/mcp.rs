use crate::marketplace::{read_installed_plugins_file, read_claude_settings};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Parse plugin key "pluginName@marketplaceName" into (plugin_name, marketplace_name)
fn parse_plugin_key(key: &str) -> (String, String) {
    if let Some(at_pos) = key.rfind('@') {
        (key[..at_pos].to_string(), key[at_pos + 1..].to_string())
    } else {
        (key.to_string(), "unknown".to_string())
    }
}

/// MCP Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub server_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Global MCP server info for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalMcpInfo {
    pub name: String,
    pub config: McpServerConfig,
    pub enabled: bool,
    pub source: String, // "global" or "plugin:{marketplace_name}"
}

/// Project MCP server info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMcpInfo {
    pub name: String,
    pub config: McpServerConfig,
    pub enabled: bool,
}

/// Project info with MCP servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub path: String,
    pub mcp_servers: Vec<ProjectMcpInfo>,
}

/// Get the path to ~/.claude.json
fn get_claude_json_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Failed to get home directory")?;
    Ok(home.join(".claude.json"))
}

/// Read and parse ~/.claude.json
fn read_claude_json() -> Result<Value, String> {
    let path = get_claude_json_path()?;
    if !path.exists() {
        return Ok(json!({}));
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read .claude.json: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse .claude.json: {}", e))
}

/// Write ~/.claude.json
fn write_claude_json(value: &Value) -> Result<(), String> {
    let path = get_claude_json_path()?;
    let content = serde_json::to_string_pretty(value)
        .map_err(|e| format!("Failed to serialize .claude.json: {}", e))?;
    fs::write(&path, content)
        .map_err(|e| format!("Failed to write .claude.json: {}", e))
}

/// Get global MCP servers from ~/.claude.json
#[tauri::command]
pub fn get_global_mcp_servers() -> Result<Vec<GlobalMcpInfo>, String> {
    let claude_json = read_claude_json()?;
    let mut result = Vec::new();

    // Get enabled/disabled lists from root level (for global MCPs)
    let _enabled_list: Vec<String> = claude_json
        .get("enabledMcpjsonServers")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let disabled_list: Vec<String> = claude_json
        .get("disabledMcpjsonServers")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    // Get mcpServers from root level
    if let Some(mcp_servers) = claude_json.get("mcpServers").and_then(|v| v.as_object()) {
        for (name, config) in mcp_servers {
            let server_config: McpServerConfig = serde_json::from_value(config.clone())
                .unwrap_or(McpServerConfig {
                    command: None,
                    args: None,
                    env: None,
                    server_type: None,
                    url: None,
                });

            // Check if enabled or disabled
            let enabled = if disabled_list.contains(name) {
                false
            } else {
                true // Default to enabled if not in disabled list
            };

            result.push(GlobalMcpInfo {
                name: name.clone(),
                config: server_config,
                enabled,
                source: "global".to_string(),
            });
        }
    }

    Ok(result)
}

/// Get MCP servers from installed plugins (from installed_plugins.json)
/// Only returns MCPs from enabled plugins, deduplicates by name within each marketplace
#[tauri::command]
pub fn get_plugin_mcp_servers() -> Result<Vec<GlobalMcpInfo>, String> {
    // Read installed plugins from installed_plugins.json
    let installed = read_installed_plugins_file()?;

    // Read enabled plugins from settings.json
    let settings = read_claude_settings()?;
    let enabled_plugins = &settings.enabled_plugins;

    let mut result = Vec::new();
    // Track seen MCP names per marketplace to deduplicate
    let mut seen_per_marketplace: HashMap<String, std::collections::HashSet<String>> = HashMap::new();

    for (plugin_key, entries) in &installed.plugins {
        // Check if plugin is enabled (default to false if not in settings)
        let is_enabled = enabled_plugins.get(plugin_key).copied().unwrap_or(false);
        if !is_enabled {
            continue;
        }

        let (_plugin_name, marketplace_name) = parse_plugin_key(plugin_key);

        // Get the first entry's install_path
        let install_path = match entries.first() {
            Some(e) => PathBuf::from(&e.install_path),
            None => continue,
        };

        if !install_path.exists() {
            continue;
        }

        // Get or create the seen set for this marketplace
        let seen = seen_per_marketplace.entry(marketplace_name.clone()).or_default();

        // Find all .mcp.json files recursively in this plugin's install_path
        collect_mcp_files(&install_path, &marketplace_name, &mut result, seen);
    }

    Ok(result)
}

/// Recursively collect .mcp.json files from a directory
/// Deduplicates by name within the same marketplace using the `seen` set
fn collect_mcp_files(dir: &PathBuf, marketplace_name: &str, result: &mut Vec<GlobalMcpInfo>, seen: &mut std::collections::HashSet<String>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();

            if path.is_dir() {
                collect_mcp_files(&path, marketplace_name, result, seen);
            } else if path.file_name().and_then(|n| n.to_str()) == Some(".mcp.json") {
                // Parse .mcp.json file
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(mcp_json) = serde_json::from_str::<HashMap<String, Value>>(&content) {
                        for (name, config) in mcp_json {
                            // Skip if already seen in this marketplace
                            if seen.contains(&name) {
                                continue;
                            }
                            seen.insert(name.clone());

                            let server_config: McpServerConfig = serde_json::from_value(config)
                                .unwrap_or(McpServerConfig {
                                    command: None,
                                    args: None,
                                    env: None,
                                    server_type: None,
                                    url: None,
                                });

                            result.push(GlobalMcpInfo {
                                name,
                                config: server_config,
                                enabled: true, // Plugin MCPs are managed by plugin enable/disable
                                source: format!("plugin:{}", marketplace_name),
                            });
                        }
                    }
                }
            }
        }
    }
}

/// Get all projects with their MCP configurations
#[tauri::command]
pub fn get_projects_with_mcp() -> Result<Vec<ProjectInfo>, String> {
    let claude_json = read_claude_json()?;
    let mut result = Vec::new();

    if let Some(projects) = claude_json.get("projects").and_then(|v| v.as_object()) {
        for (path, project_config) in projects {
            let mut mcp_servers = Vec::new();

            // Get enabled/disabled lists for this project
            let _enabled_list: Vec<String> = project_config
                .get("enabledMcpjsonServers")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let disabled_list: Vec<String> = project_config
                .get("disabledMcpjsonServers")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();

            // Get mcpServers for this project
            if let Some(servers) = project_config.get("mcpServers").and_then(|v| v.as_object()) {
                for (name, config) in servers {
                    let server_config: McpServerConfig = serde_json::from_value(config.clone())
                        .unwrap_or(McpServerConfig {
                            command: None,
                            args: None,
                            env: None,
                            server_type: None,
                            url: None,
                        });

                    let enabled = if disabled_list.contains(name) {
                        false
                    } else {
                        true
                    };

                    mcp_servers.push(ProjectMcpInfo {
                        name: name.clone(),
                        config: server_config,
                        enabled,
                    });
                }
            }

            // Only include projects that have MCP servers
            if !mcp_servers.is_empty() {
                result.push(ProjectInfo {
                    path: path.clone(),
                    mcp_servers,
                });
            }
        }
    }

    Ok(result)
}

/// Toggle global MCP server enabled/disabled
#[tauri::command]
pub fn toggle_global_mcp(name: String, enabled: bool) -> Result<(), String> {
    let mut claude_json = read_claude_json()?;

    // Get or create enabled/disabled lists
    let mut enabled_list: Vec<String> = claude_json
        .get("enabledMcpjsonServers")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let mut disabled_list: Vec<String> = claude_json
        .get("disabledMcpjsonServers")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    if enabled {
        // Remove from disabled, add to enabled
        disabled_list.retain(|n| n != &name);
        if !enabled_list.contains(&name) {
            enabled_list.push(name);
        }
    } else {
        // Remove from enabled, add to disabled
        enabled_list.retain(|n| n != &name);
        if !disabled_list.contains(&name) {
            disabled_list.push(name);
        }
    }

    claude_json["enabledMcpjsonServers"] = json!(enabled_list);
    claude_json["disabledMcpjsonServers"] = json!(disabled_list);

    write_claude_json(&claude_json)
}

/// Toggle project MCP server enabled/disabled
#[tauri::command]
pub fn toggle_project_mcp(project_path: String, name: String, enabled: bool) -> Result<(), String> {
    let mut claude_json = read_claude_json()?;

    // Get the project config
    let projects = claude_json
        .get_mut("projects")
        .and_then(|v| v.as_object_mut())
        .ok_or("No projects found")?;

    let project = projects
        .get_mut(&project_path)
        .and_then(|v| v.as_object_mut())
        .ok_or("Project not found")?;

    // Get or create enabled/disabled lists
    let mut enabled_list: Vec<String> = project
        .get("enabledMcpjsonServers")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let mut disabled_list: Vec<String> = project
        .get("disabledMcpjsonServers")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    if enabled {
        disabled_list.retain(|n| n != &name);
        if !enabled_list.contains(&name) {
            enabled_list.push(name);
        }
    } else {
        enabled_list.retain(|n| n != &name);
        if !disabled_list.contains(&name) {
            disabled_list.push(name);
        }
    }

    project.insert("enabledMcpjsonServers".to_string(), json!(enabled_list));
    project.insert("disabledMcpjsonServers".to_string(), json!(disabled_list));

    write_claude_json(&claude_json)
}

/// Delete a global MCP server
#[tauri::command]
pub fn delete_global_mcp(name: String) -> Result<(), String> {
    let mut claude_json = read_claude_json()?;

    // Remove from mcpServers
    if let Some(mcp_servers) = claude_json.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
        mcp_servers.remove(&name);
    }

    // Also remove from enabled/disabled lists
    if let Some(enabled) = claude_json.get_mut("enabledMcpjsonServers").and_then(|v| v.as_array_mut()) {
        enabled.retain(|v| v.as_str() != Some(&name));
    }
    if let Some(disabled) = claude_json.get_mut("disabledMcpjsonServers").and_then(|v| v.as_array_mut()) {
        disabled.retain(|v| v.as_str() != Some(&name));
    }

    write_claude_json(&claude_json)
}

/// Delete a project MCP server
#[tauri::command]
pub fn delete_project_mcp(project_path: String, name: String) -> Result<(), String> {
    let mut claude_json = read_claude_json()?;

    let projects = claude_json
        .get_mut("projects")
        .and_then(|v| v.as_object_mut())
        .ok_or("No projects found")?;

    let project = projects
        .get_mut(&project_path)
        .and_then(|v| v.as_object_mut())
        .ok_or("Project not found")?;

    // Remove from mcpServers
    if let Some(mcp_servers) = project.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
        mcp_servers.remove(&name);
    }

    // Also remove from enabled/disabled lists
    if let Some(enabled) = project.get_mut("enabledMcpjsonServers").and_then(|v| v.as_array_mut()) {
        enabled.retain(|v| v.as_str() != Some(&name));
    }
    if let Some(disabled) = project.get_mut("disabledMcpjsonServers").and_then(|v| v.as_array_mut()) {
        disabled.retain(|v| v.as_str() != Some(&name));
    }

    write_claude_json(&claude_json)
}

/// Add a new global MCP server
#[tauri::command]
pub fn add_global_mcp(name: String, config: McpServerConfig) -> Result<(), String> {
    let mut claude_json = read_claude_json()?;

    // Ensure mcpServers exists
    if claude_json.get("mcpServers").is_none() {
        claude_json["mcpServers"] = json!({});
    }

    // Add the new server
    let mcp_servers = claude_json
        .get_mut("mcpServers")
        .and_then(|v| v.as_object_mut())
        .ok_or("Failed to access mcpServers")?;

    let config_value = serde_json::to_value(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    mcp_servers.insert(name, config_value);

    write_claude_json(&claude_json)
}

/// Add a new project MCP server
#[tauri::command]
pub fn add_project_mcp(project_path: String, name: String, config: McpServerConfig) -> Result<(), String> {
    let mut claude_json = read_claude_json()?;

    // Ensure projects exists
    if claude_json.get("projects").is_none() {
        claude_json["projects"] = json!({});
    }

    let projects = claude_json
        .get_mut("projects")
        .and_then(|v| v.as_object_mut())
        .ok_or("Failed to access projects")?;

    // Ensure the project exists
    if !projects.contains_key(&project_path) {
        projects.insert(project_path.clone(), json!({}));
    }

    let project = projects
        .get_mut(&project_path)
        .and_then(|v| v.as_object_mut())
        .ok_or("Failed to access project")?;

    // Ensure mcpServers exists in project
    if !project.contains_key("mcpServers") {
        project.insert("mcpServers".to_string(), json!({}));
    }

    let mcp_servers = project
        .get_mut("mcpServers")
        .and_then(|v| v.as_object_mut())
        .ok_or("Failed to access project mcpServers")?;

    let config_value = serde_json::to_value(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    mcp_servers.insert(name, config_value);

    write_claude_json(&claude_json)
}

/// Get all known projects (even those without MCP servers)
#[tauri::command]
pub fn get_all_projects() -> Result<Vec<String>, String> {
    let claude_json = read_claude_json()?;
    let mut result = Vec::new();

    if let Some(projects) = claude_json.get("projects").and_then(|v| v.as_object()) {
        for path in projects.keys() {
            result.push(path.clone());
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_plugin_key ──────────────────────────────────────────────

    #[test]
    fn parse_plugin_key_with_at_sign() {
        let (name, marketplace) = parse_plugin_key("my-plugin@marketplace-a");
        assert_eq!(name, "my-plugin");
        assert_eq!(marketplace, "marketplace-a");
    }

    #[test]
    fn parse_plugin_key_multiple_at_signs() {
        // Should split on the LAST '@'
        let (name, marketplace) = parse_plugin_key("@scope/plugin@marketplace");
        assert_eq!(name, "@scope/plugin");
        assert_eq!(marketplace, "marketplace");
    }

    #[test]
    fn parse_plugin_key_no_at_sign() {
        let (name, marketplace) = parse_plugin_key("standalone-plugin");
        assert_eq!(name, "standalone-plugin");
        assert_eq!(marketplace, "unknown");
    }

    #[test]
    fn parse_plugin_key_empty_string() {
        let (name, marketplace) = parse_plugin_key("");
        assert_eq!(name, "");
        assert_eq!(marketplace, "unknown");
    }

    #[test]
    fn parse_plugin_key_at_end() {
        let (name, marketplace) = parse_plugin_key("plugin@");
        assert_eq!(name, "plugin");
        assert_eq!(marketplace, "");
    }

    // ── McpServerConfig serialization roundtrip ───────────────────────

    #[test]
    fn mcp_server_config_roundtrip() {
        let config = McpServerConfig {
            command: Some("node".to_string()),
            args: Some(vec!["server.js".to_string()]),
            env: Some(HashMap::from([("PORT".to_string(), "3000".to_string())])),
            server_type: None,
            url: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: McpServerConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.command, Some("node".to_string()));
        assert_eq!(parsed.args.as_ref().unwrap().len(), 1);
        assert!(parsed.server_type.is_none());
        assert!(parsed.url.is_none());
    }

    #[test]
    fn mcp_server_config_with_url_type() {
        let json = r#"{"type": "sse", "url": "http://localhost:3000/sse"}"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.server_type, Some("sse".to_string()));
        assert_eq!(config.url, Some("http://localhost:3000/sse".to_string()));
        assert!(config.command.is_none());
    }

    #[test]
    fn mcp_server_config_empty() {
        let json = r#"{}"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();
        assert!(config.command.is_none());
        assert!(config.args.is_none());
        assert!(config.env.is_none());
        assert!(config.server_type.is_none());
        assert!(config.url.is_none());
    }
}
