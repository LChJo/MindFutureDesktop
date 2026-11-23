//! Node.js PTY implementation for Windows (ConPTY)
//!
//! This module provides a PTY implementation that uses Node.js with node-pty
//! and the ConPTY backend — the native Windows pseudo-console API.
//!
//! Data flow (3 layers):
//! - stdout: Raw PTY bytes → Rust thread reads → Tauri event emit → Vue
//! - stderr: JSON control messages (init/ready/exit/error) → Rust thread reads → log
//! - stdin: JSON commands (spawn/write/resize/kill) → Rust sends
//!
//! ConPTY provides proper terminal reflow, better Unicode support,
//! and is the recommended backend for Windows 10+.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Arc;
use std::thread;
use tauri::Emitter;
use uuid::Uuid;

use crate::git::get_git_bin_path;
use crate::llm_router::{get_ccr_bin_path, get_mcd_bin_path};
use crate::nodejs::{get_nodejs_bin_path, get_npm_global_bin_path};
use crate::uv::get_uv_bin_path;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Serialize)]
#[serde(tag = "type")]
enum PtyCommand {
    #[serde(rename = "spawn")]
    Spawn {
        cols: u16,
        rows: u16,
        cwd: Option<String>,
        env: Option<HashMap<String, String>>,
        #[serde(rename = "promptConfig")]
        prompt_config: Option<serde_json::Value>,
        aliases: Option<Vec<crate::ClaudeAliasConfig>>,
    },
    #[serde(rename = "write")]
    Write { data: String },
    #[serde(rename = "resize")]
    Resize { cols: u16, rows: u16 },
    #[serde(rename = "kill")]
    Kill,
}

/// Control messages received via stderr (JSON, low frequency)
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum ControlMessage {
    #[serde(rename = "init")]
    Init { platform: String, version: String },
    #[serde(rename = "ready")]
    Ready { pid: Option<u32> },
    #[serde(rename = "exit")]
    Exit {
        code: Option<i32>,
        #[allow(dead_code)]
        signal: Option<i32>,
    },
    #[serde(rename = "error")]
    Error { message: String },
}

pub struct NodePtyInstance {
    stdin: Arc<Mutex<ChildStdin>>,
    #[allow(dead_code)]
    child: Arc<Mutex<Child>>,
    /// PID of the shell process (PowerShell) inside the PTY
    #[allow(dead_code)]
    shell_pid: Arc<Mutex<Option<u32>>>,
}

pub struct NodePtyManager {
    pub instances: Mutex<HashMap<String, Arc<NodePtyInstance>>>,
    node_path: String,
    pty_server_path: String,
}

impl NodePtyManager {
    pub fn new(node_path: String, pty_server_path: String) -> Self {
        log::info!(
            "NodePtyManager initialized with node: {}, server: {}",
            node_path,
            pty_server_path
        );
        Self {
            instances: Mutex::new(HashMap::new()),
            node_path,
            pty_server_path,
        }
    }

    pub fn spawn<R: tauri::Runtime>(
        &self,
        app_handle: tauri::AppHandle<R>,
        cols: u16,
        rows: u16,
        cwd: Option<String>,
        env: Option<HashMap<String, String>>,
        prompt_config: Option<serde_json::Value>,
        aliases: Option<Vec<crate::ClaudeAliasConfig>>,
    ) -> Result<String, String> {
        log::info!("NodePtyManager: spawning PTY via Node.js");

        // Start the Node.js PTY server process
        let mut cmd = Command::new(&self.node_path);
        cmd.arg(&self.pty_server_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to start Node.js PTY server: {}", e))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Failed to get stdin".to_string())?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to get stdout".to_string())?;

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "Failed to get stderr".to_string())?;

        let id = Uuid::new_v4().to_string();
        let id_clone = id.clone();
        let id_for_stderr = id.clone();

        let shell_pid: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let shell_pid_clone = shell_pid.clone();

        let instance = Arc::new(NodePtyInstance {
            stdin: Arc::new(Mutex::new(stdin)),
            child: Arc::new(Mutex::new(child)),
            shell_pid,
        });

        // Build environment with tool paths injected into PATH
        let mut final_env = env.unwrap_or_default();

        // Collect tool paths to inject
        let mut path_additions = Vec::new();

        if let Some(nodejs_path) = get_nodejs_bin_path() {
            path_additions.push(nodejs_path.clone());
            log::info!("[NodePty] Injected Node.js path: {}", nodejs_path);
        }

        if let Some(git_path) = get_git_bin_path() {
            path_additions.push(git_path.clone());
            log::info!("[NodePty] Injected Git path: {}", git_path);
        }

        if let Some(uv_path) = get_uv_bin_path() {
            path_additions.push(uv_path.clone());
            log::info!("[NodePty] Injected UV path: {}", uv_path);
        }

        if let Some(npm_bin_path) = get_npm_global_bin_path() {
            path_additions.push(npm_bin_path.clone());
            log::info!("[NodePty] Injected npm global bin path: {}", npm_bin_path);
        }

        if let Some(ccr_path) = get_ccr_bin_path() {
            path_additions.push(ccr_path.clone());
            log::info!("[NodePty] Injected CCR path: {}", ccr_path);
        }

