//! US-007: Wispr-style clipboard-respecting paste with a focus guard.
//!
//! Flow: save the user's clipboard -> write the transcript (marked with
//! clipboard-manager exclusion formats) -> synthesize Ctrl+V -> wait briefly
//! -> restore the saved clipboard. Restoration only happens on the success
//! path, matching Wispr's documented behavior (a failed paste leaves the
//! transcript on the clipboard instead of silently losing it or the user's
//! prior clipboard content).
//!
//! FOCUS GUARD (real user bug, 2026-07-19): transcription can take long
//! enough that the user switches apps before it finishes, so a plain
//! "paste at whatever has focus now" lands the transcript in the wrong
//! window. recorder.rs captures the foreground window HWND the instant
//! capture stops (see `capture_foreground_window`, called well before the
//! slow transcription step) and passes it into `paste_text`; if focus has
//! since moved on, we try to steal it back before pasting, and fall back to
//! clipboard-only + the paste-fallback overlay state if that fails.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteOutcome {
    Pasted,
    /// Refocus or the paste itself failed. The transcript is left on the
    /// clipboard (with the exclusion formats already applied) for the user
    /// to paste manually; the caller shows the paste-fallback overlay state.
    Fallback,
}

/// Snapshot the currently focused window so a later `paste_text` call can
/// re-target it even if the user has since switched apps. Call this the
/// instant capture stops, not at paste time (paste time is after the slow
/// transcription step).
#[cfg(target_os = "windows")]
pub fn capture_foreground_window() -> Option<isize> {
    windows_impl::capture_foreground_window()
}

#[cfg(not(target_os = "windows"))]
pub fn capture_foreground_window() -> Option<isize> {
    None
}

#[cfg(target_os = "windows")]
const PRE_PASTE_DELAY: std::time::Duration = std::time::Duration::from_millis(50);
#[cfg(target_os = "windows")]
const POST_PASTE_RESTORE_DELAY: std::time::Duration = std::time::Duration::from_millis(150);

#[cfg(target_os = "windows")]
pub fn paste_text(text: &str, target_hwnd: Option<isize>) -> PasteOutcome {
    // Refocus the window that was in the foreground when recording stopped,
    // if the user has since switched apps. A missing/gone/refusing target
    // means we must not guess where to paste — leave it on the clipboard.
    if let Some(hwnd) = target_hwnd {
        if !windows_impl::is_foreground(hwnd) && !windows_impl::refocus(hwnd) {
            windows_impl::write_transcript_clipboard(text);
            return PasteOutcome::Fallback;
        }
    }

    let saved = arboard::Clipboard::new()
        .ok()
        .and_then(|mut c| c.get_text().ok());

    if !windows_impl::write_transcript_clipboard(text) {
        return PasteOutcome::Fallback;
    }

    std::thread::sleep(PRE_PASTE_DELAY);

    if !send_ctrl_v() {
        return PasteOutcome::Fallback;
    }

    std::thread::sleep(POST_PASTE_RESTORE_DELAY);

    if let Some(saved) = saved {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(saved);
        }
    }

    PasteOutcome::Pasted
}

#[cfg(target_os = "windows")]
fn send_ctrl_v() -> bool {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    let Ok(mut enigo) = Enigo::new(&Settings::default()) else {
        return false;
    };
    let pressed = enigo.key(Key::Control, Direction::Press).is_ok();
    let clicked = pressed && enigo.key(Key::Unicode('v'), Direction::Click).is_ok();
    // Always try to release Control, even if the click failed, so a
    // transient input error can't leave the modifier stuck down.
    let released = enigo.key(Key::Control, Direction::Release).is_ok();
    pressed && clicked && released
}

