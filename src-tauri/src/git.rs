use serde::{Deserialize, Serialize};
#[cfg(target_os = "windows")]
use std::fs::{self, File};

#[cfg(target_os = "windows")]
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::time::Duration;
use tauri::AppHandle;

#[cfg(target_os = "windows")]
use tauri::Emitter;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Git installation status
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitStatus {
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub source: Option<String>, // "system" or "bundled"
    pub error: Option<String>,  // Error details if detection failed
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

// The progress payload is currently used by the Windows PortableGit downloader.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
type _DownloadProgressKeep = DownloadProgress;

// NOTE: The bundled PortableGit downloader is currently Windows-only.
// Gate the related constants behind cfg to avoid dead_code warnings on macOS/Linux.

#[cfg(target_os = "windows")]
// PortableGit download info
const GIT_VERSION: &str = "2.47.1.2";

#[cfg(target_os = "windows")]
// Multiple mirror URLs for faster download (in priority order)
const GIT_DOWNLOAD_MIRRORS: &[&str] = &[
    // gh-proxy.com 镜像 (测试可用，速度约 1 MB/s)
    "https://gh-proxy.com/https://github.com/git-for-windows/git/releases/download/v2.47.1.windows.2/PortableGit-2.47.1.2-64-bit.7z.exe",
    // GitHub 官方 (备用，速度较慢)
    "https://github.com/git-for-windows/git/releases/download/v2.47.1.windows.2/PortableGit-2.47.1.2-64-bit.7z.exe",
];

#[cfg(target_os = "windows")]
// Download configuration
const MAX_RETRIES: u32 = 3;

#[cfg(target_os = "windows")]
const CONNECT_TIMEOUT_SECS: u64 = 15;
#[cfg(target_os = "windows")]
fn create_download_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(DOWNLOAD_TIMEOUT_SECS))
        .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .gzip(true)
        .brotli(true)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}

#[cfg(target_os = "windows")]
const DOWNLOAD_TIMEOUT_SECS: u64 = 300; // 5 minutes

/// Get the git directory path (~/.MoontonClaudeDesktop/git)
pub fn get_git_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir()
        .ok_or_else(|| "Failed to get home directory".to_string())?;
    Ok(home.join(".MoontonClaudeDesktop").join("git"))
}

/// Get the git executable path (bundled)
pub fn get_git_exe_path() -> Option<PathBuf> {
    let git_dir = get_git_dir().ok()?;
    // PortableGit extracts with cmd/git.exe as the main executable
    let git_cmd = git_dir.join("cmd").join("git.exe");
    if git_cmd.exists() {
        return Some(git_cmd);
    }
    // Also check bin/git.exe
    let git_bin = git_dir.join("bin").join("git.exe");
    if git_bin.exists() {
        return Some(git_bin);
    }
    None
}

