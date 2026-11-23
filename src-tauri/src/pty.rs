use crate::git::get_git_bin_path;
use crate::llm_router::{get_ccr_bin_path, get_mcd_bin_path};
use crate::nodejs::{get_nodejs_bin_path, get_npm_global_bin_path};
use crate::uv::get_uv_bin_path;
use parking_lot::Mutex;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use std::thread;
use tauri::Emitter;
use uuid::Uuid;

/// Escape a string for use inside PowerShell double-quoted strings.
/// Handles backtick (escape char), double-quote, and dollar (variable expansion).
#[cfg(target_os = "windows")]
fn escape_powershell(s: &str) -> String {
    s.replace('`', "``")
     .replace('"', "`\"")
     .replace('$', "`$")
}

/// Build zsh prompt string based on configuration
/// Returns a PROMPT string with zsh escape sequences for colors
#[cfg(target_os = "macos")]
fn build_zsh_prompt(config: Option<&serde_json::Value>) -> String {
    // If config not provided or disabled, use simple default prompt with OSC 7
    let config = match config {
        Some(c) => c,
        None => return r#"%F{cyan}%~%f > "#.to_string(),
    };

    let enabled = config.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    if !enabled {
        return r#"%F{cyan}%~%f > "#.to_string();
    }

    let style = config.get("style").and_then(|v| v.as_str()).unwrap_or("minimal");
    let show_git_branch = config.get("showGitBranch").and_then(|v| v.as_bool()).unwrap_or(true);
    let show_python_env = config.get("showPythonEnv").and_then(|v| v.as_bool()).unwrap_or(true);
    let show_path = config.get("showPath").and_then(|v| v.as_str()).unwrap_or("short");

    // Get colors from config
    let colors = config.get("colors");
    let color_path = colors
        .and_then(|c| c.get("path"))
        .and_then(|v| v.as_str())
        .unwrap_or("cyan");
    let color_git_clean = colors
        .and_then(|c| c.get("gitClean"))
        .and_then(|v| v.as_str())
        .unwrap_or("blue");
    let color_git_dirty = colors
        .and_then(|c| c.get("gitDirty"))
        .and_then(|v| v.as_str())
        .unwrap_or("yellow");
    let color_env = colors
        .and_then(|c| c.get("env"))
        .and_then(|v| v.as_str())
        .unwrap_or("magenta");

    // Map ANSI color codes to zsh color names
    fn map_color(code: &str) -> &str {
        match code {
            "30" => "black",
            "31" => "red",
            "32" => "green",
            "33" => "yellow",
            "34" => "blue",
            "35" => "magenta",
            "36" => "cyan",
            "37" => "white",
            "39" => "cyan", // default -> cyan
            "90" => "bright-black",
            "91" => "bright-red",
            "92" => "bright-green",
            "93" => "bright-yellow",
            "94" => "bright-blue",
            "95" => "bright-magenta",
            "96" => "bright-cyan",
            "97" => "bright-white",
            other => other, // pass through color names
        }
    }

    let path_color = map_color(color_path);
    let git_clean_color = map_color(color_git_clean);
    let git_dirty_color = map_color(color_git_dirty);
    let env_color = map_color(color_env);

    // Path display format
    let path_format = match show_path {
        "basename" => "%1~",  // Just the last directory component
        "short" => "%~",      // Path with ~ substitution
        _ => "%/",            // Full path
    };

    // Build prompt based on style
    let mut prompt = String::new();

    match style {
        "powerline" => {
            // Powerline style
            if show_python_env {
                prompt.push_str(&format!(
                    "%(1V.%F{{{}}} %1v %f.)",
                    env_color
                ));
            }
            prompt.push_str(&format!("%F{{{}}} {} %f", path_color, path_format));
            if show_git_branch {
                prompt.push_str(&format!(
                    "%(2V.%F{{%(3V.{}.{})}} [%2v%(3V.*.)]) %f.)",
                    git_dirty_color, git_clean_color
                ));
            }
            prompt.push_str(" ❯ ");
        }
        "classic" => {
            // Classic PS style
            prompt.push_str("PS ");
            prompt.push_str(&format!("%F{{{}}}{}%f", path_color, path_format));
            if show_git_branch {
                prompt.push_str(&format!(
                    "%(2V. %F{{%(3V.{}.{})}}(%2v%(3V.*.)%)%f.)",
                    git_dirty_color, git_clean_color
                ));
            }
            if show_python_env {
                prompt.push_str(&format!("%(1V. %F{{{}}}[%1v]%f.)", env_color));
            }
            prompt.push_str("> ");
        }
        _ => {
            // Minimal style (default)
            if show_python_env {
                prompt.push_str(&format!("%(1V.%F{{{}}}(%1v)%f .)", env_color));
            }
            prompt.push_str(&format!("%F{{{}}}{}%f", path_color, path_format));
            if show_git_branch {
                prompt.push_str(&format!(
                    "%(2V. %F{{%(3V.{}.{})}}%2v%(3V.*.%)%f.)",
                    git_dirty_color, git_clean_color
                ));
            }
            prompt.push_str(" ❯ ");
        }
    }

    prompt
}

