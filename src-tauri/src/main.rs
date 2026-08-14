#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod audio;
mod transcription;
mod injection;
mod clipboard;
mod engine;

use std::sync::{Arc, Mutex};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
#[cfg(target_os = "windows")]
use std::process::Command;
use std::time::Duration;

use tauri::{
    Manager, Emitter, PhysicalPosition, AppHandle, State, LogicalSize, Size,
};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri_plugin_global_shortcut::{ShortcutState, GlobalShortcutExt, Shortcut};
use arboard::Clipboard;
use serde::{Deserialize, Serialize};

const MAX_HISTORY: usize = 10;
const WIDGET_BOTTOM_MARGIN_PX: u32 = 96;
const KOKORO_HELPER_URL: &str = "http://127.0.0.1:8765";

// ─── Shared State ────────────────────────────────────────────────────────────

pub struct AppState {
    pub is_recording:   Arc<Mutex<bool>>,
    pub recording_session_id: Arc<Mutex<u64>>,
    pub hotkey_down: Arc<Mutex<bool>>,
    pub current_hotkey: Arc<Mutex<String>>,
    pub rec_mode:       Arc<Mutex<String>>,
    /// Path to the selected downloaded model file.
    pub selected_model: Mutex<Option<String>>,
    pub language_mode: Mutex<String>,
}

#[derive(Default)]
struct RuntimeState {
    paused: Mutex<bool>,
    captured_len: Mutex<usize>,
    is_terminal: Mutex<bool>,
    captured_text: Mutex<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderSettings {
    provider: String,
    model: String,
    base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsState {
    provider: ProviderSettings,
    shortcut: String,
    close_to_tray: bool,
    run_in_tray: bool,
    launch_at_startup: bool,
    restore_clipboard: bool,
    history_enabled: bool,
    sensitive_mode: bool,
    timeout_ms: u64,
    max_output_tokens: u32,
    temperature: f32,
    default_action_id: String,
    paused: bool,
    #[serde(default = "default_enhance_prompt_mode")]
    enhance_prompt_mode: String,
    #[serde(default = "default_voice_engine")]
    voice_engine: String,
    #[serde(default = "default_voice_id")]
    voice_id: String,
    #[serde(default = "default_brain_mode")]
    brain_mode: String,
}

fn default_enhance_prompt_mode() -> String {
    "auto".to_string()
}

fn default_voice_engine() -> String {
    "kokoro".to_string()
}

fn default_voice_id() -> String {
    "kokoro-default".to_string()
}

fn default_brain_mode() -> String {
    "api".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HistoryItem {
    id: String,
    action_id: String,
    action_label: String,
    provider: String,
    model: String,
    input: String,
    output: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PromptAppStatePayload {
    settings: SettingsState,
    history: Vec<HistoryItem>,
    key_status: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MailboxPathsPayload {
    root: String,
    inbox: String,
    outbox: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MailboxJobPayload {
    id: String,
    inbox_dir: String,
    outbox_dir: String,
    screenshot_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DossierMeta {
    id: String,
    created_at: String,
    note: String,
    screenshot_path: String,
    screenshot_hash: String,
    source_title: Option<String>,
    analysis: Option<String>,
}

fn default_settings() -> SettingsState {
    SettingsState {
        provider: ProviderSettings {
            provider: "groq".to_string(),
            model: "llama-3.1-8b-instant".to_string(),
            base_url: None,
        },
        shortcut: "Ctrl+Shift+Space".to_string(),
        close_to_tray: true,
        run_in_tray: true,
        launch_at_startup: true,
        restore_clipboard: true,
        history_enabled: true,
        sensitive_mode: false,
        timeout_ms: 60_000,
        max_output_tokens: 1_800,
        temperature: 0.35,
        default_action_id: "enhance-prompt".to_string(),
        paused: false,
        enhance_prompt_mode: "auto".to_string(),
        voice_engine: default_voice_engine(),
        voice_id: default_voice_id(),
        brain_mode: default_brain_mode(),
    }
}

// ─── Path & Helpers ─────────────────────────────────────────────────────────

fn app_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_local_data_dir()
        .map_err(|err| format!("Failed to resolve app data directory: {err}"))?;
    fs::create_dir_all(&dir).map_err(|err| format!("Failed to create app data directory: {err}"))?;
    Ok(dir)
}

fn mailbox_root() -> Result<PathBuf, String> {
    let documents = dirs::document_dir()
        .or_else(dirs::home_dir)
        .ok_or_else(|| "Could not find the user's Documents folder.".to_string())?;
    Ok(documents.join("OleMailbox"))
}

fn dossier_root() -> Result<PathBuf, String> {
    let documents = dirs::document_dir()
        .or_else(dirs::home_dir)
        .ok_or_else(|| "Could not find the user's Documents folder.".to_string())?;
    Ok(documents.join("OleDossier"))
}

fn dossier_items_dir() -> Result<PathBuf, String> {
    let root = dossier_root()?;
    for table in ["items", "ole_containers", "source_artifacts", "ole_artifact_links"] {
        fs::create_dir_all(root.join(table)).map_err(|err| format!("Failed to create dossier table folder: {err}"))?;
    }
    let dir = root.join("items");
    fs::create_dir_all(&dir).map_err(|err| format!("Failed to create dossier folder: {err}"))?;
    Ok(dir)
}

fn new_dossier_item_id() -> String {
    format!(
        "ole-{}-{}",
        chrono::Utc::now().format("%Y%m%d%H%M%S%3f"),
        std::process::id()
    )
}

fn hash_bytes(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

fn read_dossier_meta(path: &std::path::Path) -> Result<DossierMeta, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("Failed to read dossier metadata: {err}"))?;
    serde_json::from_str(&raw).map_err(|err| format!("Failed to parse dossier metadata: {err}"))
}

fn write_dossier_meta(item_dir: &std::path::Path, meta: &DossierMeta) -> Result<(), String> {
    fs::write(
        item_dir.join("meta.json"),
        serde_json::to_string_pretty(meta).map_err(|err| format!("Failed to encode dossier metadata: {err}"))?,
    )
    .map_err(|err| format!("Failed to write dossier metadata: {err}"))
}

fn ensure_mailbox_dirs_at(root: &std::path::Path) -> Result<(PathBuf, PathBuf), String> {
    let inbox = root.join("inbox");
    let outbox = root.join("outbox");
    fs::create_dir_all(&inbox).map_err(|err| format!("Failed to create mailbox inbox: {err}"))?;
    fs::create_dir_all(&outbox).map_err(|err| format!("Failed to create mailbox outbox: {err}"))?;
    Ok((inbox, outbox))
}

fn new_mailbox_job_id() -> String {
    format!(
        "job-{}-{}",
        chrono::Utc::now().format("%Y%m%d%H%M%S%3f"),
        std::process::id()
    )
}

#[cfg_attr(not(test), allow(dead_code))]
fn write_mailbox_job_at(
    root: &std::path::Path,
    request: &str,
    screenshot: Option<&[u8]>,
) -> Result<MailboxJobPayload, String> {
    let (inbox_root, outbox_root) = ensure_mailbox_dirs_at(root)?;
    let id = new_mailbox_job_id();
    let inbox_dir = inbox_root.join(&id);
    let outbox_dir = outbox_root.join(&id);
    fs::create_dir_all(&inbox_dir).map_err(|err| format!("Failed to create mailbox job: {err}"))?;
    fs::create_dir_all(&outbox_dir).map_err(|err| format!("Failed to create mailbox reply folder: {err}"))?;
    fs::write(inbox_dir.join("request.txt"), request).map_err(|err| format!("Failed to write mailbox request: {err}"))?;

    let screenshot_path = if let Some(bytes) = screenshot {
        let path = inbox_dir.join("screenshot.png");
        fs::write(&path, bytes).map_err(|err| format!("Failed to write mailbox screenshot: {err}"))?;
        Some(path.to_string_lossy().to_string())
    } else {
        None
    };

    let meta = serde_json::json!({
        "id": id,
        "createdAt": chrono::Utc::now().to_rfc3339(),
        "hasScreenshot": screenshot_path.is_some(),
    });
    fs::write(
        inbox_dir.join("meta.json"),
        serde_json::to_string_pretty(&meta).map_err(|err| format!("Failed to encode mailbox metadata: {err}"))?,
    )
    .map_err(|err| format!("Failed to write mailbox metadata: {err}"))?;

    Ok(MailboxJobPayload {
        id,
        inbox_dir: inbox_dir.to_string_lossy().to_string(),
        outbox_dir: outbox_dir.to_string_lossy().to_string(),
        screenshot_path,
    })
}

fn read_mailbox_reply_at(root: &std::path::Path, job_id: &str) -> Result<Option<String>, String> {
    let (inbox_root, outbox_root) = ensure_mailbox_dirs_at(root)?;
    let safe_id = job_id.trim();
    if safe_id.is_empty() || safe_id.contains('/') || safe_id.contains('\\') {
        return Err("Invalid mailbox job id.".to_string());
    }

    let outbox_dir = outbox_root.join(safe_id);
    let reply_path = ["reply.txt", "reply.md"]
        .iter()
        .map(|name| outbox_dir.join(name))
        .find(|path| path.exists());

    let Some(reply_path) = reply_path else {
        return Ok(None);
    };

    let reply = fs::read_to_string(&reply_path).map_err(|err| format!("Failed to read mailbox reply: {err}"))?;
    let done = serde_json::json!({
        "id": safe_id,
        "replyPath": reply_path.to_string_lossy(),
        "completedAt": chrono::Utc::now().to_rfc3339(),
    });
    let inbox_dir = inbox_root.join(safe_id);
    fs::create_dir_all(&inbox_dir).map_err(|err| format!("Failed to mark mailbox job done: {err}"))?;
    fs::write(
        inbox_dir.join("done.json"),
        serde_json::to_string_pretty(&done).map_err(|err| format!("Failed to encode mailbox done marker: {err}"))?,
    )
    .map_err(|err| format!("Failed to write mailbox done marker: {err}"))?;
    Ok(Some(reply))
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_dir(app)?.join("settings.json"))
}

fn prompt_history_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_dir(app)?.join("history.json"))
}

fn keys_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_dir(app)?.join("keys");
    fs::create_dir_all(&dir).map_err(|err| format!("Failed to create key directory: {err}"))?;
    Ok(dir)
}

fn key_path(app: &AppHandle, provider: &str) -> Result<PathBuf, String> {
    let provider = provider.trim().to_lowercase();
    if !provider_ids().contains(&provider.as_str()) {
        return Err("Invalid provider id.".to_string());
    }
    Ok(keys_dir(app)?.join(format!("{provider}.bin")))
}

fn read_settings(app: &AppHandle) -> SettingsState {
    let Ok(path) = settings_path(app) else {
        return default_settings();
    };
    let Ok(raw) = fs::read_to_string(path) else {
        return default_settings();
    };
    serde_json::from_str(&raw).unwrap_or_else(|_| default_settings())
}

fn write_settings(app: &AppHandle, settings: &SettingsState) -> Result<(), String> {
    let path = settings_path(app)?;
    let raw = serde_json::to_string_pretty(settings).map_err(|err| format!("Failed to encode settings: {err}"))?;
    fs::write(path, raw).map_err(|err| format!("Failed to write settings: {err}"))
}

fn read_prompt_history(app: &AppHandle) -> Vec<HistoryItem> {
    let Ok(path) = prompt_history_path(app) else {
        return Vec::new();
    };
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn write_prompt_history(app: &AppHandle, history: &[HistoryItem]) -> Result<(), String> {
    let path = prompt_history_path(app)?;
    let raw = serde_json::to_string_pretty(history).map_err(|err| format!("Failed to encode history: {err}"))?;
    fs::write(path, raw).map_err(|err| format!("Failed to write history: {err}"))
}

pub(crate) fn read_provider_key(app: &AppHandle, provider: &str) -> Result<String, String> {
    let encrypted = fs::read(key_path(app, provider)?).map_err(|err| format!("API key is not configured: {err}"))?;
    let decrypted = decrypt_secret(&encrypted)?;
    String::from_utf8(decrypted).map_err(|err| format!("Stored API key is invalid UTF-8: {err}"))
}

#[cfg(target_os = "windows")]
fn encrypt_secret(bytes: &[u8]) -> Result<Vec<u8>, String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB};

