use std::path::PathBuf;
use std::time::Duration;
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;

pub const SERVER_PORT: u16 = 8178;

/// Threads for whisper inference: half the logical cores, clamped to 4..=8.
pub fn whisper_threads() -> usize {
    let logical = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(8);
    (logical / 2).clamp(4, 8)
}

/// Fast path: POST the WAV to the persistent whisper-server (model already
/// loaded in memory). Falls back to the one-shot CLI sidecar if the server
/// is not reachable.
async fn transcribe_via_server(audio_path: &PathBuf) -> Result<String, String> {
    let bytes = std::fs::read(audio_path).map_err(|e| format!("read wav: {}", e))?;
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;
    let form = reqwest::multipart::Form::new()
        .text("response_format", "json")
        .part("file", part);

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(500))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(format!("http://127.0.0.1:{}/inference", SERVER_PORT))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("server unreachable: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("server error: {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    json["text"]
        .as_str()
        .map(|s| s.trim().to_string())
        .ok_or("no 'text' in server response".to_string())
}

pub async fn transcribe_local(
    app: &AppHandle,
    model_path: &PathBuf,
    audio_path: &PathBuf,
    language: &str,
    dictionary: &str,
) -> Result<String, String> {
    if !model_path.exists() {
        return Err("Whisper model not found. Please download a model first.".to_string());
    }

    match transcribe_via_server(audio_path).await {
        Ok(text) => {
            println!("[Demod] Transcribed via warm whisper-server");
            return Ok(text);
        }
        Err(e) => {
            eprintln!("[Demod] whisper-server path failed ({}), falling back to CLI", e);
        }
    }

    println!("[Demod] Running whisper.cpp sidecar with model {:?}", model_path);

    let threads = whisper_threads().to_string();
    let mut cmd = app
        .shell()
        .sidecar("whisper-cpp")
        .map_err(|e| format!("Failed to create sidecar command: {}", e))?
        .args([
            "-m",
            model_path.to_str().unwrap(),
            "-f",
            audio_path.to_str().unwrap(),
            "--no-timestamps",
            // Was hardcoded "en". whisper.cpp accepts "auto" for detection.
            "-l",
            language,
            "-t",
            &threads,
        ]);
    // whisper.cpp biases recognition toward an initial prompt, which is how a
    // custom dictionary (names, jargon, product terms) actually takes effect.
    // Only pass it when there is one - an empty prompt is not the same as none.
    let dict = dictionary.trim();
    if !dict.is_empty() {
        cmd = cmd.args(["--prompt", dict]);
    }
    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Failed to run whisper.cpp: {}", e))?;

    if output.status.code() != Some(0) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("whisper.cpp failed: {}", stderr));
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // Length only, never the content: this is the user's dictated speech and
    // may contain anything they said. See log_transcript in main.rs.
    println!("[Demod] Whisper output: {} chars", text.chars().count());
    Ok(text)
}

pub fn model_filename(model_size: &str) -> String {
    format!("ggml-{}.bin", model_size)
}

pub fn model_download_url(model_size: &str) -> String {
    format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
        model_size
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_filename() {
        assert_eq!(model_filename("small"), "ggml-small.bin");
        assert_eq!(model_filename("medium"), "ggml-medium.bin");
        assert_eq!(model_filename("large-v3"), "ggml-large-v3.bin");
    }

    #[test]
    fn test_model_download_url() {
        assert_eq!(
            model_download_url("small"),
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin"
        );
        assert_eq!(
            model_download_url("large-v3"),
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin"
        );
    }
}
