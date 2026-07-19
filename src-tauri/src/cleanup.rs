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

pub fn cleanup_text(text: &str) -> String {
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

    // Ensure ending punctuation
    if let Some(last) = result.chars().last() {
        if !matches!(last, '.' | '!' | '?') {
            result.push('.');
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_whitespace() {
        assert_eq!(cleanup_text("  hello world  "), "Hello world.");
    }

    #[test]
    fn test_normalize_spaces() {
        assert_eq!(cleanup_text("hello    world"), "Hello world.");
    }

    #[test]
    fn test_capitalize_first_letter() {
        assert_eq!(cleanup_text("hello world"), "Hello world.");
    }

    #[test]
    fn test_capitalize_after_period() {
        assert_eq!(cleanup_text("hello. world"), "Hello. World.");
    }

    #[test]
    fn test_capitalize_after_question_mark() {
        assert_eq!(cleanup_text("hello? world"), "Hello? World.");
    }

    #[test]
    fn test_capitalize_after_exclamation() {
        assert_eq!(cleanup_text("hello! world"), "Hello! World.");
    }

    #[test]
    fn test_ensure_ending_punctuation() {
        assert_eq!(cleanup_text("hello world"), "Hello world.");
    }

    #[test]
    fn test_preserve_existing_ending_punctuation() {
        assert_eq!(cleanup_text("hello world."), "Hello world.");
        assert_eq!(cleanup_text("hello world!"), "Hello world!");
        assert_eq!(cleanup_text("hello world?"), "Hello world?");
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(cleanup_text(""), "");
        assert_eq!(cleanup_text("   "), "");
    }

    #[test]
    fn test_already_clean() {
        assert_eq!(cleanup_text("Hello world."), "Hello world.");
    }

    #[test]
    fn test_strip_blank_audio() {
        assert_eq!(cleanup_text("[BLANK_AUDIO]"), "");
        assert_eq!(cleanup_text("[BLANK_AUDIO]."), "");
        assert_eq!(cleanup_text("[BLANK_AUDIO].[BLANK_AUDIO]."), "");
    }

    #[test]
    fn test_strip_annotations_inside_speech() {
        assert_eq!(cleanup_text("hello [MUSIC] world"), "Hello world.");
        assert_eq!(cleanup_text("(laughs) hello world"), "Hello world.");
        assert_eq!(cleanup_text("[BLANK_AUDIO] hey, what's up?"), "Hey, what's up?");
    }
}
