/**
 * Dynamic Function Manager
 * Manages dynamic functions: listing, saving, starting/stopping Node.js backends
 * Functions are stored in ~/.MoontonClaudeDesktop/dynamic-functions/{name}/
 */
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::thread;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use crate::nodejs::get_nodejs_path;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const MAX_MANIFEST_SIZE: usize = 100_000; // 100KB
const MAX_FILE_SIZE: usize = 10_000_000; // 10MB
const MAX_TOTAL_SIZE: usize = 50_000_000; // 50MB

/// Manifest for a dynamic function — tolerant of extra fields and variations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicFunctionManifest {
    pub name: String,
    #[serde(alias = "displayName", alias = "display_name", alias = "title")]
    pub display_name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_icon", deserialize_with = "deserialize_icon")]
    pub icon: String,
    #[serde(default)]
    pub window: WindowConfig,
    #[serde(default)]
    pub capabilities: Capabilities,
    #[serde(default)]
    pub backend: BackendConfig,
    // Absorb any extra top-level fields Claude might add
    #[serde(flatten)]
    _extra: HashMap<String, serde_json::Value>,
}

fn default_icon() -> String {
    String::from("fa-solid fa-cube")
}

fn default_version() -> String {
    String::from("1.0.0")
}

/// If the icon is not a Font Awesome class (e.g., it's an emoji), use default
fn deserialize_icon<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.starts_with("fa-") || s.starts_with("fas ") || s.starts_with("far ") || s.starts_with("fab ") {
        Ok(s)
    } else {
        Ok(default_icon())
    }
}

/// Window configuration — tolerant of extra fields Claude might add
#[derive(Debug, Clone, Serialize, Default)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub min_width: u32,
    pub min_height: u32,
}

/// Custom deserializer for WindowConfig to ignore unknown fields and provide defaults
impl<'de> Deserialize<'de> for WindowConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawWindow {
            #[serde(default = "default_width")]
            width: u32,
            #[serde(default = "default_height")]
            height: u32,
            #[serde(rename = "minWidth", default = "default_min_width")]
            min_width: u32,
            #[serde(rename = "minHeight", default = "default_min_height")]
            min_height: u32,
            // Absorb any extra fields Claude might add (resizable, etc.)
            #[serde(flatten)]
            _extra: HashMap<String, serde_json::Value>,
        }

        let raw = RawWindow::deserialize(deserializer)?;
        Ok(WindowConfig {
            width: raw.width,
            height: raw.height,
            min_width: raw.min_width,
            min_height: raw.min_height,
        })
    }
}

fn default_width() -> u32 { 800 }
fn default_height() -> u32 { 600 }
fn default_min_width() -> u32 { 400 }
fn default_min_height() -> u32 { 300 }

/// Capabilities — can be either an object or an array of strings
/// Object format: { "network": true, "localStorage": true }
/// Array format:  ["storage", "network", "notifications"]
#[derive(Debug, Clone, Serialize, Default)]
pub struct Capabilities {
    pub network: bool,
    pub local_storage: bool,
    pub exec: bool,
}

impl<'de> Deserialize<'de> for Capabilities {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CapabilitiesVisitor;

        impl<'de> Visitor<'de> for CapabilitiesVisitor {
            type Value = Capabilities;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an object with boolean fields or an array of capability strings")
            }

            /// Handle array format: ["storage", "network"]
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut caps = Capabilities::default();
                while let Some(val) = seq.next_element::<String>()? {
                    match val.to_lowercase().as_str() {
                        "network" | "fetch" | "http" => caps.network = true,
                        "localstorage" | "storage" | "local_storage" => caps.local_storage = true,
                        "exec" | "shell" | "command" => caps.exec = true,
                        _ => {} // Ignore unknown capabilities
                    }
                }
                Ok(caps)
            }

            /// Handle object format: { "network": true, "localStorage": true }
            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: de::MapAccess<'de>,
            {
                let mut caps = Capabilities::default();
                while let Some(key) = map.next_key::<String>()? {
                    match key.to_lowercase().as_str() {
                        "network" | "fetch" | "http" => {
                            // Accept both bool and string
                            let val: serde_json::Value = map.next_value()?;
                            caps.network = val.as_bool().unwrap_or(false)
                                || val.as_str().map_or(false, |s| s == "true");
                        }
                        "localstorage" | "local_storage" | "storage" => {
                            let val: serde_json::Value = map.next_value()?;
                            caps.local_storage = val.as_bool().unwrap_or(false)
                                || val.as_str().map_or(false, |s| s == "true");
                        }
                        "exec" | "shell" | "command" => {
                            let val: serde_json::Value = map.next_value()?;
                            caps.exec = val.as_bool().unwrap_or(false)
                                || val.as_str().map_or(false, |s| s == "true");
                        }
                        _ => {
                            // Skip unknown fields
                            let _: serde_json::Value = map.next_value()?;
                        }
                    }
                }
                Ok(caps)
            }
        }

        deserializer.deserialize_any(CapabilitiesVisitor)
    }
}

