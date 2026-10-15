use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

#[derive(serde::Serialize)]
pub struct MicrophoneStatus {
    pub available: bool,
    pub ready: bool,
    pub selected_device: Option<String>,
    pub default_device: Option<String>,
    pub error: Option<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct TranscriptionComplete {
    pub text: String,
    pub word_count: i64,
    pub duration_ms: i64,
    pub source: String,
}

pub static AUDIO_BUFFER: std::sync::LazyLock<Arc<Mutex<Vec<f32>>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(Vec::new())));

pub(crate) static DEVICE_SAMPLE_RATE: Mutex<u32> = Mutex::new(44100);
static CAPTURE_CONTROL: std::sync::LazyLock<Mutex<Option<std::sync::mpsc::SyncSender<CaptureCommand>>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));
static CAPTURE_ACTIVE: AtomicBool = AtomicBool::new(false);
static LAST_RMS: Mutex<f32> = Mutex::new(0.0);

const MAX_CAPTURE_SECONDS: usize = 600;
const PREROLL_MS: usize = 240;
const START_RMS_THRESHOLD: f32 = 0.004;
const END_RMS_THRESHOLD: f32 = 0.003;
const MIN_SPEECH_MS: usize = 90;
const TRAILING_SILENCE_MS: usize = 360;

enum CaptureCommand {
    Start(std::sync::mpsc::SyncSender<Result<u32, String>>),
}

pub fn recordings_dir() -> std::path::PathBuf {
    let mut p = dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    p.push("MeshVoice"); p.push("recordings");
    std::fs::create_dir_all(&p).ok(); p
}

pub fn models_dir() -> std::path::PathBuf {
    let mut p = dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    p.push("MeshVoice"); p.push("models");
    std::fs::create_dir_all(&p).ok(); p
}

#[tauri::command]
pub fn open_mic_settings() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(&["/C", "start", "ms-settings:privacy-microphone"])
            .spawn()
            .map_err(|e| format!("Could not open Windows microphone settings: {}", e))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    Ok(())
}

#[tauri::command]
pub fn get_audio_devices() -> Vec<String> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    let mut devices = Vec::new();
    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            if let Ok(name) = device.name() {
                devices.push(name);
            }
        }
    }
    devices
}

#[tauri::command]
pub fn check_microphone_status() -> MicrophoneStatus {
    let host = cpal::default_host();
    let default_device = host.default_input_device().and_then(|d| d.name().ok());
    let device = match get_selected_device() {
        Some(device) => device,
        None => {
            return MicrophoneStatus {
                available: false,
                ready: false,
                selected_device: None,
                default_device,
                error: Some("No microphone found. Connect a microphone or enable one in Windows Sound settings.".into()),
            };
        }
    };
    let selected_device = device.name().ok();
    let config = match device.default_input_config() {
        Ok(config) => config,
        Err(error) => {
            return MicrophoneStatus {
                available: true,
                ready: false,
                selected_device,
                default_device,
                error: Some(format!("Microphone is available but cannot be configured: {}", error)),
            };
        }
    };

    match probe_input_stream(&device, &config) {
        Ok(()) => MicrophoneStatus {
            available: true,
            ready: true,
            selected_device,
            default_device,
            error: None,
        },
        Err(error) => MicrophoneStatus {
            available: true,
            ready: false,
            selected_device,
            default_device,
            error: Some(error),
        },
    }
}

