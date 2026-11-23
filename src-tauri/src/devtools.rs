use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{Emitter, State};

// ============================================================
// JSONL Entry Types (raw data from Claude Code session files)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageMetadata {
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessageInner {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub content: serde_json::Value,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub usage: Option<UsageMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonlEntry {
    #[serde(default)]
    pub uuid: Option<String>,
    #[serde(rename = "type", default)]
    pub entry_type: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(rename = "parentUuid", default)]
    pub parent_uuid: Option<String>,
    #[serde(rename = "isSidechain", default)]
    pub is_sidechain: Option<bool>,
    #[serde(rename = "isMeta", default)]
    pub is_meta: Option<bool>,
    #[serde(default)]
    pub message: Option<serde_json::Value>,
    #[serde(rename = "sessionId", default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(rename = "gitBranch", default)]
    pub git_branch: Option<String>,
    #[serde(rename = "agentId", default)]
    pub agent_id: Option<String>,
    #[serde(rename = "toolUseResult", default)]
    pub tool_use_result: Option<serde_json::Value>,
    #[serde(rename = "requestId", default)]
    pub request_id: Option<String>,
    #[serde(rename = "durationMs", default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub content: Option<serde_json::Value>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(rename = "parentToolUseID", default)]
    pub parent_tool_use_id: Option<String>,
    #[serde(rename = "toolUseID", default)]
    pub tool_use_id: Option<String>,
}

