use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};
use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings};
use once_cell::sync::Lazy;

/// Global, thread-safe Clipboard Manager wrapper.
pub struct ClipboardManager {
    // We wrap arboard::Clipboard in a Mutex to ensure only one thread acts on it at a time.
    inner: Mutex<Clipboard>,
}

// A globally shared, single instance of the Clipboard Manager.
pub static GLOBAL_CLIPBOARD: Lazy<Arc<ClipboardManager>> = Lazy::new(|| {
    Arc::new(ClipboardManager::new().expect("Failed to initialize system clipboard"))
});

impl ClipboardManager {
    pub fn new() -> Result<Self, String> {
        let cb = Clipboard::new().map_err(|e| format!("Clipboard initialization failed: {e}"))?;
        Ok(Self {
            inner: Mutex::new(cb),
        })
    }

    /// Safely read text from the clipboard, with configurable retries and backoff
    /// if the clipboard is occupied by another process.
    pub fn get_text_custom(
        &self, 
        guard: &mut MutexGuard<'_, Clipboard>,
        max_attempts: usize,
        initial_delay: Duration
    ) -> Result<String, String> {
        let mut attempts = 0;
        let mut delay = initial_delay;
        
        loop {
            match guard.get_text() {
                Ok(text) => return Ok(text),
                Err(arboard::Error::ClipboardOccupied) if attempts < max_attempts => {
                    attempts += 1;
                    thread::sleep(delay);
                    delay *= 2; // Exponential backoff
                }
                Err(err) => return Err(format!("Failed to read clipboard: {err}")),
            }
        }
    }

    /// Safely read text from the clipboard, with default exponential backoff retries.
    pub fn get_text_safe(&self, guard: &mut MutexGuard<'_, Clipboard>) -> Result<String, String> {
        self.get_text_custom(guard, 5, Duration::from_millis(10))
    }

    /// Safely write text to the clipboard, with exponential backoff retries 
    /// if the clipboard is occupied by another process.
    pub fn set_text_safe(&self, guard: &mut MutexGuard<'_, Clipboard>, text: String) -> Result<(), String> {
        let mut attempts = 0;
        let mut delay = Duration::from_millis(10);
        
        loop {
            match guard.set_text(text.clone()) {
                Ok(()) => return Ok(()),
                Err(arboard::Error::ClipboardOccupied) if attempts < 5 => {
                    attempts += 1;
                    thread::sleep(delay);
                    delay *= 2;
                }
                Err(err) => return Err(format!("Failed to write clipboard: {err}")),
            }
        }
    }