fn probe_input_stream(device: &cpal::Device, cfg: &cpal::SupportedStreamConfig) -> Result<(), String> {
    let stream_config: cpal::StreamConfig = cfg.clone().into();
    let err = |e| eprintln!("[MeshVoice] microphone probe: {}", e);
    let stream = match cfg.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(&stream_config, |_data: &[f32], _| {}, err, None),
        cpal::SampleFormat::I16 => device.build_input_stream(&stream_config, |_data: &[i16], _| {}, err, None),
        cpal::SampleFormat::U16 => device.build_input_stream(&stream_config, |_data: &[u16], _| {}, err, None),
        cpal::SampleFormat::I8 => device.build_input_stream(&stream_config, |_data: &[i8], _| {}, err, None),
        cpal::SampleFormat::U8 => device.build_input_stream(&stream_config, |_data: &[u8], _| {}, err, None),
        cpal::SampleFormat::I32 => device.build_input_stream(&stream_config, |_data: &[i32], _| {}, err, None),
        cpal::SampleFormat::U32 => device.build_input_stream(&stream_config, |_data: &[u32], _| {}, err, None),
        cpal::SampleFormat::F64 => device.build_input_stream(&stream_config, |_data: &[f64], _| {}, err, None),
        _ => return Err("Unsupported microphone sample format.".into()),
    }.map_err(|e| format_microphone_error("Microphone stream creation failed", e))?;

    stream.play().map_err(|e| format_microphone_error("Microphone access failed", e))?;
    std::thread::sleep(std::time::Duration::from_millis(220));
    Ok(())
}

fn get_selected_device() -> Option<cpal::Device> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    
    let preferred = {
        let conn = crate::db::DB_CONN.lock().unwrap();
        conn.query_row("SELECT value FROM settings WHERE key='microphone'", [], |r| r.get::<_,String>(0)).ok()
    };
    
    if let Some(pref) = preferred.filter(|p| !p.trim().is_empty()) {
        if let Ok(input_devices) = host.input_devices() {
            for device in input_devices {
                if device.name().unwrap_or_default() == pref && device.default_input_config().is_ok() {
                    return Some(device);
                }
            }
        }
    }

    if let Some(device) = host.default_input_device() {
        if device.default_input_config().is_ok() {
            return Some(device);
        }
    }

    host.input_devices().ok()?.find(|device| device.default_input_config().is_ok())
}

fn ensure_capture_stream() -> Result<(), String> {
    let sender = {
        let mut guard = CAPTURE_CONTROL.lock().unwrap();
        if let Some(sender) = guard.as_ref() {
            sender.clone()
        } else {
            let sender = spawn_capture_thread()?;
            *guard = Some(sender.clone());
            sender
        }
    };

    let (reply_tx, reply_rx) = std::sync::mpsc::sync_channel(1);
    if sender.send(CaptureCommand::Start(reply_tx)).is_err() {
        *CAPTURE_CONTROL.lock().unwrap() = None;
        return ensure_capture_stream();
    }

    let rate = reply_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .map_err(|_| "Microphone stream did not become ready.".to_string())??;
    *DEVICE_SAMPLE_RATE.lock().unwrap() = rate;
    Ok(())
}

fn spawn_capture_thread() -> Result<std::sync::mpsc::SyncSender<CaptureCommand>, String> {
    let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel::<Result<std::sync::mpsc::SyncSender<CaptureCommand>, String>>(1);
    std::thread::spawn(move || {
        run_capture_thread(ready_tx);
    });

    ready_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .map_err(|_| "Microphone thread did not initialize.".to_string())?
}

