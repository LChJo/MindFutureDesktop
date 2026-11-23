use crate::marketplace::read_installed_plugins_file;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub name: String,
    pub title: String,
    pub description: String,
    pub path: String,
}

/// Get the skills directory path
fn get_skills_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Failed to get home directory")?;
    Ok(home.join(".claude").join("skills"))
}

/// Parse SKILL.md to extract title and description
fn parse_skill_md(content: &str) -> (String, String) {
    let lines: Vec<&str> = content.lines().collect();
    let mut title = String::new();
    let mut description = String::new();
    let mut in_description = false;

    for line in lines {
        let trimmed = line.trim();

        // Title is the first # heading
        if title.is_empty() && trimmed.starts_with("# ") {
            title = trimmed.trim_start_matches("# ").to_string();
            in_description = true;
            continue;
        }

        // Description is the first non-empty paragraph after title
        if in_description {
            if trimmed.is_empty() && !description.is_empty() {
                break;
            }
            if !trimmed.is_empty() && !trimmed.starts_with("#") && !trimmed.starts_with("---") {
                if !description.is_empty() {
                    description.push(' ');
                }
                description.push_str(trimmed);
            }
        }
    }

    // Truncate description if too long
    if description.len() > 200 {
        description = description.chars().take(200).collect::<String>() + "...";
    }

    (title, description)
}

/// Get all skills from the skills directory
#[tauri::command]
pub fn get_local_skills() -> Result<Vec<SkillInfo>, String> {
    let skills_dir = get_skills_dir()?;

    if !skills_dir.exists() {
        return Ok(Vec::new());
    }

    let mut skills = Vec::new();

    let entries = fs::read_dir(&skills_dir)
        .map_err(|e| format!("Failed to read skills directory: {}", e))?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        // Skip if not a directory
        if !path.is_dir() {
            continue;
        }

        // Check for SKILL.md
        let skill_md_path = path.join("SKILL.md");
        if !skill_md_path.exists() {
            continue;
        }

        // Read and parse SKILL.md
        let content = match fs::read_to_string(&skill_md_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let (title, description) = parse_skill_md(&content);

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        skills.push(SkillInfo {
            name: name.clone(),
            title: if title.is_empty() { name } else { title },
            description,
            path: path.to_string_lossy().to_string(),
        });
    }

    // Sort by name
    skills.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(skills)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub name: String,
    pub title: String,
    pub description: String,
    pub file_path: String,
    pub dir_path: String,
}

/// Get the agents directory path
fn get_agents_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Failed to get home directory")?;
    Ok(home.join(".claude").join("agents"))
}

/// Parse agent .md file to extract title and description
fn parse_agent_md(content: &str) -> (String, String) {
    let lines: Vec<&str> = content.lines().collect();
    let mut title = String::new();
    let mut description = String::new();
    let mut in_description = false;

    for line in lines {
        let trimmed = line.trim();

        // Title is the first # heading
        if title.is_empty() && trimmed.starts_with("# ") {
            title = trimmed.trim_start_matches("# ").to_string();
            in_description = true;
            continue;
        }

        // Description is the first non-empty paragraph after title
        if in_description {
            if trimmed.is_empty() && !description.is_empty() {
                break;
            }
            if !trimmed.is_empty() && !trimmed.starts_with("#") && !trimmed.starts_with("---") {
                if !description.is_empty() {
                    description.push(' ');
                }
                description.push_str(trimmed);
            }
        }
    }

    // Truncate description if too long
    if description.len() > 200 {
        description = description.chars().take(200).collect::<String>() + "...";
    }

    (title, description)
}

/// Get all agents from the agents directory
#[tauri::command]
pub fn get_local_agents() -> Result<Vec<AgentInfo>, String> {
    let agents_dir = get_agents_dir()?;

    if !agents_dir.exists() {
        return Ok(Vec::new());
    }

    let mut agents = Vec::new();

    let entries = fs::read_dir(&agents_dir)
        .map_err(|e| format!("Failed to read agents directory: {}", e))?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        // Only process .md files
        if !path.is_file() {
            continue;
        }

        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if extension.to_lowercase() != "md" {
            continue;
        }

        // Read and parse the .md file
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let (title, description) = parse_agent_md(&content);

        let name = path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        agents.push(AgentInfo {
            name: name.clone(),
            title: if title.is_empty() { name } else { title },
            description,
            file_path: path.to_string_lossy().to_string(),
            dir_path: agents_dir.to_string_lossy().to_string(),
        });
    }

    // Sort by name
    agents.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(agents)
}

/// Plugin agent info (from marketplace plugins)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAgentInfo {
    pub name: String,
    pub title: String,
    pub description: String,
    pub plugin_name: String,
    pub marketplace_name: String,
}