// ============================================================
// Parsed / Domain Types (sent to frontend)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevToolsProject {
    pub id: String,
    pub path: String,
    pub name: String,
    pub session_count: usize,
    pub most_recent: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub project_id: String,
    pub created_at: u64,
    pub first_message: Option<String>,
    pub message_count: usize,
    pub has_subagents: bool,
    pub is_ongoing: bool,
    pub git_branch: Option<String>,
    pub metrics: SessionMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionMetrics {
    pub duration_ms: u64,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub message_count: usize,
    pub tool_call_count: usize,
    pub cost_usd: f64,
    pub tool_error_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBreakdown {
    pub total: f64,
    pub input_cost: f64,
    pub output_cost: f64,
    pub cache_read_cost: f64,
    pub cache_creation_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedMessage {
    pub uuid: String,
    pub parent_uuid: Option<String>,
    pub message_type: String,
    pub timestamp: String,
    pub role: Option<String>,
    pub text_content: String,
    pub usage: Option<UsageMetadata>,
    pub model: Option<String>,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub is_sidechain: bool,
    pub is_meta: bool,
    pub tool_calls: Vec<ToolCallInfo>,
    pub tool_results: Vec<ToolResultInfo>,
    pub cost_usd: Option<f64>,
    pub cost_breakdown: Option<CostBreakdown>,
    pub subtype: Option<String>,
    pub message_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProgress {
    pub output_lines: usize,
    pub elapsed_ms: u64,
    pub data_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
    pub is_task: bool,
    pub task_description: Option<String>,
    pub progress: Option<ToolProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultInfo {
    pub tool_use_id: String,
    pub content: serde_json::Value,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentInfo {
    pub id: String,
    pub file_path: String,
    pub description: Option<String>,
    pub parent_task_id: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub duration_ms: u64,
    pub metrics: SessionMetrics,
    pub messages: Vec<ParsedMessage>,
    pub subagent_type: Option<String>,
    pub is_parallel: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDetail {
    pub session: SessionSummary,
    pub messages: Vec<ParsedMessage>,
    pub subagents: Vec<SubagentInfo>,
    pub metrics: SessionMetrics,
}

// ============================================================
// Model Pricing
// ============================================================

struct ModelPricing {
    input_cost_per_token: f64,
    output_cost_per_token: f64,
    cache_creation_cost: f64,
    cache_read_cost: f64,
}

fn get_model_pricing(model: &str) -> Option<ModelPricing> {
    let model_lower = model.to_lowercase();
    // Match known model patterns
    if model_lower.contains("opus") {
        Some(ModelPricing {
            input_cost_per_token: 0.000005,   // $5/M tokens
            output_cost_per_token: 0.000025,  // $25/M tokens
            cache_creation_cost: 0.00000625,  // $6.25/M tokens
            cache_read_cost: 0.0000005,       // $0.50/M tokens
        })
    } else if model_lower.contains("sonnet") {
        Some(ModelPricing {
            input_cost_per_token: 0.000003,
            output_cost_per_token: 0.000015,
            cache_creation_cost: 0.00000375,
            cache_read_cost: 0.0000003,
        })
    } else if model_lower.contains("haiku") {
        Some(ModelPricing {
            input_cost_per_token: 0.000001,
            output_cost_per_token: 0.000005,
            cache_creation_cost: 0.00000125,
            cache_read_cost: 0.0000001,
        })
    } else {
        // Default to Sonnet pricing
        Some(ModelPricing {
            input_cost_per_token: 0.000003,
            output_cost_per_token: 0.000015,
            cache_creation_cost: 0.00000375,
            cache_read_cost: 0.0000003,
        })
    }
}

fn calculate_cost_breakdown(
    model: &str,
    input: u64,
    output: u64,
    cache_read: u64,
    cache_create: u64,
) -> CostBreakdown {
    let p = get_model_pricing(model).unwrap_or(ModelPricing {
        input_cost_per_token: 0.000003,
        output_cost_per_token: 0.000015,
        cache_creation_cost: 0.00000375,
        cache_read_cost: 0.0000003,
    });
    let input_cost = input as f64 * p.input_cost_per_token;
    let output_cost = output as f64 * p.output_cost_per_token;
    let cache_read_cost = cache_read as f64 * p.cache_read_cost;
    let cache_creation_cost = cache_create as f64 * p.cache_creation_cost;
    CostBreakdown {
        total: input_cost + output_cost + cache_read_cost + cache_creation_cost,
        input_cost,
        output_cost,
        cache_read_cost,
        cache_creation_cost,
    }
}

// ============================================================
// Path Decoding
// ============================================================

fn decode_project_path(encoded: &str) -> String {
    if encoded.is_empty() {
        return String::new();
    }

    // Handle composite IDs: encoded_path::hex
    let base = if let Some(idx) = encoded.find("::") {
        &encoded[..idx]
    } else {
        encoded
    };

    // Legacy Windows format: C--Users-name-project
    if base.len() >= 3
        && base.chars().next().map_or(false, |c| c.is_ascii_alphabetic())
        && base.chars().nth(1) == Some('-')
        && base.chars().nth(2) == Some('-')
    {
        let drive = base.chars().next().unwrap().to_uppercase().to_string();
        let rest = base[3..].replace('-', "/");
        return format!("{}:/{}", drive, rest);
    }

    // Standard format: -Users-name-project or -I:-AI-project
    let without_leading = if base.starts_with('-') {
        &base[1..]
    } else {
        base
    };

    let decoded = without_leading.replace('-', "/");

    // Windows drive paths: I:/AI/project
    if decoded.len() >= 3
        && decoded.chars().next().map_or(false, |c| c.is_ascii_alphabetic())
        && decoded.chars().nth(1) == Some(':')
    {
        return decoded;
    }

    // POSIX path
    if decoded.starts_with('/') {
        decoded
    } else {
        format!("/{}", decoded)
    }
}

fn extract_project_name(encoded: &str, cwd_hint: Option<&str>) -> String {
    if let Some(cwd) = cwd_hint {
        if let Some(name) = cwd
            .split(&['/', '\\'][..])
            .filter(|s| !s.is_empty())
            .last()
        {
            return name.to_string();
        }
    }
    let decoded = decode_project_path(encoded);
    decoded
        .split('/')
        .filter(|s| !s.is_empty())
        .last()
        .unwrap_or(encoded)
        .to_string()
}

// ============================================================
// JSONL Parsing
// ============================================================

fn extract_text_content(content: &serde_json::Value) -> String {
    match content {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => {
            let mut texts = Vec::new();
            for block in arr {
                if let Some(block_type) = block.get("type").and_then(|t| t.as_str()) {
                    match block_type {
                        "text" => {
                            if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                texts.push(text.to_string());
                            }
                        }
                        "thinking" => {
                            if let Some(text) = block.get("thinking").and_then(|t| t.as_str()) {
                                texts.push(format!("[thinking] {}", text));
                            }
                        }
                        _ => {}
                    }
                }
            }
            texts.join("\n")
        }
        _ => String::new(),
    }
}

fn extract_tool_calls(content: &serde_json::Value) -> Vec<ToolCallInfo> {
    let arr = match content.as_array() {
        Some(a) => a,
        None => return Vec::new(),
    };

    arr.iter()
        .filter_map(|block| {
            let block_type = block.get("type")?.as_str()?;
            if block_type != "tool_use" {
                return None;
            }
            let id = block.get("id")?.as_str()?.to_string();
            let name = block.get("name")?.as_str()?.to_string();
            let input = block.get("input").cloned().unwrap_or(serde_json::Value::Null);
            let is_task = name == "Task";
            let task_description = if is_task {
                input
                    .get("description")
                    .and_then(|d| d.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        input
                            .get("prompt")
                            .and_then(|p| p.as_str())
                            .map(|s| truncate_str(s, 200))
                    })
            } else {
                None
            };
            Some(ToolCallInfo {
                id,
                name,
                input,
                is_task,
                task_description,
                progress: None,
            })
        })
        .collect()
}

fn extract_tool_results(content: &serde_json::Value) -> Vec<ToolResultInfo> {
    let arr = match content.as_array() {
        Some(a) => a,
        None => return Vec::new(),
    };

    arr.iter()
        .filter_map(|block| {
            let block_type = block.get("type")?.as_str()?;
            if block_type != "tool_result" {
                return None;
            }
            let tool_use_id = block.get("tool_use_id")?.as_str()?.to_string();
            let content = block.get("content").cloned().unwrap_or(serde_json::Value::Null);
            let is_error = block
                .get("is_error")
                .and_then(|e| e.as_bool())
                .unwrap_or(false);
            Some(ToolResultInfo {
                tool_use_id,
                content,
                is_error,
            })
        })
        .collect()
}

fn parse_entry(entry: JsonlEntry) -> Option<ParsedMessage> {
    let entry_type = entry.entry_type.as_deref()?;

    // Skip non-conversational types
    match entry_type {
        "user" | "assistant" | "system" | "summary" => {}
        _ => return None,
    }

    let uuid = entry.uuid.unwrap_or_default();
    let timestamp = entry.timestamp.unwrap_or_default();
    let is_sidechain = entry.is_sidechain.unwrap_or(false);
    let is_meta = entry.is_meta.unwrap_or(false);

    let (role, text_content, usage, model, tool_calls, tool_results, message_id) = match entry_type {
        "assistant" => {
            if let Some(msg_val) = &entry.message {
                let msg: AssistantMessageInner =
                    serde_json::from_value(msg_val.clone()).unwrap_or(AssistantMessageInner {
                        role: Some("assistant".to_string()),
                        model: None,
                        id: None,
                        content: serde_json::Value::Null,
                        stop_reason: None,
                        usage: None,
                    });
                let message_id = msg.id.clone();
                let text = extract_text_content(&msg.content);
                let tc = extract_tool_calls(&msg.content);
                (
                    Some("assistant".to_string()),
                    text,
                    msg.usage,
                    msg.model,
                    tc,
                    Vec::new(),
                    message_id,
                )
            } else {
                return None;
            }
        }
        "user" => {
            if let Some(msg_val) = &entry.message {
                let content = if let Some(obj) = msg_val.as_object() {
                    obj.get("content")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null)
                } else {
                    msg_val.clone()
                };
                let text = extract_text_content(&content);
                let tr = extract_tool_results(&content);
                (Some("user".to_string()), text, None, None, Vec::new(), tr, None)
            } else {
                (
                    Some("user".to_string()),
                    String::new(),
                    None,
                    None,
                    Vec::new(),
                    Vec::new(),
                    None,
                )
            }
        }
        "system" => {
            let subtype = entry.subtype.as_deref().unwrap_or("unknown");
            let text = match subtype {
                "turn_duration" => {
                    let dur = entry.duration_ms.unwrap_or(0);
                    if dur >= 60000 {
                        format!("Turn completed · {}m {}s", dur / 60000, (dur % 60000) / 1000)
                    } else {
                        format!("Turn completed · {:.1}s", dur as f64 / 1000.0)
                    }
                }
                "api_error" => "API Error (retrying)".to_string(),
                "compact_boundary" => "Context compacted".to_string(),
                "microcompact_boundary" => "Context micro-compacted".to_string(),
                "stop_hook_summary" => "Hooks executed".to_string(),
                "local_command" => {
                    entry.content
                        .as_ref()
                        .and_then(|c| c.as_str())
                        .and_then(|s| {
                            s.split_once("<command-name>")
                                .and_then(|(_, rest)| rest.split_once("</command-name>"))
                                .map(|(cmd, _)| format!("Command: {}", cmd))
                        })
                        .unwrap_or_else(|| "Local command".to_string())
                }
                _ => format!("System: {}", subtype),
            };
            (
                Some("system".to_string()),
                text,
                None,
                None,
                Vec::new(),
                Vec::new(),
                None,
            )
        }
        "summary" => {
            let text = entry.summary.clone().unwrap_or_default();
            if text.is_empty() {
                return None;
            }
            (
                Some("summary".to_string()),
                text,
                None,
                None,
                Vec::new(),
                Vec::new(),
                None,
            )
        }
        _ => return None,
    };

    Some(ParsedMessage {
        uuid,
        parent_uuid: entry.parent_uuid,
        message_type: entry_type.to_string(),
        timestamp,
        role,
        text_content,
        usage,
        model,
        cwd: entry.cwd,
        git_branch: entry.git_branch,
        is_sidechain,
        is_meta,
        tool_calls,
        tool_results,
        cost_usd: None,
        cost_breakdown: None,
        subtype: entry.subtype,
        message_id,
    })
}

/// Merge assistant messages that share the same `message_id`.
/// Claude Code writes one JSONL line per content block (thinking, text, tool_use, …)
/// for the same API response. Each line carries cumulative usage — output_tokens
/// grows as streaming progresses. Only the LAST entry has the final/complete usage,
/// so we always overwrite usage with the latest value during merge.
fn merge_assistant_messages(messages: Vec<ParsedMessage>) -> Vec<ParsedMessage> {
    use std::collections::HashMap;

    let mut result: Vec<ParsedMessage> = Vec::new();
    let mut id_to_idx: HashMap<String, usize> = HashMap::new();

    for msg in messages {
        if msg.message_type == "assistant" {
            if let Some(ref mid) = msg.message_id {
                if let Some(&idx) = id_to_idx.get(mid) {
                    let existing = &mut result[idx];
                    // Append non-empty text
                    let new_text = msg.text_content.trim();
                    if !new_text.is_empty() {
                        if !existing.text_content.is_empty() {
                            existing.text_content.push('\n');
                        }
                        existing.text_content.push_str(new_text);
                    }
                    // Merge tool_calls (deduplicate by id)
                    for tc in msg.tool_calls {
                        if !existing.tool_calls.iter().any(|t| t.id == tc.id) {
                            existing.tool_calls.push(tc);
                        }
                    }
                    // Always overwrite usage with the latest (last entry has final cumulative values)
                    if msg.usage.is_some() {
                        existing.usage = msg.usage;
                    }
                    if msg.model.is_some() {
                        existing.model = msg.model;
                    }
                    continue;
                }
                id_to_idx.insert(mid.clone(), result.len());
            }
        }
        result.push(msg);
    }
    result
}

fn parse_jsonl_file(file_path: &Path) -> Result<Vec<ParsedMessage>, String> {
    use std::collections::HashMap;

    let file =
        std::fs::File::open(file_path).map_err(|e| format!("Failed to open {:?}: {}", file_path, e))?;
    let reader = BufReader::new(file);
    let mut messages = Vec::new();

    // Collect progress data keyed by parentToolUseID
    let mut progress_map: HashMap<String, ToolProgress> = HashMap::new();

    for line in reader.lines() {
        let line = line.map_err(|e| format!("Read error: {}", e))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match serde_json::from_str::<JsonlEntry>(trimmed) {
            Ok(entry) => {
                let entry_type = entry.entry_type.as_deref().unwrap_or("");

                if entry_type == "progress" {
                    // Aggregate progress data by parentToolUseID
                    if let Some(parent_id) = &entry.parent_tool_use_id {
                        let data_type = entry.data
                            .as_ref()
                            .and_then(|d| d.get("type"))
                            .and_then(|t| t.as_str())
                            .unwrap_or("unknown")
                            .to_string();

                        let elapsed = entry.data
                            .as_ref()
                            .and_then(|d| d.get("elapsedTime"))
                            .and_then(|e| e.as_u64())
                            .unwrap_or(0);

                        let output_lines = entry.data
                            .as_ref()
                            .and_then(|d| d.get("output"))
                            .and_then(|o| o.as_str())
                            .map(|s| s.lines().count())
                            .unwrap_or(0);

                        let prog = progress_map
                            .entry(parent_id.clone())
                            .or_insert_with(|| ToolProgress {
                                output_lines: 0,
                                elapsed_ms: 0,
                                data_type: data_type.clone(),
                            });
                        if output_lines > prog.output_lines {
                            prog.output_lines = output_lines;
                        }
                        if elapsed > prog.elapsed_ms {
                            prog.elapsed_ms = elapsed;
                        }
                        if prog.data_type == "unknown" && data_type != "unknown" {
                            prog.data_type = data_type;
                        }
                    }
                } else if let Some(msg) = parse_entry(entry) {
                    messages.push(msg);
                }
            }
            Err(_) => {
                // Skip malformed lines silently
            }
        }
    }

    // Merge assistant entries sharing the same message.id into one ParsedMessage
    let mut messages = merge_assistant_messages(messages);

    // Associate progress data with tool calls
    if !progress_map.is_empty() {
        for msg in &mut messages {
            for tc in &mut msg.tool_calls {
                if let Some(prog) = progress_map.remove(&tc.id) {
                    tc.progress = Some(prog);
                }
            }
        }
    }

    Ok(messages)
}

// ============================================================
// Metrics Calculation
// ============================================================

fn calculate_and_enrich_metrics(messages: &mut [ParsedMessage]) -> SessionMetrics {
    let mut metrics = SessionMetrics::default();
    metrics.message_count = messages.len();

    // Collect timestamps with O(1) clones (immutable pass)
    let first_timestamp = messages.iter()
        .find(|m| !m.timestamp.is_empty())
        .map(|m| m.timestamp.clone());
    let last_timestamp = messages.iter()
        .rev()
        .find(|m| !m.timestamp.is_empty())
        .map(|m| m.timestamp.clone());

    // Mutable pass: enrich each message with cost data and accumulate metrics
    let mut last_model = String::from("claude-sonnet-4-5-20250514");

    for msg in messages.iter_mut() {
        if let Some(model) = &msg.model {
            last_model = model.clone();
        }

        if let Some(usage) = &msg.usage {
            let input = usage.input_tokens.unwrap_or(0);
            let output = usage.output_tokens.unwrap_or(0);
            let cache_read = usage.cache_read_input_tokens.unwrap_or(0);
            let cache_create = usage.cache_creation_input_tokens.unwrap_or(0);

            metrics.input_tokens += input;
            metrics.output_tokens += output;
            metrics.cache_read_tokens += cache_read;
            metrics.cache_creation_tokens += cache_create;
            metrics.total_tokens += input + output;

            let breakdown = calculate_cost_breakdown(
                &last_model, input, output, cache_read, cache_create,
            );
            metrics.cost_usd += breakdown.total;
            msg.cost_usd = Some(breakdown.total);
            msg.cost_breakdown = Some(breakdown);
        }

        metrics.tool_call_count += msg.tool_calls.len();

        for tr in &msg.tool_results {
            if tr.is_error {
                metrics.tool_error_count += 1;
            }
        }
    }

    // Calculate duration from timestamps
    if let (Some(first), Some(last)) = (&first_timestamp, &last_timestamp) {
        if let (Ok(start), Ok(end)) = (
            chrono::DateTime::parse_from_rfc3339(first),
            chrono::DateTime::parse_from_rfc3339(last),
        ) {
            let duration = end.signed_duration_since(start);
            metrics.duration_ms = duration.num_milliseconds().max(0) as u64;
        }
    }

    metrics
}

// ============================================================
// Quick Scan (for session list - avoid full parse)
// ============================================================

fn quick_scan_session(file_path: &Path, project_id: &str) -> Result<SessionSummary, String> {
    let file =
        std::fs::File::open(file_path).map_err(|e| format!("Failed to open {:?}: {}", file_path, e))?;
    let reader = BufReader::new(file);

    let session_id = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut first_message: Option<String> = None;
    let mut message_count: usize = 0;
    let mut has_subagents = false;
    let mut git_branch: Option<String> = None;
    let mut first_timestamp: Option<String> = None;
    let mut last_timestamp: Option<String> = None;
    let mut total_input: u64 = 0;
    let mut total_output: u64 = 0;
    let mut total_cache_read: u64 = 0;
    let mut total_cache_create: u64 = 0;
    let mut tool_call_count: usize = 0;
    let mut tool_error_count: usize = 0;
    let mut last_model = String::from("claude-sonnet-4-5-20250514");
    let mut cost: f64 = 0.0;
    // Track latest usage per message_id (output_tokens is cumulative during streaming,
    // so only the last entry per message_id has the final/complete usage).
    let mut seen_assistant_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut assistant_usage: std::collections::HashMap<String, (u64, u64, u64, u64, String)> =
        std::collections::HashMap::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let entry: JsonlEntry = match serde_json::from_str(trimmed) {
            Ok(e) => e,
            Err(_) => continue,
        };

        let entry_type = match entry.entry_type.as_deref() {
            Some(t) => t.to_string(),
            None => continue,
        };

        match entry_type.as_str() {
            "user" | "system" | "summary" => {
                message_count += 1;
            }
            "assistant" => {
                // message_count for assistant is handled below after dedup
            }
            _ => continue,
        }

        if let Some(ts) = &entry.timestamp {
            if first_timestamp.is_none() {
                first_timestamp = Some(ts.clone());
            }
            last_timestamp = Some(ts.clone());
        }

        if git_branch.is_none() {
            if let Some(gb) = &entry.git_branch {
                git_branch = Some(gb.clone());
            }
        }

        // Process user messages: extract first message + count tool errors
        if entry_type == "user" {
            if let Some(msg_val) = &entry.message {
                let content = if let Some(obj) = msg_val.as_object() {
                    obj.get("content")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null)
                } else {
                    msg_val.clone()
                };

                // Extract first real user message
                if first_message.is_none() && !entry.is_meta.unwrap_or(false) {
                    let text = extract_text_content(&content);
                    if !text.is_empty() {
                        first_message = Some(truncate_str(&text, 120));
                    }
                }

                // Count tool errors (tool_result with is_error)
                if let Some(arr) = content.as_array() {
                    for block in arr {
                        if block.get("type").and_then(|t| t.as_str()) == Some("tool_result") {
                            if block.get("is_error").and_then(|e| e.as_bool()).unwrap_or(false) {
                                tool_error_count += 1;
                            }
                        }
                    }
                }
            }
        }

        // Check for subagents (Task tool calls in assistant messages)
        if entry_type == "assistant" {
            if let Some(msg_val) = &entry.message {
                let msg_id = msg_val.get("id").and_then(|i| i.as_str()).unwrap_or("");
                let is_new = msg_id.is_empty() || seen_assistant_ids.insert(msg_id.to_string());

                // Count message and tool calls only on first occurrence
                if is_new {
                    message_count += 1;
                }

                if let Some(content) = msg_val.get("content") {
                    if let Some(arr) = content.as_array() {
                        for block in arr {
                            if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                                tool_call_count += 1;
                                if block.get("name").and_then(|n| n.as_str()) == Some("Task") {
                                    has_subagents = true;
                                }
                            }
                        }
                    }
                }

                if let Some(msg_obj) = msg_val.as_object() {
                    if let Some(model) = msg_obj.get("model").and_then(|m| m.as_str()) {
                        last_model = model.to_string();
                    }
                    if let Some(usage) = msg_obj.get("usage") {
                        let input = usage
                            .get("input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let output = usage
                            .get("output_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let cr = usage
                            .get("cache_read_input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let cc = usage
                            .get("cache_creation_input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);

                        if msg_id.is_empty() {
                            // No message_id: accumulate directly
                            total_input += input;
                            total_output += output;
                            total_cache_read += cr;
                            total_cache_create += cc;
                            cost += calculate_cost_breakdown(
                                &last_model, input, output, cr, cc,
                            ).total;
                        } else {
                            // Overwrite: keep the latest (last) usage per message_id
                            assistant_usage.insert(
                                msg_id.to_string(),
                                (input, output, cr, cc, last_model.clone()),
                            );
                        }
                    }
                }
            }
        }
    }

    // Sum final usage from each unique message_id (last entry has complete values)
    for (input, output, cr, cc, model) in assistant_usage.values() {
        total_input += input;
        total_output += output;
        total_cache_read += cr;
        total_cache_create += cc;
        cost += calculate_cost_breakdown(model, *input, *output, *cr, *cc).total;
    }

    // Calculate duration
    let mut duration_ms: u64 = 0;
    if let (Some(first), Some(last)) = (&first_timestamp, &last_timestamp) {
        if let (Ok(start), Ok(end)) = (
            chrono::DateTime::parse_from_rfc3339(first),
            chrono::DateTime::parse_from_rfc3339(last),
        ) {
            let duration = end.signed_duration_since(start);
            duration_ms = duration.num_milliseconds().max(0) as u64;
        }
    }

    // Determine if session is ongoing (last modified within 5 minutes)
    let is_ongoing = file_path
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|modified| {
            modified
                .elapsed()
                .map(|elapsed| elapsed.as_secs() < 300)
                .unwrap_or(false)
        })
        .unwrap_or(false);

    let created_at = first_timestamp
        .as_ref()
        .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
        .map(|dt| dt.timestamp_millis() as u64)
        .unwrap_or_else(|| {
            file_path
                .metadata()
                .ok()
                .and_then(|m| m.created().ok())
                .and_then(|ct| ct.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0)
        });

    Ok(SessionSummary {
        id: session_id,
        project_id: project_id.to_string(),
        created_at,
        first_message,
        message_count,
        has_subagents,
        is_ongoing,
        git_branch,
        metrics: SessionMetrics {
            duration_ms,
            total_tokens: total_input + total_output,
            input_tokens: total_input,
            output_tokens: total_output,
            cache_read_tokens: total_cache_read,
            cache_creation_tokens: total_cache_create,
            message_count,
            tool_call_count,
            cost_usd: cost,
            tool_error_count,
        },
    })
}

// ============================================================
// Subagent Resolution
// ============================================================

fn find_subagent_files(project_dir: &Path, session_id: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();

    // New structure: {session_id}/subagents/agent-*.jsonl
    let new_dir = project_dir.join(session_id).join("subagents");
    if new_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&new_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".jsonl") {
                    files.push(entry.path());
                }
            }
        }
    }

    // Legacy structure: agent-*.jsonl at project root
    if files.is_empty() {
        if let Ok(entries) = std::fs::read_dir(project_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("agent-") && name.ends_with(".jsonl") {
                    files.push(entry.path());
                }
            }
        }
    }

    files
}