fn run_capture_thread(ready_tx: std::sync::mpsc::SyncSender<Result<std::sync::mpsc::SyncSender<CaptureCommand>, String>>) {
    let device = match get_selected_device() {
        Some(device) => device,
        None => {
            let _ = ready_tx.send(Err("No microphone found. Please check your system settings.".into()));
            return;
        }
    };
    let cfg = match device.default_input_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            let _ = ready_tx.send(Err(format!("Mic config error: {}", e)));
            return;
        }
    };
    let sample_rate = cfg.sample_rate().0;
    let stream_config: cpal::StreamConfig = cfg.clone().into();
    let channels = cfg.channels() as usize;
    let sample_rate_for_callback = sample_rate;
    let err = |e| eprintln!("[MeshVoice] {}", e);

    let stream = match cfg.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &stream_config,
            move |data: &[f32], _| append_input_frames(data.chunks(channels).map(|frame| frame.iter().sum::<f32>() / channels as f32), sample_rate_for_callback),
            err,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &stream_config,
            move |data: &[i16], _| append_input_frames(data.chunks(channels).map(|frame| frame.iter().map(|&s| s as f32 / 32768.0).sum::<f32>() / channels as f32), sample_rate_for_callback),
            err,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_input_stream(
            &stream_config,
            move |data: &[u16], _| append_input_frames(data.chunks(channels).map(|frame| frame.iter().map(|&s| s as f32 / 32768.0 - 1.0).sum::<f32>() / channels as f32), sample_rate_for_callback),
            err,
            None,
        ),
        cpal::SampleFormat::I8 => device.build_input_stream(
            &stream_config,
            move |data: &[i8], _| append_input_frames(data.chunks(channels).map(|frame| frame.iter().map(|&s| s as f32 / 128.0).sum::<f32>() / channels as f32), sample_rate_for_callback),
            err,
            None,
        ),
        cpal::SampleFormat::U8 => device.build_input_stream(
            &stream_config,
            move |data: &[u8], _| append_input_frames(data.chunks(channels).map(|frame| frame.iter().map(|&s| s as f32 / 128.0 - 1.0).sum::<f32>() / channels as f32), sample_rate_for_callback),
            err,
            None,
        ),
        cpal::SampleFormat::I32 => device.build_input_stream(
            &stream_config,
            move |data: &[i32], _| append_input_frames(data.chunks(channels).map(|frame| frame.iter().map(|&s| s as f32 / 2_147_483_648.0).sum::<f32>() / channels as f32), sample_rate_for_callback),
            err,
            None,
        ),
        cpal::SampleFormat::U32 => device.build_input_stream(
            &stream_config,
            move |data: &[u32], _| append_input_frames(data.chunks(channels).map(|frame| frame.iter().map(|&s| s as f32 / 2_147_483_648.0 - 1.0).sum::<f32>() / channels as f32), sample_rate_for_callback),
            err,
            None,
        ),
        cpal::SampleFormat::F64 => device.build_input_stream(
            &stream_config,
            move |data: &[f64], _| append_input_frames(data.chunks(channels).map(|frame| (frame.iter().sum::<f64>() / channels as f64) as f32), sample_rate_for_callback),
            err,
            None,
        ),
        _ => {
            let _ = ready_tx.send(Err("Unsupported microphone sample format.".into()));
            return;
        }
    };
    let stream = match stream {
        Ok(stream) => stream,
        Err(e) => {
            let _ = ready_tx.send(Err(format_microphone_error("Stream creation failed", e)));
            return;
        }
    };

    if let Err(e) = stream.play() {
        let _ = ready_tx.send(Err(format_microphone_error("Microphone access failed", e)));
        return;
    }
    let (tx, rx) = std::sync::mpsc::sync_channel::<CaptureCommand>(8);
    let _ = ready_tx.send(Ok(tx.clone()));
    let _stream = stream;
    while let Ok(command) = rx.recv() {
        match command {
            CaptureCommand::Start(reply) => {
                let _ = reply.send(Ok(sample_rate));
            }
        };
    }
}

fn append_input_frames<I>(samples: I, sample_rate: u32)
where
    I: IntoIterator<Item = f32>,
{
    if !CAPTURE_ACTIVE.load(Ordering::Relaxed) {
        return;
    }

    let mut b = AUDIO_BUFFER.lock().unwrap();
    let max_samples = sample_rate as usize * MAX_CAPTURE_SECONDS;
    let before = b.len();
    for sample in samples {
        b.push(sample.clamp(-1.0, 1.0));
    }

    if b.len() > max_samples {
        let overflow = b.len() - max_samples;
        b.drain(0..overflow);
    }

    let new = &b[before.min(b.len())..];
    if !new.is_empty() {
        let rms = (new.iter().map(|s| s * s).sum::<f32>() / new.len() as f32).sqrt().clamp(0.0, 1.0);
        *LAST_RMS.lock().unwrap() = rms;
    }
}

#[tauri::command]
pub async fn start_recording(
    _app: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
) -> Result<u64, String> {
    if *state.is_recording.lock().unwrap() {
        return Ok(*state.recording_session_id.lock().unwrap());
    }
    AUDIO_BUFFER.lock().unwrap().clear();
    ensure_capture_stream()?;
    let session_id = {
        let mut id = state.recording_session_id.lock().unwrap();
        *id = if *id == u64::MAX { 1 } else { *id + 1 };
        *id
    };
    CAPTURE_ACTIVE.store(true, Ordering::SeqCst);
    *state.is_recording.lock().unwrap() = true;

    Ok(session_id)
}

