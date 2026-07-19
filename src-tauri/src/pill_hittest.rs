//! US-002: default click-through overlay with a Rust-side cursor hit-test.
//!
//! The fixed 440x160 transparent host window (US-001) would swallow every
//! click in its bottom-center region, so it is click-through by default via
//! `set_ignore_cursor_events(true)`. A WH_MOUSE_LL low-level mouse hook —
//! installed once on the main thread, which the Tauri event loop keeps
//! pumping — watches the global cursor and flips cursor events back on only
//! while the cursor is inside the pill's screen rect, so the pill stays
//! hoverable and draggable. The same hook sees the left-button release that
//! ends a pill drag and clamps + persists the window position. Everything is
//! event-driven: no timers, no polling, no JS involvement.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, Ordering};
use std::sync::OnceLock;

use tauri::{AppHandle, Manager, PhysicalPosition, WebviewWindow, WindowEvent};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, MSLLHOOKSTRUCT, WH_MOUSE_LL,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
};

use crate::AppState;

/// Interactive zone around the pill, logical px, centered on the pill.
/// Must track overlay.html geometry: #mic sits at bottom:16px height:40px,
/// so the pill center is 36px above the host bottom; the widest state
/// (listening) is 180px wide. 200x48 adds a small hover/grab margin.
const PILL_HIT_W: f64 = 200.0;
const PILL_HIT_H: f64 = 48.0;
const PILL_CENTER_FROM_BOTTOM: f64 = 36.0;

static APP: OnceLock<AppHandle> = OnceLock::new();
static HOOK: AtomicIsize = AtomicIsize::new(0);
/// Pill screen rect in physical px. Written on the main thread from window
/// Moved/ScaleFactorChanged events; read by the hook on every mouse move.
/// All-zero (empty rect) until the first update, i.e. "never inside".
static RECT_LEFT: AtomicI32 = AtomicI32::new(0);
static RECT_TOP: AtomicI32 = AtomicI32::new(0);
static RECT_RIGHT: AtomicI32 = AtomicI32::new(0);
static RECT_BOTTOM: AtomicI32 = AtomicI32::new(0);
static CURSOR_INSIDE: AtomicBool = AtomicBool::new(false);
static DRAGGING: AtomicBool = AtomicBool::new(false);

/// Make the overlay click-through and start the cursor hit-test. Must be
/// called once from setup on the main thread: WH_MOUSE_LL delivers its
/// callbacks through the installing thread's message loop.
pub fn install(app: &AppHandle, overlay: &WebviewWindow) {
    if APP.set(app.clone()).is_err() {
        return;
    }

    if let Err(e) = overlay.set_ignore_cursor_events(true) {
        eprintln!("[Typr] Failed to make overlay click-through: {}", e);
    }

    update_pill_rect(overlay);
    let handle = overlay.clone();
    overlay.on_window_event(move |event| {
        if matches!(
            event,
            WindowEvent::Moved(_) | WindowEvent::ScaleFactorChanged { .. }
        ) {
            update_pill_rect(&handle);
        }
    });

    match unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), None, 0) } {
        Ok(hook) => {
            HOOK.store(hook.0 as isize, Ordering::Release);
            println!("[Typr] Pill hit-test mouse hook installed");
        }
        // Without the hook we must not enable cursor events (the whole
        // 440x160 host would swallow clicks), so the overlay stays
        // click-through: dictation still works, the pill just isn't
        // clickable this session.
        Err(e) => eprintln!("[Typr] Failed to install mouse hook: {}", e),
    }
}

/// Uninstall the WH_MOUSE_LL hook. Called from the Exit run-event.
pub fn uninstall() {
    let hook = HOOK.swap(0, Ordering::AcqRel);
    if hook != 0 {
        let _ = unsafe { UnhookWindowsHookEx(HHOOK(hook as *mut _)) };
    }
}

/// Reapply a persisted overlay position at startup, clamped into the work
/// area of whichever monitor now contains it. Leaves the default position
/// when no monitor contains the saved point (e.g. that monitor is gone).
pub fn restore_position(app: &AppHandle, overlay: &WebviewWindow, x: i32, y: i32) {
    let Ok(Some(monitor)) = app.monitor_from_point(x as f64, y as f64) else {
        return;
    };
    let scale = monitor.scale_factor();
    let work = monitor.work_area();
    let width = (crate::OVERLAY_W * scale).round() as i32;
    let height = (crate::OVERLAY_H * scale).round() as i32;
    let x = clamp_axis(
        x,
        work.position.x,
        work.position.x + work.size.width as i32 - width,
    );
    let y = clamp_axis(
        y,
        work.position.y,
        work.position.y + work.size.height as i32 - height,
    );
    let _ = overlay.set_position(PhysicalPosition::new(x, y));
}

