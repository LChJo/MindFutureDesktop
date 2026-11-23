use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{PhysicalPosition, PhysicalSize, WebviewWindow};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

// Minimum valid window dimensions (from tauri.conf.json)
const MIN_WIDTH: u32 = 800;
const MIN_HEIGHT: u32 = 600;

impl WindowState {
    pub fn from_window(window: &WebviewWindow) -> Result<Self, String> {
        // Don't save state if window is minimized (would save invalid 0x0 size)
        if window.is_minimized().unwrap_or(false) {
            return Err("Window is minimized".to_string());
        }

        let maximized = window.is_maximized().unwrap_or(false);
        let position = window.outer_position().map_err(|e| e.to_string())?;
        let size = window.inner_size().map_err(|e| e.to_string())?;

        // Validate size is reasonable
        if size.width < MIN_WIDTH || size.height < MIN_HEIGHT {
            return Err("Window size too small".to_string());
        }

        Ok(Self {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
            maximized,
        })
    }

    /// Check if the saved state is valid for restoration
    pub fn is_valid(&self) -> bool {
        self.width >= MIN_WIDTH && self.height >= MIN_HEIGHT && self.x > -10000 && self.y > -10000
    }

    pub fn apply_to_window(&self, window: &WebviewWindow) -> Result<(), String> {
        // Skip if state is invalid (e.g., saved while minimized)
        if !self.is_valid() {
            log::warn!(
                "Invalid window state ({}x{} at {},{}), using defaults",
                self.width,
                self.height,
                self.x,
                self.y
            );
            return Ok(());
        }

        // Check if window position is within any available monitor
        let position_valid = self.is_position_on_screen(window);

        if position_valid {
            // Position is valid, restore normally
            let _ = window.set_position(PhysicalPosition::new(self.x, self.y));
            let _ = window.set_size(PhysicalSize::new(self.width, self.height));
        } else {
            // Position is off-screen (e.g., monitor disconnected), center on primary monitor
            log::warn!(
                "Window position ({},{}) is off-screen, centering on primary monitor",
                self.x,
                self.y
            );
            // Only restore size, let the window center itself (tauri.conf.json has center: true)
            let _ = window.set_size(PhysicalSize::new(self.width, self.height));
            let _ = window.center();
        }

        // Then handle maximized state
        if self.maximized {
            let _ = window.maximize();
        }
        Ok(())
    }

    /// Check if the window position is visible on any available monitor
    /// Returns true if at least the top-left corner area (for dragging) is on screen
    fn is_position_on_screen(&self, window: &WebviewWindow) -> bool {
        // Get all available monitors
        let monitors = match window.available_monitors() {
            Ok(m) => m,
            Err(_) => return true, // If we can't get monitors, assume valid
        };

        if monitors.is_empty() {
            return true; // No monitor info, assume valid
        }

        // Window needs at least top-left corner visible for user to drag it
        // Check if point (x + 50, y + 50) is within any monitor
        // Using 50px margin to ensure title bar is grabbable
        let check_x = self.x + 50;
        let check_y = self.y + 50;

        for monitor in monitors {
            let pos = monitor.position();
            let size = monitor.size();

            let mon_left = pos.x;
            let mon_top = pos.y;
            let mon_right = pos.x + size.width as i32;
            let mon_bottom = pos.y + size.height as i32;

            // Check if the point is within this monitor's bounds
            if check_x >= mon_left && check_x < mon_right &&
               check_y >= mon_top && check_y < mon_bottom {
                return true;
            }
        }

        false
    }
}

fn get_state_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir()
        .ok_or_else(|| "Failed to get home directory".to_string())?;
    Ok(home
        .join(".MoontonClaudeDesktop")
        .join("config")
        .join("window-state.json"))
}

pub fn save(state: &WindowState) -> Result<(), String> {
    let path = get_state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    log::info!("Window state saved to {:?}", path);
    Ok(())
}

pub fn load() -> Option<WindowState> {
    let path = get_state_path().ok()?;
    let json = fs::read_to_string(&path).ok()?;
    let state: WindowState = serde_json::from_str(&json).ok()?;
    log::info!("Window state loaded from {:?}", path);
    Some(state)
}