fn resolve_subagents(project_dir: &Path, session_id: &str) -> Vec<SubagentInfo> {
    let files = find_subagent_files(project_dir, session_id);
    let mut subagents = Vec::new();

    for file_path in files {
        let agent_id = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        match parse_jsonl_file(&file_path) {
            Ok(mut messages) => {
                let metrics = calculate_and_enrich_metrics(&mut messages);
                let start_time = messages.first().map(|m| m.timestamp.clone());
                let end_time = messages.last().map(|m| m.timestamp.clone());

                // Extract description from first user message
                let description = messages
                    .iter()
                    .find(|m| m.message_type == "user" && !m.is_meta && !m.text_content.is_empty())
                    .map(|m| truncate_str(&m.text_content, 200));

                subagents.push(SubagentInfo {
                    id: agent_id,
                    file_path: file_path.to_string_lossy().to_string(),
                    description,
                    parent_task_id: None,
                    start_time,
                    end_time,
                    duration_ms: metrics.duration_ms,
                    metrics,
                    messages,
                    subagent_type: None,
                    is_parallel: false,
                });
            }
            Err(_) => {}
        }
    }

    subagents
}

// ============================================================
// Subagent Linking
// ============================================================

/// Link subagents to their parent Task tool_calls using 3-phase matching.
fn link_subagents_to_tasks(messages: &[ParsedMessage], subagents: &mut Vec<SubagentInfo>) {
    if subagents.is_empty() {
        return;
    }

    // Collect all Task tool_calls: (tool_use_id, task_description, input_json)
    let mut task_calls: Vec<(String, Option<String>, serde_json::Value)> = Vec::new();
    for msg in messages {
        if msg.message_type != "assistant" {
            continue;
        }
        for tc in &msg.tool_calls {
            if tc.is_task {
                task_calls.push((tc.id.clone(), tc.task_description.clone(), tc.input.clone()));
            }
        }
    }

    if task_calls.is_empty() {
        return;
    }

    // Build set of Task tool_use_ids for quick lookup
    let task_ids: std::collections::HashSet<String> =
        task_calls.iter().map(|(id, _, _)| id.clone()).collect();

    // Phase A: Result-based matching (most precise)
    // Look at tool_results in user messages for agentId references
    let mut agent_id_to_task_id: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for msg in messages {
        if msg.message_type != "user" {
            continue;
        }
        for tr in &msg.tool_results {
            if !task_ids.contains(&tr.tool_use_id) {
                continue;
            }
            // Extract agentId from tool_result content (string, array, or object)
            if let Some(agent_id) = extract_agent_id_from_result(&tr.content) {
                agent_id_to_task_id.insert(agent_id, tr.tool_use_id.clone());
            }
        }
    }

    let mut matched_task_ids: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    // Apply Phase A matches
    for sa in subagents.iter_mut() {
        if let Some(task_id) = agent_id_to_task_id.get(&sa.id) {
            sa.parent_task_id = Some(task_id.clone());
            matched_task_ids.insert(task_id.clone());
        }
    }

    // Phase B: Description-based matching
    let unmatched_tasks: Vec<&(String, Option<String>, serde_json::Value)> = task_calls
        .iter()
        .filter(|(id, _, _)| !matched_task_ids.contains(id))
        .collect();

    for sa in subagents.iter_mut() {
        if sa.parent_task_id.is_some() {
            continue;
        }
        let sa_desc = match &sa.description {
            Some(d) => d.to_lowercase(),
            None => continue,
        };
        for (task_id, task_desc, _) in &unmatched_tasks {
            if matched_task_ids.contains(task_id.as_str()) {
                continue;
            }
            if let Some(td) = task_desc {
                // Require minimum 10 chars to avoid false positives with short descriptions
                if td.len() < 10 {
                    continue;
                }
                let td_lower = td.to_lowercase();
                if sa_desc.starts_with(&td_lower) || td_lower.starts_with(&sa_desc) {
                    sa.parent_task_id = Some(task_id.clone());
                    matched_task_ids.insert(task_id.clone());
                    break;
                }
            }
        }
    }

    // Phase C: Positional fallback — match remaining by order
    let remaining_tasks: Vec<&str> = task_calls
        .iter()
        .filter(|(id, _, _)| !matched_task_ids.contains(id))
        .map(|(id, _, _)| id.as_str())
        .collect();

    let mut remaining_agents: Vec<&mut SubagentInfo> = subagents
        .iter_mut()
        .filter(|sa| sa.parent_task_id.is_none())
        .collect();

    // Sort remaining agents by start_time
    remaining_agents.sort_by(|a, b| a.start_time.cmp(&b.start_time));

    for (sa, task_id) in remaining_agents.iter_mut().zip(remaining_tasks.iter()) {
        sa.parent_task_id = Some(task_id.to_string());
    }

    // Extract subagent_type from linked Task tool_call input
    let task_input_map: std::collections::HashMap<String, serde_json::Value> = task_calls
        .iter()
        .map(|(id, _, input)| (id.clone(), input.clone()))
        .collect();

    for sa in subagents.iter_mut() {
        if let Some(ref task_id) = sa.parent_task_id {
            if let Some(input) = task_input_map.get(task_id) {
                sa.subagent_type = input
                    .get("subagent_type")
                    .and_then(|v| v.as_str())
                    .map(|s| normalize_subagent_type(s));
            }
        }
        // Fallback: infer from description keywords
        if sa.subagent_type.is_none() {
            sa.subagent_type = infer_subagent_type(&sa.description);
        }
    }
}

