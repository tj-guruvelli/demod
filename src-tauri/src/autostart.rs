//! Windows autostart management for Typr.
//!
//! Exactly one autostart mechanism may be active at a time: the HKCU Run
//! registry key. A shortcut in the per-user Startup folder was the
//! (unmanaged, invisible-from-the-app) mechanism a prior manual install used;
//! if both were ever active simultaneously, Typr would launch twice at boot
//! and the second process would silently lose the race for the global
//! hotkey. So:
//! - `enable` deletes the legacy .lnk (if present) before writing the Run key.
//! - `disable` deletes the Run key AND the .lnk (if present) — belt and
//!   suspenders, since disable must guarantee "nothing launches Typr at boot"
//!   regardless of how it got enabled.
//! - `get_state` reports enabled if EITHER mechanism is present, since either
//!   one alone is enough to launch Typr at boot.

use std::path::PathBuf;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};
use winreg::RegKey;

const RUN_KEY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE_NAME: &str = "Typr";

fn current_exe_quoted() -> Result<String, String> {
    let exe = std::env::current_exe()
        .map_err(|e| format!("Could not determine the current executable path: {}", e))?;
    let exe_str = exe
        .to_str()
        .ok_or_else(|| "Executable path is not valid UTF-8".to_string())?;
    Ok(format!("\"{}\"", exe_str))
}

/// %APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\Typr.lnk — the
/// legacy, unmanaged autostart mechanism this app must never leave active
/// alongside the registry key.
fn startup_lnk_path() -> Result<PathBuf, String> {
    let appdata = dirs::config_dir()
        .ok_or_else(|| "Could not determine %APPDATA%".to_string())?;
    Ok(appdata
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Startup")
        .join("Typr.lnk"))
}

/// Delete the legacy Startup-folder shortcut if it exists. Not finding it is
/// not an error (nothing to remove); an existing file that fails to delete
/// is a real error the caller must surface.
fn remove_startup_lnk() -> Result<(), String> {
    let lnk = startup_lnk_path()?;
    if lnk.exists() {
        std::fs::remove_file(&lnk)
            .map_err(|e| format!("Could not remove existing startup shortcut at {}: {}", lnk.display(), e))?;
    }
    Ok(())
}

fn run_key_has_value() -> Result<bool, String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    match hkcu.open_subkey_with_flags(RUN_KEY_PATH, KEY_READ) {
        Ok(key) => {
            let value: Result<String, _> = key.get_value(RUN_VALUE_NAME);
            Ok(value.is_ok())
        }
        Err(_) => Ok(false),
    }
}

/// Reports "enabled" if EITHER the Run registry value or the legacy .lnk is
/// present — either alone is sufficient to launch Typr at boot, so either
/// alone means the feature reads as on.
#[tauri::command]
pub fn autostart_get_state() -> Result<bool, String> {
    let registry_enabled = run_key_has_value()?;
    let lnk_present = startup_lnk_path()?.exists();
    Ok(registry_enabled || lnk_present)
}

/// Enable: remove the legacy .lnk (so it can never launch a second instance
/// alongside the registry entry), then write the Run key pointing at the
/// current executable.
#[tauri::command]
pub fn autostart_enable() -> Result<(), String> {
    remove_startup_lnk()?;

    let exe_quoted = current_exe_quoted()?;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(RUN_KEY_PATH)
        .map_err(|e| format!("Could not open the startup registry key: {}", e))?;
    key.set_value(RUN_VALUE_NAME, &exe_quoted)
        .map_err(|e| format!("Could not write the startup registry value: {}", e))?;
    Ok(())
}

/// Disable: remove BOTH mechanisms, so "disabled" always means neither one
/// can launch Typr at boot, regardless of which was active.
#[tauri::command]
pub fn autostart_disable() -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey_with_flags(RUN_KEY_PATH, KEY_SET_VALUE) {
        match key.delete_value(RUN_VALUE_NAME) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("Could not remove the startup registry value: {}", e)),
        }
    }
    remove_startup_lnk()?;
    Ok(())
}