    /// Executes a fully synchronized, atomic Copy Transaction.
    /// Locks the clipboard globally, gets original text, stages a unique Canary,
    /// triggers Copy keystroke, polls for a value different from the Canary, 
    /// restores original text safely, and unlocks.
    pub fn capture_selection_transaction(&self, restore_clipboard: bool) -> Result<String, String> {
        // 1. Wait for physical modifier keys to be released (up to 500ms timeout)
        let start_wait = Instant::now();
        while modifiers_physically_down() {
            if start_wait.elapsed() > Duration::from_millis(500) {
                eprintln!("[Olé] Warning: Physical modifier keys did not release in time.");
                break;
            }
            thread::sleep(Duration::from_millis(15));
        }
        
        // Brief settling delay before simulating keyboard input
        thread::sleep(Duration::from_millis(30));

        // Determine adaptive polling timeout based on active foreground window
        let is_heavy = is_heavy_app();
        let polling_timeout = if is_heavy {
            Duration::from_millis(600)
        } else {
            Duration::from_millis(300)
        };

        // 2. Acquire global lock. This lock is held across the entire transaction!
        let mut guard = self.inner.lock().map_err(|e| format!("Lock poisoned: {e}"))?;
        
        // 3. Read and backup the original content
        let original_content = self.get_text_safe(&mut guard).ok();

        // 4. Stage a unique Sentinel / Canary value to the clipboard
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let canary = format!("\u{0000}_MeshPilot_Canary_{}_\u{0000}", timestamp);
        self.set_text_safe(&mut guard, canary.clone())?;

        // Brief settling delay to let the OS process the write before keystroke injection
        thread::sleep(Duration::from_millis(15));

        // 5. Trigger simulated OS Copy keystroke
        send_copy_keystroke()?;
        
        // 6. Adaptive Polling Loop
        let start_polling = Instant::now();
        let mut captured = String::new();
        let mut success = false;
        
        while start_polling.elapsed() < polling_timeout {
            // Use lightweight read attempts (max 1 retry, 5ms delay) so we don't consume the outer budget
            if let Ok(current_text) = self.get_text_custom(&mut guard, 1, Duration::from_millis(5)) {
                // If the clipboard content is no longer the Canary, the target app has successfully copied!
                if current_text != canary && !current_text.is_empty() {
                    captured = current_text;
                    success = true;
                    break;
                }
            }
            thread::sleep(Duration::from_millis(15));
        }

        // 7. Restore original clipboard content safely with collision avoidance
        if restore_clipboard {
            if let Some(original) = original_content {
                if success {
                    // Fast path: target application completed write successfully.
                    // A tiny settling delay to ensure OS finishes queueing, then write.
                    thread::sleep(Duration::from_millis(15));
                    let _ = self.set_text_safe(&mut guard, original);
                } else {
                    // Timeout/Failure path: to avoid collision, only restore if the canary is still active.
                    // If the canary is NOT active, some other process or a very delayed target process
                    // wrote to the clipboard, so we skip overwriting to avoid data loss.
                    if let Ok(current_text) = self.get_text_custom(&mut guard, 2, Duration::from_millis(5)) {
                        if current_text == canary {
                            let _ = self.set_text_safe(&mut guard, original);
                        }
                    }
                }
            }
        }

        let trimmed = captured.trim().to_string();
        if !success || trimmed.is_empty() {
            Err("No selected text found or copy timed out.".to_string())
        } else {
            Ok(trimmed)
        }
    }

    /// Executes a fully synchronized, atomic Paste Transaction.
    /// Locks the clipboard globally, gets original text, writes temporary text,
    /// triggers Paste, sleeps adaptively based on target application latency, 
    /// restores original text, and unlocks.
    pub fn paste_text_transaction(
        &self, 
        text: &str, 
        restore_clipboard: bool, 
        is_term: bool, 
        cap_len: usize
    ) -> Result<(), String> {
        // 1. Acquire global lock. Held across the entire paste-and-restore cycle!
        let mut guard = self.inner.lock().map_err(|e| format!("Lock poisoned: {e}"))?;

        // 2. Read original content
        let original_content = self.get_text_safe(&mut guard).ok();

        // 3. Stage the replacement text on the clipboard
        self.set_text_safe(&mut guard, text.to_string())?;
        thread::sleep(Duration::from_millis(50));

        // 4. Delete old text in terminal if needed
        if is_term && cap_len > 0 {
            send_backspaces(cap_len)?;
            thread::sleep(Duration::from_millis(50));
        }

        // 5. Trigger simulated OS Paste keystroke
        send_paste_keystroke()?;

        // 6. Determine adaptive sleep duration based on foreground application
        let target_delay = get_adaptive_restoration_delay();
        thread::sleep(target_delay);

        // 7. Restore original content safely
        if restore_clipboard {
            if let Some(original) = original_content {
                let _ = self.set_text_safe(&mut guard, original);
            }
        }

        Ok(())
    }
}