    let mut input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptProtectData(&mut input, PCWSTR::null(), None, None, None, 0, &mut output)
            .map_err(|err| format!("Failed to encrypt API key with Windows DPAPI: {err}"))?;
        let encrypted = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(HLOCAL(output.pbData.cast()));
        Ok(encrypted)
    }
}

#[cfg(target_os = "windows")]
fn decrypt_secret(bytes: &[u8]) -> Result<Vec<u8>, String> {
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};

    let mut input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptUnprotectData(&mut input, None, None, None, None, 0, &mut output)
            .map_err(|err| format!("Failed to decrypt API key with Windows DPAPI: {err}"))?;
        let decrypted = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(HLOCAL(output.pbData.cast()));
        Ok(decrypted)
    }
}

#[cfg(not(target_os = "windows"))]
fn encrypt_secret(bytes: &[u8]) -> Result<Vec<u8>, String> {
    Ok(bytes.to_vec())
}

#[cfg(not(target_os = "windows"))]
fn decrypt_secret(bytes: &[u8]) -> Result<Vec<u8>, String> {
    Ok(bytes.to_vec())
}

fn provider_ids() -> [&'static str; 8] {
    ["xai", "openai", "anthropic", "gemini", "openrouter", "ollama", "groq", "elevenlabs"]
}

fn build_key_status(app: &AppHandle) -> BTreeMap<String, bool> {
    provider_ids()
        .iter()
        .map(|provider| {
            let exists = read_provider_key(app, provider).map(|key| !key.trim().is_empty()).unwrap_or(false);
            ((*provider).to_string(), exists)
        })
        .collect()
}

// ─── Tauri Commands (MeshPrompt) ─────────────────────────────────────────────

#[tauri::command]
fn get_app_state(app: AppHandle) -> PromptAppStatePayload {
    PromptAppStatePayload {
        settings: read_settings(&app),
        history: read_prompt_history(&app),
        key_status: build_key_status(&app),
    }
}

#[tauri::command]
fn save_settings(app: AppHandle, state: State<RuntimeState>, settings: SettingsState) -> Result<(), String> {
    let old_settings = read_settings(&app);
    set_launch_at_startup(&app, settings.launch_at_startup)?;
    if let Ok(mut paused) = state.paused.lock() {
        *paused = settings.paused;
    }
    
    // Live update shortcut registration if changed
    if old_settings.shortcut != settings.shortcut {
        if let Ok(old_sc) = normalize_shortcut(&old_settings.shortcut).parse::<tauri_plugin_global_shortcut::Shortcut>() {
            let _ = app.global_shortcut().unregister(old_sc);
        }
        let _ = register_global_shortcut(&app);
    }
    
    write_settings(&app, &settings)
}

#[tauri::command]
fn save_provider_key(app: AppHandle, provider: String, api_key: String) -> Result<(), String> {
    let provider = provider.trim().to_lowercase();
    let api_key = api_key.trim().to_string();
    if provider.is_empty() {
        return Err("Provider is required.".to_string());
    }
    if api_key.is_empty() {
        return Err("API key is required.".to_string());
    }
    let encrypted = encrypt_secret(api_key.as_bytes())?;
    fs::write(key_path(&app, &provider)?, encrypted).map_err(|err| format!("Failed to save API key: {err}"))
}

#[tauri::command]
fn get_provider_key(app: AppHandle, provider: String) -> Result<Option<String>, String> {
    Ok(read_provider_key(&app, provider.trim()).ok())
}

#[tauri::command]
fn delete_provider_key(app: AppHandle, provider: String) -> Result<(), String> {
    match fs::remove_file(key_path(&app, provider.trim())?) {
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("Failed to delete API key: {err}")),
    }
}

#[tauri::command]
fn add_history(app: AppHandle, item: HistoryItem) -> Result<(), String> {
    let settings = read_settings(&app);
    if !settings.history_enabled || settings.sensitive_mode {
        return Ok(());
    }
    let mut history = read_prompt_history(&app);
    history.retain(|entry| entry.id != item.id);
    history.insert(0, item);
    history.truncate(MAX_HISTORY);
    write_prompt_history(&app, &history)
}

#[tauri::command]
fn clear_history(app: AppHandle) -> Result<(), String> {
    write_prompt_history(&app, &[])
}

#[tauri::command]
fn capture_selected_text(app: AppHandle) -> Result<String, String> {
    let settings = read_settings(&app);
    let captured = capture_selection(&settings)?;
    let state = app.state::<RuntimeState>();
    if let Ok(mut len) = state.captured_len.lock() {
        *len = captured.chars().count();
    }
    if let Ok(mut term) = state.is_terminal.lock() {
        *term = is_terminal_foreground();
    }
    if let Ok(mut cap_text) = state.captured_text.lock() {
        *cap_text = captured.clone();
    }
    Ok(captured)
}

#[tauri::command]
fn get_captured_text(state: State<RuntimeState>) -> String {
    if let Ok(mut cap_text) = state.captured_text.lock() {
        let val = cap_text.clone();
        *cap_text = String::new();
        val
    } else {
        String::new()
    }
}

#[tauri::command]
fn copy_text(text: String) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|err| format!("Clipboard unavailable: {err}"))?;
    clipboard
        .set_text(text)
        .map_err(|err| format!("Failed to copy text: {err}"))
}

#[tauri::command]
fn replace_selected_text(app: AppHandle, text: String) -> Result<(), String> {
    let settings = read_settings(&app);
    let state = app.state::<RuntimeState>();
    let is_term = state.is_terminal.lock().map(|t| *t).unwrap_or(false);
    let cap_len = state.captured_len.lock().map(|l| *l).unwrap_or(0);
    paste_text(&text, settings.restore_clipboard, is_term, cap_len)
}

#[tauri::command]
fn show_overlay(app: AppHandle) -> Result<(), String> {
    show_overlay_window(&app)
}

#[tauri::command]
fn hide_overlay(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("overlay") {
        window.hide().map_err(|err| format!("Failed to hide overlay: {err}"))?;
    }
    Ok(())
}

#[tauri::command]
fn set_widget_click_through(app: AppHandle, enabled: bool) -> Result<(), String> {
    let widget = app
        .get_webview_window("widget")
        .ok_or_else(|| "Widget window is unavailable.".to_string())?;
    widget
        .set_ignore_cursor_events(enabled)
        .map_err(|err| format!("Failed to update widget click-through: {err}"))
}

#[tauri::command]
fn open_widget_chat(app: AppHandle) -> Result<(), String> {
    let widget = app
        .get_webview_window("widget")
        .ok_or_else(|| "Widget window is unavailable.".to_string())?;
    widget
        .set_ignore_cursor_events(false)
        .map_err(|err| format!("Failed to enable widget clicks: {err}"))?;
    show_widget_window(&app)?;
    app.emit("ole://open-chat", ()).map_err(|err| format!("Failed to open chat: {err}"))
}

#[tauri::command]
fn resize_overlay(app: AppHandle, width: f64, height: f64) -> Result<(), String> {
    let window = app
        .get_webview_window("overlay")
        .ok_or_else(|| "Overlay window is unavailable.".to_string())?;
    
    window
        .set_size(Size::Logical(LogicalSize::new(width, height)))
        .map_err(|err| format!("Failed to size overlay: {err}"))?;

    if let Ok(Some(monitor)) = window.primary_monitor() {
        let scale_factor = monitor.scale_factor();
        let area = monitor.work_area();
        
        let work_area_x = area.position.x as f64 / scale_factor;
        let work_area_y = area.position.y as f64 / scale_factor;
        let work_area_width = area.size.width as f64 / scale_factor;
        let work_area_height = area.size.height as f64 / scale_factor;

        let x = work_area_x + (work_area_width - width) / 2.0;
        let y = work_area_y + work_area_height - height - 80.0;

        let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)));
    }
    Ok(())
}

