use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use tauri::{AppHandle, Emitter};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// UV installation status
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UvStatus {
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub source: Option<String>, // "system" or "local"
    pub error: Option<String>,
}

/// Download progress event payload
#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub component: String,
    pub downloaded: u64,
    pub total: u64,
    pub percent: f64,
    pub stage: String, // "downloading", "extracting", "done", "error"
    pub message: String,
}

/// Emit download progress event
fn emit_progress<R: tauri::Runtime>(
    app_handle: &AppHandle<R>,
    stage: &str,
    percent: f64,
    message: &str,
) {
    let progress = DownloadProgress {
        component: "uv".to_string(),
        downloaded: 0,
        total: 0,
        percent,
        stage: stage.to_string(),
        message: message.to_string(),
    };

    let _ = app_handle.emit("download-progress", progress);
}

/// Get the UV executable path in user's local bin
pub fn get_local_uv_path() -> Option<PathBuf> {
    let home_dir = dirs::home_dir()?;
    let uv_path = home_dir.join(".local").join("bin").join("uv.exe");
    if uv_path.exists() {
        return Some(uv_path);
    }
    // Also check without .exe for cross-platform
    let uv_path_no_ext = home_dir.join(".local").join("bin").join("uv");
    if uv_path_no_ext.exists() {
        return Some(uv_path_no_ext);
    }
    None
}

/// Check for system-installed UV (in PATH)
/// Returns (version, path) if found
/// Note: This uses 'where' command first to avoid hanging on missing executables
pub fn check_system_uv() -> Option<(String, String)> {
    // First, try to find uv using 'where' command (fast, won't hang)
    #[cfg(target_os = "windows")]
    {
        let mut where_cmd = Command::new("where");
        where_cmd.arg("uv");
        where_cmd.creation_flags(CREATE_NO_WINDOW);

        let where_output = where_cmd.output().ok()?;
        if !where_output.status.success() {
            // uv not found in PATH
            return None;
        }

        let uv_path = String::from_utf8_lossy(&where_output.stdout)
            .lines()
            .next()
            .map(|p| p.trim().to_string())?;

        if uv_path.is_empty() {
            return None;
        }

        // Now get version using the full path
        let mut cmd = Command::new(&uv_path);
        cmd.args(["--version"]);
        cmd.creation_flags(CREATE_NO_WINDOW);

        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let version_output = String::from_utf8_lossy(&output.stdout);
                let version = version_output
                    .trim()
                    .strip_prefix("uv ")
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| version_output.trim().to_string());

                let path = PathBuf::from(&uv_path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "System PATH".to_string());

                return Some((version, path));
            }
        }

        // Found in PATH but couldn't get version
        let path = PathBuf::from(&uv_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "System PATH".to_string());
        return Some(("unknown".to_string(), path));
    }

    #[cfg(target_os = "macos")]
    {
        // First try the known local installation path (~/.local/bin/uv)
        if let Some(home_dir) = dirs::home_dir() {
            let local_uv_path = home_dir.join(".local").join("bin").join("uv");
            if local_uv_path.exists() {
                log::info!("Checking UV version at: {:?}", local_uv_path);
                let mut cmd = Command::new(&local_uv_path);
                cmd.args(["--version"]);

                if let Ok(output) = cmd.output() {
                    if output.status.success() {
                        let version_output = String::from_utf8_lossy(&output.stdout);
                        let version = version_output
                            .trim()
                            .strip_prefix("uv ")
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| version_output.trim().to_string());

                        log::info!("UV version output: {}", version);

                        let path = local_uv_path
                            .parent()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|| "~/.local/bin".to_string());

                        return Some((version, path));
                    } else {
                        log::warn!("UV command failed with status: {:?}", output.status);
                    }
                } else {
                    log::warn!("Failed to execute UV command at: {:?}", local_uv_path);
                }
            } else {
                log::info!("UV not found at default local path: {:?}", local_uv_path);
            }
        }

        // Fallback: Use 'which' command to find UV in system PATH
        log::info!("Attempting to find UV via 'which' command");
        let mut which_cmd = Command::new("which");
        which_cmd.arg("uv");

        if let Ok(which_output) = which_cmd.output() {
            if which_output.status.success() {
                let uv_path = String::from_utf8_lossy(&which_output.stdout)
                    .trim()
                    .to_string();

                if !uv_path.is_empty() {
                    log::info!("Found UV via 'which' at: {}", uv_path);

                    // Get version
                    let mut cmd = Command::new(&uv_path);
                    cmd.args(["--version"]);

                    if let Ok(output) = cmd.output() {
                        if output.status.success() {
                            let version_output = String::from_utf8_lossy(&output.stdout);
                            let version = version_output
                                .trim()
                                .strip_prefix("uv ")
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| version_output.trim().to_string());

                            log::info!("UV version from system PATH: {}", version);

                            let path = PathBuf::from(&uv_path)
                                .parent()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_else(|| "System PATH".to_string());

                            return Some((version, path));
                        }
                    }
                }
            }
        }

        // Last resort: try direct command
        log::info!("Attempting direct UV command execution");
        let mut cmd = Command::new("uv");
        cmd.args(["--version"]);

        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let version_output = String::from_utf8_lossy(&output.stdout);
                let version = version_output
                    .trim()
                    .strip_prefix("uv ")
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| version_output.trim().to_string());

                return Some((version, "System PATH".to_string()));
            }
        }

        log::info!("UV not found in any known location");
        None
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        // On other Unix systems, use 'which' command
        let mut which_cmd = Command::new("which");
        which_cmd.arg("uv");

        let which_output = which_cmd.output().ok()?;
        if !which_output.status.success() {
            return None;
        }

        let uv_path = String::from_utf8_lossy(&which_output.stdout)
            .trim()
            .to_string();

        if uv_path.is_empty() {
            return None;
        }

        // Get version
        let mut cmd = Command::new(&uv_path);
        cmd.args(["--version"]);

        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let version_output = String::from_utf8_lossy(&output.stdout);
                let version = version_output
                    .trim()
                    .strip_prefix("uv ")
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| version_output.trim().to_string());

                let path = PathBuf::from(&uv_path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "System PATH".to_string());

                return Some((version, path));
            }
        }

        None
    }
}