// ─── Input Automation Helpers ────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn send_copy_keystroke() -> Result<(), String> {
    let mut enigo = Enigo::new(&EnigoSettings::default())
        .map_err(|err| format!("Input automation unavailable: {err}"))?;
    enigo.key(Key::Control, Direction::Press).map_err(|err| err.to_string())?;
    enigo.key(Key::Insert, Direction::Click).map_err(|err| err.to_string())?;
    enigo.key(Key::Control, Direction::Release).map_err(|err| err.to_string())?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn send_copy_keystroke() -> Result<(), String> {
    let mut enigo = Enigo::new(&EnigoSettings::default())
        .map_err(|err| format!("Input automation unavailable: {err}"))?;
    enigo.key(Key::Meta, Direction::Press).map_err(|err| err.to_string())?;
    enigo.key(Key::Unicode('c'), Direction::Click).map_err(|err| err.to_string())?;
    enigo.key(Key::Meta, Direction::Release).map_err(|err| err.to_string())?;
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn send_copy_keystroke() -> Result<(), String> {
    let mut enigo = Enigo::new(&EnigoSettings::default())
        .map_err(|err| format!("Input automation unavailable: {err}"))?;
    enigo.key(Key::Control, Direction::Press).map_err(|err| err.to_string())?;
    enigo.key(Key::Unicode('c'), Direction::Click).map_err(|err| err.to_string())?;
    enigo.key(Key::Control, Direction::Release).map_err(|err| err.to_string())?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn send_paste_keystroke() -> Result<(), String> {
    let mut enigo = Enigo::new(&EnigoSettings::default())
        .map_err(|err| format!("Input automation unavailable: {err}"))?;
    enigo.key(Key::Shift, Direction::Press).map_err(|err| err.to_string())?;
    enigo.key(Key::Insert, Direction::Click).map_err(|err| err.to_string())?;
    enigo.key(Key::Shift, Direction::Release).map_err(|err| err.to_string())?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn send_paste_keystroke() -> Result<(), String> {
    let mut enigo = Enigo::new(&EnigoSettings::default())
        .map_err(|err| format!("Input automation unavailable: {err}"))?;
    enigo.key(Key::Meta, Direction::Press).map_err(|err| err.to_string())?;
    enigo.key(Key::Unicode('v'), Direction::Click).map_err(|err| err.to_string())?;
    enigo.key(Key::Meta, Direction::Release).map_err(|err| err.to_string())?;
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn send_paste_keystroke() -> Result<(), String> {
    let mut enigo = Enigo::new(&EnigoSettings::default())
        .map_err(|err| format!("Input automation unavailable: {err}"))?;
    enigo.key(Key::Control, Direction::Press).map_err(|err| err.to_string())?;
    enigo.key(Key::Unicode('v'), Direction::Click).map_err(|err| err.to_string())?;
    enigo.key(Key::Control, Direction::Release).map_err(|err| err.to_string())?;
    Ok(())
}

fn send_backspaces(count: usize) -> Result<(), String> {
    let mut enigo = Enigo::new(&EnigoSettings::default())
        .map_err(|err| format!("Input automation unavailable: {err}"))?;
    for _ in 0..count {
        enigo.key(Key::Backspace, Direction::Click).map_err(|err| err.to_string())?;
        thread::sleep(Duration::from_millis(8));
    }
    Ok(())
}

// ─── OS Modifier Key Polling Helper ──────────────────────────────────────────

#[cfg(target_os = "windows")]
fn modifiers_physically_down() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN
    };

    let modifiers = [
        VK_CONTROL.0 as i32,
        VK_SHIFT.0 as i32,
        VK_MENU.0 as i32, // Alt key
        VK_LWIN.0 as i32,
        VK_RWIN.0 as i32,
    ];

    modifiers.iter().any(|&vk| {
        let state = unsafe { GetAsyncKeyState(vk) };
        (state as u16 & 0x8000) != 0
    })
}

#[cfg(not(target_os = "windows"))]
fn modifiers_physically_down() -> bool {
    false
}

// ─── OS Window Class Analysis ─────────────────────────────────────────────────

fn is_heavy_app() -> bool {
    #[cfg(target_os = "windows")]
    {
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
                return name.contains("chrome_widgetwin_1") || name.contains("electron");
            }
        }
    }
    false
}

/// Queries the operating system for the foreground window's class name and
/// returns a safe sleep duration based on estimated app frame/event loop latency.
fn get_adaptive_restoration_delay() -> Duration {
    if is_heavy_app() {
        // Electron apps: high event loop / rendering thread latency
        Duration::from_millis(250)
    } else {
        // Lightweight / Native apps (e.g. Notepad, Cmd, Terminal): very fast processing
        Duration::from_millis(80)
    }
}
