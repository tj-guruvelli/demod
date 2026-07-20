use reqwest::multipart;
use std::path::PathBuf;
use std::time::Duration;

/// Outcome of probing a Groq API key without uploading any audio. Three
/// distinct states because they demand three different user fixes: a
/// rejected key means "get a real key", an unreachable network means "check
/// your connection", and conflating them (as a single "network error"
/// string would) sends the user chasing the wrong fix.
#[derive(Debug, PartialEq)]
pub enum GroqKeyCheck {
    Valid,
    /// Groq answered and said no (401/403) — the key itself is bad.
    Rejected,
    /// No usable answer came back at all (timeout, DNS, offline, or an
    /// unexpected non-auth status) — we never learned whether the key is
    /// good or bad.
    Unreachable(String),
}

/// Validate a Groq API key with a cheap, audio-free request: GET
/// /openai/v1/models: 200 means the key is accepted, 401/403 means Groq
/// rejected it, anything else (including a timeout or DNS failure) means we
/// could not reach Groq at all and must not guess which is true.
///
/// Never logs or echoes `api_key` — reqwest sends it only as the
/// Authorization header value, which is not part of the URL and does not
/// appear in error messages below.
pub async fn check_groq_key(api_key: &str) -> GroqKeyCheck {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
    {
        Ok(c) => c,
        Err(e) => return GroqKeyCheck::Unreachable(format!("Could not start request: {}", e)),
    };

    match client
        .get("https://api.groq.com/openai/v1/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                GroqKeyCheck::Valid
            } else if status.as_u16() == 401 || status.as_u16() == 403 {
                GroqKeyCheck::Rejected
            } else {
                GroqKeyCheck::Unreachable(format!("Groq returned an unexpected status ({})", status))
            }
        }
        Err(e) if e.is_timeout() => {
            GroqKeyCheck::Unreachable("Timed out reaching Groq".to_string())
        }
        Err(e) => GroqKeyCheck::Unreachable(format!("Could not reach Groq: {}", e)),
    }
}

pub async fn transcribe_groq(
    api_key: &str,
    audio_path: &PathBuf,
    language: &str,
    dictionary: &str,
    model: &str,
) -> Result<String, String> {
    if api_key.is_empty() {
        return Err("Groq API key not set. Please enter your API key in settings.".to_string());
    }

    let audio_bytes = std::fs::read(audio_path)
        .map_err(|e| format!("Failed to read audio file: {}", e))?;

    let file_part = multipart::Part::bytes(audio_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;

    // Language was hardcoded "en" here, so the setting silently did nothing on
    // the cloud engine - anyone dictating another language was mis-transcribed
    // with no indication. Groq follows the OpenAI API, where "auto" is NOT a
    // valid value: detection is requested by omitting the field entirely.
    let mut form = multipart::Form::new()
        .text("model", model.to_string())
        .text("response_format", "json")
        .part("file", file_part);
    if language != "auto" {
        form = form.text("language", language.to_string());
    }
    // Same purpose as whisper.cpp's --prompt: bias the model toward the user's
    // own vocabulary. The API ignores an empty value, so only send a real one.
    if !dictionary.trim().is_empty() {
        form = form.text("prompt", dictionary.trim().to_string());
    }

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.groq.com/openai/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Groq API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Groq API error ({}): {}", status, body));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Groq response: {}", e))?;

    json["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or("No 'text' field in Groq response".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_api_key() {
        let path = PathBuf::from("/tmp/test.wav");
        let result = transcribe_groq("", &path, "en", "", "whisper-large-v3").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("API key not set"));
    }
}