/// Check for system-installed Git (in PATH)
/// Returns (version, path) if found
pub fn check_system_git() -> Option<(String, String)> {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("git");
        cmd.args(["--version"]);
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = cmd.output().ok()?;

        if !output.status.success() {
            return None;
        }

        let version_output = String::from_utf8_lossy(&output.stdout);
        // Parse "git version 2.52.0.windows.1" to extract version
        let version = version_output
            .trim()
            .strip_prefix("git version ")
            .map(|v| {
                // Extract just the version number (e.g., "2.52.0" from "2.52.0.windows.1")
                v.split('.')
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(".")
            })
            .unwrap_or_else(|| version_output.trim().to_string());

        if version.is_empty() {
            return None;
        }

        // Try to get git path using where command
        let mut where_cmd = Command::new("where");
        where_cmd.arg("git");
        where_cmd.creation_flags(CREATE_NO_WINDOW);

        if let Ok(where_output) = where_cmd.output() {
            if where_output.status.success() {
                let path = String::from_utf8_lossy(&where_output.stdout)
                    .lines()
                    .next()
                    .map(|p| {
                        // Get parent directory of git.exe
                        PathBuf::from(p.trim())
                            .parent()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|| p.trim().to_string())
                    })
                    .unwrap_or_default();

                if !path.is_empty() {
                    return Some((version, path));
                }
            }
        }

        // Fallback: just return version without path
        Some((version, "System PATH".to_string()))
    }

    #[cfg(target_os = "macos")]
    {
        // First try known standard paths (doesn't depend on PATH)
        let known_paths = vec![
            "/usr/bin/git",                          // System default
            "/usr/local/bin/git",                       // Homebrew (Intel) / Manual install
            "/opt/homebrew/bin/git",                     // Homebrew (Apple Silicon)
            "/Applications/Xcode.app/Contents/Developer/usr/bin/git",  // Xcode
        ];

        for git_path_str in known_paths {
            let git_path = PathBuf::from(git_path_str);
            if git_path.exists() {
                log::info!("Checking Git version at: {:?}", git_path);
                let mut cmd = Command::new(&git_path);
                cmd.args(["--version"]);

                if let Ok(output) = cmd.output() {
                    if output.status.success() {
                        let version_output = String::from_utf8_lossy(&output.stdout);
                        let version = version_output
                            .trim()
                            .strip_prefix("git version ")
                            .map(|v| {
                                v.split('.')
                                    .take(3)
                                    .collect::<Vec<_>>()
                                    .join(".")
                            })
                            .unwrap_or_else(|| version_output.trim().to_string());

                        if !version.is_empty() {
                            log::info!("Git version output: {}", version);
                            let path = git_path
                                .parent()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_else(|| git_path_str.to_string());
                            return Some((version, path));
                        }
                    } else {
                        log::warn!("Git command failed with status: {:?}", output.status);
                    }
                } else {
                    log::warn!("Failed to execute git command at: {:?}", git_path);
                }
            }
        }

        log::info!("Git not found in known paths, trying 'which' command");

        // Fallback: Use 'which' command to find Git in system PATH
        if let Ok(which_output) = Command::new("which").arg("git").output() {
            if which_output.status.success() {
                let git_path_str = String::from_utf8_lossy(&which_output.stdout)
                    .trim()
                    .to_string();

                if !git_path_str.is_empty() {
                    log::info!("Found Git via 'which' at: {}", git_path_str);
                    let mut cmd = Command::new(&git_path_str);
                    cmd.args(["--version"]);

                    if let Ok(output) = cmd.output() {
                        if output.status.success() {
                            let version_output = String::from_utf8_lossy(&output.stdout);
                            let version = version_output
                                .trim()
                                .strip_prefix("git version ")
                                .map(|v| {
                                    v.split('.')
                                        .take(3)
                                        .collect::<Vec<_>>()
                                        .join(".")
                                })
                                .unwrap_or_else(|| version_output.trim().to_string());

                            if !version.is_empty() {
                                log::info!("Git version from system PATH: {}", version);
                                let path = PathBuf::from(&git_path_str)
                                    .parent()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "System PATH".to_string());
                                return Some((version, path));
                            }
                        }
                    }
                }
            }
        }

        // Last resort: try direct command
        log::info!("Attempting direct git command execution");
        let mut cmd = Command::new("git");
        cmd.args(["--version"]);

        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let version_output = String::from_utf8_lossy(&output.stdout);
                let version = version_output
                    .trim()
                    .strip_prefix("git version ")
                    .map(|v| {
                        v.split('.')
                            .take(3)
                            .collect::<Vec<_>>()
                            .join(".")
                    })
                    .unwrap_or_else(|| version_output.trim().to_string());

                if !version.is_empty() {
                    return Some((version, "System PATH".to_string()));
                }
            }
        }

        log::info!("Git not found in any known location");
        None
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let mut cmd = Command::new("git");
        cmd.args(["--version"]);

        let output = cmd.output().ok()?;

        if !output.status.success() {
            return None;
        }

        let version_output = String::from_utf8_lossy(&output.stdout);
        let version = version_output
            .trim()
            .strip_prefix("git version ")
            .map(|v| {
                v.split('.')
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(".")
            })
            .unwrap_or_else(|| version_output.trim().to_string());

        if version.is_empty() {
            return None;
        }

        Some((version, "System PATH".to_string()))
    }
}

/// Check if Git is installed in AppData (bundled)
#[cfg(target_os = "windows")]
pub fn is_git_installed() -> bool {
    get_git_exe_path().is_some()
}