        if let Some(mcd_path) = get_mcd_bin_path() {
            path_additions.push(mcd_path.clone());
            log::info!("[NodePty] Injected mcd path: {}", mcd_path);
        }

        // Build new PATH: always include system PATH, prepend tool paths if any
        let current_path = std::env::var("PATH").unwrap_or_default();
        let path_separator = ";"; // Windows
        let new_path = if !path_additions.is_empty() {
            format!(
                "{}{}{}",
                path_additions.join(path_separator),
                path_separator,
                current_path
            )
        } else {
            current_path
        };
        final_env.insert("PATH".to_string(), new_path);
        log::info!(
            "[NodePty] PATH set with {} tool paths prepended",
            path_additions.len()
        );

        // On Windows the bundled node binary lives at nodejs\node.exe (not nodejs\bin\node.exe),
        // so npm resolves its default global prefix to .MoontonClaudeDesktop instead of
        // .MoontonClaudeDesktop\nodejs.  Pin it to %APPDATA%\npm so manual
        // `npm install -g` lands in the same directory that PATH already contains.
        #[cfg(target_os = "windows")]
        if let Some(npm_prefix) = dirs::data_dir().map(|d| d.join("npm")) {
            final_env.insert(
                "NPM_CONFIG_PREFIX".to_string(),
                npm_prefix.to_string_lossy().to_string(),
            );
            log::info!("[NodePty] Set NPM_CONFIG_PREFIX={:?}", npm_prefix);
        }

        // Send spawn command with constructed environment and prompt config
        let spawn_cmd = PtyCommand::Spawn {
            cols,
            rows,
            cwd,
            env: Some(final_env),
            prompt_config,
            aliases,
        };
        self.send_command(&instance, &spawn_cmd)?;

        self.instances.lock().insert(id.clone(), instance);

        // Thread 1: Read raw PTY data from stdout (same pattern as pty.rs)
        thread::spawn(move || {
            let mut buf = [0u8; 16384];
            let mut stdout = stdout;
            loop {
                match stdout.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = buf[..n].to_vec();
                        let event_name = format!("terminal-output-{}", id_clone);
                        let _ = app_handle.emit(&event_name, data);
                    }
                    Err(e) => {
                        log::error!("Error reading stdout from Node PTY: {}", e);
                        break;
                    }
                }
            }
        });

        // Thread 2: Read JSON control messages from stderr
        thread::spawn(move || {
            let reader = BufReader::new(stderr);

            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if let Ok(msg) = serde_json::from_str::<ControlMessage>(&line) {
                            match msg {
                                ControlMessage::Init { platform, version } => {
                                    log::info!(
                                        "Node PTY server initialized: {} {}",
                                        platform,
                                        version
                                    );
                                }
                                ControlMessage::Ready { pid } => {
                                    log::info!(
                                        "Node PTY [{}] ready with PID: {:?}",
                                        id_for_stderr,
                                        pid
                                    );
                                    if let Some(p) = pid {
                                        *shell_pid_clone.lock() = Some(p);
                                    }
                                }
                                ControlMessage::Exit { code, .. } => {
                                    log::info!("Node PTY exited with code: {:?}", code);
                                    break;
                                }
                                ControlMessage::Error { message } => {
                                    log::error!("Node PTY error: {}", message);
                                }
                            }
                        } else {
                            // Non-JSON stderr output (e.g. Node.js warnings)
                            log::warn!("[NodePty stderr] {}", line);
                        }
                    }
                    Err(e) => {
                        log::error!("Error reading stderr from Node PTY: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(id)
    }

    fn send_command(&self, instance: &NodePtyInstance, cmd: &PtyCommand) -> Result<(), String> {
        let json = serde_json::to_string(cmd).map_err(|e| e.to_string())?;
        let mut stdin = instance.stdin.lock();
        writeln!(stdin, "{}", json).map_err(|e| format!("Failed to write to PTY: {}", e))?;
        stdin
            .flush()
            .map_err(|e| format!("Failed to flush PTY: {}", e))?;
        Ok(())
    }

    pub fn write(&self, id: &str, data: &[u8]) -> Result<(), String> {
        let instances = self.instances.lock();
        let instance = instances
            .get(id)
            .ok_or_else(|| "PTY instance not found".to_string())?;

        // Send UTF-8 text directly (no base64 encoding)
        let text = String::from_utf8_lossy(data);
        let cmd = PtyCommand::Write {
            data: text.into_owned(),
        };
        self.send_command(instance, &cmd)
    }

    pub fn kill(&self, id: &str) -> Result<(), String> {
        let mut instances = self.instances.lock();
        if let Some(instance) = instances.get(id) {
            let cmd = PtyCommand::Kill;
            let _ = self.send_command(instance, &cmd);
        }
        instances.remove(id);
        Ok(())
    }

    pub fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<(), String> {
        let instances = self.instances.lock();
        let instance = instances
            .get(id)
            .ok_or_else(|| "PTY instance not found".to_string())?;

        let cmd = PtyCommand::Resize { cols, rows };
        self.send_command(instance, &cmd)
    }
}