/// Build zsh precmd function for dynamic prompt updates (git branch, venv detection)
#[cfg(target_os = "macos")]
fn build_zsh_precmd(config: Option<&serde_json::Value>) -> String {
    let config = match config {
        Some(c) => c,
        None => return String::new(),
    };

    let enabled = config.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    if !enabled {
        return String::new();
    }

    let show_git_branch = config.get("showGitBranch").and_then(|v| v.as_bool()).unwrap_or(true);
    let show_python_env = config.get("showPythonEnv").and_then(|v| v.as_bool()).unwrap_or(true);

    let mut precmd = String::from("precmd() {\n");

    // OSC 7 directory tracking (for terminal integration)
    precmd.push_str("  print -Pn '\\e]7;file://%m%~\\a'\n");

    // Python virtual environment detection (psvar[1])
    if show_python_env {
        precmd.push_str("  if [[ -n \"$VIRTUAL_ENV\" ]]; then\n");
        precmd.push_str("    psvar[1]=\"${VIRTUAL_ENV:t}\"\n");
        precmd.push_str("  else\n");
        precmd.push_str("    psvar[1]=\"\"\n");
        precmd.push_str("  fi\n");
    }

    // Git branch and dirty status detection (psvar[2] and psvar[3])
    if show_git_branch {
        precmd.push_str("  local git_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null)\n");
        precmd.push_str("  if [[ -n \"$git_branch\" ]]; then\n");
        precmd.push_str("    psvar[2]=\"$git_branch\"\n");
        precmd.push_str("    if [[ -n \"$(git status --porcelain 2>/dev/null)\" ]]; then\n");
        precmd.push_str("      psvar[3]=\"dirty\"\n");
        precmd.push_str("    else\n");
        precmd.push_str("      psvar[3]=\"\"\n");
        precmd.push_str("    fi\n");
        precmd.push_str("  else\n");
        precmd.push_str("    psvar[2]=\"\"\n");
        precmd.push_str("    psvar[3]=\"\"\n");
        precmd.push_str("  fi\n");
    }

    precmd.push_str("}\n");
    precmd
}

pub struct PtyInstance {
    pub writer: Arc<Mutex<Box<dyn Write + Send>>>,
    #[allow(dead_code)]
    pub child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
    #[allow(dead_code)]
    pub master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
}