#[tauri::command]
pub async fn stop_recording_and_transcribe(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
    duration_ms: i64,
    access_token: Option<String>,
    session_id: Option<u64>,
) -> Result<String, String> {
    if let Some(stop_session_id) = session_id {
        let current_session_id = *state.recording_session_id.lock().unwrap();
        if stop_session_id != current_session_id {
            return Err("Stale recording stop ignored.".into());
        }
    }
    {
        let mut is_recording = state.is_recording.lock().unwrap();
        if !*is_recording {
            return Err("Recording already stopped.".into());
        }
        *is_recording = false;
    }
    CAPTURE_ACTIVE.store(false, Ordering::SeqCst);
    tokio::time::sleep(std::time::Duration::from_millis(40)).await;

    let raw = AUDIO_BUFFER.lock().unwrap().clone();
    let rate = *DEVICE_SAMPLE_RATE.lock().unwrap();

    if raw.len() < 800 {
        if duration_ms > 1000 {
            return Err("Microphone access appears blocked. Enable microphone access for desktop apps in Windows Privacy settings, then try again.".into());
        } else {
            return Err("Recording too short. Hold to talk.".into());
        }
    }

    let samples = trim_voice_activity(&raw, rate)
        .ok_or_else(|| "No speech detected.".to_string())?;
    let mut samples = resample_to_16k(&samples, rate);
    let recorded_duration_ms = duration_ms.max(((samples.len() as i64 * 1000) / 16000).max(1));

    // Normalize only after VAD so silence/noise is not amplified into hallucinations.
    let mut max_amp: f32 = 0.0;
    for &s in samples.iter() {
        if s.abs() > max_amp { max_amp = s.abs(); }
    }
    if max_amp > 0.0 && max_amp < 0.8 {
        let factor = 0.8 / max_amp;
        for s in samples.iter_mut() {
            *s *= factor;
        }
    }


    // Save WAV for history playback in parallel with inference. Handy uses the
    // same pattern: disk I/O must not delay the model from starting.
    let wav_fname = format!("rec_{}.wav", chrono::Utc::now().timestamp_millis());
    let wav_path  = recordings_dir().join(&wav_fname);
    let audio_path = wav_path.to_string_lossy().to_string();
    let samples_for_wav = samples.clone();
    let wav_path_for_save = wav_path.clone();
    let wav_save = tokio::task::spawn_blocking(move || {
        save_wav(&wav_path_for_save, &samples_for_wav, 16000)
    });

    let (text, source) = transcribe(&samples, &state, &wav_path, &app_handle, access_token).await?;
    let text = if source == "parakeet" {
        normalize_technical_transcript(&text)
    } else {
        text
    };

    if text.trim().is_empty() {
        return Err("No speech detected.".into());
    }

    // Filter common Whisper-Large-v3 hallucinations on silent or noisy audio
    let lower_text = text.trim().to_lowercase();
    let is_hallucination = lower_text == "you" 
        || lower_text == "you." 
        || lower_text == "thank you" 
        || lower_text == "thank you."
        || lower_text == "thanks"
        || lower_text == "thanks."
        || lower_text == "bye"
        || lower_text == "bye.";
        
    if is_hallucination {
        return Err("No speech detected (hallucination filtered).".into());
    }

    // Inject into active app
    let final_text = crate::injection::apply_dictionary(&text)?;
    crate::injection::inject_text(&final_text)?;

    let wc = final_text.split_whitespace().count() as i64;
    let complete = TranscriptionComplete {
        text: final_text.clone(),
        word_count: wc,
        duration_ms: recorded_duration_ms,
        source: source.clone(),
    };
    app_handle.emit("transcription-complete", final_text.clone()).ok();
    app_handle.emit("transcription-complete-detail", complete).ok();

    let final_text_for_history = final_text.clone();
    tauri::async_runtime::spawn(async move {
        let saved_audio_path = match wav_save.await {
            Ok(Ok(())) => Some(audio_path),
            Ok(Err(err)) => {
                eprintln!("[MeshVoice] Failed to save recording WAV: {err}");
                None
            }
            Err(err) => {
                eprintln!("[MeshVoice] WAV save task failed: {err}");
                None
            }
        };

        let _ = tokio::task::spawn_blocking(move || {
            crate::db::DB_CONN.lock().unwrap().execute(
                "INSERT INTO history (text, word_count, duration_ms, source, audio_path) VALUES (?,?,?,?,?)",
                rusqlite::params![final_text_for_history, wc, recorded_duration_ms, source, saved_audio_path],
            ).ok();
            crate::db::record_lifetime_stats(wc, recorded_duration_ms);
            crate::db::prune_history();
        }).await;
    });

    Ok(final_text)
}