/// Get the installed Git version from bundled installation
#[cfg(target_os = "windows")]
pub fn get_bundled_git_version() -> Option<String> {
    let git_exe = get_git_exe_path()?;

    let mut cmd = Command::new(&git_exe);
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
        .strip_prefix("git version ")
        .map(|v| {
            v.split('.')
                .take(3)
                .collect::<Vec<_>>()
                .join(".")
        })
}

/// Emit download progress event
#[cfg(target_os = "windows")]
fn emit_progress<R: tauri::Runtime>(
    app_handle: &AppHandle<R>,
    component: &str,
    downloaded: u64,
    total: u64,
    stage: &str,
    message: &str,
) {
    let percent = if total > 0 {
        (downloaded as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    let progress = DownloadProgress {
        component: component.to_string(),
        downloaded,
        total,
        percent,
        stage: stage.to_string(),
        message: message.to_string(),
    };

    let _ = app_handle.emit("download-progress", progress);
}

/// Attempt to download from a single URL with streaming progress
#[cfg(target_os = "windows")]
async fn attempt_download<R: tauri::Runtime>(
    app_handle: &AppHandle<R>,
    client: &reqwest::Client,
    url: &str,
    download_path: &PathBuf,
) -> Result<u64, String> {
    log::info!("Attempting download from: {}", url);

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("连接失败: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP 错误: {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);
    log::info!("Download size: {} bytes", total_size);

    emit_progress(
        app_handle,
        "git",
        0,
        total_size,
        "downloading",
        &format!("正在下载 PortableGit ({:.1} MB)...", total_size as f64 / 1024.0 / 1024.0),
    );

    // Stream download with progress
    let mut file = File::create(download_path).map_err(|e| {
        format!("创建文件失败: {}", e)
    })?;

    use futures_util::StreamExt;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_progress_update = std::time::Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("下载数据错误: {}", e))?;

        file.write_all(&chunk).map_err(|e| {
            format!("写入文件失败: {}", e)
        })?;

        downloaded += chunk.len() as u64;

        // Update progress at most every 100ms to avoid overwhelming the UI
        if last_progress_update.elapsed().as_millis() >= 100 {
            let percent = if total_size > 0 {
                (downloaded as f64 / total_size as f64) * 100.0
            } else {
                0.0
            };
            emit_progress(
                app_handle,
                "git",
                downloaded,
                total_size,
                "downloading",
                &format!("正在下载 Git... {:.1}%", percent),
            );
            last_progress_update = std::time::Instant::now();
        }
    }

    drop(file);
    log::info!("Download completed: {} bytes", downloaded);
    Ok(total_size)
}

/// Download from a URL with retry logic
#[cfg(target_os = "windows")]
async fn download_with_retry<R: tauri::Runtime>(
    app_handle: &AppHandle<R>,
    client: &reqwest::Client,
    url: &str,
    download_path: &PathBuf,
) -> Result<u64, String> {
    let mut last_error = String::new();

    for retry in 0..MAX_RETRIES {
        if retry > 0 {
            let delay_secs = 2_u64.pow(retry); // 2s, 4s, 8s exponential backoff
            log::warn!("Retry {}/{} after {}s delay", retry, MAX_RETRIES, delay_secs);
            emit_progress(
                app_handle,
                "git",
                0,
                0,
                "downloading",
                &format!("下载失败，{}秒后重试 ({}/{})...", delay_secs, retry, MAX_RETRIES),
            );
            tokio::time::sleep(Duration::from_secs(delay_secs)).await;
        }

        match attempt_download(app_handle, client, url, download_path).await {
            Ok(size) => return Ok(size),
            Err(e) => {
                log::warn!("Download attempt {} failed: {}", retry + 1, e);
                last_error = e;
            }
        }
    }

    Err(last_error)
}

/// Try to download from multiple mirrors with failover
#[cfg(target_os = "windows")]
async fn download_from_mirrors<R: tauri::Runtime>(
    app_handle: &AppHandle<R>,
    client: &reqwest::Client,
    download_path: &PathBuf,
) -> Result<u64, String> {
    let total_mirrors = GIT_DOWNLOAD_MIRRORS.len();
    let mut last_error = String::new();

    for (index, mirror_url) in GIT_DOWNLOAD_MIRRORS.iter().enumerate() {
        log::info!("Trying mirror {}/{}: {}", index + 1, total_mirrors, mirror_url);
        emit_progress(
            app_handle,
            "git",
            0,
            0,
            "downloading",
            &format!("尝试下载源 {}/{}...", index + 1, total_mirrors),
        );

        match download_with_retry(app_handle, client, mirror_url, download_path).await {
            Ok(size) => {
                log::info!("Successfully downloaded from mirror {}", index + 1);
                return Ok(size);
            }
            Err(e) => {
                log::warn!("Mirror {} failed: {}", mirror_url, e);
                last_error = format!("镜像源 {} 失败: {}", index + 1, e);
                continue;
            }
        }
    }

    Err(format!("所有下载源均失败: {}", last_error))
}

/// Download and install PortableGit
#[cfg(target_os = "windows")]
pub async fn download_and_install_git<R: tauri::Runtime>(
    app_handle: AppHandle<R>,
) -> Result<(), String> {
    log::info!("Starting PortableGit download with {} mirrors...", GIT_DOWNLOAD_MIRRORS.len());

    let git_dir = get_git_dir().map_err(|e| {
        log::error!("Failed to get git directory: {}", e);
        e
    })?;

    // Create temp directory for download
    let temp_dir = std::env::temp_dir().join("moonton_claude_git_install");
    fs::create_dir_all(&temp_dir).map_err(|e| {
        let err = format!("Failed to create temp directory: {}", e);
        log::error!("{}", err);
        err
    })?;

    let download_path = temp_dir.join("PortableGit.7z.exe");

    emit_progress(&app_handle, "git", 0, 0, "downloading", "正在初始化下载...");

    // Create optimized HTTP client
    let client = create_download_client().map_err(|e| {
        emit_progress(&app_handle, "git", 0, 0, "error", &e);
        e
    })?;

    // Try downloading from mirrors with failover and retry
    let total_size = download_from_mirrors(&app_handle, &client, &download_path)
        .await
        .map_err(|e| {
            emit_progress(&app_handle, "git", 0, 0, "error", &e);
            e
        })?;

    log::info!("Download completed: {:?}", download_path);

    emit_progress(&app_handle, "git", total_size, total_size, "extracting", "正在解压 Git...");

    // Create git directory
    fs::create_dir_all(&git_dir).map_err(|e| {
        let err = format!("Failed to create git directory: {}", e);
        log::error!("{}", err);
        emit_progress(&app_handle, "git", 0, 0, "error", &err);
        err
    })?;

    // Extract using the self-extracting archive
    // PortableGit-*.7z.exe supports -o<path> -y for silent extraction
    log::info!("Extracting to: {:?}", git_dir);

    let mut extract_cmd = Command::new(&download_path);
    extract_cmd.args([
        &format!("-o{}", git_dir.to_string_lossy()),
        "-y", // Yes to all
    ]);

    #[cfg(target_os = "windows")]
    extract_cmd.creation_flags(CREATE_NO_WINDOW);

    let extract_output = extract_cmd.output().map_err(|e| {
        let err = format!("Failed to run extractor: {}", e);
        log::error!("{}", err);
        emit_progress(&app_handle, "git", 0, 0, "error", &err);
        err
    })?;

    if !extract_output.status.success() {
        let stderr = String::from_utf8_lossy(&extract_output.stderr);
        let stdout = String::from_utf8_lossy(&extract_output.stdout);
        let err = format!("Extraction failed: stdout={} stderr={}", stdout, stderr);
        log::error!("{}", err);
        emit_progress(&app_handle, "git", 0, 0, "error", "解压失败");
        return Err(err);
    }

    log::info!("Extraction completed");

    // Clean up temp file
    let _ = fs::remove_file(&download_path);
    let _ = fs::remove_dir_all(&temp_dir);

    // Verify installation
    if !is_git_installed() {
        let err = "Git installation completed but executable not found".to_string();
        log::error!("{}", err);
        emit_progress(&app_handle, "git", 0, 0, "error", "安装验证失败");
        return Err(err);
    }

    emit_progress(
        &app_handle,
        "git",
        total_size,
        total_size,
        "done",
        &format!("Git {} 安装完成", GIT_VERSION),
    );

    log::info!(
        "PortableGit {} installed successfully to: {:?}",
        GIT_VERSION,
        git_dir
    );
    Ok(())
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub async fn download_and_install_git<R: tauri::Runtime>(
    _app_handle: AppHandle<R>,
) -> Result<(), String> {
    Err("Bundled Git installer is only supported on Windows.".to_string())
}

/// Get the git directory path as string
#[allow(dead_code)]
pub fn get_git_dir_string() -> String {
    get_git_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "~/.MoontonClaudeDesktop/git".to_string())
}

// Tauri commands

/// Check Git installation status
#[tauri::command]
pub fn check_git_status() -> GitStatus {
    // First, check for system-installed Git
    if let Some((version, path)) = check_system_git() {
        return GitStatus {
            installed: true,
            version: Some(version),
            path: Some(path),
            source: Some("system".to_string()),
            error: None,
        };
    }

    // On Windows, check bundled Git in AppData
    #[cfg(target_os = "windows")]
    {
        if is_git_installed() {
            let version = get_bundled_git_version();
            let path = get_git_dir().ok().map(|p| p.to_string_lossy().to_string());
            return GitStatus {
                installed: true,
                version,
                path,
                source: Some("bundled".to_string()),
                error: None,
            };
        }
    }

    // On macOS, provide installation instructions if Git is missing
    #[cfg(target_os = "macos")]
    {
        return GitStatus {
            installed: false,
            version: None,
            path: None,
            source: None,
            error: Some(
                "Git not found. Please install via:\n\
                 1. Xcode Command Line Tools: xcode-select --install\n\
                 2. Or Homebrew: brew install git"
                    .to_string()
            ),
        };
    }

    // Windows fallback
    #[cfg(target_os = "windows")]
    {
        GitStatus {
            installed: false,
            version: None,
            path: None,
            source: None,
            error: None,
        }
    }
}

/// Download and install Git (async command)
#[tauri::command]
pub async fn install_git_cmd(_app_handle: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        download_and_install_git(_app_handle).await
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, we don't auto-install Git. Guide user to install manually.
        Err("Please install Git manually using one of these methods:\n\
             1. Xcode Command Line Tools: Run 'xcode-select --install' in Terminal\n\
             2. Homebrew: Run 'brew install git' in Terminal"
            .to_string())
    }
}

