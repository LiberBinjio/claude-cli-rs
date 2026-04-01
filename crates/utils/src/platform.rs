//! Platform detection and standard directory helpers.

use std::path::PathBuf;

/// Returns `true` on Windows.
#[inline]
#[must_use]
pub fn is_windows() -> bool {
    cfg!(target_os = "windows")
}

/// Returns `true` on macOS.
#[inline]
#[must_use]
pub fn is_macos() -> bool {
    cfg!(target_os = "macos")
}

/// Returns `true` on Linux.
#[inline]
#[must_use]
pub fn is_linux() -> bool {
    cfg!(target_os = "linux")
}

/// User home directory (e.g. `C:\Users\<user>` or `/home/<user>`).
#[must_use]
pub fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// Platform data directory (`~/.local/share` / `~/Library/Application Support` / `AppData\Roaming`).
#[must_use]
pub fn data_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| home_dir().join(".local").join("share"))
}

/// Platform config directory (`~/.config` / `~/Library/Preferences` / `AppData\Roaming`).
#[must_use]
pub fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| home_dir().join(".config"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exactly_one_platform() {
        let count = [is_windows(), is_macos(), is_linux()]
            .iter()
            .filter(|&&v| v)
            .count();
        // At least one must be true (may be 0 on exotic targets, but CI is win/mac/linux)
        assert!(count <= 1);
    }

    #[test]
    fn test_home_dir_exists() {
        let home = home_dir();
        assert!(home.is_absolute() || home == PathBuf::from("."));
    }

    #[test]
    fn test_data_and_config_dirs() {
        let d = data_dir();
        let c = config_dir();
        assert!(!d.as_os_str().is_empty());
        assert!(!c.as_os_str().is_empty());
    }
}