/// Extract agentId from a Task tool_result content.
/// The content can be a string, array of content blocks, or object.
fn extract_agent_id_from_result(content: &serde_json::Value) -> Option<String> {
    // Case 1: String content — look for "agentId": "..." or agent ID pattern
    if let Some(s) = content.as_str() {
        return extract_agent_id_from_str(s);
    }
    // Case 2: Array of content blocks
    if let Some(arr) = content.as_array() {
        for item in arr {
            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                if let Some(id) = extract_agent_id_from_str(text) {
                    return Some(id);
                }
            }
        }
    }
    // Case 3: Object with "text" field
    if let Some(text) = content.get("text").and_then(|t| t.as_str()) {
        return extract_agent_id_from_str(text);
    }
    None
}

/// Extract agent ID from a text string.
/// Looks for patterns like: agentId: "xxx" or "agent_id":"xxx"
fn extract_agent_id_from_str(s: &str) -> Option<String> {
    // Pattern 1: JSON-like "agentId":"value" or "agent_id":"value"
    for prefix in &["\"agentId\":\"", "\"agentId\": \"", "\"agent_id\":\"", "\"agent_id\": \"", "agentId: "] {
        if let Some(start) = s.find(prefix) {
            let after = &s[start + prefix.len()..];
            let end = after.find(|c: char| c == '"' || c == ',' || c == '}' || c == '\n')
                .unwrap_or(after.len());
            let id = after[..end].trim().to_string();
            if !id.is_empty() {
                return Some(id);
            }
        }
    }
    // Pattern 2: UUID-like agent ID (agent-XXXXXXXX format in the text)
    if let Some(start) = s.find("agent-") {
        let after = &s[start..];
        let end = after.find(|c: char| c.is_whitespace() || c == '"' || c == ',' || c == '}')
            .unwrap_or(after.len());
        let id = after[..end].trim_end_matches(".jsonl").to_string();
        if id.len() > 7 {
            return Some(id);
        }
    }
    None
}