#[tauri::command]
async fn proxy_request(
    url: String,
    method: String,
    headers: std::collections::HashMap<String, String>,
    body: String,
) -> Result<(u16, String), String> {
    if !is_allowed_provider_url(&url) {
        return Err("Requests are limited to configured AI providers and local Ollama.".to_string());
    }

    let client = reqwest::Client::new();
    let mut req = match method.to_uppercase().as_str() {
        "POST" => client.post(&url),
        "GET" => client.get(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        _ => return Err(format!("Unsupported method: {}", method)),
    };

    for (k, v) in headers {
        req = req.header(k, v);
    }

    if !body.is_empty() {
        req = req.body(body);
    }

    let res = req.send().await.map_err(|e| e.to_string())?;
    let status = res.status().as_u16();
    let text = res.text().await.map_err(|e| e.to_string())?;

    Ok((status, text))
}

fn is_allowed_provider_url(url: &str) -> bool {
    const ALLOWED_PREFIXES: [&str; 8] = [
        "https://api.openai.com/",
        "https://api.anthropic.com/",
        "https://generativelanguage.googleapis.com/",
        "https://api.groq.com/",
        "https://api.x.ai/",
        "https://openrouter.ai/",
        "http://localhost:",
        "http://127.0.0.1:",
    ];
    ALLOWED_PREFIXES.iter().any(|prefix| url.starts_with(prefix))
}

#[tauri::command]
fn get_mailbox_paths() -> Result<MailboxPathsPayload, String> {
    let root = mailbox_root()?;
    let (inbox, outbox) = ensure_mailbox_dirs_at(&root)?;
    Ok(MailboxPathsPayload {
        root: root.to_string_lossy().to_string(),
        inbox: inbox.to_string_lossy().to_string(),
        outbox: outbox.to_string_lossy().to_string(),
    })
}

#[tauri::command]
fn create_mailbox_job(request: String, include_screenshot: bool) -> Result<MailboxJobPayload, String> {
    let root = mailbox_root()?;
    let id = new_mailbox_job_id();
    let (inbox_root, outbox_root) = ensure_mailbox_dirs_at(&root)?;
    let inbox_dir = inbox_root.join(&id);
    let outbox_dir = outbox_root.join(&id);
    fs::create_dir_all(&inbox_dir).map_err(|err| format!("Failed to create mailbox job: {err}"))?;
    fs::create_dir_all(&outbox_dir).map_err(|err| format!("Failed to create mailbox reply folder: {err}"))?;

    let screenshot_path = if include_screenshot {
        let path = inbox_dir.join("screenshot.png");
        capture_screenshot_to_file(&path)?;
        Some(path.to_string_lossy().to_string())
    } else {
        None
    };

    fs::write(inbox_dir.join("request.txt"), request.trim())
        .map_err(|err| format!("Failed to write mailbox request: {err}"))?;
    let meta = serde_json::json!({
        "id": id,
        "createdAt": chrono::Utc::now().to_rfc3339(),
        "hasScreenshot": screenshot_path.is_some(),
    });
    fs::write(
        inbox_dir.join("meta.json"),
        serde_json::to_string_pretty(&meta).map_err(|err| format!("Failed to encode mailbox metadata: {err}"))?,
    )
    .map_err(|err| format!("Failed to write mailbox metadata: {err}"))?;

    Ok(MailboxJobPayload {
        id,
        inbox_dir: inbox_dir.to_string_lossy().to_string(),
        outbox_dir: outbox_dir.to_string_lossy().to_string(),
        screenshot_path,
    })
}

#[tauri::command]
fn read_mailbox_reply(job_id: String) -> Result<Option<String>, String> {
    read_mailbox_reply_at(&mailbox_root()?, &job_id)
}

#[tauri::command]
fn create_dossier_item(note: String) -> Result<DossierMeta, String> {
    let id = new_dossier_item_id();
    let item_dir = dossier_items_dir()?.join(&id);
    fs::create_dir_all(&item_dir).map_err(|err| format!("Failed to create dossier item: {err}"))?;
    let screenshot_path = item_dir.join("screenshot.png");
    capture_screenshot_to_file(&screenshot_path)?;
    let screenshot_bytes = fs::read(&screenshot_path).map_err(|err| format!("Failed to read dossier screenshot: {err}"))?;
    let source_title = foreground_window_title();
    let captured_at = chrono::Utc::now().to_rfc3339();
    let screenshot_hash = hash_bytes(&screenshot_bytes);
    let meta = DossierMeta {
        id: id.clone(),
        created_at: captured_at.clone(),
        note,
        screenshot_path: screenshot_path.to_string_lossy().to_string(),
        screenshot_hash: screenshot_hash.clone(),
        source_title: source_title.clone(),
        analysis: None,
    };
    write_dossier_meta(&item_dir, &meta)?;
    let metadata = serde_json::json!({
        "capture_type": "screenshot",
        "referring_app": meta.source_title.clone().unwrap_or_else(|| "UNVERIFIED".to_string()),
    });
    write_android_compatible_dossier_files(&id, &meta, &captured_at, &screenshot_hash, metadata)?;
    Ok(meta)
}

fn write_android_compatible_dossier_files(
    id: &str,
    meta: &DossierMeta,
    captured_at: &str,
    screenshot_hash: &str,
    metadata: serde_json::Value,
) -> Result<(), String> {
    let root = dossier_root()?;
    fs::create_dir_all(root.join("ole_containers")).map_err(|err| format!("Failed to create ole_containers: {err}"))?;
    fs::create_dir_all(root.join("source_artifacts")).map_err(|err| format!("Failed to create source_artifacts: {err}"))?;
    fs::create_dir_all(root.join("ole_artifact_links")).map_err(|err| format!("Failed to create ole_artifact_links: {err}"))?;
    let container = serde_json::json!({
        "id": id,
        "title": meta.note,
        "createdAt": captured_at,
    });
    let artifact = serde_json::json!({
        "id": id,
        "filePath": meta.screenshot_path,
        "sha256": screenshot_hash,
        "capturedAt": captured_at,
        "metadataJson": metadata.to_string(),
    });
    let link = serde_json::json!({
        "containerId": id,
        "artifactId": id,
        "linkedAt": captured_at,
    });
    fs::write(root.join("ole_containers").join(format!("{id}.json")), serde_json::to_string_pretty(&container).unwrap())
        .map_err(|err| format!("Failed to write ole_containers row: {err}"))?;
    fs::write(root.join("source_artifacts").join(format!("{id}.json")), serde_json::to_string_pretty(&artifact).unwrap())
        .map_err(|err| format!("Failed to write source_artifacts row: {err}"))?;
    fs::write(root.join("ole_artifact_links").join(format!("{id}.json")), serde_json::to_string_pretty(&link).unwrap())
        .map_err(|err| format!("Failed to write ole_artifact_links row: {err}"))
}

#[tauri::command]
fn list_dossier_items() -> Result<Vec<DossierMeta>, String> {
    let dir = dossier_items_dir()?;
    let mut items = Vec::new();
    for entry in fs::read_dir(dir).map_err(|err| format!("Failed to read dossier folder: {err}"))? {
        let entry = entry.map_err(|err| format!("Failed to read dossier item: {err}"))?;
        let meta_path = entry.path().join("meta.json");
        if meta_path.exists() {
            if let Ok(meta) = read_dossier_meta(&meta_path) {
                items.push(meta);
            }
        }
    }
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(items)
}

#[tauri::command]
fn attach_dossier_analysis(id: String, analysis: String) -> Result<(), String> {
    let safe_id = id.trim();
    if safe_id.is_empty() || safe_id.contains('/') || safe_id.contains('\\') {
        return Err("Invalid dossier item id.".to_string());
    }
    let item_dir = dossier_items_dir()?.join(safe_id);
    let meta_path = item_dir.join("meta.json");
    let mut meta = read_dossier_meta(&meta_path)?;
    meta.analysis = Some(analysis);
    write_dossier_meta(&item_dir, &meta)
}

#[tauri::command]
fn read_dossier_screenshot_base64(id: String) -> Result<String, String> {
    let safe_id = id.trim();
    if safe_id.is_empty() || safe_id.contains('/') || safe_id.contains('\\') {
        return Err("Invalid dossier item id.".to_string());
    }
    let meta = read_dossier_meta(&dossier_items_dir()?.join(safe_id).join("meta.json"))?;
    let bytes = fs::read(meta.screenshot_path).map_err(|err| format!("Failed to read dossier screenshot: {err}"))?;
    use base64::Engine;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

#[tauri::command]
fn import_dossier_file(file_name: String, mime: String, data_base64: String) -> Result<DossierMeta, String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_base64)
        .map_err(|err| format!("Failed to decode dropped file: {err}"))?;
    let id = new_dossier_item_id();
    let item_dir = dossier_items_dir()?.join(&id);
    fs::create_dir_all(&item_dir).map_err(|err| format!("Failed to create dossier item: {err}"))?;
    let safe_name = file_name
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') { ch } else { '_' })
        .collect::<String>();
    let stored_path = item_dir.join(if safe_name.trim().is_empty() { "drop.bin" } else { safe_name.trim() });
    fs::write(&stored_path, &bytes).map_err(|err| format!("Failed to write dropped file: {err}"))?;
    let captured_at = chrono::Utc::now().to_rfc3339();
    let sha256 = hash_bytes(&bytes);
    let meta = DossierMeta {
        id: id.clone(),
        created_at: captured_at.clone(),
        note: "Send to Olé".to_string(),
        screenshot_path: stored_path.to_string_lossy().to_string(),
        screenshot_hash: sha256.clone(),
        source_title: None,
        analysis: None,
    };
    write_dossier_meta(&item_dir, &meta)?;
    let metadata = serde_json::json!({
        "capture_type": "share",
        "share_mime": mime,
        "share_extra_text": "",
        "referring_app": "UNVERIFIED",
        "source_uri": "",
        "received_at": captured_at,
    });
    write_android_compatible_dossier_files(&id, &meta, &captured_at, &sha256, metadata)?;
    Ok(meta)
}

