//! Configuration and persistence for Coworks
//!
//! Manages chat histories, tasks, and configuration stored in ~/.MoontonClaudeDesktop/coworks/

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Get the coworks data directory (~/.MoontonClaudeDesktop/coworks/)
fn get_coworks_data_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Failed to get home directory")?;
    let data_dir = home.join(".MoontonClaudeDesktop").join("coworks");

    // Ensure directory exists
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create coworks data directory: {}", e))?;
    }

    Ok(data_dir)
}

/// Get subdirectory within coworks data dir
fn get_subdir(name: &str) -> Result<PathBuf, String> {
    let base = get_coworks_data_dir()?;
    let subdir = base.join(name);

    if !subdir.exists() {
        fs::create_dir_all(&subdir)
            .map_err(|e| format!("Failed to create {} directory: {}", name, e))?;
    }

    Ok(subdir)
}

// ============================================================================
// Code Project Session Management (for SDK session persistence)
// ============================================================================

/// Code project session for Code mode SDK session persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeProjectSession {
    #[serde(rename = "projectPath")]
    pub project_path: String,
    #[serde(rename = "sdkSessionId")]
    pub sdk_session_id: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Simple hash function for project path (used as filename)
fn simple_hash(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Save a project session
#[tauri::command]
pub fn coworks_save_project_session(
    project_path: String,
    sdk_session_id: String,
    model: Option<String>,
) -> Result<(), String> {
    let projects_dir = get_subdir("projects")?;

    // Use hash of project path as filename
    let hash = simple_hash(&project_path);
    let file_path = projects_dir.join(format!("{}.json", hash));

    let session = CodeProjectSession {
        project_path,
        sdk_session_id,
        updated_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        model,
    };

    let content = serde_json::to_string_pretty(&session)
        .map_err(|e| format!("Failed to serialize project session: {}", e))?;
    fs::write(&file_path, content)
        .map_err(|e| format!("Failed to write project session: {}", e))?;

    Ok(())
}

/// List all project sessions
#[tauri::command]
pub fn coworks_list_project_sessions() -> Result<Vec<CodeProjectSession>, String> {
    let projects_dir = get_subdir("projects")?;

    let mut sessions = Vec::new();

    if let Ok(entries) = fs::read_dir(&projects_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(session) = serde_json::from_str::<CodeProjectSession>(&content) {
                            sessions.push(session);
                        }
                    }
                }
            }
        }
    }

    Ok(sessions)
}

// ============================================================================
// Chat History Management
// ============================================================================

/// Chat history index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatHistoryIndexEntry {
    pub id: String,
    pub title: String,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    #[serde(rename = "updatedAt")]
    pub updated_at: u64,
}

/// Full chat history with messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatHistory {
    pub id: String,
    pub title: String,
    pub messages: Vec<serde_json::Value>,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    #[serde(rename = "updatedAt")]
    pub updated_at: u64,
    #[serde(rename = "sdkSessionId", skip_serializing_if = "Option::is_none")]
    pub sdk_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// List all chat histories (index only)