/// Normalize subagent_type to a canonical display name.
fn normalize_subagent_type(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("explore") { return "Explore".to_string(); }
    if lower.contains("plan") { return "Plan".to_string(); }
    if lower.contains("general") { return "Implement".to_string(); }
    if lower.contains("code-review") || lower.contains("reviewer") { return "Review".to_string(); }
    if lower.contains("tdd") || lower.contains("test") { return "Test".to_string(); }
    if lower.contains("build-error") || lower.contains("fix") { return "Fix".to_string(); }
    if lower.contains("security") { return "Review".to_string(); }
    if lower.contains("architect") { return "Plan".to_string(); }
    if lower.contains("refactor") { return "Implement".to_string(); }
    // Return the raw type with first letter capitalized
    let mut chars = raw.chars();
    match chars.next() {
        None => raw.to_string(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

/// Infer subagent type from description keywords.
fn infer_subagent_type(description: &Option<String>) -> Option<String> {
    let desc = description.as_ref()?.to_lowercase();
    if desc.contains("explore") || desc.contains("search") || desc.contains("find") {
        return Some("Explore".to_string());
    }
    if desc.contains("plan") || desc.contains("architect") || desc.contains("design") {
        return Some("Plan".to_string());
    }
    if desc.contains("implement") || desc.contains("build") || desc.contains("create") || desc.contains("write") {
        return Some("Implement".to_string());
    }
    if desc.contains("test") || desc.contains("tdd") || desc.contains("spec") {
        return Some("Test".to_string());
    }
    if desc.contains("review") || desc.contains("audit") || desc.contains("check") {
        return Some("Review".to_string());
    }
    if desc.contains("fix") || desc.contains("resolve") || desc.contains("debug") {
        return Some("Fix".to_string());
    }
    None
}

/// Detect parallel subagents by comparing start_times (≤100ms apart).
/// Uses sorted sliding window for O(n log n) performance.
fn detect_parallel_subagents(subagents: &mut Vec<SubagentInfo>) {
    const PARALLEL_THRESHOLD_MS: i64 = 100;

    if subagents.len() < 2 {
        return;
    }

    // Parse start_times into (index, millis) pairs
    let mut indexed_times: Vec<(usize, i64)> = subagents
        .iter()
        .enumerate()
        .filter_map(|(i, sa)| {
            sa.start_time.as_ref().and_then(|t| {
                chrono::DateTime::parse_from_rfc3339(t)
                    .ok()
                    .map(|dt| (i, dt.timestamp_millis()))
            })
        })
        .collect();

    indexed_times.sort_by_key(|(_, t)| *t);

    // Mark adjacent pairs within threshold
    for window in indexed_times.windows(2) {
        if (window[1].1 - window[0].1).abs() <= PARALLEL_THRESHOLD_MS {
            subagents[window[0].0].is_parallel = true;
            subagents[window[1].0].is_parallel = true;
        }
    }
}

// ============================================================
// Helpers
// ============================================================

fn get_claude_projects_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Failed to get home directory")?;
    Ok(home.join(".claude").join("projects"))
}

fn is_session_file(name: &str) -> bool {
    name.ends_with(".jsonl") && !name.starts_with("agent-") && !name.starts_with(".")
}

/// Safely truncate a string at a character boundary (not byte boundary).
fn truncate_str(s: &str, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => format!("{}...", &s[..idx]),
        None => s.to_string(),
    }
}

/// Disambiguate projects that share the same display name by appending parent path segments.
fn disambiguate_project_names(projects: &mut [DevToolsProject]) {
    use std::collections::HashMap;

    // Count how many projects share each name
    let mut name_counts: HashMap<String, usize> = HashMap::new();
    for p in projects.iter() {
        *name_counts.entry(p.name.clone()).or_insert(0) += 1;
    }

    // For duplicates, append distinguishing parent segment from path
    for p in projects.iter_mut() {
        if name_counts.get(&p.name).copied().unwrap_or(0) > 1 {
            let segments: Vec<&str> = p.path
                .split(&['/', '\\'][..])
                .filter(|s| !s.is_empty())
                .collect();
            // Use second-to-last segment as disambiguator (parent folder)
            if segments.len() >= 2 {
                let parent = segments[segments.len() - 2];
                p.name = format!("{} ({})", p.name, parent);
            }
        }
    }
}

/// Validate that a resolved path stays within the expected base directory.
fn validate_within_base(resolved: &Path, base: &Path) -> Result<(), String> {
    let canonical_base = base
        .canonicalize()
        .map_err(|e| format!("Cannot canonicalize base path: {}", e))?;
    let canonical_resolved = resolved
        .canonicalize()
        .map_err(|e| format!("Invalid path: {}", e))?;
    if !canonical_resolved.starts_with(&canonical_base) {
        return Err("Path traversal detected".to_string());
    }
    Ok(())
}

// ============================================================
// Build Summary From Parsed Messages (avoids double file read)
// ============================================================

fn build_summary_from_messages(
    messages: &[ParsedMessage],
    file_path: &Path,
    project_id: &str,
    metrics: &SessionMetrics,
) -> Result<SessionSummary, String> {
    let session_id = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let first_message = messages
        .iter()
        .find(|m| m.message_type == "user" && !m.is_meta && !m.text_content.is_empty())
        .map(|m| truncate_str(&m.text_content, 120));

    let has_subagents = messages
        .iter()
        .any(|m| m.tool_calls.iter().any(|tc| tc.is_task));

    let git_branch = messages
        .iter()
        .find_map(|m| m.git_branch.clone());

    let is_ongoing = file_path
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|modified| {
            modified
                .elapsed()
                .map(|elapsed| elapsed.as_secs() < 300)
                .unwrap_or(false)
        })
        .unwrap_or(false);

    let created_at = messages
        .first()
        .and_then(|m| chrono::DateTime::parse_from_rfc3339(&m.timestamp).ok())
        .map(|dt| dt.timestamp_millis() as u64)
        .unwrap_or_else(|| {
            file_path
                .metadata()
                .ok()
                .and_then(|m| m.created().ok())
                .and_then(|ct| ct.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0)
        });

    Ok(SessionSummary {
        id: session_id,
        project_id: project_id.to_string(),
        created_at,
        first_message,
        message_count: messages.len(),
        has_subagents,
        is_ongoing,
        git_branch,
        metrics: metrics.clone(),
    })
}

