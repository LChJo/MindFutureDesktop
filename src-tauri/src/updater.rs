use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::AppHandle;
use tauri::Emitter;

const PROGRESS_THROTTLE_MS: u128 = 200;

/// External config file: ~/.MoontonClaudeDesktop/config/update-config.json
/// Format: { "updateUrl": "https://your-server.com/update-manifest.json" }
#[derive(Debug, Deserialize)]
struct UpdateConfig {
    #[serde(rename = "updateUrl")]
    update_url: Option<String>,
}

/// Read the default update URL from external config file.
/// Returns None if the file doesn't exist or is invalid.
fn read_update_config_url() -> Option<String> {
    let config_path = dirs::home_dir()?
        .join(".MoontonClaudeDesktop")
        .join("config")
        .join("update-config.json");

    let content = std::fs::read_to_string(&config_path).ok()?;
    let config: UpdateConfig = serde_json::from_str(&content).ok()?;
    config
        .update_url
        .map(|url| url.trim().to_owned())
        .filter(|url| !url.is_empty())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformAsset {
    pub url: String,
    pub size: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateManifest {
    pub version: String,
    pub release_date: Option<String>,
    pub release_notes: Option<String>,
    pub platforms: HashMap<String, PlatformAsset>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateCheckResult {
    pub update_available: bool,
    pub current_version: String,
    pub remote_version: Option<String>,
    pub release_notes: Option<String>,
    pub download_url: Option<String>,
    pub download_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgressPayload {
    pub downloaded: u64,
    pub total: u64,
    pub percent: f64,
}

/// Compare semver strings. Returns true if `remote` is newer than `current`.
fn is_newer_version(current: &str, remote: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.trim_start_matches('v')
            .split('.')
            .filter_map(|s| s.parse::<u64>().ok())
            .collect()
    };

    let current_parts = parse(current);
    let remote_parts = parse(remote);

    for i in 0..current_parts.len().max(remote_parts.len()) {
        let c = current_parts.get(i).copied().unwrap_or(0);
        let r = remote_parts.get(i).copied().unwrap_or(0);
        if r > c {
            return true;
        }
        if r < c {
            return false;
        }
    }
    false
}

/// Get the platform key for the current OS/architecture.
fn get_platform_key() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "windows-x86_64"
    }
    #[cfg(target_os = "macos")]
    {
        "darwin-universal"
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        "linux-x86_64"
    }
}

/// Validate that a URL uses HTTPS.
fn validate_https_url(url: &str) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err(format!("Only HTTPS URLs are allowed: {}", url));
    }
    Ok(())
}

/// Get the update cache directory.
fn get_update_cache_dir() -> Result<std::path::PathBuf, String> {
    let cache_dir = dirs::home_dir()
        .ok_or_else(|| "Failed to resolve home directory".to_string())?
        .join(".MoontonClaudeDesktop")
        .join("cache");
    Ok(cache_dir)
}

/// Sanitize a filename extracted from a URL: strip path separators and
/// reject anything suspicious.
fn sanitize_filename(raw: &str) -> String {
    // URL-decode first (e.g. %2F -> /)
    let decoded = urlencoding_decode(raw);
    // Take only the last path component, stripping any directory traversal
    let name = decoded
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("update-installer");
    // Remove any remaining path-unfriendly characters
    let sanitized: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect();
    if sanitized.is_empty() {
        "update-installer".to_string()
    } else {
        sanitized
    }
}

/// Simple percent-decoding for URL filenames.
fn urlencoding_decode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            } else {
                result.push('%');
                result.push_str(&hex);
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Validate that a file extension is an expected installer type.
fn validate_installer_extension(path: &std::path::Path) -> Result<(), String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    #[cfg(target_os = "windows")]
    let allowed = ["exe", "msi"];
    #[cfg(target_os = "macos")]
    let allowed = ["dmg", "pkg"];
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let allowed = ["appimage", "deb", "rpm"];

    if !allowed.contains(&ext.as_str()) {
        return Err(format!(
            "Unexpected installer extension '.{}'. Expected one of: {}",
            ext,
            allowed.join(", ")
        ));
    }
    Ok(())
}

/// Get the default update URL configured by the installation.
/// The frontend uses this to show the effective URL in the Settings UI.
#[tauri::command]
pub fn get_default_update_url() -> String {
    read_update_config_url().unwrap_or_default()
}