/// Transcription strategy:
/// Honors the selected engine: local whisper.cpp or Groq cloud.
async fn transcribe(
    samples: &[f32],
    state: &tauri::State<'_, crate::AppState>,
    _wav_path: &std::path::Path,
    app_handle: &tauri::AppHandle,
    access_token: Option<String>,
) -> Result<(String, String), String> {
    let engine = {
        let conn = crate::db::DB_CONN.lock().unwrap();
        conn.query_row("SELECT value FROM settings WHERE key='engine'", [], |r| r.get::<_,String>(0))
            .unwrap_or_else(|_| "local".into())
    };

    if engine == "cloud" {
        if let Ok(key) = crate::read_provider_key(app_handle, "groq") {
            return crate::transcription::transcribe_via_groq(samples, &key).await
                .map(|text| (text, "cloud".into()));
        }
        if let Some(token) = access_token.as_deref().map(str::trim).filter(|token| !token.is_empty()) {
            return crate::transcription::transcribe_via_meshpilot_cloud(samples, token).await
                .map(|text| (text, "cloud".into()));
        }
        return Err("Cloud mode requires a Groq API key in Settings.".to_string());
    }

    // Read language setting
    let mut lang_flag = {
        let conn = crate::db::DB_CONN.lock().unwrap();
        let mode = conn.query_row("SELECT value FROM settings WHERE key='language_mode'", [], |r| r.get::<_,String>(0))
            .unwrap_or_else(|_| "auto".into());
        match mode.as_str() {
            "en" => "en".to_string(),
            "hi" | "hinglish" => "hi".to_string(),
            _ => "auto".to_string(),
        }
    };

    use tauri::Manager;

    let model_path = state.selected_model.lock().unwrap().clone();
    let Some(model) = model_path else {
        return Err("Local mode requires a downloaded model. Open Settings and download Parakeet V3 or a Whisper model.".into());
    };

    // A directory is the in-memory Parakeet bundle; a file is a whisper.cpp model.
    let is_parakeet = std::path::Path::new(&model).is_dir();

    // Fine-tuned Hinglish models expect Hindi and must skip auto-detection.
    if model.contains("hindi2hinglish") {
        lang_flag = "hi".to_string();
    }
    // Standard (non-fine-tuned) whisper benefits from a Hinglish priming prompt.
    let initial_prompt = if lang_flag == "hi" && !is_parakeet && !model.contains("hindi2hinglish") {
        Some("Haan bhai, this is a Hinglish sentence, theek hai?".to_string())
    } else {
        None
    };

    let source = if is_parakeet { "parakeet" } else { "local" };

    // Run against the persistent in-memory engine. The model is kept warm in a
    // cache, so consecutive dictations (and switching back to a previously used
    // model) skip the expensive reload entirely. `transcribe` lazy-loads if the
    // model isn't warm yet. Blocking inference is moved off the async runtime.
    let voice_engine = app_handle.state::<crate::engine::VoiceEngine>().inner().clone();
    let samples_vec = samples.to_vec();
    let lang = lang_flag.clone();
    let model_for_run = model.clone();
    let text = tokio::task::spawn_blocking(move || {
        voice_engine.transcribe(&model_for_run, samples_vec, &lang, initial_prompt)
    })
    .await
    .map_err(|e| format!("Transcription task failed: {e}"))??;

    Ok((text, source.to_string()))
}