pub struct PtyManager {
    pub instances: Mutex<HashMap<String, Arc<PtyInstance>>>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            instances: Mutex::new(HashMap::new()),
        }
    }

    #[allow(unused_variables)] // prompt_config is only used on macOS
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
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to open pty: {}", e))?;

        // Platform-specific shell detection
        #[cfg(target_os = "windows")]
        let shell_command = "powershell.exe".to_string();

        #[cfg(target_os = "macos")]
        let shell_command = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());

        let mut cmd = CommandBuilder::new(&shell_command);

        // Platform-specific shell initialization
        #[cfg(target_os = "windows")]
        {
            // Build initialization commands including custom prompt for OSC 7 directory tracking
            let prompt_func = r#"function prompt { $p = $executionContext.SessionState.Path.CurrentLocation.Path; $e = [char]27; $b = [char]7; Write-Host -NoNewline "$e]7;file://localhost/$($p.Replace('\','/'))$b"; "PS $p> " }"#;

            // Unbind Shift+Tab from PSReadLine so it passes through to applications like Claude Code
            // PSReadLine by default binds Shift+Tab to MenuComplete (reverse completion)
            let unbind_shift_tab = r#"if (Get-Module PSReadLine) { Remove-PSReadLineKeyHandler -Key Shift+Tab }"#;

            let mut init_commands = vec![unbind_shift_tab.to_string(), prompt_func.to_string()];

            // Clear CLAUDECODE inherited from parent process so `claude` works directly
            init_commands.push("$env:CLAUDECODE=$null".to_string());

            // Add environment variables if provided
            if let Some(ref env_vars) = env {
                for (k, v) in env_vars {
                    init_commands.push(format!("$env:{}=\"{}\"", k, v.replace("\"", "`\"")));
                }
            }

            // Add Claude alias function definitions
            if let Some(ref alias_list) = aliases {
                for alias in alias_list {
                    // Validate alias name: must be a safe shell identifier
                    if alias.name.is_empty()
                        || !alias.name.chars().next().unwrap().is_ascii_alphabetic()
                        || !alias.name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                    {
                        log::warn!("Skipping alias with invalid name: {}", alias.name);
                        continue;
                    }

                    let mut env_setters = Vec::new();
                    // Always inject the three Anthropic vars (even if empty) to prevent
                    // inheriting stale system environment variables.
                    {
                        let model = alias.anthropic_model.as_deref().unwrap_or("");
                        env_setters.push(format!("$env:ANTHROPIC_MODEL=\"{}\"", escape_powershell(model)));
                    }
                    {
                        let token = alias.anthropic_auth_token.as_deref().unwrap_or("");
                        env_setters.push(format!("$env:ANTHROPIC_AUTH_TOKEN=\"{}\"", escape_powershell(token)));
                    }
                    {
                        let url = alias.anthropic_base_url.as_deref().unwrap_or("");
                        env_setters.push(format!("$env:ANTHROPIC_BASE_URL=\"{}\"", escape_powershell(url)));
                    }
                    if let Some(ref custom_vars) = alias.custom_env_vars {
                        for (k, v) in custom_vars {
                            // Validate env var key: POSIX-safe identifier
                            if k.is_empty() || !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                                log::warn!("Skipping custom env var with invalid key: {}", k);
                                continue;
                            }
                            if !v.is_empty() {
                                env_setters.push(format!("$env:{}=\"{}\"", k, escape_powershell(v)));
                            }
                        }
                    }
                    // Clear CLAUDECODE before AND after claude to prevent "nested session" detection.
                    // Claude sets CLAUDECODE during execution; must clear after exit too.
                    let clear_nested = "$env:CLAUDECODE=$null";
                    let func_body = if env_setters.is_empty() {
                        format!("{}; claude @args; {}", clear_nested, clear_nested)
                    } else {
                        format!("{}; {}; claude @args; {}", clear_nested, env_setters.join("; "), clear_nested)
                    };
                    init_commands.push(format!("function {} {{ {} }}", alias.name, func_body));
                }
            }

            let combined_cmd = init_commands.join("; ");

            cmd.args(["-NoLogo", "-NoExit", "-ExecutionPolicy", "Bypass", "-Command", &combined_cmd]);
        }

        #[cfg(target_os = "macos")]
        {
            // For macOS, check if custom prompt is configured
            if let Some(ref config) = prompt_config {
                if config.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false) {
                    // Build custom zsh prompt if configured
                    let prompt = build_zsh_prompt(Some(config));
                    let precmd = build_zsh_precmd(Some(config));

                    // Create a temporary directory for zsh config
                    let temp_dir = std::env::temp_dir().join(format!("moonton_claude_zsh_{}", std::process::id()));
                    if let Ok(()) = std::fs::create_dir_all(&temp_dir) {
                        // Create .zshenv file with our custom configuration
                        let zshenv_path = temp_dir.join(".zshenv");
                        let mut zshenv_content = String::new();

                        // Source the user's original .zshenv if it exists
                        if let Some(home) = std::env::var_os("HOME") {
                            zshenv_content.push_str(&format!("[ -f '{}/.zshenv' ] && source '{}/.zshenv'\n",
                                home.to_string_lossy(), home.to_string_lossy()));
                        }

                        // Clear CLAUDECODE inherited from parent process so `claude` works directly
                        zshenv_content.push_str("unset CLAUDECODE\n");

                        // Add our custom configuration
                        zshenv_content.push_str("export VIRTUAL_ENV_DISABLE_PROMPT=1\n");
                        zshenv_content.push_str(&format!("PROMPT='{}'\n", prompt));
                        if !precmd.is_empty() {
                            zshenv_content.push_str(&precmd);
                            zshenv_content.push('\n');
                        }

                        // Add environment variables
                        if let Some(ref env_vars) = env {
                            for (k, v) in env_vars {
                                let escaped = v.replace("'", "'\\''");
                                zshenv_content.push_str(&format!("export {}='{}'\n", k, escaped));
                            }
                        }

                        // Add Claude alias function definitions
                        if let Some(ref alias_list) = aliases {
                            for alias in alias_list {
                                // Validate alias name: must be a safe shell identifier
                                if alias.name.is_empty()
                                    || !alias.name.chars().next().unwrap().is_ascii_alphabetic()
                                    || !alias.name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                                {
                                    log::warn!("Skipping alias with invalid name: {}", alias.name);
                                    continue;
                                }

                                let mut env_parts = Vec::new();
                                // Always inject the three Anthropic vars (even if empty) to prevent
                                // inheriting stale system environment variables.
                                {
                                    let model = alias.anthropic_model.as_deref().unwrap_or("");
                                    let escaped = model.replace("'", "'\\''");
                                    env_parts.push(format!("ANTHROPIC_MODEL='{}'", escaped));
                                }
                                {
                                    let token = alias.anthropic_auth_token.as_deref().unwrap_or("");
                                    let escaped = token.replace("'", "'\\''");
                                    env_parts.push(format!("ANTHROPIC_AUTH_TOKEN='{}'", escaped));
                                }
                                {
                                    let url = alias.anthropic_base_url.as_deref().unwrap_or("");
                                    let escaped = url.replace("'", "'\\''");
                                    env_parts.push(format!("ANTHROPIC_BASE_URL='{}'", escaped));
                                }
                                if let Some(ref custom_vars) = alias.custom_env_vars {
                                    for (k, v) in custom_vars {
                                        // Validate env var key: POSIX-safe identifier
                                        if k.is_empty() || !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                                            log::warn!("Skipping custom env var with invalid key: {}", k);
                                            continue;
                                        }
                                        if !v.is_empty() {
                                            let escaped = v.replace("'", "'\\''");
                                            env_parts.push(format!("{}='{}'", k, escaped));
                                        }
                                    }
                                }
                                // Use env -u CLAUDECODE to prevent "nested session" detection
                                let func_body = if env_parts.is_empty() {
                                    "env -u CLAUDECODE claude \"$@\"".to_string()
                                } else {
                                    format!("env -u CLAUDECODE {} claude \"$@\"", env_parts.join(" "))
                                };
                                zshenv_content.push_str(&format!("{}() {{ {} }}\n", alias.name, func_body));
                            }
                        }

                        if let Ok(()) = std::fs::write(&zshenv_path, &zshenv_content) {
                            // Set ZDOTDIR to our temporary directory
                            cmd.env("ZDOTDIR", &temp_dir);
                            log::info!("macOS: Using custom ZDOTDIR for prompt configuration: {}", temp_dir.display());

                            // Clean up temp directory on exit (best effort)
                            let temp_dir_clone = temp_dir.clone();
                            thread::spawn(move || {
                                thread::sleep(std::time::Duration::from_secs(60));
                                let _ = std::fs::remove_dir_all(&temp_dir_clone);
                            });
                        }
                    }
                }
            }

            // Start a normal login shell
            cmd.args(["-l"]);
        }

        if let Some(dir) = cwd {
            cmd.cwd(dir);
        }

        // Set TERM environment variable for color support on macOS/Linux
        #[cfg(not(target_os = "windows"))]
        {
            cmd.env("TERM", "xterm-256color");
            cmd.env("COLORTERM", "truecolor");
            log::info!("Set TERM=xterm-256color and COLORTERM=truecolor for terminal color support");
        }

        // Also set env vars at process level (for child processes)
        if let Some(ref env_vars) = env {
            for (key, value) in env_vars {
                cmd.env(key, value);
            }
        }

        // Inject embedded Node.js and Git paths into PATH if available
        // IMPORTANT: npm global bin must be FIRST to ensure Claude uses the correct version
        let mut path_additions = Vec::new();

        // Add npm global bin FIRST (highest priority for Claude, etc.)
        if let Some(npm_bin_path) = get_npm_global_bin_path() {
            path_additions.push(npm_bin_path.clone());
            log::info!("Injected npm global bin path into terminal PATH (highest priority): {}", npm_bin_path);
        }

        if let Some(nodejs_path) = get_nodejs_bin_path() {
            path_additions.push(nodejs_path.clone());
            log::info!("Injected Node.js path into terminal PATH: {}", nodejs_path);
        }

        if let Some(git_path) = get_git_bin_path() {
            path_additions.push(git_path.clone());
            log::info!("Injected Git path into terminal PATH: {}", git_path);
        }

        if let Some(uv_path) = get_uv_bin_path() {
            path_additions.push(uv_path.clone());
            log::info!("Injected UV path into terminal PATH: {}", uv_path);
        }

        if let Some(ccr_path) = get_ccr_bin_path() {
            path_additions.push(ccr_path.clone());
            log::info!("Injected CCR path into terminal PATH: {}", ccr_path);
        }

        if let Some(mcd_path) = get_mcd_bin_path() {
            path_additions.push(mcd_path.clone());
            log::info!("Injected mcd path into terminal PATH: {}", mcd_path);
        }

        if !path_additions.is_empty() {
            let current_path = std::env::var("PATH").unwrap_or_default();

            // Platform-specific PATH separator
            #[cfg(target_os = "windows")]
            let path_separator = ";";

            #[cfg(not(target_os = "windows"))]
            let path_separator = ":";

            let new_path = format!(
                "{}{}{}",
                path_additions.join(path_separator),
                path_separator,
                current_path
            );

            log::info!("Terminal PATH configured. Priority paths: {}", path_additions.join(", "));
            cmd.env("PATH", new_path);
        }

        // On macOS the bundled node binary lives at nodejs/node (not nodejs/bin/node), so
        // npm computes its default global prefix as ~/.MoontonClaudeDesktop instead of
        // ~/.MoontonClaudeDesktop/nodejs.  Pin it to ~/.npm-global so manual
        // `npm install -g` lands in the same directory that PATH already contains.
        #[cfg(target_os = "macos")]
        if let Some(home) = dirs::home_dir() {
            cmd.env("NPM_CONFIG_PREFIX", home.join(".npm-global"));
            log::info!("Set NPM_CONFIG_PREFIX=~/.npm-global for terminal session");
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn command: {}", e))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("Failed to get writer: {}", e))?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("Failed to get reader: {}", e))?;

        let id = Uuid::new_v4().to_string();
        let id_clone = id.clone();

        let instance = Arc::new(PtyInstance {
            writer: Arc::new(Mutex::new(writer)),
            child: Arc::new(Mutex::new(child)),
            master: Arc::new(Mutex::new(pair.master)),
        });

        self.instances.lock().insert(id.clone(), instance);

        // Spawn a dedicated thread to read from this PTY
        thread::spawn(move || {
            // 16KB buffer for better throughput with large outputs
            let mut buf = [0u8; 16384];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        // EOF - PTY closed
                        break;
                    }
                    Ok(n) => {
                        let data = buf[..n].to_vec();
                        let event_name = format!("terminal-output-{}", id_clone);
                        let _ = app_handle.emit(&event_name, data);
                    }
                    Err(e) => {
                        log::error!("PTY read error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(id)
    }

    pub fn write(&self, id: &str, data: &[u8]) -> Result<(), String> {
        let instances = self.instances.lock();
        let instance = instances
            .get(id)
            .ok_or_else(|| "PTY instance not found".to_string())?;

        let mut writer = instance.writer.lock();
        writer
            .write_all(data)
            .map_err(|e| format!("Failed to write: {}", e))?;
        writer
            .flush()
            .map_err(|e| format!("Failed to flush: {}", e))?;

        Ok(())
    }

    pub fn kill(&self, id: &str) -> Result<(), String> {
        let mut instances = self.instances.lock();
        instances.remove(id);
        Ok(())
    }

    pub fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<(), String> {
        let instances = self.instances.lock();
        let instance = instances
            .get(id)
            .ok_or_else(|| "PTY instance not found".to_string())?;

        let master = instance.master.lock();
        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to resize PTY: {}", e))?;

        Ok(())
    }
}

impl Default for PtyManager {
    fn default() -> Self {
        Self::new()
    }
}