fn capture_screenshot_to_file(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let path_arg = path.to_string_lossy().replace('\'', "''");
        let script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; Add-Type -AssemblyName System.Drawing; $b=[System.Windows.Forms.Screen]::PrimaryScreen.Bounds; $bmp=New-Object System.Drawing.Bitmap $b.Width,$b.Height; $g=[System.Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($b.Location,[System.Drawing.Point]::Empty,$b.Size); $bmp.Save('{path_arg}',[System.Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose()"
        );
        let status = Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
            .status()
            .map_err(|err| format!("Failed to start screenshot capture: {err}"))?;
        if status.success() && path.exists() {
            Ok(())
        } else {
            Err("Could not capture the screen.".to_string())
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        Err("Screenshot capture is available in the Windows app.".to_string())
    }
}

fn foreground_window_title() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let script = "$sig='[DllImport(\"user32.dll\")] public static extern IntPtr GetForegroundWindow(); [DllImport(\"user32.dll\", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, System.Text.StringBuilder text, int count);'; Add-Type -MemberDefinition $sig -Name Native -Namespace Win32; $b=New-Object System.Text.StringBuilder 512; $h=[Win32.Native]::GetForegroundWindow(); [void][Win32.Native]::GetWindowText($h,$b,$b.Capacity); $b.ToString()";
        let output = Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let title = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if title.is_empty() { None } else { Some(title) }
    }

    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestVoiceRequest {
    engine: String,
    voice_id: String,
    text: String,
}

#[tauri::command]
async fn test_voice(app: AppHandle, request: TestVoiceRequest) -> Result<String, String> {
    let engine = request.engine.trim().to_lowercase();
    let text = if request.text.trim().is_empty() {
        "Hello, I am ready.".to_string()
    } else {
        request.text.trim().to_string()
    };

    match engine.as_str() {
        "kokoro" => synthesize_kokoro_tts(&app, &request.voice_id, &text).await,
        "edge" => speak_with_local_windows_voice(&text).await,
        "openai" => {
            let key = read_provider_key(&app, "openai")?;
            synthesize_openai_tts(&key, &request.voice_id, &text).await
        }
        "elevenlabs" => {
            let key = read_provider_key(&app, "elevenlabs")?;
            synthesize_elevenlabs_tts(&key, &request.voice_id, &text).await
        }
        _ => Err("Choose Local (Kokoro), ChatGPT / OpenAI, or ElevenLabs.".to_string()),
    }
}

async fn synthesize_kokoro_tts(app: &AppHandle, voice_id: &str, text: &str) -> Result<String, String> {
    let helper_result = match ensure_kokoro_helper_started(app).await {
        Ok(()) => request_kokoro_helper_speech(voice_id, text).await,
        Err(err) => Err(err),
    };

    match helper_result {
        Ok(bytes) => play_audio_bytes("ole-kokoro.wav", &bytes).await.map(|_| "Kokoro voice test played.".to_string()),
        Err(err) => {
            let fallback = speak_with_local_windows_voice(text).await?;
            Ok(format!("Kokoro helper was not ready ({err}). {fallback}"))
        }
    }
}

async fn request_kokoro_helper_speech(voice_id: &str, text: &str) -> Result<Vec<u8>, String> {
    let body = serde_json::json!({
        "model": "kokoro",
        "voice": voice_id,
        "input": text,
        "format": "wav"
    });
    let bytes = reqwest::Client::new()
        .post(format!("{KOKORO_HELPER_URL}/v1/audio/speech"))
        .json(&body)
        .send()
        .await
        .map_err(|err| format!("Kokoro helper request failed: {err}"))?
        .error_for_status()
        .map_err(|err| format!("Kokoro helper returned an error: {err}"))?
        .bytes()
        .await
        .map_err(|err| format!("Kokoro helper audio failed: {err}"))?;
    Ok(bytes.to_vec())
}

async fn ensure_kokoro_helper_started(app: &AppHandle) -> Result<(), String> {
    if kokoro_helper_healthy().await {
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        let helper_dir = app_dir(app)?.join("kokoro-helper");
        fs::create_dir_all(&helper_dir).map_err(|err| format!("Failed to create Kokoro helper folder: {err}"))?;
        fs::write(helper_dir.join("kokoro_server.py"), KOKORO_SERVER_PY)
            .map_err(|err| format!("Failed to write Kokoro server: {err}"))?;
        fs::write(helper_dir.join("start-kokoro.ps1"), KOKORO_START_PS1)
            .map_err(|err| format!("Failed to write Kokoro starter: {err}"))?;

        let script = helper_dir.join("start-kokoro.ps1");
        let helper_arg = helper_dir.to_string_lossy().to_string();
        let script_arg = script.to_string_lossy().to_string();
        let mut command = Command::new("powershell");
        command.args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &script_arg,
            "-Root",
            &helper_arg,
        ]);
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        command.spawn().map_err(|err| format!("Failed to start Kokoro helper: {err}"))?;

        for _ in 0..90 {
            if kokoro_helper_healthy().await {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        Err("Kokoro helper is still installing. Try Test voice again in a minute.".to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        Err("Kokoro helper starts from the Windows app.".to_string())
    }
}

async fn kokoro_helper_healthy() -> bool {
    let Ok(client) = reqwest::Client::builder().timeout(Duration::from_millis(700)).build() else {
        return false;
    };
    client
        .get(format!("{KOKORO_HELPER_URL}/health"))
        .send()
        .await
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
const KOKORO_START_PS1: &str = r#"
param([string]$Root)
$ErrorActionPreference = "Stop"

New-Item -ItemType Directory -Force -Path $Root | Out-Null
$UvDir = Join-Path $Root "uv"
$UvExe = Join-Path $UvDir "uv.exe"
$Venv = Join-Path $Root ".venv"
$Server = Join-Path $Root "kokoro_server.py"
$Model = Join-Path $Root "kokoro-v1.0.int8.onnx"
$Voices = Join-Path $Root "voices-v1.0.bin"

if (!(Test-Path $UvExe)) {
  New-Item -ItemType Directory -Force -Path $UvDir | Out-Null
  $Zip = Join-Path $Root "uv.zip"
  Invoke-WebRequest -Uri "https://github.com/astral-sh/uv/releases/latest/download/uv-x86_64-pc-windows-msvc.zip" -OutFile $Zip
  Expand-Archive -Path $Zip -DestinationPath $UvDir -Force
  $Found = Get-ChildItem -Path $UvDir -Recurse -Filter "uv.exe" | Select-Object -First 1
  if ($null -eq $Found) { throw "uv.exe was not found after download." }
  Copy-Item $Found.FullName $UvExe -Force
}

if (!(Test-Path $Model)) {
  Invoke-WebRequest -Uri "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/kokoro-v1.0.int8.onnx" -OutFile $Model
}
if (!(Test-Path $Voices)) {
  Invoke-WebRequest -Uri "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/voices-v1.0.bin" -OutFile $Voices
}

if (!(Test-Path (Join-Path $Venv "Scripts\python.exe"))) {
  & $UvExe venv $Venv --python 3.12 --seed
}
& $UvExe pip install --python (Join-Path $Venv "Scripts\python.exe") --upgrade kokoro-onnx soundfile
& (Join-Path $Venv "Scripts\python.exe") $Server --root $Root --port 8765
"#;

#[cfg(target_os = "windows")]
const KOKORO_SERVER_PY: &str = r#"
import argparse
import io
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import soundfile as sf
from kokoro_onnx import Kokoro

VOICE_MAP = {
    "kokoro-default": "af_sarah",
    "kokoro-warm": "af_heart",
    "kokoro-clear": "am_adam",
}

parser = argparse.ArgumentParser()
parser.add_argument("--root", required=True)
parser.add_argument("--port", type=int, default=8765)
args = parser.parse_args()

root = Path(args.root)
kokoro = Kokoro(str(root / "kokoro-v1.0.int8.onnx"), str(root / "voices-v1.0.bin"))

class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_):
        return

    def do_GET(self):
        if self.path == "/health":
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b"ok")
            return
        self.send_response(404)
        self.end_headers()

    def do_POST(self):
        if self.path != "/v1/audio/speech":
            self.send_response(404)
            self.end_headers()
            return
        length = int(self.headers.get("content-length", "0"))
        data = json.loads(self.rfile.read(length).decode("utf-8") or "{}")
        text = (data.get("input") or "Hello from Kokoro.").strip()
        voice = VOICE_MAP.get(data.get("voice"), "af_sarah")
        samples, sample_rate = kokoro.create(text, voice=voice, speed=1.0, lang="en-us")
        audio = io.BytesIO()
        sf.write(audio, samples, sample_rate, format="WAV")
        payload = audio.getvalue()
        self.send_response(200)
        self.send_header("content-type", "audio/wav")
        self.send_header("content-length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

ThreadingHTTPServer(("127.0.0.1", args.port), Handler).serve_forever()
"#;

async fn speak_with_local_windows_voice(text: &str) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let script = format!(
            "Add-Type -AssemblyName System.Speech; $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; $s.Speak({:?}); $s.Dispose()",
            text
        );
        let status = Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
            .status()
            .map_err(|err| format!("Failed to start local voice: {err}"))?;
        if status.success() {
            Ok("Played with this PC's local voice.".to_string())
        } else {
            Err("Local voice test failed on this PC.".to_string())
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = text;
        Ok("Local voice test is available in the Windows app.".to_string())
    }
}

async fn synthesize_openai_tts(api_key: &str, voice_id: &str, text: &str) -> Result<String, String> {
    let voice = if voice_id.trim().is_empty() || voice_id == "openai-alloy" {
        "alloy"
    } else {
        voice_id.trim().strip_prefix("openai-").unwrap_or(voice_id.trim())
    };
    let body = serde_json::json!({
        "model": "gpt-4o-mini-tts",
        "voice": voice,
        "input": text,
        "format": "mp3"
    });
    let bytes = reqwest::Client::new()
        .post("https://api.openai.com/v1/audio/speech")
        .bearer_auth(api_key.trim())
        .json(&body)
        .send()
        .await
        .map_err(|err| format!("OpenAI voice request failed: {err}"))?
        .error_for_status()
        .map_err(|err| format!("OpenAI voice request failed: {err}"))?
        .bytes()
        .await
        .map_err(|err| format!("OpenAI voice response failed: {err}"))?;
    play_audio_bytes("ole-openai-tts.mp3", &bytes).await
}

async fn synthesize_elevenlabs_tts(api_key: &str, voice_id: &str, text: &str) -> Result<String, String> {
    let voice = if voice_id.trim().is_empty() || voice_id == "eleven-rachel" {
        "21m00Tcm4TlvDq8ikWAM"
    } else {
        voice_id.trim().strip_prefix("eleven-").unwrap_or(voice_id.trim())
    };
    let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{voice}");
    let body = serde_json::json!({
        "text": text,
        "model_id": "eleven_multilingual_v2"
    });
    let bytes = reqwest::Client::new()
        .post(url)
        .header("xi-api-key", api_key.trim())
        .json(&body)
        .send()
        .await
        .map_err(|err| format!("ElevenLabs voice request failed: {err}"))?
        .error_for_status()
        .map_err(|err| format!("ElevenLabs voice request failed: {err}"))?
        .bytes()
        .await
        .map_err(|err| format!("ElevenLabs voice response failed: {err}"))?;
    play_audio_bytes("ole-elevenlabs-tts.mp3", &bytes).await
}

async fn play_audio_bytes(filename: &str, bytes: &[u8]) -> Result<String, String> {
    let path = std::env::temp_dir().join(filename);
    fs::write(&path, bytes).map_err(|err| format!("Failed to save voice test audio: {err}"))?;

    #[cfg(target_os = "windows")]
    {
        let path_string = path.to_string_lossy().replace('\'', "''");
        let script = format!(
            "Add-Type -AssemblyName presentationCore; $p = New-Object System.Windows.Media.MediaPlayer; $p.Open([Uri]'{path_string}'); $p.Play(); Start-Sleep -Seconds 5; $p.Close()"
        );
        let status = Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
            .status()
            .map_err(|err| format!("Failed to play voice test audio: {err}"))?;
        if status.success() {
            Ok("Voice test played.".to_string())
        } else {
            Err("Voice test audio was created but could not be played.".to_string())
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(format!("Voice test audio was created at {}.", path.display()))
    }
}

#[tauri::command]
fn set_paused(state: State<RuntimeState>, paused: bool) -> Result<(), String> {
    let mut value = state.paused.lock().map_err(|_| "Shortcut state unavailable.".to_string())?;
    *value = paused;
    Ok(())
}

// ─── Input Automation ────────────────────────────────────────────────────────

fn capture_selection(settings: &SettingsState) -> Result<String, String> {
    crate::clipboard::GLOBAL_CLIPBOARD.capture_selection_transaction(settings.restore_clipboard)
}

fn paste_text(text: &str, restore_clipboard: bool, is_term: bool, cap_len: usize) -> Result<(), String> {
    crate::clipboard::GLOBAL_CLIPBOARD.paste_text_transaction(text, restore_clipboard, is_term, cap_len)
}

#[cfg(target_os = "windows")]
fn is_terminal_foreground() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow};
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return false;
        }
        let mut class_name = [0u16; 256];
        let len = GetClassNameW(hwnd, &mut class_name);
        if len > 0 {
            let name = String::from_utf16_lossy(&class_name[..len as usize]).to_lowercase();
            name.contains("console") || name.contains("terminal") || name.contains("cascadia") || name.contains("mintty")
        } else {
            false
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn is_terminal_foreground() -> bool {
    false
}

// ─── Windows & Autostart Management ──────────────────────────────────────────

#[tauri::command]
async fn show_main_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        w.show().map_err(|e| e.to_string())?;
        w.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}