#[cfg(target_os = "macos")]
pub fn paste_text(text: &str, _target_hwnd: Option<isize>) -> PasteOutcome {
    let saved = arboard::Clipboard::new()
        .ok()
        .and_then(|mut c| c.get_text().ok());

    let Ok(mut clipboard) = arboard::Clipboard::new() else {
        return PasteOutcome::Fallback;
    };
    if clipboard.set_text(text).is_err() {
        return PasteOutcome::Fallback;
    }

    std::thread::sleep(std::time::Duration::from_millis(50));

    let sent = std::process::Command::new("osascript")
        .args([
            "-e",
            r#"tell application "System Events" to keystroke "v" using command down"#,
        ])
        .output()
        .is_ok();
    if !sent {
        return PasteOutcome::Fallback;
    }

    std::thread::sleep(std::time::Duration::from_millis(150));

    if let Some(saved) = saved {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(saved);
        }
    }

    PasteOutcome::Pasted
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use std::time::Duration;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HANDLE, HWND};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    use windows::Win32::System::Ole::CF_UNICODETEXT;
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId, IsWindow, SetForegroundWindow,
    };

    /// Clipboard-manager exclusion formats (US-007 research section 3):
    /// their mere presence on the clipboard tells well-behaved clipboard
    /// history tools (Windows Clipboard History, Ditto, 1Password, etc.) to
    /// skip recording this transient transcript.
    const EXCLUSION_FORMATS: [&str; 2] = [
        "ExcludeClipboardContentFromMonitorProcessing",
        "Clipboard Viewer Ignore", // CF_CLIPBOARD_VIEWER_IGNORE
    ];

    pub fn capture_foreground_window() -> Option<isize> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.0.is_null() {
            None
        } else {
            Some(hwnd.0 as isize)
        }
    }

    pub fn is_foreground(hwnd: isize) -> bool {
        unsafe { GetForegroundWindow().0 as isize == hwnd }
    }

    /// Steal focus back to `hwnd` (the window that was focused when
    /// recording stopped). Windows blocks background processes from calling
    /// SetForegroundWindow directly; the documented workaround is to
    /// briefly attach our thread's input queue to both the current
    /// foreground thread and the target thread. Returns false (caller falls
    /// back to clipboard + paste-fallback) if the window is gone or the OS
    /// still refuses the switch.
    pub fn refocus(hwnd: isize) -> bool {
        unsafe {
            let target = HWND(hwnd as *mut _);
            if !IsWindow(Some(target)).as_bool() {
                return false;
            }

            let foreground = GetForegroundWindow();
            let current_thread = GetCurrentThreadId();
            let foreground_thread = GetWindowThreadProcessId(foreground, None);
            let target_thread = GetWindowThreadProcessId(target, None);

            let attached_fg = foreground_thread != current_thread
                && foreground_thread != 0
                && AttachThreadInput(current_thread, foreground_thread, true).as_bool();
            let attached_target = target_thread != current_thread
                && target_thread != 0
                && AttachThreadInput(current_thread, target_thread, true).as_bool();

            let requested = SetForegroundWindow(target).as_bool();

            if attached_target {
                let _ = AttachThreadInput(current_thread, target_thread, false);
            }
            if attached_fg {
                let _ = AttachThreadInput(current_thread, foreground_thread, false);
            }

            requested && GetForegroundWindow() == target
        }
    }

    /// Write `text` as CF_UNICODETEXT plus the clipboard-manager exclusion
    /// formats. Retries OpenClipboard briefly since other apps (clipboard
    /// managers in particular) transiently hold it open.
    pub fn write_transcript_clipboard(text: &str) -> bool {
        unsafe {
            let mut opened = false;
            for _ in 0..5 {
                if OpenClipboard(None).is_ok() {
                    opened = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            if !opened {
                return false;
            }

            let ok = write_locked(text);
            let _ = CloseClipboard();
            ok
        }
    }

    unsafe fn write_locked(text: &str) -> bool {
        if EmptyClipboard().is_err() {
            return false;
        }
        let Some(text_mem) = alloc_global_utf16(text) else {
            return false;
        };
        if SetClipboardData(CF_UNICODETEXT.0 as u32, Some(HANDLE(text_mem as *mut _))).is_err() {
            return false;
        }
        for name in EXCLUSION_FORMATS {
            let format = register_format(name);
            if format == 0 {
                continue;
            }
            if let Some(marker) = alloc_global_bytes(&[0u8]) {
                let _ = SetClipboardData(format, Some(HANDLE(marker as *mut _)));
            }
        }
        true
    }

    fn register_format(name: &str) -> u32 {
        let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe { RegisterClipboardFormatW(PCWSTR(wide.as_ptr())) }
    }

    /// Allocate a moveable global block containing `text` as NUL-terminated
    /// UTF-16, the layout CF_UNICODETEXT requires. Ownership passes to the
    /// clipboard once SetClipboardData succeeds; Windows frees it.
    fn alloc_global_utf16(text: &str) -> Option<isize> {
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let bytes: &[u8] =
            unsafe { std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2) };
        alloc_global_bytes(bytes)
    }

    fn alloc_global_bytes(bytes: &[u8]) -> Option<isize> {
        unsafe {
            let hmem = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1)).ok()?;
            let ptr = GlobalLock(hmem);
            if ptr.is_null() {
                return None;
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, bytes.len());
            let _ = GlobalUnlock(hmem);
            Some(hmem.0 as isize)
        }
    }
}