/// Check for updates by fetching the remote manifest.
#[tauri::command]
pub async fn check_for_update(
    update_url: Option<String>,
) -> Result<UpdateCheckResult, String> {
    // A URL entered by the user takes precedence over the installation config.
    let url = update_url
        .map(|url| url.trim().to_owned())
        .filter(|url| !url.is_empty())
        .or_else(read_update_config_url)
        .ok_or_else(|| "请在设置中配置更新源".to_string())?;

    validate_https_url(&url)?;

    let current_version = env!("CARGO_PKG_VERSION").to_string();

    let manifest = tauri::async_runtime::spawn_blocking(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("Moonton-Claude-Desktop")
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let response = client
            .get(&url)
            .send()
            .map_err(|e| format!("Failed to fetch update manifest: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Update server returned status {}",
                response.status()
            ));
        }

        response
            .json::<UpdateManifest>()
            .map_err(|e| format!("Failed to parse update manifest: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))??;

    let platform_key = get_platform_key();
    let asset = manifest.platforms.get(platform_key);

    // Validate download URL uses HTTPS if present
    let download_url = match asset {
        Some(a) => {
            if let Err(e) = validate_https_url(&a.url) {
                log::warn!("Manifest contains non-HTTPS download URL: {}", e);
                None
            } else {
                Some(a.url.clone())
            }
        }
        None => None,
    };

    let update_available = is_newer_version(&current_version, &manifest.version);

    Ok(UpdateCheckResult {
        update_available,
        current_version,
        remote_version: Some(manifest.version),
        release_notes: manifest.release_notes,
        download_url,
        download_size: asset.and_then(|a| a.size),
    })
}

/// Download the update installer with progress events.
#[tauri::command]
pub async fn download_update(
    app_handle: AppHandle,
    download_url: String,
    total_size: Option<u64>,
) -> Result<String, String> {
    use futures_util::StreamExt;
    use std::io::Write;

    validate_https_url(&download_url)?;

    let cache_dir = get_update_cache_dir()?;
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;

    // Extract and sanitize filename from URL
    let raw_filename = download_url.rsplit('/').next().unwrap_or("update-installer");
    let filename = sanitize_filename(raw_filename);
    let dest_path = cache_dir.join(&filename);

    // Validate installer extension
    validate_installer_extension(&dest_path)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .user_agent("Moonton-Claude-Desktop")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to start download: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Download server returned status {}",
            response.status()
        ));
    }

    let content_length = response.content_length().or(total_size).unwrap_or(0);

    let mut file = std::fs::File::create(&dest_path)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_progress_update = std::time::Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        downloaded += chunk.len() as u64;

        if last_progress_update.elapsed().as_millis() >= PROGRESS_THROTTLE_MS {
            let percent = if content_length > 0 {
                (downloaded as f64 / content_length as f64) * 100.0
            } else {
                0.0
            };

            let _ = app_handle.emit(
                "update-download-progress",
                DownloadProgressPayload {
                    downloaded,
                    total: content_length,
                    percent,
                },
            );

            last_progress_update = std::time::Instant::now();
        }
    }

    // Flush all data to disk before returning
    file.sync_all()
        .map_err(|e| format!("Failed to sync file: {}", e))?;

    // Emit final 100% progress
    let _ = app_handle.emit(
        "update-download-progress",
        DownloadProgressPayload {
            downloaded,
            total: content_length,
            percent: 100.0,
        },
    );

    Ok(dest_path.to_string_lossy().to_string())
}

/// Launch the downloaded installer and exit the app.
#[tauri::command]
pub async fn install_update(
    app_handle: AppHandle,
    installer_path: String,
) -> Result<(), String> {
    let path = std::path::Path::new(&installer_path);
    if !path.exists() {
        return Err(format!("Installer not found: {}", installer_path));
    }

    // Validate the installer resides within the expected cache directory
    let cache_dir = get_update_cache_dir()?;
    let canonical_path = path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve installer path: {}", e))?;
    let canonical_cache = cache_dir
        .canonicalize()
        .map_err(|e| format!("Failed to resolve cache directory: {}", e))?;

    if !canonical_path.starts_with(&canonical_cache) {
        return Err("Installer path is outside the update cache directory".to_string());
    }

    // Validate file extension
    validate_installer_extension(&canonical_path)?;

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        std::process::Command::new(&installer_path)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("Failed to launch installer: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        opener::open(&installer_path)
            .map_err(|e| format!("Failed to open installer: {}", e))?;
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        opener::open(&installer_path)
            .map_err(|e| format!("Failed to open installer: {}", e))?;
    }

    app_handle.exit(0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("1.2.20", "1.3.0"));
        assert!(is_newer_version("1.2.20", "1.2.21"));
        assert!(is_newer_version("1.2.20", "2.0.0"));
        assert!(!is_newer_version("1.2.20", "1.2.20"));
        assert!(!is_newer_version("1.2.20", "1.2.19"));
        assert!(!is_newer_version("1.3.0", "1.2.20"));
        assert!(is_newer_version("1.2", "1.2.1"));
        assert!(!is_newer_version("1.2.1", "1.2"));
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("installer.exe"), "installer.exe");
        assert_eq!(sanitize_filename("..%2F..%2Fevil.exe"), "evil.exe");
        assert_eq!(sanitize_filename("path/to/file.dmg"), "file.dmg");
        assert_eq!(sanitize_filename(""), "update-installer");
        assert_eq!(
            sanitize_filename("Moonton-Claude-Desktop_1.3.0_x64-setup.exe"),
            "Moonton-Claude-Desktop_1.3.0_x64-setup.exe"
        );
    }

    #[test]
    fn test_validate_https_url() {
        assert!(validate_https_url("https://example.com/file.exe").is_ok());
        assert!(validate_https_url("http://example.com/file.exe").is_err());
        assert!(validate_https_url("ftp://example.com/file.exe").is_err());
    }

    #[test]
    fn test_urlencoding_decode() {
        assert_eq!(urlencoding_decode("hello%20world"), "hello world");
        assert_eq!(urlencoding_decode("..%2F..%2Fevil"), "../../evil");
        assert_eq!(urlencoding_decode("normal"), "normal");
    }
}