/// Get all agents from installed plugins (from installed_plugins.json)
#[tauri::command]
pub fn get_plugin_agents() -> Result<Vec<PluginAgentInfo>, String> {
    // Read installed plugins from installed_plugins.json
    let installed = read_installed_plugins_file()?;

    let mut agents = Vec::new();
    let mut seen_agents: HashSet<String> = HashSet::new();

    for (plugin_key, entries) in &installed.plugins {
        let (plugin_name, marketplace_name) = parse_plugin_key(plugin_key);

        // Get the first entry's install_path
        let install_path = match entries.first() {
            Some(e) => PathBuf::from(&e.install_path),
            None => continue,
        };

        if !install_path.exists() {
            continue;
        }

        // Look for agents in install_path/agents/
        let agents_dir = install_path.join("agents");
        if agents_dir.exists() && agents_dir.is_dir() {
            collect_agents_from_dir(
                &agents_dir,
                &marketplace_name,
                &plugin_name,
                &mut agents,
                &mut seen_agents,
            );
        }
    }

    // Sort by name
    agents.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(agents)
}
/// Helper to collect agents from an agents directory
fn collect_agents_from_dir(
    agents_dir: &PathBuf,
    marketplace_name: &str,
    plugin_name: &str,
    agents: &mut Vec<PluginAgentInfo>,
    seen_agents: &mut std::collections::HashSet<String>,
) {
    if let Ok(agent_entries) = fs::read_dir(agents_dir) {
        for agent_entry in agent_entries {
            let agent_entry = match agent_entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let agent_path = agent_entry.path();
            if !agent_path.is_file() {
                continue;
            }

            let extension = agent_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if extension.to_lowercase() != "md" {
                continue;
            }

            let agent_name = agent_path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Create unique key to avoid duplicates
            let unique_key = format!("{}@{}", agent_name, marketplace_name);
            if seen_agents.contains(&unique_key) {
                continue;
            }
            seen_agents.insert(unique_key);

            // Read and parse the .md file
            let content = match fs::read_to_string(&agent_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let (title, description) = parse_agent_md(&content);

            agents.push(PluginAgentInfo {
                name: agent_name.clone(),
                title: if title.is_empty() { agent_name } else { title },
                description,
                plugin_name: plugin_name.to_string(),
                marketplace_name: marketplace_name.to_string(),
            });
        }
    }
}

/// Plugin skill info (from marketplace plugins)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSkillInfo {
    pub name: String,
    pub title: String,
    pub description: String,
    pub plugin_name: String,
    pub marketplace_name: String,
    pub plugin_path: String,
}

/// Get all skills from installed plugins (from installed_plugins.json)
#[tauri::command]
pub fn get_plugin_skills() -> Result<Vec<PluginSkillInfo>, String> {
    // Read installed plugins from installed_plugins.json
    let installed = read_installed_plugins_file()?;

    let mut skills = Vec::new();
    let mut seen_skills: HashSet<String> = HashSet::new();

    for (plugin_key, entries) in &installed.plugins {
        let (plugin_name, marketplace_name) = parse_plugin_key(plugin_key);

        // Get the first entry's install_path
        let install_path = match entries.first() {
            Some(e) => PathBuf::from(&e.install_path),
            None => continue,
        };

        if !install_path.exists() {
            continue;
        }

        // Get install_path as string for plugin_path
        let plugin_path_str = install_path.to_string_lossy().to_string();

        // Check if the root directory itself is a skill (has SKILL.md)
        let root_skill_md = install_path.join("SKILL.md");
        if root_skill_md.exists() {
            collect_root_skill(
                &install_path,
                &marketplace_name,
                &plugin_name,
                &plugin_path_str,
                &mut skills,
                &mut seen_skills,
            );
        }

        // Look for skills in install_path/skills/
        let skills_dir = install_path.join("skills");
        if skills_dir.exists() && skills_dir.is_dir() {
            collect_skills_from_dir(
                &skills_dir,
                &marketplace_name,
                &plugin_name,
                &plugin_path_str,
                &mut skills,
                &mut seen_skills,
            );
        }
    }

    // Sort by name
    skills.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(skills)
}

/// Collect skill from root directory (when install_path itself is a skill)
fn collect_root_skill(
    skill_path: &PathBuf,
    marketplace_name: &str,
    plugin_name: &str,
    plugin_path: &str,
    skills: &mut Vec<PluginSkillInfo>,
    seen_skills: &mut HashSet<String>,
) {
    let skill_md_path = skill_path.join("SKILL.md");
    if !skill_md_path.exists() {
        return;
    }

    let skill_name = skill_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    let unique_key = format!("{}@{}", skill_name, marketplace_name);
    if seen_skills.contains(&unique_key) {
        return;
    }
    seen_skills.insert(unique_key);

    let content = match fs::read_to_string(&skill_md_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let (title, description) = parse_skill_md(&content);

    skills.push(PluginSkillInfo {
        name: skill_name.clone(),
        title: if title.is_empty() { skill_name } else { title },
        description,
        marketplace_name: marketplace_name.to_string(),
        plugin_name: plugin_name.to_string(),
        plugin_path: plugin_path.to_string(),
    });
}
/// Helper to collect skills from a skills directory
fn collect_skills_from_dir(
    skills_dir: &PathBuf,
    marketplace_name: &str,
    plugin_name: &str,
    plugin_path: &str,
    skills: &mut Vec<PluginSkillInfo>,
    seen_skills: &mut std::collections::HashSet<String>,
) {
    if let Ok(skill_entries) = fs::read_dir(skills_dir) {
        for skill_entry in skill_entries {
            let skill_entry = match skill_entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let skill_path = skill_entry.path();
            if !skill_path.is_dir() {
                continue;
            }

            let skill_md_path = skill_path.join("SKILL.md");
            if !skill_md_path.exists() {
                continue;
            }

            let skill_name = skill_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Create unique key to avoid duplicates
            let unique_key = format!("{}@{}", skill_name, marketplace_name);
            if seen_skills.contains(&unique_key) {
                continue;
            }
            seen_skills.insert(unique_key);

            // Read and parse SKILL.md
            let content = match fs::read_to_string(&skill_md_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let (title, description) = parse_skill_md(&content);

            skills.push(PluginSkillInfo {
                name: skill_name.clone(),
                title: if title.is_empty() { skill_name } else { title },
                description,
                plugin_name: plugin_name.to_string(),
                marketplace_name: marketplace_name.to_string(),
                plugin_path: plugin_path.to_string(),
            });
        }
    }
}

/// Open a directory in the system file manager
#[tauri::command]
pub fn open_directory(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        std::process::Command::new("explorer")
            .arg(&path)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("Failed to open directory: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open directory: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open directory: {}", e))?;
    }

    Ok(())
}