/// Backend configuration — tolerant of extra fields
#[derive(Debug, Clone, Serialize, Default)]
pub struct BackendConfig {
    pub enabled: bool,
    pub dependencies: HashMap<String, String>,
}

impl<'de> Deserialize<'de> for BackendConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawBackend {
            #[serde(default)]
            enabled: bool,
            #[serde(default)]
            dependencies: HashMap<String, String>,
            // Absorb extra fields (entry, port, etc.)
            #[serde(flatten)]
            _extra: HashMap<String, serde_json::Value>,
        }

        // Handle "backend": null by deserializing as Option first
        let raw = Option::<RawBackend>::deserialize(deserializer)?;
        match raw {
            Some(r) => Ok(BackendConfig {
                enabled: r.enabled,
                dependencies: r.dependencies,
            }),
            None => Ok(BackendConfig::default()),
        }
    }
}

/// Summary info returned when listing functions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicFunctionInfo {
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub version: String,
    pub description: String,
    pub icon: String,
    #[serde(rename = "backendEnabled")]
    pub backend_enabled: bool,
    pub path: String,
}

/// Files to save for a dynamic function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionFile {
    /// Relative path within the function directory (e.g., "frontend/index.html")
    pub path: String,
    pub content: String,
}

/// Running backend instance
struct BackendInstance {
    process: Child,
    port: u16,
    #[allow(dead_code)]
    function_name: String,
    ref_count: usize,
}

/// Dynamic Function Manager
pub struct DynamicFunctionManager {
    instances: Mutex<HashMap<String, BackendInstance>>,
}

impl Default for DynamicFunctionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for DynamicFunctionManager {
    fn drop(&mut self) {
        let mut instances = self.instances.lock().unwrap();
        for (name, mut instance) in instances.drain() {
            let _ = instance.process.kill();
            log::info!("Stopped backend '{}' on shutdown", name);
        }
    }
}

/// Validate that a function name contains only safe characters
fn validate_function_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 100 {
        return Err("Function name must be 1-100 characters".to_string());
    }
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(
            "Function name must contain only alphanumeric characters, hyphens, and underscores"
                .to_string(),
        );
    }
    Ok(())
}

/// Validate that a relative file path is safe (no traversal)
fn validate_relative_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("File path cannot be empty".to_string());
    }
    // Block path traversal
    if path.contains("..") {
        return Err("Path traversal not allowed".to_string());
    }
    // Block absolute paths
    if path.starts_with('/') || path.starts_with('\\') {
        return Err("Absolute paths not allowed".to_string());
    }
    // Block Windows drive letters
    if path.len() >= 2 && path.as_bytes()[1] == b':' {
        return Err("Absolute paths not allowed".to_string());
    }
    // Block null bytes
    if path.contains('\0') {
        return Err("Null bytes not allowed in path".to_string());
    }
    Ok(())
}

impl DynamicFunctionManager {
    pub fn new() -> Self {
        Self {
            instances: Mutex::new(HashMap::new()),
        }
    }

    /// Get the base directory for all dynamic functions
    fn base_dir() -> Result<PathBuf, String> {
        let home =
            dirs::home_dir().ok_or_else(|| "Failed to get home directory".to_string())?;
        Ok(home.join(".MoontonClaudeDesktop").join("dynamic-functions"))
    }

    /// Get directory for a specific function (validates name)
    fn function_dir(name: &str) -> Result<PathBuf, String> {
        validate_function_name(name)?;
        Ok(Self::base_dir()?.join(name))
    }