#[tauri::command]
pub fn coworks_list_chat_histories() -> Result<Vec<ChatHistoryIndexEntry>, String> {
    let chats_dir = get_subdir("chats")?;
    let index_path = chats_dir.join("index.json");

    if !index_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&index_path)
        .map_err(|e| format!("Failed to read chat index: {}", e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse chat index: {}", e))
}

/// Load a specific chat history
#[tauri::command]
pub fn coworks_load_chat_history(id: String) -> Result<ChatHistory, String> {
    let chats_dir = get_subdir("chats")?;
    let chat_path = chats_dir.join(format!("{}.json", id));

    if !chat_path.exists() {
        return Err(format!("Chat history not found: {}", id));
    }

    let content = fs::read_to_string(&chat_path)
        .map_err(|e| format!("Failed to read chat history: {}", e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse chat history: {}", e))
}

/// Save a chat history
#[tauri::command]
pub fn coworks_save_chat_history(history: ChatHistory) -> Result<(), String> {
    let chats_dir = get_subdir("chats")?;

    // Save the chat file
    let chat_path = chats_dir.join(format!("{}.json", history.id));
    let content = serde_json::to_string_pretty(&history)
        .map_err(|e| format!("Failed to serialize chat history: {}", e))?;
    fs::write(&chat_path, content)
        .map_err(|e| format!("Failed to write chat history: {}", e))?;

    // Update the index
    let index_path = chats_dir.join("index.json");
    let mut index: Vec<ChatHistoryIndexEntry> = if index_path.exists() {
        let content = fs::read_to_string(&index_path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    // Update or add entry
    let entry = ChatHistoryIndexEntry {
        id: history.id.clone(),
        title: history.title,
        created_at: history.created_at,
        updated_at: history.updated_at,
    };

    if let Some(pos) = index.iter().position(|e| e.id == history.id) {
        index[pos] = entry;
    } else {
        index.push(entry);
    }

    // Sort by updated_at descending
    index.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    let index_content = serde_json::to_string_pretty(&index)
        .map_err(|e| format!("Failed to serialize chat index: {}", e))?;
    fs::write(&index_path, index_content)
        .map_err(|e| format!("Failed to write chat index: {}", e))?;

    Ok(())
}

/// Delete a chat history
#[tauri::command]
pub fn coworks_delete_chat_history(id: String) -> Result<(), String> {
    let chats_dir = get_subdir("chats")?;

    // Delete the chat file
    let chat_path = chats_dir.join(format!("{}.json", id));
    if chat_path.exists() {
        fs::remove_file(&chat_path)
            .map_err(|e| format!("Failed to delete chat history: {}", e))?;
    }

    // Update the index
    let index_path = chats_dir.join("index.json");
    if index_path.exists() {
        let content = fs::read_to_string(&index_path).unwrap_or_default();
        let mut index: Vec<ChatHistoryIndexEntry> = serde_json::from_str(&content).unwrap_or_default();
        index.retain(|e| e.id != id);

        let index_content = serde_json::to_string_pretty(&index)
            .map_err(|e| format!("Failed to serialize chat index: {}", e))?;
        fs::write(&index_path, index_content)
            .map_err(|e| format!("Failed to write chat index: {}", e))?;
    }

    Ok(())
}

// ============================================================================
// Task Management
// ============================================================================

/// Task index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskIndexEntry {
    pub id: String,
    pub name: String,
    #[serde(rename = "workDir")]
    pub work_dir: String,
    pub status: String,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    #[serde(rename = "updatedAt")]
    pub updated_at: u64,
}

/// Skill info for tasks (stored when task is created with a skill)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSkillInfo {
    #[serde(rename = "skillName")]
    pub skill_name: String,
    #[serde(rename = "skillDescription")]
    pub skill_description: String,
    #[serde(rename = "pluginName")]
    pub plugin_name: String,
    #[serde(rename = "pluginPath")]
    pub plugin_path: String,
    #[serde(rename = "marketplaceName")]
    pub marketplace_name: String,
}

/// Full task data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoworksTask {
    pub id: String,
    pub name: String,
    #[serde(rename = "appId")]
    pub app_id: Option<String>,
    #[serde(rename = "workDir")]
    pub work_dir: String,
    pub messages: Vec<serde_json::Value>,
    #[serde(rename = "relatedFiles")]
    pub related_files: Vec<serde_json::Value>,
    #[serde(rename = "workLogs")]
    pub work_logs: Vec<serde_json::Value>,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    #[serde(rename = "updatedAt")]
    pub updated_at: u64,
    pub status: String,
    #[serde(rename = "skillInfo", skip_serializing_if = "Option::is_none")]
    pub skill_info: Option<TaskSkillInfo>,
    #[serde(rename = "sdkSessionId", skip_serializing_if = "Option::is_none")]
    pub sdk_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// List all tasks (index only)
#[tauri::command]
pub fn coworks_list_tasks() -> Result<Vec<TaskIndexEntry>, String> {
    let tasks_dir = get_subdir("tasks")?;
    let index_path = tasks_dir.join("index.json");

    if !index_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&index_path)
        .map_err(|e| format!("Failed to read task index: {}", e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse task index: {}", e))
}

/// Load a specific task
#[tauri::command]
pub fn coworks_load_task(id: String) -> Result<CoworksTask, String> {
    let tasks_dir = get_subdir("tasks")?;
    let task_path = tasks_dir.join(format!("{}.json", id));

    if !task_path.exists() {
        return Err(format!("Task not found: {}", id));
    }

    let content = fs::read_to_string(&task_path)
        .map_err(|e| format!("Failed to read task: {}", e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse task: {}", e))
}

/// Save a task
#[tauri::command]
pub fn coworks_save_task(task: CoworksTask) -> Result<(), String> {
    let tasks_dir = get_subdir("tasks")?;

    // Save the task file
    let task_path = tasks_dir.join(format!("{}.json", task.id));
    let content = serde_json::to_string_pretty(&task)
        .map_err(|e| format!("Failed to serialize task: {}", e))?;
    fs::write(&task_path, content)
        .map_err(|e| format!("Failed to write task: {}", e))?;

    // Update the index
    let index_path = tasks_dir.join("index.json");
    let mut index: Vec<TaskIndexEntry> = if index_path.exists() {
        let content = fs::read_to_string(&index_path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    // Update or add entry
    let entry = TaskIndexEntry {
        id: task.id.clone(),
        name: task.name,
        work_dir: task.work_dir,
        status: task.status,
        created_at: task.created_at,
        updated_at: task.updated_at,
    };

    if let Some(pos) = index.iter().position(|e| e.id == task.id) {
        index[pos] = entry;
    } else {
        index.push(entry);
    }

    // Sort by updated_at descending
    index.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    let index_content = serde_json::to_string_pretty(&index)
        .map_err(|e| format!("Failed to serialize task index: {}", e))?;
    fs::write(&index_path, index_content)
        .map_err(|e| format!("Failed to write task index: {}", e))?;

    Ok(())
}

/// Delete a task
#[tauri::command]
pub fn coworks_delete_task_item(id: String) -> Result<(), String> {
    let tasks_dir = get_subdir("tasks")?;

    // Delete the task file
    let task_path = tasks_dir.join(format!("{}.json", id));
    if task_path.exists() {
        fs::remove_file(&task_path)
            .map_err(|e| format!("Failed to delete task: {}", e))?;
    }

    // Update the index
    let index_path = tasks_dir.join("index.json");
    if index_path.exists() {
        let content = fs::read_to_string(&index_path).unwrap_or_default();
        let mut index: Vec<TaskIndexEntry> = serde_json::from_str(&content).unwrap_or_default();
        index.retain(|e| e.id != id);

        let index_content = serde_json::to_string_pretty(&index)
            .map_err(|e| format!("Failed to serialize task index: {}", e))?;
        fs::write(&index_path, index_content)
            .map_err(|e| format!("Failed to write task index: {}", e))?;
    }

    Ok(())
}

// ============================================================================
// Recent Workspaces Management
// ============================================================================

/// Recent workspace entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentWorkspace {
    pub path: String,
    pub name: String,
    #[serde(rename = "lastOpened")]
    pub last_opened: u64,
}

/// Get recent workspaces for a mode
#[tauri::command]
pub fn coworks_get_recent_workspaces(mode: String) -> Result<Vec<RecentWorkspace>, String> {
    let data_dir = get_coworks_data_dir()?;
    let file_path = data_dir.join(format!("recent-{}.json", mode));

    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read recent workspaces: {}", e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse recent workspaces: {}", e))
}

/// Save recent workspaces for a mode
#[tauri::command]
pub fn coworks_save_recent_workspaces(mode: String, workspaces: Vec<RecentWorkspace>) -> Result<(), String> {
    let data_dir = get_coworks_data_dir()?;
    let file_path = data_dir.join(format!("recent-{}.json", mode));

    let content = serde_json::to_string_pretty(&workspaces)
        .map_err(|e| format!("Failed to serialize recent workspaces: {}", e))?;

    fs::write(&file_path, content)
        .map_err(|e| format!("Failed to write recent workspaces: {}", e))?;

    Ok(())
}

// ============================================================================
// Claude Apps Conversation Management
// ============================================================================

/// Claude Apps conversation index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeAppConversationIndex {
    pub id: String,
    pub title: String,
    #[serde(rename = "appName", skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    #[serde(rename = "updatedAt")]
    pub updated_at: u64,
}

/// Full Claude Apps conversation with messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeAppConversation {
    pub id: String,
    pub title: String,
    #[serde(rename = "appName", skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
    pub messages: Vec<serde_json::Value>,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    #[serde(rename = "updatedAt")]
    pub updated_at: u64,
    #[serde(rename = "sdkSessionId", skip_serializing_if = "Option::is_none")]
    pub sdk_session_id: Option<String>,
}

/// List all Claude Apps conversations (index only)
#[tauri::command]
pub fn claude_apps_list_conversations() -> Result<Vec<ClaudeAppConversationIndex>, String> {
    let chats_dir = get_subdir("claude-apps-chats")?;
    let index_path = chats_dir.join("index.json");

    if !index_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&index_path)
        .map_err(|e| format!("Failed to read claude-apps index: {}", e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse claude-apps index: {}", e))
}

/// Load a specific Claude Apps conversation
#[tauri::command]
pub fn claude_apps_load_conversation(id: String) -> Result<ClaudeAppConversation, String> {
    let chats_dir = get_subdir("claude-apps-chats")?;
    let chat_path = chats_dir.join(format!("{}.json", id));

    if !chat_path.exists() {
        return Err(format!("Claude Apps conversation not found: {}", id));
    }

    let content = fs::read_to_string(&chat_path)
        .map_err(|e| format!("Failed to read claude-apps conversation: {}", e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse claude-apps conversation: {}", e))
}

/// Save a Claude Apps conversation
#[tauri::command]
pub fn claude_apps_save_conversation(conversation: ClaudeAppConversation) -> Result<(), String> {
    let chats_dir = get_subdir("claude-apps-chats")?;

    // Save the conversation file
    let chat_path = chats_dir.join(format!("{}.json", conversation.id));
    let content = serde_json::to_string_pretty(&conversation)
        .map_err(|e| format!("Failed to serialize claude-apps conversation: {}", e))?;
    fs::write(&chat_path, content)
        .map_err(|e| format!("Failed to write claude-apps conversation: {}", e))?;

    // Update the index
    let index_path = chats_dir.join("index.json");
    let mut index: Vec<ClaudeAppConversationIndex> = if index_path.exists() {
        let content = fs::read_to_string(&index_path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    // Update or add entry
    let entry = ClaudeAppConversationIndex {
        id: conversation.id.clone(),
        title: conversation.title,
        app_name: conversation.app_name,
        created_at: conversation.created_at,
        updated_at: conversation.updated_at,
    };

    if let Some(pos) = index.iter().position(|e| e.id == conversation.id) {
        index[pos] = entry;
    } else {
        index.push(entry);
    }

    // Sort by updated_at descending
    index.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    let index_content = serde_json::to_string_pretty(&index)
        .map_err(|e| format!("Failed to serialize claude-apps index: {}", e))?;
    fs::write(&index_path, index_content)
        .map_err(|e| format!("Failed to write claude-apps index: {}", e))?;

    Ok(())
}

/// Delete a Claude Apps conversation
#[tauri::command]
pub fn claude_apps_delete_conversation(id: String) -> Result<(), String> {
    let chats_dir = get_subdir("claude-apps-chats")?;

    // Delete the conversation file
    let chat_path = chats_dir.join(format!("{}.json", id));
    if chat_path.exists() {
        fs::remove_file(&chat_path)
            .map_err(|e| format!("Failed to delete claude-apps conversation: {}", e))?;
    }

    // Update the index
    let index_path = chats_dir.join("index.json");
    if index_path.exists() {
        let content = fs::read_to_string(&index_path).unwrap_or_default();
        let mut index: Vec<ClaudeAppConversationIndex> = serde_json::from_str(&content).unwrap_or_default();
        index.retain(|e| e.id != id);

        let index_content = serde_json::to_string_pretty(&index)
            .map_err(|e| format!("Failed to serialize claude-apps index: {}", e))?;
        fs::write(&index_path, index_content)
            .map_err(|e| format!("Failed to write claude-apps index: {}", e))?;
    }

    Ok(())
}

// ============================================================================
// Configuration
// ============================================================================

/// Coworks configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoworksConfig {
    #[serde(rename = "defaultModel")]
    pub default_model: Option<String>,
    #[serde(rename = "lastMode")]
    pub last_mode: Option<String>,
}

/// Get coworks configuration
#[tauri::command]
pub fn coworks_get_config() -> Result<CoworksConfig, String> {
    let data_dir = get_coworks_data_dir()?;
    let config_path = data_dir.join("config.json");

    if !config_path.exists() {
        return Ok(CoworksConfig::default());
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config: {}", e))
}

/// Save coworks configuration
#[tauri::command]
pub fn coworks_save_config(config: CoworksConfig) -> Result<(), String> {
    let data_dir = get_coworks_data_dir()?;
    let config_path = data_dir.join("config.json");

    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&config_path, content)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}