fn normalize_transcript_text(text: &str) -> String {
    text
        .chars()
        .filter(|c| !c.is_control() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn normalize_technical_transcript(text: &str) -> String {
    let mut normalized = normalize_transcript_text(text);
    let replacements = [
        (r"(?i)\bstreaming ASN architecture\b", "streaming ASR architecture"),
        (r"(?i)\bAS and architecture\b", "ASR architecture"),
        (r"(?i)\bA S N architecture\b", "ASR architecture"),
        (r"(?i)\bShepa Unex\b", "Sherpa-ONNX"),
        (r"(?i)\bSherpa Unex\b", "Sherpa-ONNX"),
        (r"(?i)\bShepa ONNX\b", "Sherpa-ONNX"),
        (r"(?i)\bSherpa ONNX\b", "Sherpa-ONNX"),
        (r"(?i)\bShepa Onyx\b", "Sherpa-ONNX"),
        (r"(?i)\bSherpa Onyx\b", "Sherpa-ONNX"),
        (r"(?i)\bstreaming Zformer\b", "streaming Zipformer"),
        (r"(?i)\bZformer\b", "Zipformer"),
        (r"(?i)\bZip former\b", "Zipformer"),
        (r"(?i)\bTransduer\b", "Transducer"),
        (r"(?i)\bINT eight\b", "INT8"),
        (r"(?i)\bfull audio chart\b", "full audio chunks"),
        (r"(?i)\baudio chart\b", "audio chunks"),
    ];

    for (pattern, replacement) in replacements {
        if let Ok(regex) = regex::Regex::new(pattern) {
            normalized = regex.replace_all(&normalized, replacement).to_string();
        }
    }

    normalized
}

fn format_microphone_error(prefix: &str, error: impl std::fmt::Display) -> String {
    format!(
        "{}: {}. Enable microphone access for desktop apps in Windows Privacy settings, confirm the selected input device is active, then retry.",
        prefix,
        error
    )
}

pub(crate) fn resample_to_16k(input: &[f32], from: u32) -> Vec<f32> {
    if from == 16000 { return input.to_vec(); }
    let ratio = from as f64 / 16000.0;
    let out_len = (input.len() as f64 / ratio) as usize;
    (0..out_len).map(|i| {
        let src = i as f64 * ratio;
        let lo = src.floor() as usize;
        let hi = (lo + 1).min(input.len() - 1);
        input[lo] * (1.0 - (src - lo as f64) as f32) + input[hi] * (src - lo as f64) as f32
    }).collect()
}

fn trim_voice_activity(input: &[f32], rate: u32) -> Option<Vec<f32>> {
    if input.is_empty() {
        return None;
    }

    let frame = ((rate as usize * 30) / 1000).max(1);
    let preroll = ((rate as usize * PREROLL_MS) / 1000).max(frame);
    let min_speech_frames = (MIN_SPEECH_MS / 30).max(1);
    let trailing_silence_frames = (TRAILING_SILENCE_MS / 30).max(1);
    let frame_rms: Vec<f32> = input
        .chunks(frame)
        .map(|chunk| (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt())
        .collect();

    if frame_rms.is_empty() {
        return None;
    }

    let noise_sample_count = ((300 / 30).max(1) as usize).min(frame_rms.len());
    let mut noise = frame_rms[..noise_sample_count].to_vec();
    noise.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let noise_floor = noise[noise.len() / 2];
    let start_threshold = START_RMS_THRESHOLD.max(noise_floor * 2.2);
    let end_threshold = END_RMS_THRESHOLD.max(noise_floor * 1.5);

    let mut speech_start = None;
    let mut speech_end = None;
    let mut speech_frames = 0usize;
    for (idx, rms) in frame_rms.iter().copied().enumerate() {
        if speech_start.is_none() {
            if rms >= start_threshold {
                speech_frames += 1;
                if speech_frames >= min_speech_frames {
                    let start_frame = idx.saturating_sub(speech_frames - 1);
                    speech_start = Some(start_frame * frame);
                    speech_end = Some(((idx + 1) * frame).min(input.len()));
                }
            } else {
                speech_frames = 0;
            }
            continue;
        }

        if rms >= end_threshold {
            speech_end = Some(((idx + 1) * frame).min(input.len()));
        }
    }

    if speech_start.is_none() {
        let rms = (input.iter().map(|s| s * s).sum::<f32>() / input.len() as f32).sqrt();
        let peak = input.iter().fold(0.0_f32, |max, s| max.max(s.abs()));
        if input.len() >= rate as usize / 3 && rms >= 0.0025 && peak >= 0.012 {
            return Some(input.to_vec());
        }
    }

    let start = speech_start?.saturating_sub(preroll);
    let trailing = trailing_silence_frames * frame;
    let end = speech_end?.saturating_add(trailing).min(input.len());
    if end <= start {
        return None;
    }

    Some(input[start..end].to_vec())
}

pub(crate) fn save_wav(path: &std::path::Path, samples: &[f32], rate: u32) -> std::io::Result<()> {
    use std::io::Write;
    let ds = (samples.len() * 2) as u32;
    let mut f = std::fs::File::create(path)?;
    f.write_all(b"RIFF")?; f.write_all(&(36+ds).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?; f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?; f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&rate.to_le_bytes())?; f.write_all(&(rate*2).to_le_bytes())?;
    f.write_all(&2u16.to_le_bytes())?; f.write_all(&16u16.to_le_bytes())?;
    f.write_all(b"data")?; f.write_all(&ds.to_le_bytes())?;
    for &s in samples { f.write_all(&((s.clamp(-1.,1.)*32767.) as i16).to_le_bytes())?; }
    Ok(())
}

pub fn start_level_emitter(app: tauri::AppHandle, is_recording: Arc<Mutex<bool>>) {
    std::thread::spawn(move || {
        let mut was_recording = false;
        loop {
            let rec = *is_recording.lock().unwrap();
            if rec {
                was_recording = true;
                let rms = (*LAST_RMS.lock().unwrap()).max(0.02).clamp(0., 1.);
                let shape = [0.5f32, 0.7, 0.9, 1.0, 0.9, 0.7, 0.5];
                let levels: Vec<f32> = shape.iter().map(|&s| (rms * s).clamp(0., 1.)).collect();
                let _ = app.emit("audio-levels", levels);
                std::thread::sleep(std::time::Duration::from_millis(60));
            } else {
                if was_recording {
                    // Reset the visualizer levels exactly once when transitioning to idle.
                    let _ = app.emit("audio-levels", vec![0.0f32; 7]);
                    was_recording = false;
                }
                // Sleep for a longer duration when inactive to avoid lock contention
                // and WebView2 IPC flooding, ensuring a lightweight idle state.
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
        }
    });
}




#[cfg(test)]
mod tests {
    use super::{normalize_technical_transcript, trim_voice_activity};

    #[test]
    fn technical_normalizer_repairs_common_asr_terms() {
        assert_eq!(
            normalize_technical_transcript("Use streaming ASN architecture for real time transcription instead of reprocessing full audio chart."),
            "Use streaming ASR architecture for real time transcription instead of reprocessing full audio chunks."
        );
        assert_eq!(
            normalize_technical_transcript("You Shepa Unex Streaming Zformer."),
            "You Sherpa-ONNX streaming Zipformer."
        );
    }

    #[test]
    fn vad_keeps_speech_after_natural_pause() {
        let rate = 16_000;
        let mut samples = Vec::new();
        samples.extend(std::iter::repeat(0.0).take(rate / 5));
        samples.extend(std::iter::repeat(0.06).take(rate));
        samples.extend(std::iter::repeat(0.0).take(rate / 2));
        samples.extend(std::iter::repeat(0.06).take(rate));
        samples.extend(std::iter::repeat(0.0).take(rate / 5));

        let trimmed = trim_voice_activity(&samples, rate as u32).expect("speech should be detected");
        assert!(
            trimmed.len() >= rate * 2,
            "VAD must preserve speech after internal pauses; got {} samples",
            trimmed.len()
        );
    }
}