fn launched_from_autostart() -> bool {
    std::env::args().any(|arg| arg == "--autostart")
}

#[cfg(target_os = "windows")]
fn ensure_autostart_enabled() {
    let exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(_) => return,
    };
    let value = format!("\"{}\" --autostart", exe.display());
    let mut command = std::process::Command::new("reg");
    command.args([
        "add",
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
        "/v",
        "Olé",
        "/t",
        "REG_SZ",
        "/d",
        &value,
        "/f",
    ]);
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let _ = command.output();
}

#[cfg(not(target_os = "windows"))]
fn ensure_autostart_enabled() {}

fn set_launch_at_startup(_app: &AppHandle, enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        use windows::Win32::System::Threading::CREATE_NO_WINDOW;

        let exe = std::env::current_exe().map_err(|err| format!("Failed to resolve executable path: {err}"))?;
        let value = format!("\"{}\" --autostart", exe.display());
        let mut command = Command::new("reg");
        if enabled {
            command.args([
                "add",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "Olé",
                "/t",
                "REG_SZ",
                "/d",
                &value,
                "/f",
            ]);
        } else {
            command.args([
                "delete",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "Olé",
                "/f",
            ]);
        }
        let output = command.creation_flags(CREATE_NO_WINDOW.0).output();
        match output {
            Ok(result) if result.status.success() || !enabled => Ok(()),
            Ok(result) => Err(format!(
                "Failed to update startup setting: {}",
                String::from_utf8_lossy(&result.stderr)
            )),
            Err(err) => Err(format!("Failed to update startup setting: {err}")),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = _app;
        let _ = enabled;
        Ok(())
    }
}

fn position_widget_bottom_center(app: &tauri::AppHandle) {
    let Some(widget) = app.get_webview_window("widget") else { return };

    let monitor = (0..5).find_map(|attempt| {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        widget.current_monitor().ok().flatten()
            .or_else(|| app.primary_monitor().ok().flatten())
    });

    let Some(monitor) = monitor else {
        let _ = widget.set_position(PhysicalPosition::new(864_i32, 992_i32));
        return;
    };

    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let widget_size = widget.outer_size().unwrap_or_else(|_| tauri::PhysicalSize::new(192, 56));
    let x = monitor_position.x + ((monitor_size.width.saturating_sub(widget_size.width)) / 2) as i32;
    let y = monitor_position.y + monitor_size.height.saturating_sub(widget_size.height + WIDGET_BOTTOM_MARGIN_PX) as i32;
    let _ = widget.set_position(PhysicalPosition::new(x, y));
}

fn widget_enabled_from_db() -> bool {
    db::get_setting("widget_enabled".to_string())
        .map(|value| value != "false" && value != "0")
        .unwrap_or(true)
}

fn show_widget_window(app: &AppHandle) -> Result<(), String> {
    position_widget_bottom_center(app);
    let widget = app
        .get_webview_window("widget")
        .ok_or_else(|| "Widget window is unavailable.".to_string())?;
    widget.show().map_err(|err| format!("Failed to show widget: {err}"))?;
    Ok(())
}

#[tauri::command]
fn dock_ole_widget(app: AppHandle, side: String, vertical: f64) -> Result<(), String> {
    dock_widget_to_edge(&app, &side, vertical)
}

#[tauri::command]
fn snap_ole_widget_to_edge(app: AppHandle, side: String) -> Result<f64, String> {
    let widget = app
        .get_webview_window("widget")
        .ok_or_else(|| "Widget window is unavailable.".to_string())?;
    let monitor = widget.current_monitor()
        .map_err(|err| format!("Failed to read widget monitor: {err}"))?
        .or_else(|| app.primary_monitor().ok().flatten())
        .ok_or_else(|| "No monitor found for widget.".to_string())?;
    let area = monitor.work_area();
    let size = widget.outer_size().map_err(|err| format!("Failed to read widget size: {err}"))?;
    let pos = widget.outer_position().map_err(|err| format!("Failed to read widget position: {err}"))?;
    let max_y = area.size.height.saturating_sub(size.height).max(1);
    let relative_y = pos.y.saturating_sub(area.position.y) as f64;
    let vertical = ((relative_y / max_y as f64) * 100.0).clamp(0.0, 100.0);
    dock_widget_to_edge(&app, &side, vertical)?;
    db::set_setting("ole_dock_y".to_string(), format!("{vertical:.0}"));
    Ok(vertical)
}

fn dock_widget_to_edge(app: &AppHandle, side: &str, vertical: f64) -> Result<(), String> {
    let widget = app
        .get_webview_window("widget")
        .ok_or_else(|| "Widget window is unavailable.".to_string())?;
    let monitor = widget.current_monitor()
        .map_err(|err| format!("Failed to read widget monitor: {err}"))?
        .or_else(|| app.primary_monitor().ok().flatten())
        .ok_or_else(|| "No monitor found for widget.".to_string())?;
    let area = monitor.work_area();
    let size = widget.outer_size().unwrap_or_else(|_| tauri::PhysicalSize::new(84, 124));
    let margin = 8_i32;
    let x = if side == "left" {
        area.position.x + margin
    } else {
        area.position.x + area.size.width.saturating_sub(size.width) as i32 - margin
    };
    let max_y = area.size.height.saturating_sub(size.height);
    let y = area.position.y + ((max_y as f64) * (vertical.clamp(0.0, 100.0) / 100.0)) as i32;
    widget
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|err| format!("Failed to dock Olé badge: {err}"))
}

#[tauri::command]
fn show_widget(app: AppHandle) -> Result<(), String> {
    show_widget_window(&app)
}

#[tauri::command]
fn hide_widget(app: AppHandle) -> Result<(), String> {
    if let Some(widget) = app.get_webview_window("widget") {
        widget.hide().map_err(|err| format!("Failed to hide widget: {err}"))?;
    }
    Ok(())
}

#[tauri::command]
fn set_widget_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    db::set_setting("widget_enabled".to_string(), enabled.to_string());
    let _ = app.emit("widget-visibility-changed", enabled);
    if enabled {
        show_widget_window(&app)?;
    } else {
        hide_widget(app)?;
    }
    Ok(())
}

