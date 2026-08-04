use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tauri::Emitter;

/// Minimum spacing between 'audio-level' emissions (~30Hz ceiling). The
/// overlay waveform is driven by these events; see src/overlay.html.
const LEVEL_EMIT_INTERVAL: Duration = Duration::from_millis(33);

#[derive(Debug, Clone, serde::Serialize)]
pub struct MicDevice {
    pub name: String,
    pub is_default: bool,
}

pub fn list_microphones() -> Vec<MicDevice> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|d| d.name().ok())
        .unwrap_or_default();

    let mut devices = Vec::new();
    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            if let Ok(name) = device.name() {
                devices.push(MicDevice {
                    is_default: name == default_name,
                    name,
                });
            }
        }
    }
    devices
}

/// Wrapper to make cpal::Stream usable across threads.
/// SAFETY: cpal::Stream on macOS (CoreAudio) is thread-safe in practice;
/// we only access it behind a Mutex to start/stop recording.
struct SendStream(#[allow(dead_code)] cpal::Stream);
unsafe impl Send for SendStream {}
unsafe impl Sync for SendStream {}

pub struct AudioRecorder {
    samples: Arc<Mutex<Vec<f32>>>,
    stream: Option<SendStream>,
    source_sample_rate: u32,
    source_channels: u16,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            stream: None,
            source_sample_rate: 48000,
            source_channels: 1,
        }
    }

    pub fn start(&mut self, app: &tauri::AppHandle, mic_name: &str) -> Result<(), String> {
        // Clear any leftover samples from previous recording
        self.samples.lock().unwrap().clear();

        let host = cpal::default_host();

        let device = if mic_name == "default" || mic_name.is_empty() {
            host.default_input_device()
                .ok_or("No default input device found")?
        } else {
            host.input_devices()
                .map_err(|e| e.to_string())?
                .find(|d| d.name().map(|n| n == mic_name).unwrap_or(false))
                .ok_or(format!("Microphone '{}' not found", mic_name))?
        };

        // Use the device's default config instead of forcing 16kHz
        let default_config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get default input config: {}", e))?;

        let sample_rate = default_config.sample_rate().0;
        let channels = default_config.channels();

        println!("[Demod] Mic config: {}Hz, {} channels", sample_rate, channels);

        self.source_sample_rate = sample_rate;
        self.source_channels = channels;

        let config = cpal::StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let samples = self.samples.clone();
        let level_app = app.clone();
        let mut level_sum_sq = 0.0f64;
        let mut level_samples = 0usize;
        let mut last_level_emit = Instant::now();
        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    {
                        let mut buf = samples.lock().unwrap();
                        buf.extend_from_slice(data);
                    }
                    // Honest waveform feed: RMS of everything captured since
                    // the last emission, throttled to <=30Hz. The stream only
                    // exists while recording, so levels stop with capture.
                    // emit_to("overlay") only reaches window-scoped listeners,
                    // not target-Any ones (overlay.html listens accordingly).
                    for &s in data {
                        level_sum_sq += (s as f64) * (s as f64);
                    }
                    level_samples += data.len();
                    if level_samples > 0 && last_level_emit.elapsed() >= LEVEL_EMIT_INTERVAL {
                        let rms = (level_sum_sq / level_samples as f64).sqrt() as f32;
                        let _ = level_app.emit_to("overlay", "audio-level", rms);
                        level_sum_sq = 0.0;
                        level_samples = 0;
                        last_level_emit = Instant::now();
                    }
                },
                |err| {
                    eprintln!("[Demod] Audio stream error: {}", err);
                    crate::applog::log_event_default("audio", &format!("stream error: {}", err));
                },
                None,
            )
            .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;
        self.stream = Some(SendStream(stream));
        println!("[Demod] Audio recording started");
        Ok(())
    }

    /// Stop capture and discard whatever was recorded, without writing a
    /// WAV file (US-006 Esc cancel). Mirrors stop_and_save's stream
    /// teardown but skips the resample/encode/write entirely.
    pub fn cancel(&mut self) {
        self.stream = None; // Drop stops the stream
        self.samples.lock().unwrap().clear();
        println!("[Demod] Audio recording cancelled (discarded)");
    }

    pub fn stop_and_save(&mut self, output_path: &PathBuf) -> Result<PathBuf, String> {
        self.stream = None; // Drop stops the stream
        println!("[Demod] Audio recording stopped");

        let samples = self.samples.lock().unwrap();
        if samples.is_empty() {
            return Err("No audio captured".to_string());
        }

        println!("[Demod] Captured {} raw samples", samples.len());

        // Convert to mono if multi-channel
        let mono: Vec<f32> = if self.source_channels > 1 {
            samples
                .chunks(self.source_channels as usize)
                .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
                .collect()
        } else {
            samples.clone()
        };

        // Downsample to 16kHz for whisper.cpp
        let resampled = resample(&mono, self.source_sample_rate, 16000);
        println!("[Demod] Resampled to {} samples at 16kHz", resampled.len());

        let spec = WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(output_path, spec).map_err(|e| e.to_string())?;
        for &sample in resampled.iter() {
            let amplitude = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(amplitude).map_err(|e| e.to_string())?;
        }
        writer.finalize().map_err(|e| e.to_string())?;

        drop(samples);
        self.samples.lock().unwrap().clear();

        println!("[Demod] WAV saved to {:?}", output_path);
        Ok(output_path.clone())
    }
}

/// Simple linear interpolation resampler
fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (samples.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_idx = i as f64 * ratio;
        let idx = src_idx as usize;
        let frac = src_idx - idx as f64;

        let sample = if idx + 1 < samples.len() {
            samples[idx] as f64 * (1.0 - frac) + samples[idx + 1] as f64 * frac
        } else {
            samples[idx.min(samples.len() - 1)] as f64
        };

        output.push(sample as f32);
    }

    output
}
