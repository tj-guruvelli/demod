// Short interaction pings (US-004). Embedded at compile time so playback
// never touches disk on the hotkey path. Played via winmm PlaySound with
// SND_MEMORY | SND_ASYNC | SND_NODEFAULT: PlaySound returns immediately
// (async) and the OS mixer copies/streams from the buffer, but the buffer
// must stay alive for the duration of playback — 'static embeds guarantee
// that trivially.
use windows::core::PCWSTR;
use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};

static START_WAV: &[u8] = include_bytes!("../sounds/start.wav");
static STOP_WAV: &[u8] = include_bytes!("../sounds/stop.wav");

fn play(bytes: &'static [u8]) {
    unsafe {
        // PlaySoundW's pszSound param doubles as a pointer to the in-memory
        // buffer when SND_MEMORY is set; it does not read it as a string.
        let _ = PlaySoundW(
            PCWSTR(bytes.as_ptr() as *const u16),
            None,
            SND_ASYNC | SND_MEMORY | SND_NODEFAULT,
        );
    }
}

/// Play the start ping if interaction sounds are enabled. Reads the flag
/// from an in-memory settings snapshot passed by the caller — never from
/// disk on the hotkey path.
pub fn play_start(interaction_sounds: bool) {
    if interaction_sounds {
        play(START_WAV);
    }
}

/// Play the stop ping if interaction sounds are enabled.
pub fn play_stop(interaction_sounds: bool) {
    if interaction_sounds {
        play(STOP_WAV);
    }
}