fn show_overlay_window(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("overlay")
        .ok_or_else(|| "Overlay window is unavailable.".to_string())?;

    let width = 480.0;
    let height = 180.0;
    let _ = window.set_size(Size::Logical(LogicalSize::new(width, height)));
    let _ = window.set_shadow(false);

    if let Ok(Some(monitor)) = window.primary_monitor() {
        let scale_factor = monitor.scale_factor();
        let area = monitor.work_area();
        
        let work_area_x = area.position.x as f64 / scale_factor;
        let work_area_y = area.position.y as f64 / scale_factor;
        let work_area_width = area.size.width as f64 / scale_factor;
        let work_area_height = area.size.height as f64 / scale_factor;

        let x = work_area_x + (work_area_width - width) / 2.0;
        let y = work_area_y + work_area_height - height - 80.0;

        let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)));
    }

    window.show().map_err(|err| format!("Failed to show overlay: {err}"))?;
    window.set_focus().map_err(|err| format!("Failed to focus overlay: {err}"))?;
    Ok(())
}

fn open_main_window(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window is unavailable.".to_string())?;
    let _ = window.unminimize();
    window.show().map_err(|err| format!("Failed to show main window: {err}"))?;
    window.set_focus().map_err(|err| format!("Failed to focus main window: {err}"))?;
    Ok(())
}

fn show_startup_windows(app: &tauri::AppHandle, autostart: bool) {
    let widget_enabled = widget_enabled_from_db();
    if autostart {
        let app_clone = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(800));
            position_widget_bottom_center(&app_clone);
            if widget_enabled {
                if let Some(widget) = app_clone.get_webview_window("widget") {
                    let _ = widget.show();
                }
            }
        });
    } else {
        position_widget_bottom_center(app);
        if widget_enabled {
            if let Some(widget) = app.get_webview_window("widget") {
                let _ = widget.show();
                let _ = widget.set_focus();
            }
        }
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.show();
            let _ = main.set_focus();
        }
    }
}

// ─── Voice Engine Commands ───────────────────────────────────────────────────

#[tauri::command]
fn load_model(
    app: tauri::AppHandle,
    filename: String,
) -> Result<(), String> {
    // Resolve the on-disk path for the requested model.
    let model_path = if filename == "sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8" {
        if !transcription::parakeet_bundle_ready() {
            return Err("Parakeet V3 model files are incomplete. Download Parakeet V3 again from Settings.".into());
        }
        transcription::parakeet_bundle_dir().to_string_lossy().to_string()
    } else {
        if !filename.ends_with(".bin") {
            return Err("This model is not selectable by the local engine.".into());
        }
        let path = audio::models_dir().join(&filename);
        if !path.exists() {
            return Err(format!("Model file not found: {}", filename));
        }
        path.to_string_lossy().to_string()
    };

    // Record the selection immediately so it survives a restart and is used by
    // the next dictation even if the background load hasn't finished yet.
    {
        let state = app.state::<AppState>();
        *state.selected_model.lock().unwrap() = Some(model_path.clone());
    }
    db::DB_CONN.lock().unwrap()
        .execute("INSERT OR REPLACE INTO settings (key,value) VALUES ('model',?)", [&filename]).ok();

    // Load into the warm engine in the background so selecting a model feels
    // instant. The engine emits "model-loading" / "model-loaded" /
    // "model-load-error" events that drive the UI spinner. Already-warm models
    // (e.g. switching back to one you used before) load in microseconds.
    let voice_engine = app.state::<engine::VoiceEngine>().inner().clone();
    std::thread::spawn(move || {
        if let Err(e) = voice_engine.load_model(&model_path) {
            eprintln!("[engine] background load failed: {e}");
        }
    });
    Ok(())
}

#[tauri::command]
fn get_downloaded_models() -> Vec<String> {
    let dir = audio::models_dir();
    let mut models: Vec<String> = std::fs::read_dir(&dir).map(|rd| rd
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if (name.ends_with(".bin") || name.ends_with(".nemo") || name.ends_with(".safetensors")) && !name.contains(".part") {
                let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                if size > 1_000_000 { Some(name) } else { None }
            } else { None }
        }).collect()
    ).unwrap_or_default();
    if transcription::parakeet_bundle_ready() && !models.iter().any(|m| m == "sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8") {
        models.push("sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8".into());
    }

    models
}

#[tauri::command]
fn reregister_hotkey(app: tauri::AppHandle, new_hotkey: String) -> Result<(), String> {
    let old_hotkey = db::get_setting("hotkey".to_string()).unwrap_or_else(|| "Alt+Space".to_string());
    if let Ok(shortcut) = old_hotkey.parse::<Shortcut>() {
        let _ = app.global_shortcut().unregister(shortcut);
    }

    db::DB_CONN.lock().unwrap()
        .execute("INSERT OR REPLACE INTO settings (key,value) VALUES ('hotkey',?)", [&new_hotkey]).ok();

    let register_voice_res = app.global_shortcut().on_shortcut(new_hotkey.as_str(), move |app_handle, _shortcut, event| {
        let state = app_handle.state::<AppState>();
        let mode = state.rec_mode.lock().unwrap().clone();
        match event.state() {
            ShortcutState::Pressed => {
                if mode == "toggle" {
                    let rec = *state.is_recording.lock().unwrap();
                    if rec { let _ = app_handle.emit("hotkey-released", ()); }
                    else   { let _ = app_handle.emit("hotkey-pressed",  ()); }
                } else {
                    let mut hotkey_down = state.hotkey_down.lock().unwrap();
                    if !*hotkey_down {
                        *hotkey_down = true;
                        let _ = app_handle.emit("hotkey-pressed", ());
                    }
                }
            }
            ShortcutState::Released => {
                if mode == "push-to-talk" {
                    let app = app_handle.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(70));
                        let state = app.state::<AppState>();
                        let hotkey = state.current_hotkey.lock().unwrap().clone();
                        if hotkey_is_physically_down(&hotkey) {
                            return;
                        }

                        let mut hotkey_down = state.hotkey_down.lock().unwrap();
                        if *hotkey_down {
                            *hotkey_down = false;
                            let _ = app.emit("hotkey-released", ());
                        }
                    });
                }
            }
        }
    });

    if let Err(e) = register_voice_res {
        return Err(format!("Failed to register voice hotkey: {}", e));
    }

    if let Some(state) = app.try_state::<AppState>() {
        *state.current_hotkey.lock().unwrap() = new_hotkey.clone();
        *state.hotkey_down.lock().unwrap() = false;
    }
    Ok(())
}

#[tauri::command]
fn set_recording_mode(
    state: tauri::State<'_, AppState>,
    mode: String,
) -> Result<(), String> {
    if mode != "push-to-talk" && mode != "toggle" {
        return Err("Recording mode must be push-to-talk or toggle.".into());
    }
    *state.rec_mode.lock().unwrap() = mode.clone();
    db::DB_CONN.lock().unwrap()
        .execute("INSERT OR REPLACE INTO settings (key,value) VALUES ('mode',?)", [&mode]).ok();
    Ok(())
}

#[tauri::command]
fn get_language_mode(state: tauri::State<'_, AppState>) -> String {
    state.language_mode.lock().unwrap().clone()
}