// ============================================================
// Watcher State
// ============================================================

pub struct DevToolsWatcherState {
    pub watcher: Option<RecommendedWatcher>,
}

impl Default for DevToolsWatcherState {
    fn default() -> Self {
        Self { watcher: None }
    }
}

// ============================================================
// Tauri Commands
// ============================================================

#[tauri::command]
pub fn devtools_list_projects() -> Result<Vec<DevToolsProject>, String> {
    let projects_path = get_claude_projects_path()?;
    if !projects_path.exists() {
        return Ok(Vec::new());
    }

    let mut projects = Vec::new();
    let entries =
        std::fs::read_dir(&projects_path).map_err(|e| format!("Failed to read projects dir: {}", e))?;

    for entry in entries.flatten() {
        if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            continue;
        }

        let dir_name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden directories
        if dir_name.starts_with('.') {
            continue;
        }

        let decoded = decode_project_path(&dir_name);
        let name = extract_project_name(&dir_name, None);

        // Count session JSONL files and find most recent
        let mut session_count = 0;
        let mut most_recent: Option<u64> = None;
        let mut cwd_hint: Option<String> = None;

        if let Ok(files) = std::fs::read_dir(entry.path()) {
            for file in files.flatten() {
                let fname = file.file_name().to_string_lossy().to_string();
                if is_session_file(&fname) {
                    session_count += 1;
                    if let Ok(meta) = file.metadata() {
                        if let Ok(modified) = meta.modified() {
                            let ms = modified
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64;
                            most_recent = Some(most_recent.map_or(ms, |prev: u64| prev.max(ms)));
                        }
                    }

                    // Try to get cwd from first entry of first session for better project naming
                    if cwd_hint.is_none() {
                        if let Ok(f) = std::fs::File::open(file.path()) {
                            let rdr = BufReader::new(f);
                            if let Some(Ok(first_line)) = rdr.lines().next() {
                                if let Ok(entry) = serde_json::from_str::<JsonlEntry>(&first_line) {
                                    cwd_hint = entry.cwd;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Use cwd hint for better project name
        let final_name = if let Some(ref cwd) = cwd_hint {
            extract_project_name(&dir_name, Some(cwd))
        } else {
            name
        };

        if session_count > 0 {
            projects.push(DevToolsProject {
                id: dir_name,
                path: cwd_hint.unwrap_or(decoded),
                name: final_name,
                session_count,
                most_recent,
            });
        }
    }

    // Disambiguate duplicate project names by appending parent folder
    disambiguate_project_names(&mut projects);

    // Sort by most recent activity
    projects.sort_by(|a, b| b.most_recent.cmp(&a.most_recent));
    Ok(projects)
}

#[tauri::command]
pub fn devtools_list_sessions(project_id: String) -> Result<Vec<SessionSummary>, String> {
    let projects_path = get_claude_projects_path()?;
    let project_dir = projects_path.join(&project_id);

    if !project_dir.exists() {
        return Err(format!("Project directory not found: {}", project_id));
    }

    validate_within_base(&project_dir, &projects_path)?;

    let mut sessions = Vec::new();
    let entries =
        std::fs::read_dir(&project_dir).map_err(|e| format!("Failed to read project dir: {}", e))?;

    for entry in entries.flatten() {
        let fname = entry.file_name().to_string_lossy().to_string();
        if !is_session_file(&fname) {
            continue;
        }

        match quick_scan_session(&entry.path(), &project_id) {
            Ok(summary) if summary.metrics.message_count > 0 => sessions.push(summary),
            _ => {}
        }
    }

    // Sort by created_at descending (most recent first)
    sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(sessions)
}

#[tauri::command]
pub fn devtools_get_session(
    project_id: String,
    session_id: String,
) -> Result<SessionDetail, String> {
    let projects_path = get_claude_projects_path()?;
    let project_dir = projects_path.join(&project_id);
    let session_file = project_dir.join(format!("{}.jsonl", session_id));

    if !session_file.exists() {
        return Err(format!("Session file not found: {}", session_id));
    }

    validate_within_base(&project_dir, &projects_path)?;

    let mut messages = parse_jsonl_file(&session_file)?;
    let metrics = calculate_and_enrich_metrics(&mut messages);

    // Build summary from already-parsed messages (avoid double file read)
    let session = build_summary_from_messages(&messages, &session_file, &project_id, &metrics)?;

    // Resolve subagents and link to Task tool_calls
    let mut subagents = resolve_subagents(&project_dir, &session_id);
    link_subagents_to_tasks(&messages, &mut subagents);
    detect_parallel_subagents(&mut subagents);

    Ok(SessionDetail {
        session,
        messages,
        subagents,
        metrics,
    })
}

#[tauri::command]
pub fn devtools_start_watching(
    app_handle: tauri::AppHandle,
    state: State<'_, Mutex<DevToolsWatcherState>>,
) -> Result<(), String> {
    let projects_path = get_claude_projects_path()?;
    if !projects_path.exists() {
        return Ok(());
    }

    let handle = app_handle.clone();
    let watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
        if let Ok(event) = res {
            for path in &event.paths {
                if path
                    .extension()
                    .map_or(false, |ext| ext == "jsonl")
                {
                    let path_str = path.to_string_lossy().to_string();
                    let _ = handle.emit("devtools-file-change", &path_str);
                }
            }
        }
    })
    .map_err(|e| format!("Failed to create watcher: {}", e))?;

    let mut state = state
        .lock()
        .map_err(|e| format!("Failed to lock watcher state: {}", e))?;

    // Drop existing watcher if any
    state.watcher = None;

    let mut w = watcher;
    w.watch(&projects_path, RecursiveMode::Recursive)
        .map_err(|e| format!("Failed to watch: {}", e))?;

    state.watcher = Some(w);
    Ok(())
}

#[tauri::command]
pub fn devtools_stop_watching(
    state: State<'_, Mutex<DevToolsWatcherState>>,
) -> Result<(), String> {
    let mut state = state
        .lock()
        .map_err(|e| format!("Failed to lock watcher state: {}", e))?;
    state.watcher = None;
    Ok(())
}