/// Check if UV is installed in user's local bin
pub fn is_local_uv_installed() -> bool {
    get_local_uv_path().is_some()
}

/// Get the installed UV version from local installation
pub fn get_local_uv_version() -> Option<String> {
    let uv_exe = get_local_uv_path()?;

    let mut cmd = Command::new(&uv_exe);
    cmd.args(["--version"]);

    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output().ok()?;

    if !output.status.success() {
        return None;
    }

    let version_output = String::from_utf8_lossy(&output.stdout);
    version_output
        .trim()
        .strip_prefix("uv ")
        .map(|v| v.to_string())
}

/// Install UV using the official installation script (without progress)
#[allow(dead_code)]
pub fn install_uv() -> Result<(), String> {
    install_uv_internal::<tauri::Wry>(None)
}

/// Install UV using the official installation script with progress reporting
pub fn install_uv_with_progress<R: tauri::Runtime>(app_handle: &AppHandle<R>) -> Result<(), String> {
    install_uv_internal(Some(app_handle))
}

/// Get progress message based on current progress percentage
fn get_progress_message(progress: f64) -> &'static str {
    if progress < 20.0 {
        "正在准备安装环境..."
    } else if progress < 40.0 {
        "正在下载 UV..."
    } else if progress < 60.0 {
        "正在安装 UV..."
    } else if progress < 80.0 {
        "正在配置 UV..."
    } else {
        "正在完成安装..."
    }
}