    /// List all dynamic functions by scanning the base directory
    pub fn list_functions() -> Result<Vec<DynamicFunctionInfo>, String> {
        let base = Self::base_dir()?;
        if !base.exists() {
            return Ok(vec![]);
        }

        let entries = fs::read_dir(&base)
            .map_err(|e| format!("Failed to read dynamic-functions directory: {}", e))?;

        let mut functions = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            if !entry.path().is_dir() {
                continue;
            }

            let manifest_path = entry.path().join("manifest.json");
            if !manifest_path.exists() {
                continue;
            }

            match fs::read_to_string(&manifest_path) {
                Ok(content) => match serde_json::from_str::<DynamicFunctionManifest>(&content) {
                    Ok(manifest) => {
                        functions.push(DynamicFunctionInfo {
                            name: manifest.name.clone(),
                            display_name: manifest.display_name.clone(),
                            version: manifest.version.clone(),
                            description: manifest.description.clone(),
                            icon: manifest.icon.clone(),
                            backend_enabled: manifest.backend.enabled,
                            path: entry.path().to_string_lossy().to_string(),
                        });
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to parse manifest at {:?}: {}",
                            manifest_path, e
                        );
                    }
                },
                Err(e) => {
                    log::warn!(
                        "Failed to read manifest at {:?}: {}",
                        manifest_path, e
                    );
                }
            }
        }

        Ok(functions)
    }

    /// Read manifest for a specific function
    pub fn read_manifest(name: &str) -> Result<DynamicFunctionManifest, String> {
        let manifest_path = Self::function_dir(name)?.join("manifest.json");
        let content = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read manifest for {}: {}", name, e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse manifest for {}: {}", name, e))
    }

    /// Save a dynamic function (manifest + files) to disk
    pub fn save_function(
        name: &str,
        manifest: &str,
        files: &[FunctionFile],
    ) -> Result<String, String> {
        validate_function_name(name)?;

        // Validate sizes
        if manifest.len() > MAX_MANIFEST_SIZE {
            return Err(format!("Manifest too large (max {}KB)", MAX_MANIFEST_SIZE / 1000));
        }

        let mut total_size = manifest.len();
        for file in files {
            validate_relative_path(&file.path)?;
            if file.content.len() > MAX_FILE_SIZE {
                return Err(format!("File {} too large (max {}MB)", file.path, MAX_FILE_SIZE / 1_000_000));
            }
            total_size += file.content.len();
        }
        if total_size > MAX_TOTAL_SIZE {
            return Err(format!("Total size too large (max {}MB)", MAX_TOTAL_SIZE / 1_000_000));
        }

        let func_dir = Self::function_dir(name)?;

        // Create the function directory
        fs::create_dir_all(&func_dir)
            .map_err(|e| format!("Failed to create function directory: {}", e))?;

        // Write manifest
        fs::write(func_dir.join("manifest.json"), manifest)
            .map_err(|e| format!("Failed to write manifest: {}", e))?;

        // Write each file with path traversal protection
        for file in files {
            let file_path = func_dir.join(&file.path);

            // Verify resolved path stays within func_dir
            // Create parent dirs first so we can canonicalize
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directory for {}: {}", file.path, e))?;
            }

            // Canonicalize parent to resolve any symlinks
            if let Some(parent) = file_path.parent() {
                let canonical_parent = parent.canonicalize().map_err(|e| {
                    format!("Failed to resolve path for {}: {}", file.path, e)
                })?;
                let canonical_base = func_dir.canonicalize().map_err(|e| {
                    format!("Failed to resolve base directory: {}", e)
                })?;
                if !canonical_parent.starts_with(&canonical_base) {
                    return Err(format!(
                        "Path traversal blocked: {} resolves outside function directory",
                        file.path
                    ));
                }
            }

            fs::write(&file_path, &file.content)
                .map_err(|e| format!("Failed to write {}: {}", file.path, e))?;
        }

        Ok(func_dir.to_string_lossy().to_string())
    }

    /// Find an available port in the ephemeral range
    fn find_available_port() -> Result<u16, String> {
        TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("Failed to find available port: {}", e))
            .map(|listener| listener.local_addr().unwrap().port())
    }

    /// Start a backend for a dynamic function (ref-counted)
    pub fn start_backend(&self, name: &str) -> Result<u16, String> {
        validate_function_name(name)?;

        // Hold lock for the entire operation to prevent races
        let mut instances = self.instances.lock().unwrap();

        // If already running, increment ref count and return port
        if let Some(instance) = instances.get_mut(name) {
            instance.ref_count += 1;
            log::info!(
                "Backend '{}' already running (ref_count={}), port {}",
                name, instance.ref_count, instance.port
            );
            return Ok(instance.port);
        }

        let func_dir = Self::function_dir(name)?;
        let server_path = func_dir.join("backend").join("server.js");

        if !server_path.exists() {
            return Err(format!("Backend server.js not found for {}", name));
        }

        let node_path =
            get_nodejs_path().ok_or_else(|| "Node.js not found".to_string())?;

        let port = Self::find_available_port()?;

        let mut cmd = Command::new(&node_path);
        cmd.arg(&server_path)
            .current_dir(func_dir.join("backend"))
            .env("PORT", port.to_string())
            .env("HOST", "127.0.0.1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let mut process = cmd
            .spawn()
            .map_err(|e| format!("Failed to start backend for {}: {}", name, e))?;

        // Consume stdout to prevent buffer blocking
        if let Some(stdout) = process.stdout.take() {
            let name_clone = name.to_string();
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    match line {
                        Ok(line) => log::debug!("[df:{}] {}", name_clone, line),
                        Err(_) => break,
                    }
                }
            });
        }

        // Consume stderr
        if let Some(stderr) = process.stderr.take() {
            let name_clone = name.to_string();
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    match line {
                        Ok(line) => log::warn!("[df:{}] {}", name_clone, line),
                        Err(_) => break,
                    }
                }
            });
        }

        log::info!(
            "Started dynamic function backend '{}' on port {}",
            name, port
        );

        instances.insert(
            name.to_string(),
            BackendInstance {
                process,
                port,
                function_name: name.to_string(),
                ref_count: 1,
            },
        );

        Ok(port)
    }

    /// Stop a backend for a dynamic function (ref-counted)
    pub fn stop_backend(&self, name: &str) -> Result<(), String> {
        let mut instances = self.instances.lock().unwrap();
        if let Some(instance) = instances.get_mut(name) {
            instance.ref_count = instance.ref_count.saturating_sub(1);

            if instance.ref_count == 0 {
                let mut removed = instances.remove(name).unwrap();
                let _ = removed.process.kill();
                // Wait briefly for process to exit
                let _ = removed.process.wait();
                log::info!("Stopped dynamic function backend '{}'", name);
            } else {
                log::info!(
                    "Backend '{}' ref_count decremented to {}",
                    name, instance.ref_count
                );
            }
        }
        Ok(())
    }

    /// Delete a dynamic function (force stop backend + remove directory)
    pub fn delete_function(&self, name: &str) -> Result<(), String> {
        validate_function_name(name)?;

        // Force-kill backend regardless of ref count
        {
            let mut instances = self.instances.lock().unwrap();
            if let Some(mut instance) = instances.remove(name) {
                let _ = instance.process.kill();
                let _ = instance.process.wait();
                log::info!("Force-stopped backend '{}' for deletion", name);
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

        let func_dir = Self::function_dir(name)?;
        if !func_dir.exists() {
            return Ok(());
        }

        // Remove manifest.json first so the function disappears from list_functions
        // immediately, even if the directory removal is delayed by file locks.
        let manifest = func_dir.join("manifest.json");
        if manifest.exists() {
            let _ = fs::remove_file(&manifest);
        }

        // Attempt to remove the entire directory with retries
        let mut last_err = None;
        for attempt in 0..3 {
            match fs::remove_dir_all(&func_dir) {
                Ok(()) => {
                    log::info!("Deleted dynamic function '{}'", name);
                    return Ok(());
                }
                Err(e) => {
                    log::warn!(
                        "Attempt {} to delete '{}' failed: {}",
                        attempt + 1,
                        name,
                        e
                    );
                    last_err = Some(e);
                    std::thread::sleep(std::time::Duration::from_millis(300));
                }
            }
        }

        // Directory removal failed but manifest is gone, so the function is
        // effectively invisible. Spawn a background thread for deferred cleanup.
        if let Some(e) = last_err {
            log::warn!(
                "Deferred cleanup for '{}': directory locked ({}), will retry in background",
                name, e
            );
            let dir = func_dir.clone();
            let owned_name = name.to_string();
            thread::spawn(move || {
                for i in 0..5 {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    match fs::remove_dir_all(&dir) {
                        Ok(()) => {
                            log::info!("Background cleanup succeeded for '{}'", owned_name);
                            return;
                        }
                        Err(e) => {
                            log::warn!("Background cleanup attempt {} for '{}': {}", i + 1, owned_name, e);
                        }
                    }
                }
                log::warn!("Background cleanup gave up for '{}', directory may need manual removal", owned_name);
            });
        }

        Ok(())
    }
}

// Tauri commands

#[tauri::command]
pub fn df_list_functions() -> Result<Vec<DynamicFunctionInfo>, String> {
    DynamicFunctionManager::list_functions()
}

#[tauri::command]
pub fn df_read_manifest(name: String) -> Result<DynamicFunctionManifest, String> {
    DynamicFunctionManager::read_manifest(&name)
}

#[tauri::command]
pub fn df_save_function(
    name: String,
    manifest: String,
    files: Vec<FunctionFile>,
) -> Result<String, String> {
    DynamicFunctionManager::save_function(&name, &manifest, &files)
}

#[tauri::command]
pub fn df_start_backend(
    state: tauri::State<'_, DynamicFunctionManager>,
    name: String,
) -> Result<u16, String> {
    state.start_backend(&name)
}

#[tauri::command]
pub fn df_stop_backend(
    state: tauri::State<'_, DynamicFunctionManager>,
    name: String,
) -> Result<(), String> {
    state.stop_backend(&name)
}

#[tauri::command]
pub fn df_delete_function(
    state: tauri::State<'_, DynamicFunctionManager>,
    name: String,
) -> Result<(), String> {
    state.delete_function(&name)
}

// ── Process execution & management ──────────────────────────────────────

/// A background process spawned by a light app via mcd:exec
struct SpawnedProcess {
    child: Child,
    function_name: String,
}

/// Manages background processes spawned by light apps
pub struct DfProcessManager {
    processes: Mutex<HashMap<u32, SpawnedProcess>>,
}

impl Default for DfProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DfProcessManager {
    pub fn new() -> Self {
        Self {
            processes: Mutex::new(HashMap::new()),
        }
    }

    /// Reap processes that have already exited (lazy cleanup).
    fn reap_exited(map: &mut HashMap<u32, SpawnedProcess>) {
        let exited: Vec<u32> = map
            .iter_mut()
            .filter_map(|(&pid, sp)| match sp.child.try_wait() {
                Ok(Some(_)) => Some(pid),
                _ => None,
            })
            .collect();
        for pid in exited {
            map.remove(&pid);
        }
    }

    /// Insert a spawned child, returning its PID.
    pub fn track(&self, function_name: &str, child: Child) -> u32 {
        let pid = child.id();
        let mut map = self.processes.lock().unwrap();
        Self::reap_exited(&mut map);
        map.insert(
            pid,
            SpawnedProcess {
                child,
                function_name: function_name.to_string(),
            },
        );
        pid
    }

    /// Kill a process by PID, validating ownership.
    pub fn kill(&self, function_name: &str, pid: u32) -> Result<(), String> {
        let mut map = self.processes.lock().unwrap();
        match map.get(&pid) {
            Some(sp) if sp.function_name != function_name => {
                Err(format!("Process {} is not owned by {}", pid, function_name))
            }
            Some(_) => {
                let mut sp = map.remove(&pid).unwrap();
                sp.child
                    .kill()
                    .map_err(|e| format!("Failed to kill process {}: {}", pid, e))?;
                let _ = sp.child.wait();
                log::info!("Killed process {} for '{}'", pid, function_name);
                Ok(())
            }
            None => Err(format!("Process {} not found or already exited", pid)),
        }
    }

    /// Kill all processes owned by a function (for cleanup on window close).
    pub fn kill_all_for(&self, function_name: &str) {
        let mut map = self.processes.lock().unwrap();
        let pids: Vec<u32> = map
            .iter()
            .filter(|(_, sp)| sp.function_name == function_name)
            .map(|(&pid, _)| pid)
            .collect();
        for pid in pids {
            if let Some(mut sp) = map.remove(&pid) {
                let _ = sp.child.kill();
                let _ = sp.child.wait();
                log::info!(
                    "Cleaned up background process {} for '{}'",
                    pid,
                    function_name
                );
            }
        }
    }
}

impl Drop for DfProcessManager {
    fn drop(&mut self) {
        let mut map = self.processes.lock().unwrap();
        for (pid, mut sp) in map.drain() {
            let _ = sp.child.kill();
            let _ = sp.child.wait();
            log::info!(
                "Shutdown cleanup: killed process {} for '{}'",
                pid,
                sp.function_name
            );
        }
    }
}

/// Options for the df_exec command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecOptions {
    /// Working directory (absolute or relative to function dir)
    pub cwd: Option<String>,
    /// Environment variables to set
    pub env: Option<HashMap<String, String>>,
    /// If true, return PID immediately without waiting
    pub background: Option<bool>,
    /// Timeout in milliseconds for sync mode (default: 30000, max: 300000)
    #[serde(rename = "timeout_ms")]
    pub timeout_ms: Option<u64>,
}