/// Get the Git executable path for use by other modules
/// Returns the bundled git.exe path if available, otherwise None
/// Other modules should fall back to system git if this returns None
pub fn get_bundled_git_exe() -> Option<PathBuf> {
    get_git_exe_path()
}

/// Get the git bin path for terminal PATH injection
/// Returns the cmd directory path (contains git.exe) if bundled git is available
pub fn get_git_bin_path() -> Option<String> {
    // On macOS, always use system Git (no bundled version)
    #[cfg(target_os = "macos")]
    {
        if let Some((_, path)) = check_system_git() {
            if path != "System PATH" {
                return Some(path);
            }
        }
        return None;
    }

    // On Windows, prefer bundled Git
    #[cfg(target_os = "windows")]
    {
        let git_dir = get_git_dir().ok()?;
        // Prefer cmd directory as it contains git.exe and other batch files
        let cmd_dir = git_dir.join("cmd");
        if cmd_dir.join("git.exe").exists() {
            return Some(cmd_dir.to_string_lossy().to_string());
        }
        // Fallback to bin directory
        let bin_dir = git_dir.join("bin");
        if bin_dir.join("git.exe").exists() {
            return Some(bin_dir.to_string_lossy().to_string());
        }
        None
    }
}

/// Get git bash.exe path for Claude Code
/// Returns the path to bash.exe in the bundled Git installation
#[cfg(target_os = "windows")]
pub fn get_git_bash_path() -> Option<PathBuf> {
    let git_dir = get_git_dir().ok()?;

    // Check bin/bash.exe (standard location)
    let bash_bin = git_dir.join("bin").join("bash.exe");
    if bash_bin.exists() {
        return Some(bash_bin);
    }

    // Check usr/bin/bash.exe (alternative location)
    let bash_usr = git_dir.join("usr").join("bin").join("bash.exe");
    if bash_usr.exists() {
        return Some(bash_usr);
    }

    // Check git-bash.exe in root
    let git_bash = git_dir.join("git-bash.exe");
    if git_bash.exists() {
        return Some(git_bash);
    }

    None
}