fn update_pill_rect(overlay: &WebviewWindow) {
    let (Ok(pos), Ok(size), Ok(scale)) = (
        overlay.outer_position(),
        overlay.outer_size(),
        overlay.scale_factor(),
    ) else {
        return;
    };
    let center_x = pos.x as f64 + size.width as f64 / 2.0;
    let center_y = pos.y as f64 + size.height as f64 - PILL_CENTER_FROM_BOTTOM * scale;
    let half_w = PILL_HIT_W * scale / 2.0;
    let half_h = PILL_HIT_H * scale / 2.0;
    RECT_LEFT.store((center_x - half_w) as i32, Ordering::Relaxed);
    RECT_RIGHT.store((center_x + half_w) as i32, Ordering::Relaxed);
    RECT_TOP.store((center_y - half_h) as i32, Ordering::Relaxed);
    RECT_BOTTOM.store((center_y + half_h) as i32, Ordering::Relaxed);
}

unsafe extern "system" fn mouse_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && lparam.0 != 0 {
        let point = (*(lparam.0 as *const MSLLHOOKSTRUCT)).pt;
        match wparam.0 as u32 {
            WM_MOUSEMOVE => on_mouse_move(point.x, point.y),
            WM_LBUTTONDOWN => {
                // A press on the pill may become a data-tauri-drag-region
                // drag; stay interactive until the button is released.
                if CURSOR_INSIDE.load(Ordering::Relaxed) {
                    DRAGGING.store(true, Ordering::Relaxed);
                }
            }
            WM_LBUTTONUP => {
                if DRAGGING.swap(false, Ordering::Relaxed) {
                    on_drag_end();
                }
            }
            _ => {}
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

/// Hook side stays cheap: atomic reads plus a dispatch only when the
/// inside/outside answer flips (slow WH_MOUSE_LL callbacks lag the whole
/// system cursor).
fn on_mouse_move(x: i32, y: i32) {
    if DRAGGING.load(Ordering::Relaxed) {
        // The window chases the cursor mid-drag and the rect cache lags it;
        // never flip to click-through while the button is down.
        return;
    }
    let inside = x >= RECT_LEFT.load(Ordering::Relaxed)
        && x < RECT_RIGHT.load(Ordering::Relaxed)
        && y >= RECT_TOP.load(Ordering::Relaxed)
        && y < RECT_BOTTOM.load(Ordering::Relaxed);
    if CURSOR_INSIDE.swap(inside, Ordering::Relaxed) != inside {
        set_interactive(inside);
    }
}

fn set_interactive(interactive: bool) {
    let Some(app) = APP.get() else { return };
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(overlay) = handle.get_webview_window("overlay") {
            let _ = overlay.set_ignore_cursor_events(!interactive);
        }
    });
}

fn on_drag_end() {
    let Some(app) = APP.get() else { return };
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || clamp_and_persist(&handle));
}

/// After any pill left-button release (drag end or plain click), clamp the
/// host window into the current monitor's work area and persist the position
/// so it survives restarts. Skips the disk write when nothing changed.
fn clamp_and_persist(app: &AppHandle) {
    let Some(overlay) = app.get_webview_window("overlay") else {
        return;
    };
    let (Ok(pos), Ok(size)) = (overlay.outer_position(), overlay.outer_size()) else {
        return;
    };
    let Ok(Some(monitor)) = overlay.current_monitor() else {
        return;
    };

    let work = monitor.work_area();
    let x = clamp_axis(
        pos.x,
        work.position.x,
        work.position.x + work.size.width as i32 - size.width as i32,
    );
    let y = clamp_axis(
        pos.y,
        work.position.y,
        work.position.y + work.size.height as i32 - size.height as i32,
    );
    if (x, y) != (pos.x, pos.y) {
        let _ = overlay.set_position(PhysicalPosition::new(x, y));
    }

    let state = app.state::<AppState>();
    let snapshot = {
        let mut settings = state.settings.lock().unwrap();
        if settings.overlay_x != Some(x) || settings.overlay_y != Some(y) {
            settings.overlay_x = Some(x);
            settings.overlay_y = Some(y);
            Some(settings.clone())
        } else {
            None
        }
    };
    if let Some(snapshot) = snapshot {
        // Tiny JSON write, but keep it off the main thread.
        let app_dir = state.app_dir.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = snapshot.save(&app_dir) {
                eprintln!("[Typr] Failed to persist overlay position: {}", e);
            }
        });
    }
}

fn clamp_axis(value: i32, min: i32, max: i32) -> i32 {
    if max < min {
        // Window larger than the work area; pin to the near edge.
        min
    } else {
        value.clamp(min, max)
    }
}