/// Result from the df_exec command
#[derive(Debug, Clone, Serialize)]
pub struct ExecResult {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    #[serde(rename = "exitCode")]
    pub exit_code: Option<i32>,
    pub pid: Option<u32>,
}

const MAX_OUTPUT_SIZE: usize = 1_048_576; // 1MB

/// Truncate a string to a maximum byte length, appending a marker if truncated.
fn truncate_output(s: String) -> String {
    if s.len() > MAX_OUTPUT_SIZE {
        let truncated = &s[..MAX_OUTPUT_SIZE];
        format!("{}...[truncated]", truncated)
    } else {
        s
    }
}

// ── New Tauri commands ──────────────────────────────────────────────────

#[tauri::command]
pub fn df_open_external(url: String) -> Result<(), String> {
    // Security: only allow http:// and https:// URLs
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("Only http:// and https:// URLs are allowed".to_string());
    }
    opener::open(&url).map_err(|e| format!("Failed to open URL: {}", e))
}

#[tauri::command]
pub fn df_exec(
    state: tauri::State<'_, DfProcessManager>,
    function_name: String,
    command: String,
    args: Vec<String>,
    options: Option<ExecOptions>,
) -> Result<ExecResult, String> {
    validate_function_name(&function_name)?;

    let func_dir = DynamicFunctionManager::function_dir(&function_name)?;
    let opts = options.unwrap_or(ExecOptions {
        cwd: None,
        env: None,
        background: None,
        timeout_ms: None,
    });

    // Determine CWD: if provided use as-is (absolute or relative), else use func_dir
    let work_dir = if let Some(ref cwd) = opts.cwd {
        let path = std::path::Path::new(cwd);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            func_dir.join(cwd)
        }
    } else {
        func_dir
    };

    if !work_dir.exists() {
        return Err(format!("Working directory does not exist: {}", work_dir.display()));
    }

    let mut cmd = Command::new(&command);
    cmd.args(&args)
        .current_dir(&work_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Set environment variables
    if let Some(ref env_map) = opts.env {
        for (k, v) in env_map {
            cmd.env(k, v);
        }
    }

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let background = opts.background.unwrap_or(false);

    if background {
        // Background mode: spawn and return PID
        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn '{}': {}", command, e))?;
        let pid = state.track(&function_name, child);
        log::info!(
            "Spawned background process '{}' (PID {}) for '{}'",
            command,
            pid,
            function_name
        );
        Ok(ExecResult {
            stdout: None,
            stderr: None,
            exit_code: None,
            pid: Some(pid),
        })
    } else {
        // Sync mode: wait for completion
        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn '{}': {}", command, e))?;

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to wait for '{}': {}", command, e))?;

        let stdout = truncate_output(String::from_utf8_lossy(&output.stdout).to_string());
        let stderr = truncate_output(String::from_utf8_lossy(&output.stderr).to_string());

        Ok(ExecResult {
            stdout: Some(stdout),
            stderr: Some(stderr),
            exit_code: output.status.code(),
            pid: None,
        })
    }
}