/// Internal UV installation function with timeout and retry logic
/// - 3 minute timeout per attempt
/// - Up to 3 retry attempts
/// - Retries on timeout, script failure, or verification failure
fn install_uv_internal<R: tauri::Runtime>(app_handle: Option<&AppHandle<R>>) -> Result<(), String> {
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    use std::time::{Duration, Instant};

    const INSTALL_TIMEOUT_SECS: u64 = 180; // 3 minutes
    const MAX_RETRIES: u32 = 3;

    log::info!("Starting UV installation (timeout: {}s, max retries: {})...", INSTALL_TIMEOUT_SECS, MAX_RETRIES);

    let mut last_error = String::new();

    for attempt in 1..=MAX_RETRIES {
        log::info!("UV installation attempt {}/{}", attempt, MAX_RETRIES);

        if let Some(handle) = app_handle {
            if attempt == 1 {
                emit_progress(handle, "downloading", 5.0, "正在准备安装环境...");
            } else {
                emit_progress(handle, "downloading", 5.0, &format!("正在重试安装 (第{}次)...", attempt));
            }
        }

        let start_time = Instant::now();
        let install_complete = Arc::new(AtomicBool::new(false));
        let install_error = Arc::new(std::sync::Mutex::new(None::<String>));

        // Start installation in background thread
        #[cfg(target_os = "windows")]
        {
            let install_complete_clone = Arc::clone(&install_complete);
            let install_error_clone = Arc::clone(&install_error);

            let _install_thread = std::thread::spawn(move || {
                log::info!("Running UV installation script via PowerShell...");

                let mut cmd = Command::new("powershell");
                cmd.args([
                    "-ExecutionPolicy", "ByPass",
                    "-Command",
                    "irm https://astral.sh/uv/install.ps1 | iex"
                ]);
                cmd.creation_flags(CREATE_NO_WINDOW);

                match cmd.output() {
                    Ok(output) => {
                        if !output.status.success() {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let err = format!("UV installation failed: stderr={} stdout={}", stderr, stdout);
                            log::error!("{}", err);
                            *install_error_clone.lock().unwrap() = Some(err);
                        } else {
                            log::info!("UV installation script completed");
                        }
                    }
                    Err(e) => {
                        let err = format!("Failed to run PowerShell: {}", e);
                        log::error!("{}", err);
                        *install_error_clone.lock().unwrap() = Some(err);
                    }
                }

                install_complete_clone.store(true, Ordering::SeqCst);
            });
        }

        #[cfg(not(target_os = "windows"))]
        {
            let install_complete_clone = Arc::clone(&install_complete);
            let install_error_clone = Arc::clone(&install_error);

            let _install_thread = std::thread::spawn(move || {
                log::info!("Running UV installation script via curl...");

                let mut cmd = Command::new("sh");
                cmd.args([
                    "-c",
                    "curl -LsSf https://astral.sh/uv/install.sh | sh"
                ]);

                match cmd.output() {
                    Ok(output) => {
                        if !output.status.success() {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            let err = format!("UV installation failed: {}", stderr);
                            log::error!("{}", err);
                            *install_error_clone.lock().unwrap() = Some(err);
                        } else {
                            log::info!("UV installation script completed");
                        }
                    }
                    Err(e) => {
                        let err = format!("Failed to run install script: {}", e);
                        log::error!("{}", err);
                        *install_error_clone.lock().unwrap() = Some(err);
                    }
                }

                install_complete_clone.store(true, Ordering::SeqCst);
            });
        }

        // Wait for installation with timeout
        let mut progress = 10.0;
        let mut timed_out = false;

        while !install_complete.load(Ordering::SeqCst) {
            let elapsed = start_time.elapsed().as_secs();

            // Check for timeout
            if elapsed >= INSTALL_TIMEOUT_SECS {
                log::warn!("UV installation attempt {} timed out after {}s", attempt, elapsed);
                timed_out = true;
                break;
            }

            // Update progress with remaining time
            if let Some(handle) = app_handle {
                let base_message = get_progress_message(progress);
                let remaining = INSTALL_TIMEOUT_SECS - elapsed;
                let message = if attempt > 1 {
                    format!("{} (第{}次, 剩余{}s)", base_message, attempt, remaining)
                } else {
                    format!("{} (剩余{}s)", base_message, remaining)
                };
                emit_progress(handle, "downloading", progress, &message);
            }

            progress += 0.5;
            if progress > 85.0 {
                progress = 85.0;
            }

            std::thread::sleep(Duration::from_millis(300));
        }

        // Handle timeout
        if timed_out {
            last_error = format!("Installation timed out after {}s", INSTALL_TIMEOUT_SECS);
            if attempt < MAX_RETRIES {
                if let Some(handle) = app_handle {
                    emit_progress(handle, "downloading", 0.0, &format!("安装超时，准备第{}次重试...", attempt + 1));
                }
                std::thread::sleep(Duration::from_secs(2));
                continue;
            }
        } else {
            // Check for script errors
            if let Some(err) = install_error.lock().unwrap().take() {
                last_error = err.clone();
                if attempt < MAX_RETRIES {
                    log::warn!("UV installation attempt {} failed: {}", attempt, err);
                    if let Some(handle) = app_handle {
                        emit_progress(handle, "downloading", 0.0, &format!("安装失败，准备第{}次重试...", attempt + 1));
                    }
                    std::thread::sleep(Duration::from_secs(2));
                    continue;
                }
            } else {
                // Script succeeded, verify installation
                if let Some(handle) = app_handle {
                    emit_progress(handle, "extracting", 90.0, "正在验证 UV 安装...");
                }

                log::info!("Waiting for UV installation to complete...");
                std::thread::sleep(Duration::from_millis(500));

                if is_local_uv_installed() || check_system_uv().is_some() {
                    // Success!
                    if let Some(handle) = app_handle {
                        emit_progress(handle, "done", 100.0, "UV 安装完成");
                    }
                    log::info!("UV installed successfully on attempt {}", attempt);
                    return Ok(());
                }

                // Verification failed
                last_error = "UV installation completed but executable not found".to_string();
                if attempt < MAX_RETRIES {
                    log::warn!("UV not found after attempt {}, retrying...", attempt);
                    if let Some(handle) = app_handle {
                        emit_progress(handle, "downloading", 0.0, &format!("验证失败，准备第{}次重试...", attempt + 1));
                    }
                    std::thread::sleep(Duration::from_secs(2));
                    continue;
                }
            }
        }
    }

    // All retries exhausted
    log::error!("UV installation failed after {} attempts: {}", MAX_RETRIES, last_error);
    if let Some(handle) = app_handle {
        emit_progress(handle, "error", 0.0, &format!("UV 安装失败 (已重试{}次)", MAX_RETRIES));
    }
    Err(format!("UV installation failed after {} attempts: {}", MAX_RETRIES, last_error))
}

