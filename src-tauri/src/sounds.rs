// Short interaction pings (US-004). Embedded at compile time so playback
// never touches disk on the hotkey path. Played via winmm PlaySound with
// SND_MEMORY | SND_ASYNC | SND_NODEFAULT: PlaySound returns immediately
// (async) and the OS mixer copies/streams from the buffer, but the buffer
// must stay alive for the duration of playback — 'static embeds guarantee
// that trivially.
//
// The result USED to be discarded with `let _ =`. Combined with SND_NODEFAULT
// (which suppresses the fallback system beep) that made a failed ping
// completely invisible: no sound, no error, nothing in the log. A user
// reported "the sounds dont play" and there was no way to tell whether the
// call was even reached, let alone whether Windows rejected it. Now every
// call reports.
use demod_lib::applog;
use windows::core::PCWSTR;
use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};

static START_WAV: &[u8] = include_bytes!("../sounds/start.wav");
static STOP_WAV: &[u8] = include_bytes!("../sounds/stop.wav");

/// Returns whether Windows accepted the request. True does NOT prove the user
/// heard anything - PlaySound reports that it queued the buffer, not that the
/// output device was audible or unmuted - but false is a definite failure.
fn play(bytes: &'static [u8]) -> bool {
    unsafe {
        // PlaySoundW's pszSound param doubles as a pointer to the in-memory
        // buffer when SND_MEMORY is set; it does not read it as a string.
        PlaySoundW(
            PCWSTR(bytes.as_ptr() as *const u16),
            None,
            SND_ASYNC | SND_MEMORY | SND_NODEFAULT,
        )
        .as_bool()
    }
}

fn play_logged(bytes: &'static [u8], label: &str) {
    if !play(bytes) {
        eprintln!(
            "[Demod] PlaySound rejected the {} ping ({} bytes). The sound will be silent.",
            label,
            bytes.len()
        );
        applog::log_event_default(
            "sound",
            &format!("PlaySound rejected the {} ping ({} bytes)", label, bytes.len()),
        );
    }
}

/// Play the start ping if interaction sounds are enabled. Reads the flag
/// from an in-memory settings snapshot passed by the caller — never from
/// disk on the hotkey path.
pub fn play_start(interaction_sounds: bool) {
    if interaction_sounds {
        play_logged(START_WAV, "start");
    }
}

/// Play the stop ping if interaction sounds are enabled.
pub fn play_stop(interaction_sounds: bool) {
    if interaction_sounds {
        play_logged(STOP_WAV, "stop");
    }
}

/// Play a ping on demand, ignoring the interaction-sounds setting, and report
/// whether Windows accepted it.
///
/// This exists because "the sounds dont play" was previously undiagnosable:
/// the pings only fire on the hotkey path, so there was no way to test audio
/// without starting a real dictation, and a failure produced no signal at all.
/// The settings preview button calls this, so the user can tell the difference
/// between "the setting is off", "Windows refused" and "it played but I could
/// not hear it".
pub fn preview(which: &str) -> Result<(), String> {
    let (bytes, label) = match which {
        "start" => (START_WAV, "start"),
        "stop" => (STOP_WAV, "stop"),
        other => return Err(format!("Unknown sound '{}'", other)),
    };
    if play(bytes) {
        Ok(())
    } else {
        Err(format!(
            "Windows would not play the {} sound. Check that Demod is not muted in the volume mixer.",
            label
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The embedded assets must be real, non-empty RIFF/WAVE images. PlaySound
    /// with SND_MEMORY silently rejects anything that is not, so a truncated or
    /// wrong-format asset would ship as "no sound" with no other symptom.
    #[test]
    fn embedded_sounds_are_riff_wave() {
        for (bytes, name) in [(START_WAV, "start"), (STOP_WAV, "stop")] {
            assert!(bytes.len() > 44, "{} is too small to be a WAV", name);
            assert_eq!(&bytes[0..4], b"RIFF", "{} is missing the RIFF magic", name);
            assert_eq!(&bytes[8..12], b"WAVE", "{} is not a WAVE file", name);
            // RIFF size field must match the real payload, or PlaySound bails.
            let declared = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
            assert_eq!(
                declared,
                bytes.len() - 8,
                "{}'s RIFF size does not match its length",
                name
            );
        }
    }

    #[test]
    fn preview_rejects_unknown_sound() {
        assert!(preview("nonsense").is_err());
    }
}
