/**
 * Coworks Sidecar Manager
 * Manages the Node.js sidecar process for Claude integration
 * Similar to node_pty.rs but for the coworks sidecar
 */
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::nodejs::{get_nodejs_path, get_npm_global_bin_path};

/// Sidecar instance
pub struct CoworksSidecarInstance {
    /// Node.js process
    process: Child,
    /// Stdin for sending commands
    stdin: ChildStdin,
    /// Session ID
    #[allow(dead_code)]
    session_id: String,
}

/// Coworks Sidecar Manager
pub struct CoworksSidecarManager {
    /// Active sidecar instances by session ID
    instances: Arc<Mutex<HashMap<String, CoworksSidecarInstance>>>,
}

impl Default for CoworksSidecarManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CoworksSidecarManager {
    pub fn new() -> Self {
        CoworksSidecarManager {
            instances: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Spawn a new sidecar instance
    pub fn spawn<R: Runtime>(
        &self,
        app_handle: AppHandle<R>,
        session_id: String,
        env: Option<HashMap<String, String>>,
    ) -> Result<(), String> {
        log::info!(
            "CoworksSidecarManager: spawning sidecar for session {}",
            session_id
        );

        // Get node path
        let node_path = get_nodejs_path().ok_or_else(|| "Node.js not found".to_string())?;

        // Get sidecar script path
        let sidecar_path = get_sidecar_path(&app_handle)?;

        // Get sidecar directory for working directory
        // index.js is directly in coworks-sidecar/, node_modules is also there
        let sidecar_dir = std::path::Path::new(&sidecar_path)
            .parent() // coworks-sidecar/
            .ok_or_else(|| "Failed to get sidecar directory".to_string())?;

        // Build PATH with npm global bin directory (for claude command)
        let current_path = std::env::var("PATH").unwrap_or_default();
        let mut path_parts: Vec<String> = vec![];

        // Add npm global bin directory first (where claude is installed)
        if let Some(npm_bin) = get_npm_global_bin_path() {
            path_parts.push(npm_bin);
        }

        // Add Node.js directory
        if let Some(node_dir) = std::path::Path::new(&node_path).parent() {
            path_parts.push(node_dir.to_string_lossy().to_string());
        }

        // Add current PATH
        path_parts.push(current_path);

        let new_path = path_parts.join(if cfg!(windows) { ";" } else { ":" });

        // Spawn node process with sidecar directory as working directory
        let mut cmd = Command::new(&node_path);
        cmd.arg(&sidecar_path)
            .current_dir(sidecar_dir)
            .env("PATH", &new_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Hide console window on Windows
        #[cfg(windows)]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

        // Add custom environment variables (including ANTHROPIC_AUTH_TOKEN)
        if let Some(ref env_vars) = env {
            for (key, value) in env_vars {
                cmd.env(key, value);
            }
        }

        let mut process = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn sidecar: {}", e))?;

        // Get stdin for sending commands
        let stdin = process
            .stdin
            .take()
            .ok_or_else(|| "Failed to get stdin".to_string())?;

        // Get stdout for reading responses
        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| "Failed to get stdout".to_string())?;

        // Get stderr for logging
        let stderr = process
            .stderr
            .take()
            .ok_or_else(|| "Failed to get stderr".to_string())?;

        // Store instance
        let instance = CoworksSidecarInstance {
            process,
            stdin,
            session_id: session_id.clone(),
        };
        self.instances.lock().unwrap().insert(session_id.clone(), instance);

        // Spawn thread to read stdout
        let session_id_clone = session_id.clone();
        let app_handle_clone = app_handle.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            let mut ready_sent = false;
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if !line.is_empty() {
                            // Detect sidecar's own { "type": "ready" } message
                            // and emit the ready event only then (not immediately after spawn)
                            if !ready_sent && line.contains("\"type\":\"ready\"") {
                                let _ = app_handle_clone.emit(
                                    &format!("coworks-sidecar-ready-{}", session_id_clone),
                                    (),
                                );
                                ready_sent = true;
                            }
                            // Forward all stdout lines (including ready) as output events
                            let _ = app_handle_clone.emit(
                                &format!("coworks-sidecar-output-{}", session_id_clone),
                                line,
                            );
                        }
                    }
                    Err(e) => {
                        log::error!("Sidecar stdout error: {}", e);
                        break;
                    }
                }
            }
            log::info!("Sidecar stdout reader exited for session {}", session_id_clone);
        });

        // Spawn thread to read stderr
        let session_id_clone2 = session_id.clone();
        #[cfg(debug_assertions)]
        let app_handle_clone2 = app_handle.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if !line.is_empty() {
                            log::warn!("Sidecar stderr [{}]: {}", session_id_clone2, line);
                            // Emit stderr to frontend for dev console logging
                            #[cfg(debug_assertions)]
                            {
                                let _ = app_handle_clone2.emit(
                                    &format!("coworks-sidecar-stderr-{}", session_id_clone2),
                                    line,
                                );
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Sidecar stderr error: {}", e);
                        break;
                    }
                }
            }
        });

        // Note: ready event is now emitted from the stdout reader thread
        // when the sidecar sends { "type": "ready" } (not immediately after spawn)

        Ok(())
    }

    /// Send a message to a sidecar instance
    pub fn send_message(&self, session_id: &str, message: &str) -> Result<(), String> {
        log::info!("Sending to sidecar {}: {}", session_id, message);

        let mut instances = self.instances.lock().unwrap();
        let instance = instances
            .get_mut(session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;

        writeln!(instance.stdin, "{}", message)
            .map_err(|e| format!("Failed to write to sidecar: {}", e))?;

        instance
            .stdin
            .flush()
            .map_err(|e| format!("Failed to flush sidecar stdin: {}", e))?;

        Ok(())
    }

    /// Kill a sidecar instance
    pub fn kill(&self, session_id: &str) -> Result<(), String> {
        let mut instances = self.instances.lock().unwrap();
        if let Some(mut instance) = instances.remove(session_id) {
            let _ = instance.process.kill();
            log::info!("Killed sidecar for session {}", session_id);
        }
        Ok(())
    }

    /// Kill all sidecar instances
    pub fn kill_all(&self) {
        let mut instances = self.instances.lock().unwrap();
        for (session_id, mut instance) in instances.drain() {
            let _ = instance.process.kill();
            log::info!("Killed sidecar for session {}", session_id);
        }
    }
}

