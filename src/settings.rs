use std::fs;
use std::path::PathBuf;

/// File (inside the settings directory) that stores the auto start flag.
const AUTO_START_FILE: &str = "settings";

/// Returns the settings directory: `%APPDATA%\nv-ssd-guard`.
fn settings_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|appdata| PathBuf::from(appdata).join("nv-ssd-guard"))
}

/// Loads the "auto start guard" setting. Defaults to `false` if it has never
/// been saved or cannot be read.
pub fn load_auto_start() -> bool {
    settings_dir()
        .and_then(|dir| fs::read_to_string(dir.join(AUTO_START_FILE)).ok())
        .map(|contents| contents.trim() == "1")
        .unwrap_or(false)
}

/// Saves the "auto start guard" setting.
///
/// Returns `(saved, error)` where `saved` is `true` on success and `error` is
/// the error message (empty on success).
pub fn save_auto_start(enabled: bool) -> (bool, String) {
    let Some(dir) = settings_dir() else {
        return (
            false,
            "failed to save setting: APPDATA is not set".to_string(),
        );
    };
    if let Err(e) = fs::create_dir_all(&dir) {
        return (false, format!("failed to create settings directory: {}", e));
    }
    match fs::write(dir.join(AUTO_START_FILE), if enabled { "1" } else { "0" }) {
        Ok(()) => (true, String::new()),
        Err(e) => (false, format!("failed to save setting: {}", e)),
    }
}
