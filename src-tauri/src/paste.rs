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

/// The window a transcript should be pasted into: the handle, plus the
/// process that owned it when capture stopped. The pid is not redundant.
/// Windows recycles HWND values, and across a multi-second transcription the
/// original window can close and its handle be reissued to an unrelated
/// window - which would pass a handle-only identity check and get the
/// transcript pasted into a stranger, the exact bug this guard prevents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PasteTarget {
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    hwnd: isize,
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pid: u32,
}

/// Snapshot the currently focused window so a later `paste_text` call can
/// re-target it even if the user has since switched apps. Call this the
/// instant capture stops, not at paste time (paste time is after the slow
/// transcription step).
#[cfg(target_os = "windows")]
pub fn capture_foreground_window() -> Option<PasteTarget> {
    windows_impl::capture_foreground_window()
}

#[cfg(not(target_os = "windows"))]
pub fn capture_foreground_window() -> Option<PasteTarget> {
    None
}

#[cfg(target_os = "windows")]
const PRE_PASTE_DELAY: std::time::Duration = std::time::Duration::from_millis(50);
/// How long to let the target app act on the synthetic Ctrl+V before putting
/// the user's clipboard back. Nothing tells us "the app pasted" - the
/// clipboard sequence number counts writes, not reads - so this is a bounded
/// wait, and it has to be generous: Electron apps (VS Code, Slack, Discord)
/// routinely exceed 150ms for an input-to-paste round trip under load, and
/// restoring too early makes the user paste their OLD clipboard content.
#[cfg(target_os = "windows")]
const POST_PASTE_RESTORE_DELAY: std::time::Duration = std::time::Duration::from_millis(400);

#[cfg(target_os = "windows")]
pub fn paste_text(text: &str, target: Option<PasteTarget>) -> PasteOutcome {
    // Paste ONLY into the window that was focused when capture stopped, and
    // only once that is positively confirmed. `None` - GetForegroundWindow
    // returning NULL at capture time, which happens transiently around
    // activation changes, UAC/secure-desktop switches and lock/unlock - used
    // to skip this check entirely and paste wherever focus happened to be,
    // which is precisely the wrong-window bug this guard exists to stop.
    let confirmed = matches!(
        target,
        Some(t) if windows_impl::is_target_foreground(t) || windows_impl::refocus(t)
    );
    if !confirmed {
        windows_impl::write_transcript_clipboard(text);
        return PasteOutcome::Fallback;
    }

    let saved = arboard::Clipboard::new()
        .ok()
        .and_then(|mut c| c.get_text().ok());

    if !windows_impl::write_transcript_clipboard(text) {
        return PasteOutcome::Fallback;
    }
    let seq_after_write = windows_impl::clipboard_sequence();

    std::thread::sleep(PRE_PASTE_DELAY);

    if !send_ctrl_v() {
        return PasteOutcome::Fallback;
    }

    std::thread::sleep(POST_PASTE_RESTORE_DELAY);

    // Restore only if nothing else has written to the clipboard since we did.
    // If a clipboard manager or another app wrote in the meantime, restoring
    // would silently stomp their newer content.
    if let Some(saved) = saved {
        if windows_impl::clipboard_sequence() == seq_after_write {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(saved);
            }
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
    // Always release Control, even if the click failed, and retry: a stuck
    // modifier would corrupt every keystroke the user types next, which is
    // far worse than a failed paste. One failed release attempt is not
    // enough - the usual cause (a secure-desktop switch mid-send) is
    // transient.
    let mut released = false;
    for _ in 0..3 {
        if enigo.key(Key::Control, Direction::Release).is_ok() {
            released = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    pressed && clicked && released
}

#[cfg(target_os = "macos")]
pub fn paste_text(text: &str, _target: Option<PasteTarget>) -> PasteOutcome {
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
    // GlobalFree sits in Foundation, not System::Memory alongside its
    // GlobalAlloc/GlobalLock siblings - a quirk of the windows crate's
    // header grouping.
    use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, GetClipboardSequenceNumber, OpenClipboard,
        RegisterClipboardFormatW, SetClipboardData,
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

    pub fn capture_foreground_window() -> Option<super::PasteTarget> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                return None;
            }
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 {
                return None;
            }
            Some(super::PasteTarget {
                hwnd: hwnd.0 as isize,
                pid,
            })
        }
    }

    /// True only if the captured window is still the foreground window AND
    /// still belongs to the same process. The pid half defeats handle reuse:
    /// a recycled HWND would otherwise pass as "the same window".
    pub fn is_target_foreground(t: super::PasteTarget) -> bool {
        unsafe {
            let fg = GetForegroundWindow();
            if fg.0 as isize != t.hwnd {
                return false;
            }
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(fg, Some(&mut pid));
            pid == t.pid
        }
    }

    /// Windows bumps this counter on every clipboard WRITE by any process.
    /// It does not change when an app merely reads the clipboard, so it can
    /// never tell us a paste succeeded - only whether someone else has
    /// overwritten what we put there.
    pub fn clipboard_sequence() -> u32 {
        unsafe { GetClipboardSequenceNumber() }
    }

    /// Steal focus back to `hwnd` (the window that was focused when
    /// recording stopped). Windows blocks background processes from calling
    /// SetForegroundWindow directly; the documented workaround is to
    /// briefly attach our thread's input queue to both the current
    /// foreground thread and the target thread. Returns false (caller falls
    /// back to clipboard + paste-fallback) if the window is gone or the OS
    /// still refuses the switch.
    pub fn refocus(t: super::PasteTarget) -> bool {
        unsafe {
            let target = HWND(t.hwnd as *mut _);
            if !IsWindow(Some(target)).as_bool() {
                return false;
            }

            let foreground = GetForegroundWindow();
            let current_thread = GetCurrentThreadId();
            let foreground_thread = GetWindowThreadProcessId(foreground, None);
            let mut target_pid: u32 = 0;
            let target_thread = GetWindowThreadProcessId(target, Some(&mut target_pid));
            // Live handle, different process: the HWND was recycled after the
            // window we captured closed. That is not where the user dictated.
            if target_pid != t.pid {
                return false;
            }

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
            // Ownership transfers to the clipboard only on SUCCESS, so on
            // this path the block is still ours and leaks unless we free it.
            // (Freeing after a successful call would be a double free -
            // Windows owns and frees it from then on.)
            let _ = GlobalFree(Some(HGLOBAL(text_mem as *mut _)));
            return false;
        }
        for name in EXCLUSION_FORMATS {
            let format = register_format(name);
            if format == 0 {
                continue;
            }
            if let Some(marker) = alloc_global_bytes(&[0u8]) {
                if SetClipboardData(format, Some(HANDLE(marker as *mut _))).is_err() {
                    let _ = GlobalFree(Some(HGLOBAL(marker as *mut _)));
                }
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
                let _ = GlobalFree(Some(hmem));
                return None;
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, bytes.len());
            let _ = GlobalUnlock(hmem);
            Some(hmem.0 as isize)
        }
    }
}