impl Drop for CoworksSidecarManager {
    fn drop(&mut self) {
        self.kill_all();
    }
}

/// Strip Windows extended-length path prefix (\\?\) that Node.js can't handle
fn strip_windows_prefix(path: &str) -> String {
    if path.starts_with(r"\\?\") {
        path[4..].to_string()
    } else {
        path.to_string()
    }
}

/// Get the sidecar script path
fn get_sidecar_path<R: Runtime>(app_handle: &AppHandle<R>) -> Result<String, String> {
    // Try production path first (in resources)
    let resource_path = app_handle
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;

    // Production path: resources/coworks-sidecar/index.js (no dist folder)
    let prod_path = resource_path.join("resources").join("coworks-sidecar").join("index.js");
    if prod_path.exists() {
        return Ok(strip_windows_prefix(&prod_path.to_string_lossy()));
    }

    // Try development path
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // Development mode: look in src-tauri/resources (deployed JS files)
            let dev_path = exe_dir
                .join("..")
                .join("resources")
                .join("coworks-sidecar")
                .join("index.js");
            if dev_path.exists() {
                let canonical = dev_path.canonicalize().unwrap().to_string_lossy().to_string();
                return Ok(strip_windows_prefix(&canonical));
            }

            // Also try the project root's sidecar build output
            let project_path = exe_dir
                .join("..")
                .join("..")
                .join("..")
                .join("sidecar")
                .join("coworks")
                .join("dist")
                .join("index.js");
            if project_path.exists() {
                let canonical = project_path.canonicalize().unwrap().to_string_lossy().to_string();
                return Ok(strip_windows_prefix(&canonical));
            }
        }
    }

    Err("Sidecar script not found".to_string())
}

// Tauri commands

#[tauri::command]
pub fn coworks_spawn_sidecar<R: Runtime>(
    app_handle: AppHandle<R>,
    state: tauri::State<'_, CoworksSidecarManager>,
    session_id: String,
    env: Option<HashMap<String, String>>,
) -> Result<(), String> {
    state.spawn(app_handle, session_id, env)
}

#[tauri::command]
pub fn coworks_send_to_sidecar(
    state: tauri::State<'_, CoworksSidecarManager>,
    session_id: String,
    message: String,
) -> Result<(), String> {
    state.send_message(&session_id, &message)
}

#[tauri::command]
pub fn coworks_kill_sidecar(
    state: tauri::State<'_, CoworksSidecarManager>,
    session_id: String,
) -> Result<(), String> {
    state.kill(&session_id)
}