/// Get the UV bin path for terminal PATH injection
/// Returns the directory containing uv executable if available
pub fn get_uv_bin_path() -> Option<String> {
    // First check local installation
    if let Some(uv_path) = get_local_uv_path() {
        if let Some(parent) = uv_path.parent() {
            return Some(parent.to_string_lossy().to_string());
        }
    }
    None
}

// Tauri commands

/// Check UV installation status
#[tauri::command]
pub fn check_uv_status() -> UvStatus {
    // First, check for system-installed UV
    if let Some((version, path)) = check_system_uv() {
        return UvStatus {
            installed: true,
            version: Some(version),
            path: Some(path),
            source: Some("system".to_string()),
            error: None,
        };
    }

    // Fallback: check local UV in ~/.local/bin
    if is_local_uv_installed() {
        let version = get_local_uv_version();
        let path = get_local_uv_path().map(|p| {
            p.parent()
                .map(|parent| parent.to_string_lossy().to_string())
                .unwrap_or_default()
        });
        return UvStatus {
            installed: true,
            version,
            path,
            source: Some("local".to_string()),
            error: None,
        };
    }

    // Not installed
    UvStatus {
        installed: false,
        version: None,
        path: None,
        source: None,
        error: None,
    }
}

/// Install UV
#[tauri::command]
pub async fn install_uv_cmd(app_handle: AppHandle) -> Result<(), String> {
    // Run in blocking thread since it's a network operation
    tauri::async_runtime::spawn_blocking(move || {
        install_uv_with_progress(&app_handle)
    }).await.map_err(|e| format!("Task join error: {}", e))?
}
