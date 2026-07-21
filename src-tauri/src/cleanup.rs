/// Remove whisper non-speech annotations like [BLANK_AUDIO], [MUSIC], (laughs).
/// Whisper emits these for silence/noise; they are never dictated content.
fn strip_annotations(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth_sq = 0usize;
    let mut depth_par = 0usize;
    for ch in text.chars() {
        match ch {
            '[' => depth_sq += 1,
            ']' => depth_sq = depth_sq.saturating_sub(1),
            '(' => depth_par += 1,
            ')' => depth_par = depth_par.saturating_sub(1),
            _ => {
                if depth_sq == 0 && depth_par == 0 {
                    out.push(ch);
                }
            }
        }
    }
    out
}

/// Remove common vocalized filler words (um, uh, er, ...) as standalone tokens.
/// Deliberately conservative: only unambiguous vocalized pauses, never meaning-
/// bearing words like "like" or "kind of", so it runs offline with zero risk of
/// changing what the user actually said. Meant to run on the RAW transcript
/// before `cleanup_text` (so capitalization/punctuation apply to the result).
pub fn remove_fillers(text: &str) -> String {
    const FILLERS: &[&str] = &[
        "um", "umm", "ummm", "uh", "uhh", "uhhh", "uhm", "er", "err", "erm",
        "ah", "ahh", "hmm", "hm", "hmmm", "mm", "mmm",
    ];
    text.split_whitespace()
        .filter(|tok| {
            // Match only the alphanumeric core, lowercased, so "Um," and "uh."
            // still match but "hummus" (core "hummus") does not.
            let core: String = tok
                .chars()
                .filter(|c| c.is_alphanumeric())
                .flat_map(|c| c.to_lowercase())
                .collect();
            core.is_empty() || !FILLERS.contains(&core.as_str())
        })
        .collect::<Vec<&str>>()
        .join(" ")
}

/// What post-processing to apply to a raw transcript. Every one of these was
/// previously hardcoded, so a user who wanted their dictation left alone had no
/// way to say so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleanupOptions {
    /// Capitalise the first letter of each sentence.
    pub capitalize: bool,
    /// Append a full stop when the text ends without terminal punctuation.
    pub ensure_punctuation: bool,
    /// Append a single trailing space, so consecutive dictations do not run
    /// together when pasted back to back.
    pub trailing_space: bool,
}

impl Default for CleanupOptions {
    /// The historical behaviour, so existing users see no change.
    fn default() -> Self {
        Self { capitalize: true, ensure_punctuation: true, trailing_space: false }
    }
}

pub fn cleanup_text(text: &str, opts: &CleanupOptions) -> String {
    let stripped = strip_annotations(text);
    // Drop stray punctuation left behind by removed annotations
    // (e.g. "[BLANK_AUDIO]." -> "."), then trim.
    let trimmed = stripped.trim();
    if trimmed.chars().all(|c| !c.is_alphanumeric()) {
        return String::new();
    }

    // Normalize multiple spaces to single space
    let normalized: String = trimmed
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ");

    // Capitalize first letter of each sentence
    let mut result = String::new();
    if opts.capitalize {
        let mut capitalize_next = true;
        for ch in normalized.chars() {
            if capitalize_next && ch.is_alphabetic() {
                result.extend(ch.to_uppercase());
                capitalize_next = false;
            } else {
                result.push(ch);
                if ch == '.' || ch == '!' || ch == '?' {
                    capitalize_next = true;
                }
            }
        }
    } else {
        result = normalized;
    }

    // Ensure ending punctuation
    if opts.ensure_punctuation {
        if let Some(last) = result.chars().last() {
            if !matches!(last, '.' | '!' | '?') {
                result.push('.');
            }
        }
    }

    // A trailing space keeps back-to-back dictations from running together.
    if opts.trailing_space && !result.is_empty() {
        result.push(' ');
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_whitespace() {
        assert_eq!(cleanup_text("  hello world  ", &CleanupOptions::default()), "Hello world.");
    }

    #[test]
    fn test_normalize_spaces() {
        assert_eq!(cleanup_text("hello    world", &CleanupOptions::default()), "Hello world.");
    }

    #[test]
    fn test_capitalize_first_letter() {
        assert_eq!(cleanup_text("hello world", &CleanupOptions::default()), "Hello world.");
    }

    #[test]
    fn test_capitalize_after_period() {
        assert_eq!(cleanup_text("hello. world", &CleanupOptions::default()), "Hello. World.");
    }

    #[test]
    fn test_capitalize_after_question_mark() {
        assert_eq!(cleanup_text("hello? world", &CleanupOptions::default()), "Hello? World.");
    }

    #[test]
    fn test_capitalize_after_exclamation() {
        assert_eq!(cleanup_text("hello! world", &CleanupOptions::default()), "Hello! World.");
    }

    #[test]
    fn test_ensure_ending_punctuation() {
        assert_eq!(cleanup_text("hello world", &CleanupOptions::default()), "Hello world.");
    }

    #[test]
    fn test_preserve_existing_ending_punctuation() {
        assert_eq!(cleanup_text("hello world.", &CleanupOptions::default()), "Hello world.");
        assert_eq!(cleanup_text("hello world!", &CleanupOptions::default()), "Hello world!");
        assert_eq!(cleanup_text("hello world?", &CleanupOptions::default()), "Hello world?");
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(cleanup_text("", &CleanupOptions::default()), "");
        assert_eq!(cleanup_text("   ", &CleanupOptions::default()), "");
    }

    #[test]
    fn test_already_clean() {
        assert_eq!(cleanup_text("Hello world.", &CleanupOptions::default()), "Hello world.");
    }

    #[test]
    fn test_strip_blank_audio() {
        assert_eq!(cleanup_text("[BLANK_AUDIO]", &CleanupOptions::default()), "");
        assert_eq!(cleanup_text("[BLANK_AUDIO].", &CleanupOptions::default()), "");
        assert_eq!(cleanup_text("[BLANK_AUDIO].[BLANK_AUDIO].", &CleanupOptions::default()), "");
    }

    #[test]
    fn test_strip_annotations_inside_speech() {
        assert_eq!(cleanup_text("hello [MUSIC] world", &CleanupOptions::default()), "Hello world.");
        assert_eq!(cleanup_text("(laughs) hello world", &CleanupOptions::default()), "Hello world.");
        assert_eq!(cleanup_text("[BLANK_AUDIO] hey, what's up?", &CleanupOptions::default()), "Hey, what's up?");
    }

    #[test]
    fn test_remove_fillers() {
        assert_eq!(remove_fillers("um so uh like it works"), "so like it works");
        assert_eq!(remove_fillers("Um, hello there"), "hello there");
        assert_eq!(remove_fillers("hummus is great"), "hummus is great");
        assert_eq!(remove_fillers("no fillers at all"), "no fillers at all");
        assert_eq!(remove_fillers(""), "");
    }
}