#[tauri::command]
fn set_language_mode(state: tauri::State<'_, AppState>, mode: String) -> Result<(), String> {
    if !["auto", "en", "hi", "hinglish"].contains(&mode.as_str()) {
        return Err(format!("Invalid language mode: {}", mode));
    }
    *state.language_mode.lock().unwrap() = mode.clone();
    db::DB_CONN.lock().unwrap()
        .execute("INSERT OR REPLACE INTO settings (key,value) VALUES ('language_mode',?)", [&mode])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn virtual_key_for_hotkey_part(part: &str) -> Option<u16> {
    match part.trim().to_ascii_lowercase().as_str() {
        "ctrl" | "control" => Some(0x11),
        "alt" | "option" => Some(0x12),
        "shift" => Some(0x10),
        "space" => Some(0x20),
        "enter" | "return" => Some(0x0D),
        "tab" => Some(0x09),
        "escape" | "esc" => Some(0x1B),
        key if key.len() == 1 => {
            let b = key.as_bytes()[0];
            if b.is_ascii_alphanumeric() {
                Some(b.to_ascii_uppercase() as u16)
            } else {
                None
            }
        }
        key if key.starts_with('f') => key[1..]
            .parse::<u16>()
            .ok()
            .filter(|n| (1..=24).contains(n))
            .map(|n| 0x70 + n - 1),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn hotkey_is_physically_down(hotkey: &str) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

    let keys = hotkey
        .split('+')
        .filter_map(virtual_key_for_hotkey_part)
        .collect::<Vec<_>>();

    if keys.is_empty() {
        return false;
    }

    keys.into_iter().all(|key| {
        let state = unsafe { GetAsyncKeyState(key as i32) };
        (state as u16 & 0x8000) != 0
    })
}

#[cfg(not(target_os = "windows"))]
fn hotkey_is_physically_down(_hotkey: &str) -> bool {
    false
}

// ─── Shortcuts (MeshPrompt) ──────────────────────────────────────────────────

fn emit_capture(app: AppHandle) {
    let state = app.state::<RuntimeState>();
    if state.paused.lock().map(|paused| *paused).unwrap_or(false) {
        return;
    }

    
    let settings = read_settings(&app);
    let term_active = is_terminal_foreground();
    if let Ok(mut term) = state.is_terminal.lock() {
        *term = term_active;
    }
    match capture_selection(&settings) {
        Ok(text) => {
            if let Ok(mut len) = state.captured_len.lock() {
                *len = text.chars().count();
            }
            if let Ok(mut cap_text) = state.captured_text.lock() {
                *cap_text = text.clone();
            }
            let _ = show_overlay_window(&app);
            let _ = app.emit("meshprompt://captured-text", text);
        }
        Err(message) => {
            if let Ok(mut cap_text) = state.captured_text.lock() {
                *cap_text = "".to_string();
            }
            let _ = show_overlay_window(&app);
            let _ = app.emit("meshprompt://capture-error", message);
        }
    }
}

fn normalize_shortcut(shortcut: &str) -> String {
    shortcut
        .replace("Ctrl", "CommandOrControl")
        .replace(" ", "")
        .replace("++", "+")
}

fn register_global_shortcut(app: &AppHandle) -> Result<(), String> {
    let shortcut = normalize_shortcut(&read_settings(app).shortcut);
    let app_for_handler = app.clone();
    app
        .global_shortcut()
        .on_shortcut(shortcut.as_str(), move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Released {
                let app_clone = app_for_handler.clone();
                std::thread::spawn(move || {
                    emit_capture(app_clone);
                });
            }
        })
        .map_err(|err| format!("Failed to register shortcut {shortcut}: {err}"))
}

#[tauri::command]
fn unregister_global_shortcut(app: AppHandle) -> Result<(), String> {
    let settings = read_settings(&app);
    if let Ok(shortcut) = normalize_shortcut(&settings.shortcut).parse::<tauri_plugin_global_shortcut::Shortcut>() {
        let _ = app.global_shortcut().unregister(shortcut);
    }
    Ok(())
}

#[tauri::command]
fn reregister_global_shortcut(app: AppHandle) -> Result<(), String> {
    let _ = register_global_shortcut(&app);
    Ok(())
}

// ─── Auto-Updates (MeshPrompt) ───────────────────────────────────────────────

#[tauri::command]
async fn check_for_updates(app: AppHandle) -> Result<UpdateCheckResult, String> {
    let client = reqwest::Client::builder()
        .user_agent("Ole-Updater")
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let res = client
        .get("https://api.github.com/repos/Zclaw89/jarvis-overlay/releases")
        .send()
        .await;

    let mut use_fallback = false;
    let mut releases_json = None;

    match res {
        Ok(res_val) => {
            if res_val.status().is_success() {
                if let Ok(json) = res_val.json::<serde_json::Value>().await {
                    releases_json = Some(json);
                } else {
                    use_fallback = true;
                }
            } else {
                use_fallback = true;
            }
        }
        Err(_) => {
            use_fallback = true;
        }
    }

    if use_fallback {
        let fallback_url = "https://raw.githubusercontent.com/Zclaw89/jarvis-overlay/main/latest-version.json";
        let fb_res = client
            .get(fallback_url)
            .send()
            .await
            .map_err(|e| format!("Failed to check for updates (GitHub API rate limit exceeded & fallback failed): {e}"))?;

        if !fb_res.status().is_success() {
            return Err(format!("GitHub API rate limit exceeded and fallback returned: {}", fb_res.status()));
        }

        let fb_data: serde_json::Value = fb_res
            .json()
            .await
            .map_err(|e| format!("Failed to parse fallback metadata: {e}"))?;

        let tag_name = fb_data["version"].as_str().ok_or("Invalid fallback version")?.to_string();
        let changelog = fb_data["changelog"].as_str().unwrap_or("No release notes provided.").to_string();
        let download_url = fb_data["downloadUrl"].as_str().unwrap_or("").to_string();

        let current_version = app.package_info().version.to_string();
        let update_available = is_newer_version(&current_version, &tag_name);

        return Ok(UpdateCheckResult {
            update_available,
            version: tag_name,
            changelog,
            download_url,
        });
    }

    let releases = releases_json.ok_or("No releases metadata available")?;

    let releases_arr = releases.as_array().ok_or("Invalid releases list")?;

    let mut ole_release = None;
    for rel in releases_arr {
        if let Some(tag) = rel["tag_name"].as_str() {
            if tag.to_lowercase().starts_with("ole-") || tag.to_lowercase().starts_with("ole-") {
                ole_release = Some(rel);
                break;
            }
        }
    }

    let release = ole_release.or_else(|| {
        releases_arr.first().map(|r| r)
    }).ok_or("No releases found")?;

    let tag_name = release["tag_name"].as_str().ok_or("Invalid release tag")?.to_string();
    let changelog = release["body"].as_str().unwrap_or("No release notes provided.").to_string();
    
    let current_version = app.package_info().version.to_string();
    let update_available = is_newer_version(&current_version, &tag_name);

    let mut download_url = String::new();
    if let Some(assets) = release["assets"].as_array() {
        for asset in assets {
            if let Some(name) = asset["name"].as_str() {
                let name_lower = name.to_lowercase();
                if (name_lower.ends_with(".msi") || name_lower.ends_with(".exe")) && (name_lower.contains("ole") || name_lower.contains("meshpilot")) {
                    if let Some(url) = asset["browser_download_url"].as_str() {
                        download_url = url.to_string();
                        break;
                    }
                }
            }
        }
    }

    Ok(UpdateCheckResult {
        update_available,
        version: tag_name,
        changelog,
        download_url,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateCheckResult {
    update_available: bool,
    version: String,
    changelog: String,
    download_url: String,
}

fn get_version_numbers(s: &str) -> Vec<u32> {
    if let Some(start_idx) = s.find(|c: char| c.is_ascii_digit()) {
        let version_part = &s[start_idx..];
        version_part
            .split('.')
            .map(|part| {
                let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
                digits.parse::<u32>().unwrap_or(0)
            })
            .collect()
    } else {
        vec![0]
    }
}

fn is_newer_version(current: &str, latest: &str) -> bool {
    let c_nums = get_version_numbers(current);
    let l_nums = get_version_numbers(latest);

    for i in 0..std::cmp::max(c_nums.len(), l_nums.len()) {
        let c_val = *c_nums.get(i).unwrap_or(&0);
        let l_val = *l_nums.get(i).unwrap_or(&0);
        if l_val > c_val {
            return true;
        } else if c_val > l_val {
            return false;
        }
    }
    false
}

#[tauri::command]
async fn install_update(_app: AppHandle, download_url: String) -> Result<(), String> {
    if download_url.is_empty() {
        return Err("No download URL provided.".to_string());
    }

    let client = reqwest::Client::builder()
        .user_agent("Ole-Updater")
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let res = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download update: {e}"))?;

    if !res.status().is_success() {
        return Err(format!("Download failed with status: {}", res.status()));
    }

    let bytes = res.bytes().await.map_err(|e| format!("Failed to read download stream: {e}"))?;
    
    let temp_dir = std::env::temp_dir();
    let is_msi = download_url.to_lowercase().ends_with(".msi");
    let file_name = if is_msi { "ole-setup.msi" } else { "ole-setup.exe" };
    let installer_path = temp_dir.join(file_name);

    fs::write(&installer_path, bytes).map_err(|e| format!("Failed to write installer file: {e}"))?;

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        use windows::Win32::System::Threading::CREATE_NO_WINDOW;

        let exe_path = std::env::current_exe().map_err(|e| format!("Failed to get current exe path: {e}"))?;
        let exe_str = exe_path.to_str().ok_or("Invalid exe path")?;
        let installer_str = installer_path.to_str().ok_or("Invalid installer path")?;

        let script = if is_msi {
            format!(
                "Start-Sleep -Seconds 1; \
                 $p = Start-Process -FilePath 'msiexec.exe' -ArgumentList '/i', '\"{}\"', '/passive' -PassThru; \
                 $p.WaitForExit(); \
                 Start-Process -FilePath '{}'",
                installer_str, exe_str
            )
        } else {
            format!(
                "Start-Sleep -Seconds 1; \
                 $p = Start-Process -FilePath '{}' -ArgumentList '/S' -PassThru; \
                 $p.WaitForExit(); \
                 Start-Process -FilePath '{}'",
                installer_str, exe_str
            )
        };

        let mut cmd = Command::new("powershell.exe");
        cmd.args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script]);
        cmd.creation_flags(CREATE_NO_WINDOW.0).spawn().map_err(|e| format!("Failed to launch installer: {e}"))?;
        
        std::process::exit(0);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = _app;
        return Err("Auto-updates are only supported on Windows.".to_string());
    }
}


// ─── Entry Point ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mailbox_write_job_and_read_reply_marks_done() {
        let root = std::env::temp_dir().join(format!(
            "ole-mailbox-test-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));

        let job = write_mailbox_job_at(&root, "analyze this", Some(b"png-bytes")).expect("write mailbox job");
        let inbox_dir = PathBuf::from(&job.inbox_dir);
        let outbox_dir = PathBuf::from(&job.outbox_dir);

        assert_eq!(fs::read_to_string(inbox_dir.join("request.txt")).unwrap(), "analyze this");
        assert!(inbox_dir.join("screenshot.png").exists());
        assert!(read_mailbox_reply_at(&root, &job.id).unwrap().is_none());

        fs::write(outbox_dir.join("reply.txt"), "Zeus says hello.").unwrap();
        let reply = read_mailbox_reply_at(&root, &job.id).unwrap();

        assert_eq!(reply.as_deref(), Some("Zeus says hello."));
        assert!(inbox_dir.join("done.json").exists());

        let _ = fs::remove_dir_all(root);
    }
}

fn main() {
    let builder = tauri::Builder::default();

    // Only enforce a single instance in release builds. In debug (`tauri dev`)
    // we skip the lock so the dev build can run alongside the installed
    // release build, which shares the same app identifier.
    #[cfg(not(debug_assertions))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }));

    builder
        .manage(RuntimeState::default())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            db::init_db();
            std::thread::spawn(ensure_autostart_enabled);
            let autostart = launched_from_autostart();

            // ── Auth Deep Link protocol scheme ──
            use tauri_plugin_deep_link::DeepLinkExt;
            let _ = app.deep_link().register("ole");


            let (saved_mode, saved_hotkey, saved_model_file, saved_language_mode) = {
                let conn = db::DB_CONN.lock().unwrap();
                let mode = conn.query_row("SELECT value FROM settings WHERE key='mode'", [], |r| r.get::<_,String>(0))
                    .unwrap_or_else(|_| "push-to-talk".to_string());
                let hotkey = conn.query_row("SELECT value FROM settings WHERE key='hotkey'", [], |r| r.get::<_,String>(0))
                    .unwrap_or_else(|_| "Alt+Space".to_string());
                let model = conn.query_row("SELECT value FROM settings WHERE key='model'", [], |r| r.get::<_,String>(0)).ok();
                let language_mode = conn.query_row("SELECT value FROM settings WHERE key='language_mode'", [], |r| r.get::<_,String>(0))
                    .unwrap_or_else(|_| "auto".to_string());
                (mode, hotkey, model, language_mode)
            };

            let saved_model_path = saved_model_file.map(|f| {
                if f == "sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8" {
                    transcription::parakeet_bundle_dir().to_string_lossy().to_string()
                } else {
                    audio::models_dir().join(&f).to_string_lossy().to_string()
                }
            }).filter(|p| std::path::Path::new(p).exists());

            let is_recording = Arc::new(Mutex::new(false));
            let recording_session_id = Arc::new(Mutex::new(0_u64));
            let hotkey_down = Arc::new(Mutex::new(false));
            let current_hotkey = Arc::new(Mutex::new(saved_hotkey.clone()));
            let rec_mode     = Arc::new(Mutex::new(saved_mode));

            app.manage(AppState {
                is_recording:   is_recording.clone(),
                recording_session_id,
                hotkey_down,
                current_hotkey,
                rec_mode:       rec_mode.clone(),
                selected_model: Mutex::new(saved_model_path.clone()),
                language_mode: Mutex::new(saved_language_mode),
            });

            // ── In-process speech engine (keeps the model warm in memory) ──
            // Route whisper.cpp / ggml's stderr spam through the `log` crate.
            // The app installs no logger, so these messages are simply dropped.
            whisper_rs::install_logging_hooks();
            engine::apply_accelerators();
            let voice_engine = engine::VoiceEngine::new(app.handle().clone());
            app.manage(voice_engine.clone());
            // Warm-load the previously selected model in the background so the
            // first dictation is already hot instead of paying a cold reload.
            if let Some(path) = saved_model_path.clone() {
                std::thread::spawn(move || {
                    if let Err(e) = voice_engine.load_model(&path) {
                        eprintln!("[engine] warm load failed: {e}");
                    }
                });
            }

            audio::start_level_emitter(app.handle().clone(), is_recording.clone());

            // ── Dynamic Global Shortcuts Callback Injection ──
            let voice_hotkey = saved_hotkey.clone();
            let voice_reg_res = app.global_shortcut().on_shortcut(saved_hotkey.as_str(), move |app_handle, _shortcut, event| {
                let state = app_handle.state::<AppState>();
                let mode = state.rec_mode.lock().unwrap().clone();
                match event.state() {
                    ShortcutState::Pressed => {
                        if mode == "toggle" {
                            let rec = *state.is_recording.lock().unwrap();
                            if rec { let _ = app_handle.emit("hotkey-released", ()); }
                            else   { let _ = app_handle.emit("hotkey-pressed",  ()); }
                        } else {
                            let mut hotkey_down = state.hotkey_down.lock().unwrap();
                            if !*hotkey_down {
                                *hotkey_down = true;
                                let _ = app_handle.emit("hotkey-pressed", ());
                            }
                        }
                    }
                    ShortcutState::Released => {
                        if mode == "push-to-talk" {
                            let app = app_handle.clone();
                            std::thread::spawn(move || {
                                std::thread::sleep(std::time::Duration::from_millis(70));
                                let state = app.state::<AppState>();
                                let hotkey = state.current_hotkey.lock().unwrap().clone();
                                if hotkey_is_physically_down(&hotkey) {
                                    return;
                                }

                                let mut hotkey_down = state.hotkey_down.lock().unwrap();
                                if *hotkey_down {
                                    *hotkey_down = false;
                                    let _ = app.emit("hotkey-released", ());
                                }
                            });
                        }
                    }
                }
            });

            if let Err(e) = voice_reg_res {
                eprintln!("[MeshVoice] Failed to register global hotkey '{}': {}", voice_hotkey, e);
                // Fallback to Ctrl+Alt+Space if Alt+Space fails
                let fallback_hotkey = "Ctrl+Alt+Space";
                let voice_reg_fallback = app.global_shortcut().on_shortcut(fallback_hotkey, move |app_handle, _shortcut, event| {
                    let state = app_handle.state::<AppState>();
                    let mode = state.rec_mode.lock().unwrap().clone();
                    match event.state() {
                        ShortcutState::Pressed => {
                            if mode == "toggle" {
                                let rec = *state.is_recording.lock().unwrap();
                                if rec { let _ = app_handle.emit("hotkey-released", ()); }
                                else   { let _ = app_handle.emit("hotkey-pressed",  ()); }
                            } else {
                                let mut hotkey_down = state.hotkey_down.lock().unwrap();
                                if !*hotkey_down {
                                    *hotkey_down = true;
                                    let _ = app_handle.emit("hotkey-pressed", ());
                                }
                            }
                        }
                        ShortcutState::Released => {
                            if mode == "push-to-talk" {
                                let app = app_handle.clone();
                                std::thread::spawn(move || {
                                    std::thread::sleep(std::time::Duration::from_millis(70));
                                    let state = app.state::<AppState>();
                                    let hotkey = state.current_hotkey.lock().unwrap().clone();
                                    if hotkey_is_physically_down(&hotkey) {
                                        return;
                                    }

                                    let mut hotkey_down = state.hotkey_down.lock().unwrap();
                                    if *hotkey_down {
                                        *hotkey_down = false;
                                        let _ = app.emit("hotkey-released", ());
                                    }
                                });
                            }
                        }
                    }
                });
                if let Err(err) = voice_reg_fallback {
                    eprintln!("[MeshVoice] Failed to register fallback hotkey '{}': {}", fallback_hotkey, err);
                } else {
                    println!("[MeshVoice] Fallback hotkey '{}' registered successfully.", fallback_hotkey);
                }
            }

            // Register Prompt Enhancer global shortcut
            let prompt_settings = read_settings(app.handle());
            if let Ok(mut paused) = app.state::<RuntimeState>().paused.lock() {
                *paused = prompt_settings.paused;
            }
            let _ = register_global_shortcut(app.handle());

            // ── Consolidated System Tray ──
            {
                let title_i = MenuItem::with_id(app, "title", format!("Olé v{}", env!("CARGO_PKG_VERSION")), false, None::<&str>)?;
                let open_chat = MenuItem::with_id(app, "open_chat", "Open Olé Chat", true, None::<&str>)?;
                let open_voice = MenuItem::with_id(app, "open_voice", "Open Olé", true, None::<&str>)?;
                let open_prompt = MenuItem::with_id(app, "open_prompt", "Open AI Tools", true, None::<&str>)?;
                let open_overlay = MenuItem::with_id(app, "open_overlay", "Open Text Overlay", true, None::<&str>)?;
                let settings = MenuItem::with_id(app, "settings", "Settings...", true, None::<&str>)?;
                let quit = MenuItem::with_id(app, "quit", "Quit Olé", true, Some("Ctrl+Q"))?;
                let separator_1 = PredefinedMenuItem::separator(app)?;
                let separator_2 = PredefinedMenuItem::separator(app)?;
                let menu = Menu::with_items(app, &[
                    &title_i,
                    &separator_1,
                    &open_chat,
                    &open_voice,
                    &open_prompt,
                    &open_overlay,
                    &settings,
                    &separator_2,
                    &quit
                ])?;
                let mut tray_builder = TrayIconBuilder::new()
                    .menu(&menu)
                    .tooltip("Olé - running in tray")
                    .show_menu_on_left_click(false)
                    .on_tray_icon_event(|tray, event| {
                        if let tauri::tray::TrayIconEvent::Click {
                            button: tauri::tray::MouseButton::Left,
                            button_state: tauri::tray::MouseButtonState::Up, ..
                        } = event {
                            let app = tray.app_handle();
                            let _ = open_main_window(app);
                        }
                    })
                    .on_menu_event(|app, ev| match ev.id.as_ref() {
                        "open_voice" => {
                            let _ = open_main_window(app);
                            let _ = app.emit("navigate-view", "dashboard");
                        }
                        "open_chat" => {
                            let _ = open_widget_chat(app.clone());
                        }
                        "open_prompt" => {
                            let _ = open_main_window(app);
                            let _ = app.emit("navigate-view", "prompt");
                        }
                        "open_overlay" => {
                            let _ = show_overlay_window(app);
                        }
                        "settings" => {
                            let _ = open_main_window(app);
                            let _ = app.emit("navigate-view", "settings");
                        }
                        "quit" => app.exit(0),
                        _ => {}
                    });
                if let Some(icon) = app.default_window_icon() {
                    tray_builder = tray_builder.icon(icon.clone());
                }
                tray_builder.build(app)?;
            }

            show_startup_windows(app.handle(), autostart);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let settings = read_settings(window.app_handle());
                    if settings.close_to_tray {
                        let _ = window.hide();
                        api.prevent_close();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            // Voice Dictation Commands
            audio::start_recording,
            audio::stop_recording_and_transcribe,
            audio::get_audio_devices,
            audio::check_microphone_status,
            audio::open_mic_settings,
            db::get_history,
            db::get_history_audio,
            db::get_stats,
            db::get_setting,
            db::set_setting,
            db::get_dictionary,
            db::add_dictionary_entry,
            db::delete_dictionary_entry,
            db::delete_history_entry,
            db::delete_all_history,
            transcription::get_available_models,
            transcription::download_model,
            load_model,
            get_downloaded_models,
            reregister_hotkey,
            set_recording_mode,
            show_main_window,
            show_widget,
            hide_widget,
            open_widget_chat,
            set_widget_click_through,
            dock_ole_widget,
            snap_ole_widget_to_edge,
            set_widget_enabled,
            get_language_mode,
            set_language_mode,
            
            // Prompt Enhancer Commands
            get_app_state,
            save_settings,
            save_provider_key,
            get_provider_key,
            delete_provider_key,
            add_history,
            clear_history,
            capture_selected_text,
            get_captured_text,
            copy_text,
            replace_selected_text,
            show_overlay,
            hide_overlay,
            resize_overlay,
            set_paused,
            proxy_request,
            get_mailbox_paths,
            create_mailbox_job,
            read_mailbox_reply,
            create_dossier_item,
            list_dossier_items,
            attach_dossier_analysis,
            read_dossier_screenshot_base64,
            import_dossier_file,
            test_voice,
            check_for_updates,
            install_update,
            unregister_global_shortcut,
            reregister_global_shortcut
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