#[tauri::command]
pub fn df_kill(
    state: tauri::State<'_, DfProcessManager>,
    function_name: String,
    pid: u32,
) -> Result<(), String> {
    validate_function_name(&function_name)?;
    state.kill(&function_name, pid)
}

#[tauri::command]
pub fn df_cleanup_processes(
    state: tauri::State<'_, DfProcessManager>,
    function_name: String,
) -> Result<(), String> {
    validate_function_name(&function_name)?;
    state.kill_all_for(&function_name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── validate_function_name ─────────────────────────────────────────

    #[test]
    fn valid_function_names() {
        assert!(validate_function_name("my-app").is_ok());
        assert!(validate_function_name("my_app").is_ok());
        assert!(validate_function_name("app123").is_ok());
        assert!(validate_function_name("A").is_ok());
        assert!(validate_function_name("a-b_c-1").is_ok());
    }

    #[test]
    fn empty_function_name_rejected() {
        assert!(validate_function_name("").is_err());
    }

    #[test]
    fn function_name_too_long_rejected() {
        let long_name = "a".repeat(101);
        assert!(validate_function_name(&long_name).is_err());
    }

    #[test]
    fn function_name_max_length_accepted() {
        let max_name = "a".repeat(100);
        assert!(validate_function_name(&max_name).is_ok());
    }

    #[test]
    fn function_name_special_chars_rejected() {
        assert!(validate_function_name("my app").is_err());
        assert!(validate_function_name("my/app").is_err());
        assert!(validate_function_name("../escape").is_err());
        assert!(validate_function_name("app@1").is_err());
        assert!(validate_function_name("app.name").is_err());
    }

    // ── validate_relative_path ─────────────────────────────────────────

    #[test]
    fn valid_relative_paths() {
        assert!(validate_relative_path("frontend/index.html").is_ok());
        assert!(validate_relative_path("backend/server.js").is_ok());
        assert!(validate_relative_path("manifest.json").is_ok());
    }

    #[test]
    fn empty_path_rejected() {
        assert!(validate_relative_path("").is_err());
    }

    #[test]
    fn path_traversal_rejected() {
        assert!(validate_relative_path("../escape").is_err());
        assert!(validate_relative_path("a/../../etc/passwd").is_err());
        assert!(validate_relative_path("..").is_err());
    }

    #[test]
    fn absolute_path_rejected() {
        assert!(validate_relative_path("/etc/passwd").is_err());
        assert!(validate_relative_path("\\windows\\system32").is_err());
        assert!(validate_relative_path("C:\\Users").is_err());
    }

    #[test]
    fn null_bytes_rejected() {
        assert!(validate_relative_path("file\0.txt").is_err());
    }

    // ── truncate_output ────────────────────────────────────────────────

    #[test]
    fn short_output_unchanged() {
        let input = "hello".to_string();
        assert_eq!(truncate_output(input.clone()), input);
    }

    #[test]
    fn output_at_limit_unchanged() {
        let input = "x".repeat(MAX_OUTPUT_SIZE);
        assert_eq!(truncate_output(input.clone()), input);
    }

    #[test]
    fn output_over_limit_truncated() {
        let input = "x".repeat(MAX_OUTPUT_SIZE + 100);
        let result = truncate_output(input);
        assert!(result.ends_with("...[truncated]"));
        assert!(result.len() < MAX_OUTPUT_SIZE + 100);
    }

    // ── default_icon ───────────────────────────────────────────────────

    #[test]
    fn default_icon_is_fa_cube() {
        assert_eq!(default_icon(), "fa-solid fa-cube");
    }

    // ── Capabilities deserialization ───────────────────────────────────

    #[test]
    fn capabilities_from_object() {
        let json = r#"{"network": true, "localStorage": true, "exec": false}"#;
        let caps: Capabilities = serde_json::from_str(json).unwrap();
        assert!(caps.network);
        assert!(caps.local_storage);
        assert!(!caps.exec);
    }

    #[test]
    fn capabilities_from_array() {
        let json = r#"["network", "storage", "exec"]"#;
        let caps: Capabilities = serde_json::from_str(json).unwrap();
        assert!(caps.network);
        assert!(caps.local_storage);
        assert!(caps.exec);
    }

    #[test]
    fn capabilities_from_empty_object() {
        let json = r#"{}"#;
        let caps: Capabilities = serde_json::from_str(json).unwrap();
        assert!(!caps.network);
        assert!(!caps.local_storage);
        assert!(!caps.exec);
    }

    #[test]
    fn capabilities_from_empty_array() {
        let json = r#"[]"#;
        let caps: Capabilities = serde_json::from_str(json).unwrap();
        assert!(!caps.network);
        assert!(!caps.local_storage);
        assert!(!caps.exec);
    }

    #[test]
    fn capabilities_aliases_recognized() {
        // "fetch" -> network, "http" -> network, "shell" -> exec
        let json = r#"["fetch", "http", "shell", "local_storage"]"#;
        let caps: Capabilities = serde_json::from_str(json).unwrap();
        assert!(caps.network);
        assert!(caps.local_storage);
        assert!(caps.exec);
    }

    #[test]
    fn capabilities_unknown_ignored() {
        let json = r#"["network", "camera", "bluetooth"]"#;
        let caps: Capabilities = serde_json::from_str(json).unwrap();
        assert!(caps.network);
        assert!(!caps.local_storage);
        assert!(!caps.exec);
    }

    // ── WindowConfig deserialization ───────────────────────────────────

    #[test]
    fn window_config_defaults() {
        let json = r#"{}"#;
        let wc: WindowConfig = serde_json::from_str(json).unwrap();
        assert_eq!(wc.width, 800);
        assert_eq!(wc.height, 600);
        assert_eq!(wc.min_width, 400);
        assert_eq!(wc.min_height, 300);
    }

    #[test]
    fn window_config_custom_values() {
        let json = r#"{"width": 1024, "height": 768, "minWidth": 200, "minHeight": 150}"#;
        let wc: WindowConfig = serde_json::from_str(json).unwrap();
        assert_eq!(wc.width, 1024);
        assert_eq!(wc.height, 768);
        assert_eq!(wc.min_width, 200);
        assert_eq!(wc.min_height, 150);
    }

    #[test]
    fn window_config_extra_fields_ignored() {
        let json = r#"{"width": 500, "height": 400, "resizable": true, "transparent": false}"#;
        let wc: WindowConfig = serde_json::from_str(json).unwrap();
        assert_eq!(wc.width, 500);
        assert_eq!(wc.height, 400);
    }

    // ── BackendConfig deserialization ──────────────────────────────────

    #[test]
    fn backend_config_defaults() {
        let json = r#"{}"#;
        let bc: BackendConfig = serde_json::from_str(json).unwrap();
        assert!(!bc.enabled);
        assert!(bc.dependencies.is_empty());
    }

    #[test]
    fn backend_config_null_yields_default() {
        let json = r#"null"#;
        let bc: BackendConfig = serde_json::from_str(json).unwrap();
        assert!(!bc.enabled);
    }

    #[test]
    fn backend_config_with_deps() {
        let json = r#"{"enabled": true, "dependencies": {"express": "^4.18.0"}}"#;
        let bc: BackendConfig = serde_json::from_str(json).unwrap();
        assert!(bc.enabled);
        assert_eq!(bc.dependencies.get("express"), Some(&"^4.18.0".to_string()));
    }

    // ── DynamicFunctionManifest deserialization ────────────────────────

    #[test]
    fn manifest_minimal() {
        let json = r#"{
            "name": "test-app",
            "displayName": "Test App"
        }"#;
        let m: DynamicFunctionManifest = serde_json::from_str(json).unwrap();
        assert_eq!(m.name, "test-app");
        assert_eq!(m.display_name, "Test App");
        assert_eq!(m.version, "1.0.0");
        assert_eq!(m.icon, "fa-solid fa-cube");
    }

    #[test]
    fn manifest_with_emoji_icon_uses_default() {
        let json = r#"{
            "name": "emoji-app",
            "displayName": "Emoji App",
            "icon": "🎮"
        }"#;
        let m: DynamicFunctionManifest = serde_json::from_str(json).unwrap();
        assert_eq!(m.icon, "fa-solid fa-cube");
    }

    #[test]
    fn manifest_with_fa_icon_preserved() {
        let json = r#"{
            "name": "icon-app",
            "displayName": "Icon App",
            "icon": "fa-solid fa-rocket"
        }"#;
        let m: DynamicFunctionManifest = serde_json::from_str(json).unwrap();
        assert_eq!(m.icon, "fa-solid fa-rocket");
    }

    #[test]
    fn manifest_extra_fields_ignored() {
        let json = r#"{
            "name": "extra-app",
            "displayName": "Extra",
            "author": "claude",
            "license": "MIT"
        }"#;
        let m: DynamicFunctionManifest = serde_json::from_str(json).unwrap();
        assert_eq!(m.name, "extra-app");
    }

    // ── ExecOptions defaults ──────────────────────────────────────────

    #[test]
    fn exec_options_defaults() {
        let json = r#"{}"#;
        let opts: ExecOptions = serde_json::from_str(json).unwrap();
        assert!(opts.cwd.is_none());
        assert!(opts.env.is_none());
        assert!(opts.background.is_none());
        assert!(opts.timeout_ms.is_none());
    }
}
