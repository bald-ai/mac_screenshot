use std::process::Command;
use std::sync::Mutex;
use chrono::Local;
use serde::{Deserialize, Serialize};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    webview::WebviewWindowBuilder,
    Emitter, Manager, WindowEvent, State,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilenameBlock {
    pub id: String,
    pub enabled: bool,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilenameTemplate {
    pub blocks: Vec<FilenameBlock>,
    pub use_counter: bool,
}

impl Default for FilenameTemplate {
    fn default() -> Self {
        Self {
            blocks: vec![
                FilenameBlock { id: "prefix".to_string(), enabled: true, value: Some("llm-scr".to_string()) },
                FilenameBlock { id: "date".to_string(), enabled: true, value: None },
                FilenameBlock { id: "time".to_string(), enabled: true, value: None },
                FilenameBlock { id: "quality".to_string(), enabled: true, value: None },
                FilenameBlock { id: "dimensions".to_string(), enabled: true, value: None },
                FilenameBlock { id: "counter".to_string(), enabled: false, value: None },
            ],
            use_counter: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub quality: u32,
    pub max_width: u32,
    #[serde(default)]
    pub note_prefix_enabled: bool,
    #[serde(default)]
    pub note_prefix: String,
    #[serde(default)]
    pub filename_template: FilenameTemplate,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_fullscreen_shortcut")]
    pub fullscreen_shortcut: String,
    #[serde(default = "default_area_shortcut")]
    pub area_shortcut: String,
}

fn default_theme() -> String {
    "system".to_string()
}

fn default_fullscreen_shortcut() -> String {
    "Cmd+Shift+3".to_string()
}

fn default_area_shortcut() -> String {
    "Cmd+Shift+4".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            quality: 70,
            max_width: 1280,
            note_prefix_enabled: false,
            note_prefix: String::new(),
            filename_template: FilenameTemplate::default(),
            theme: "system".to_string(),
            fullscreen_shortcut: default_fullscreen_shortcut(),
            area_shortcut: default_area_shortcut(),
        }
    }
}

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub active_fullscreen_shortcut: Mutex<Shortcut>,
    pub active_area_shortcut: Mutex<Shortcut>,
}

fn get_settings_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(home).join(".screenshot_app_settings.json")
}

fn load_settings_from_file() -> Settings {
    let path = get_settings_path();
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(mut settings) = serde_json::from_str::<Settings>(&content) {
                if settings.theme == "light" {
                    settings.theme = "grey".to_string();
                }
                return settings;
            }
        }
    }
    Settings::default()
}

fn save_settings_to_file(settings: &Settings) -> Result<(), String> {
    let path = get_settings_path();
    let content = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    std::fs::write(&path, content)
        .map_err(|e| format!("Failed to write settings: {}", e))?;
    Ok(())
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(state: State<AppState>, settings: Settings) -> Result<(), String> {
    let mut current = state.settings.lock().unwrap();
    *current = settings.clone();
    save_settings_to_file(&settings)
}

#[tauri::command]
async fn update_shortcuts(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    fullscreen_shortcut: String,
    area_shortcut: String,
) -> Result<(), String> {
    let fullscreen_shortcut = normalize_shortcut_string(&fullscreen_shortcut)?;
    let area_shortcut = normalize_shortcut_string(&area_shortcut)?;
    let new_full = parse_shortcut(&fullscreen_shortcut)?;
    let new_area = parse_shortcut(&area_shortcut)?;
    if new_full.id() == new_area.id() {
        return Err("Fullscreen and area shortcuts must be different".to_string());
    }

    let (old_full_str, old_area_str) = {
        let settings = state.settings.lock().unwrap();
        (settings.fullscreen_shortcut.clone(), settings.area_shortcut.clone())
    };

    let old_full = parse_shortcut(&old_full_str).ok();
    let old_area = parse_shortcut(&old_area_str).ok();

    let global_shortcut = app.global_shortcut();

    if let Some(ref s) = old_full {
        let _ = global_shortcut.unregister(*s);
    }
    if let Some(ref s) = old_area {
        let _ = global_shortcut.unregister(*s);
    }

    if let Err(e) = global_shortcut.register(new_full) {
        if let Some(ref s) = old_full {
            let _ = global_shortcut.register(*s);
        }
        if let Some(ref s) = old_area {
            let _ = global_shortcut.register(*s);
        }
        return Err(format!("Failed to register fullscreen shortcut: {}", e));
    }

    if let Err(e) = global_shortcut.register(new_area) {
        let _ = global_shortcut.unregister(new_full);
        if let Some(ref s) = old_full {
            let _ = global_shortcut.register(*s);
        }
        if let Some(ref s) = old_area {
            let _ = global_shortcut.register(*s);
        }
        return Err(format!("Failed to register area shortcut: {}", e));
    }

    let settings_snapshot = {
        let mut settings = state.settings.lock().unwrap();
        settings.fullscreen_shortcut = fullscreen_shortcut;
        settings.area_shortcut = area_shortcut;
        settings.clone()
    };

    *state.active_fullscreen_shortcut.lock().unwrap() = new_full;
    *state.active_area_shortcut.lock().unwrap() = new_area;
    if let Err(e) = save_settings_to_file(&settings_snapshot) {
        let mut settings = state.settings.lock().unwrap();
        settings.fullscreen_shortcut = old_full_str.clone();
        settings.area_shortcut = old_area_str.clone();
        drop(settings);

        let _ = global_shortcut.unregister(new_full);
        let _ = global_shortcut.unregister(new_area);
        if let Some(ref s) = old_full {
            let _ = global_shortcut.register(*s);
        }
        if let Some(ref s) = old_area {
            let _ = global_shortcut.register(*s);
        }
        if let Some(s) = old_full {
            *state.active_fullscreen_shortcut.lock().unwrap() = s;
        }
        if let Some(s) = old_area {
            *state.active_area_shortcut.lock().unwrap() = s;
        }

        return Err(e);
    }

    update_tray_labels(&app)?;

    Ok(())
}

fn generate_temp_screenshot_path(extension: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    format!("{}/Desktop/llm-scr_tmp_{}.{}", home, timestamp, extension)
}

fn generate_screenshot_path(extension: &str, settings: &Settings, width: u32, height: u32) -> String {
    let now = Local::now();
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let template = &settings.filename_template;
    
    let mut parts: Vec<String> = Vec::new();
    
    for block in &template.blocks {
        if !block.enabled {
            continue;
        }
        
        let part = match block.id.as_str() {
            "prefix" => block.value.clone().unwrap_or_else(|| "llm-scr".to_string()),
            "date" => now.format("%m-%d").to_string(),
            "time" => now.format("%H-%M-%S").to_string(),
            "quality" => format!("{}%", settings.quality),
            "dimensions" => format!("{}x{}", width, height),
            "counter" => String::new(), // handled separately below
            _ => continue,
        };
        
        if block.id != "counter" && !part.is_empty() {
            parts.push(part);
        }
    }
    
    let base_name = parts.join("_");
    let counter_enabled = template.blocks.iter().any(|b| b.id == "counter" && b.enabled);
    
    if counter_enabled || template.use_counter {
        // Use counter for uniqueness
        let mut counter = 1u32;
        loop {
            let filename = if counter == 1 {
                format!("{}/Desktop/{}.{}", home, base_name, extension)
            } else {
                format!("{}/Desktop/{}_{}.{}", home, base_name, counter, extension)
            };
            
            if !std::path::Path::new(&filename).exists() {
                return filename;
            }
            counter += 1;
        }
    } else {
        format!("{}/Desktop/{}.{}", home, base_name, extension)
    }
}

// Get image dimensions using sips (macOS)
fn get_image_dimensions(filepath: &str) -> Result<(u32, u32), String> {
    let output = Command::new("sips")
        .args(["-g", "pixelWidth", "-g", "pixelHeight", filepath])
        .output()
        .map_err(|e| format!("Failed to get image dimensions: {}", e))?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    
    let width: u32 = output_str
        .lines()
        .find(|line| line.contains("pixelWidth"))
        .and_then(|line| line.split_whitespace().last())
        .and_then(|w| w.parse().ok())
        .unwrap_or(800);
    
    let height: u32 = output_str
        .lines()
        .find(|line| line.contains("pixelHeight"))
        .and_then(|line| line.split_whitespace().last())
        .and_then(|h| h.parse().ok())
        .unwrap_or(600);
    
    Ok((width, height))
}

// Calculate editor window size based on image dimensions and padding
// Returns (width, height) for the window
fn calculate_editor_window_size(img_width: u32, img_height: u32, padding: f64) -> (f64, f64) {
    const TOOLBAR_HEIGHT: f64 = 72.0;
    
    // Constraints
    const MIN_WIDTH: f64 = 500.0;   // Minimum to fit toolbar buttons
    const MIN_HEIGHT: f64 = 250.0;  // Minimum usable height
    const MAX_WIDTH: f64 = 1400.0;  // Maximum window width
    const MAX_HEIGHT: f64 = 900.0;  // Maximum window height
    
    let img_w = img_width as f64;
    let img_h = img_height as f64;
    
    // Window wraps tightly around image + padding
    // Only scale down if image is too large for max window
    let available_w = MAX_WIDTH - padding;
    let available_h = MAX_HEIGHT - TOOLBAR_HEIGHT - padding;
    
    let (canvas_w, canvas_h) = if img_w <= available_w && img_h <= available_h {
        // Image fits at 1:1
        (img_w, img_h)
    } else {
        // Scale down to fit max bounds
        let scale = (available_w / img_w).min(available_h / img_h);
        (img_w * scale, img_h * scale)
    };
    
    // Window size = canvas + chrome, clamped to min/max
    let window_w = (canvas_w + padding).max(MIN_WIDTH).min(MAX_WIDTH);
    let window_h = (canvas_h + TOOLBAR_HEIGHT + padding).max(MIN_HEIGHT).min(MAX_HEIGHT);
    
    (window_w, window_h)
}

// Image optimization: configurable quality and max width via Settings
// Default: 70% quality, 1280px max width
// Resizes images wider than max_width to maintain performance
fn optimize_screenshot(filepath: &str, settings: &Settings) -> Result<String, String> {
    // Convert to JPEG with configured quality and resize to max width
    let jpeg_path = filepath.replace(".png", ".jpg");

    // Use sips to resize (if wider than max_width) and convert to JPEG
    // First, get the width
    let width_output = Command::new("sips")
        .args(["-g", "pixelWidth", filepath])
        .output()
        .map_err(|e| format!("Failed to get image width: {}", e))?;

    let width_str = String::from_utf8_lossy(&width_output.stdout);
    let width: u32 = width_str
        .lines()
        .find(|line| line.contains("pixelWidth"))
        .and_then(|line| line.split_whitespace().last())
        .and_then(|w| w.parse().ok())
        .unwrap_or(0);

    // Resize if wider than max_width (0 means no resize)
    if settings.max_width > 0 && width > settings.max_width {
        Command::new("sips")
            .args(["--resampleWidth", &settings.max_width.to_string(), filepath])
            .output()
            .map_err(|e| format!("Failed to resize: {}", e))?;
    }

    // Convert to JPEG with configured quality
    let quality_str = settings.quality.to_string();
    let output = Command::new("sips")
        .args(["-s", "format", "jpeg", "-s", "formatOptions", &quality_str, filepath, "--out", &jpeg_path])
        .output()
        .map_err(|e| format!("Failed to convert to JPEG: {}", e))?;

    if output.status.success() {
        // Remove the original PNG
        let _ = std::fs::remove_file(filepath);
        Ok(jpeg_path)
    } else {
        // Fallback to PNG if conversion fails
        Ok(filepath.to_string())
    }
}

#[tauri::command]
fn take_screenshot(app: tauri::AppHandle, state: State<AppState>) -> Result<String, String> {
    // Block if rename popup is open
    if app.get_webview_window("rename").is_some() {
        return Err("Please finish renaming the current screenshot first".to_string());
    }

    let settings = state.settings.lock().unwrap().clone();
    let filepath = generate_temp_screenshot_path("png");

    // Use macOS screencapture command
    // -i = interactive mode (select area)
    // -x = no sound
    let output = Command::new("screencapture")
        .args(["-i", "-x", &filepath])
        .output()
        .map_err(|e| format!("Failed to run screencapture: {}", e))?;

    if output.status.success() {
        // Check if file was created (user might have cancelled)
        if std::path::Path::new(&filepath).exists() {
            let optimized_path = optimize_screenshot(&filepath, &settings)?;
            let (width, height) = get_image_dimensions(&optimized_path)?;
            let extension = std::path::Path::new(&optimized_path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("png");
            let final_path = generate_screenshot_path(extension, &settings, width, height);
            std::fs::rename(&optimized_path, &final_path)
                .map_err(|e| format!("Failed to rename screenshot: {}", e))?;
            Ok(final_path)
        } else {
            Err("Screenshot cancelled".to_string())
        }
    } else {
        Err("Screenshot cancelled or failed".to_string())
    }
}

#[tauri::command]
fn take_fullscreen_screenshot(app: tauri::AppHandle, state: State<AppState>) -> Result<String, String> {
    // Block if rename popup is open
    if app.get_webview_window("rename").is_some() {
        return Err("Please finish renaming the current screenshot first".to_string());
    }

    let settings = state.settings.lock().unwrap().clone();
    let filepath = generate_temp_screenshot_path("png");

    // Use macOS screencapture command
    // -x = no sound (full screen capture, no -i flag)
    let output = Command::new("screencapture")
        .args(["-x", &filepath])
        .output()
        .map_err(|e| format!("Failed to run screencapture: {}", e))?;

    if output.status.success() {
        if std::path::Path::new(&filepath).exists() {
            let optimized_path = optimize_screenshot(&filepath, &settings)?;
            let (width, height) = get_image_dimensions(&optimized_path)?;
            let extension = std::path::Path::new(&optimized_path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("png");
            let final_path = generate_screenshot_path(extension, &settings, width, height);
            std::fs::rename(&optimized_path, &final_path)
                .map_err(|e| format!("Failed to rename screenshot: {}", e))?;
            Ok(final_path)
        } else {
            Err("Screenshot cancelled".to_string())
        }
    } else {
        Err("Screenshot failed".to_string())
    }
}

#[tauri::command]
fn rename_screenshot(old_path: String, new_name: String) -> Result<String, String> {
    use std::path::Path;

    let old = Path::new(&old_path);

    // Get the directory and extension from the old path
    let dir = old.parent().ok_or("Invalid path")?;
    let ext = old.extension().and_then(|e| e.to_str()).unwrap_or("jpg");

    // Sanitize the new name - only remove macOS forbidden characters (/ and :)
    let sanitized: String = new_name
        .chars()
        .filter(|c| *c != '/' && *c != ':')
        .collect();

    let new_path = dir.join(format!("{}.{}", sanitized.trim(), ext));

    // Rename the file
    std::fs::rename(&old_path, &new_path)
        .map_err(|e| format!("Failed to rename: {}", e))?;

    Ok(new_path.to_string_lossy().to_string())
}

#[tauri::command]
fn read_image_base64(filepath: String) -> Result<String, String> {
    use base64::Engine;
    let bytes = std::fs::read(&filepath)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    let base64_data = base64::engine::general_purpose::STANDARD.encode(&bytes);
    
    // Determine MIME type from file extension
    let mime_type = if filepath.to_lowercase().ends_with(".jpg") 
        || filepath.to_lowercase().ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "image/png"
    };
    
    Ok(format!("data:{};base64,{}", mime_type, base64_data))
}

#[tauri::command]
fn save_edited_screenshot(filepath: String, base64_data: String) -> Result<String, String> {
    use base64::Engine;
    use std::io::Write;

    // Decode base64
    let bytes = base64::engine::general_purpose::STANDARD.decode(&base64_data)
        .map_err(|e| format!("Failed to decode base64: {}", e))?;

    // Write to file (overwrite original)
    let mut file = std::fs::File::create(&filepath)
        .map_err(|e| format!("Failed to create file: {}", e))?;
    file.write_all(&bytes)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(filepath)
}

// Rename popup: 380x140 fixed size
// Compact dimensions for filename input, note field, and shortcuts bar
#[tauri::command]
fn open_rename_popup(app: tauri::AppHandle, filepath: String) -> Result<(), String> {
    // URL encode the filepath for the query param
    let encoded_path = urlencoding::encode(&filepath);
    let url = format!("/rename.html?path={}", encoded_path);

    // Create compact popup window for renaming with preview
    WebviewWindowBuilder::new(&app, "rename", tauri::WebviewUrl::App(url.into()))
        .title("Screenshot")
        .inner_size(410.0, 141.0)
        .resizable(false)
        .always_on_top(true)
        .center()
        .focused(true)
        .decorations(false)
        .transparent(true)
        .build()
        .map_err(|e| format!("Failed to open rename window: {}", e))?;

    Ok(())
}

#[tauri::command]
fn close_rename_popup(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("rename") {
        let _ = window.close();
    }
}

#[tauri::command]
fn open_shortcut_config(
    app: tauri::AppHandle,
    target: String,
    current_shortcut: String,
    other_shortcut: String,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("shortcut-config") {
        let _ = window.close();
    }

    let url = format!(
        "/shortcut-config.html?target={}&current={}&other={}",
        urlencoding::encode(&target),
        urlencoding::encode(&current_shortcut),
        urlencoding::encode(&other_shortcut)
    );

    WebviewWindowBuilder::new(&app, "shortcut-config", tauri::WebviewUrl::App(url.into()))
        .title("Configure Shortcut")
        .inner_size(260.0, 180.0)
        .resizable(false)
        .always_on_top(true)
        .center()
        .focused(true)
        .decorations(false)
        .transparent(true)
        .build()
        .map_err(|e| format!("Failed to open shortcut config: {}", e))?;

    Ok(())
}

#[tauri::command]
fn close_shortcut_config(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("shortcut-config") {
        let _ = window.close();
    }
}

#[tauri::command]
fn delete_screenshot(app: tauri::AppHandle, filepath: String) -> Result<(), String> {
    // Delete the file
    std::fs::remove_file(&filepath)
        .map_err(|e| format!("Failed to delete: {}", e))?;

    // Close the rename popup
    if let Some(window) = app.get_webview_window("rename") {
        let _ = window.close();
    }

    // Close the editor window
    if let Some(window) = app.get_webview_window("editor") {
        let _ = window.close();
    }

    Ok(())
}

// Calculate padding for the editor based on image dimensions and whether it was resized
fn calculate_editor_padding(img_width: u32, img_height: u32, was_resized: bool) -> f64 {
    const TOOLBAR_HEIGHT: f64 = 72.0;
    const MAX_WIDTH: f64 = 1400.0;
    const MAX_HEIGHT: f64 = 900.0;
    const MAX_PADDING: f64 = 40.0;
    
    if was_resized {
        0.0
    } else {
        let img_w = img_width as f64;
        let img_h = img_height as f64;
        let fill_ratio_w = img_w / (MAX_WIDTH - MAX_PADDING);
        let fill_ratio_h = img_h / (MAX_HEIGHT - TOOLBAR_HEIGHT - MAX_PADDING);
        let fill_ratio = fill_ratio_w.max(fill_ratio_h).min(1.0);
        MAX_PADDING * (1.0 - fill_ratio)
    }
}

// Editor window: dynamically sized based on image dimensions
// Canvas scales within window (see editor.html resizeCanvas)
// Window size calculated to fit image with toolbar and padding
#[tauri::command]
fn open_editor_window(app: tauri::AppHandle, filepath: String, note: Option<String>, state: State<AppState>) -> Result<(), String> {
    // Close rename popup first
    if let Some(rename_window) = app.get_webview_window("rename") {
        let _ = rename_window.close();
    }

    // Get image dimensions and calculate appropriate window size
    let (img_width, img_height) = get_image_dimensions(&filepath).unwrap_or((800, 600));
    
    // Check if image was resized down by optimize_screenshot
    // If width matches max_width setting, it was originally larger (fullscreen/large capture)
    let settings = state.settings.lock().unwrap();
    let was_resized = settings.max_width > 0 && img_width == settings.max_width;
    drop(settings);
    
    let padding = calculate_editor_padding(img_width, img_height, was_resized);
    let (window_w, window_h) = calculate_editor_window_size(img_width, img_height, padding);

    let encoded_path = urlencoding::encode(&filepath);
    let note_value = note.unwrap_or_default();
    let encoded_note = urlencoding::encode(&note_value);
    let url = format!("/editor.html?path={}&padding={}&note={}", encoded_path, padding.round() as i32, encoded_note);

    WebviewWindowBuilder::new(&app, "editor", tauri::WebviewUrl::App(url.into()))
        .title("Edit Screenshot")
        .inner_size(window_w, window_h)
        .min_inner_size(500.0, 250.0)
        .resizable(true)
        .center()
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn close_editor_and_open_rename(app: tauri::AppHandle, filepath: String, note: Option<String>) -> Result<(), String> {
    // Close editor window first
    if let Some(editor_window) = app.get_webview_window("editor") {
        let _ = editor_window.close();
    }

    // Open rename popup with note preserved
    let encoded_path = urlencoding::encode(&filepath);
    let note_value = note.unwrap_or_default();
    let encoded_note = urlencoding::encode(&note_value);
    let url = format!("/rename.html?path={}&note={}", encoded_path, encoded_note);

    WebviewWindowBuilder::new(&app, "rename", tauri::WebviewUrl::App(url.into()))
        .title("Screenshot")
        .inner_size(410.0, 141.0)
        .resizable(false)
        .always_on_top(true)
        .center()
        .decorations(false)
        .transparent(true)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn close_editor_window(app: tauri::AppHandle) {
    if let Some(editor_window) = app.get_webview_window("editor") {
        let _ = editor_window.close();
    }
}

#[tauri::command]
fn copy_image_to_clipboard(base64_data: String) -> Result<(), String> {
    use arboard::{Clipboard, ImageData};
    use base64::Engine;

    // Decode base64 to PNG bytes
    let png_bytes = base64::engine::general_purpose::STANDARD
        .decode(&base64_data)
        .map_err(|e| format!("Failed to decode base64: {}", e))?;

    // Decode PNG to raw RGBA pixels
    let decoder = png::Decoder::new(std::io::Cursor::new(&png_bytes));
    let mut reader = decoder.read_info().map_err(|e| format!("Failed to read PNG info: {}", e))?;
    
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| format!("Failed to decode PNG frame: {}", e))?;
    
    // Ensure we have RGBA data
    let rgba_data = match info.color_type {
        png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
        png::ColorType::Rgb => {
            // Convert RGB to RGBA
            let rgb = &buf[..info.buffer_size()];
            let mut rgba = Vec::with_capacity(rgb.len() / 3 * 4);
            for chunk in rgb.chunks(3) {
                rgba.extend_from_slice(chunk);
                rgba.push(255); // Alpha
            }
            rgba
        }
        _ => return Err(format!("Unsupported color type: {:?}", info.color_type)),
    };

    let img_data = ImageData {
        width: info.width as usize,
        height: info.height as usize,
        bytes: rgba_data.into(),
    };

    let mut clipboard = Clipboard::new().map_err(|e| format!("Failed to access clipboard: {}", e))?;
    clipboard
        .set_image(img_data)
        .map_err(|e| format!("Failed to copy image to clipboard: {}", e))?;

    Ok(())
}

#[tauri::command]
fn copy_file_to_clipboard(filepath: String) -> Result<(), String> {
    use arboard::{Clipboard, ImageData};
    use std::fs::File;
    use std::io::BufReader;
    
    let file = File::open(&filepath)
        .map_err(|e| format!("Failed to open file: {}", e))?;
    let reader = BufReader::new(file);
    
    // Detect format by extension
    let is_jpeg = filepath.to_lowercase().ends_with(".jpg") 
        || filepath.to_lowercase().ends_with(".jpeg");
    
    let (width, height, rgba_data) = if is_jpeg {
        // Decode JPEG
        let mut decoder = jpeg_decoder::Decoder::new(reader);
        let pixels = decoder.decode()
            .map_err(|e| format!("Failed to decode JPEG: {}", e))?;
        let info = decoder.info().ok_or("Failed to get JPEG info")?;
        
        // Convert to RGBA
        let rgba = match info.pixel_format {
            jpeg_decoder::PixelFormat::RGB24 => {
                let mut rgba = Vec::with_capacity(pixels.len() / 3 * 4);
                for chunk in pixels.chunks(3) {
                    rgba.extend_from_slice(chunk);
                    rgba.push(255);
                }
                rgba
            }
            jpeg_decoder::PixelFormat::L8 => {
                let mut rgba = Vec::with_capacity(pixels.len() * 4);
                for &gray in &pixels {
                    rgba.extend_from_slice(&[gray, gray, gray, 255]);
                }
                rgba
            }
            _ => return Err("Unsupported JPEG pixel format".to_string()),
        };
        
        (info.width as usize, info.height as usize, rgba)
    } else {
        // Decode PNG
        let decoder = png::Decoder::new(reader);
        let mut reader = decoder.read_info()
            .map_err(|e| format!("Failed to read PNG info: {}", e))?;
        
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf)
            .map_err(|e| format!("Failed to decode PNG frame: {}", e))?;
        
        let rgba = match info.color_type {
            png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
            png::ColorType::Rgb => {
                let rgb = &buf[..info.buffer_size()];
                let mut rgba = Vec::with_capacity(rgb.len() / 3 * 4);
                for chunk in rgb.chunks(3) {
                    rgba.extend_from_slice(chunk);
                    rgba.push(255);
                }
                rgba
            }
            png::ColorType::Grayscale => {
                let gray = &buf[..info.buffer_size()];
                let mut rgba = Vec::with_capacity(gray.len() * 4);
                for &g in gray {
                    rgba.extend_from_slice(&[g, g, g, 255]);
                }
                rgba
            }
            png::ColorType::GrayscaleAlpha => {
                let ga = &buf[..info.buffer_size()];
                let mut rgba = Vec::with_capacity(ga.len() * 2);
                for chunk in ga.chunks(2) {
                    rgba.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
                }
                rgba
            }
            _ => return Err(format!("Unsupported PNG color type: {:?}", info.color_type)),
        };
        
        (info.width as usize, info.height as usize, rgba)
    };
    
    let img_data = ImageData {
        width,
        height,
        bytes: rgba_data.into(),
    };
    
    let mut clipboard = Clipboard::new().map_err(|e| format!("Failed to access clipboard: {}", e))?;
    clipboard
        .set_image(img_data)
        .map_err(|e| format!("Failed to copy image to clipboard: {}", e))?;
    
    Ok(())
}

#[derive(Clone, Copy)]
struct ShortcutParts {
    modifiers: Modifiers,
    key: Code,
}

fn parse_shortcut(shortcut_str: &str) -> Result<Shortcut, String> {
    let parts = parse_shortcut_parts(shortcut_str)?;
    Ok(Shortcut::new(Some(parts.modifiers), parts.key))
}

fn normalize_and_parse(shortcut_str: &str) -> Result<(String, Shortcut), String> {
    let parts = parse_shortcut_parts(shortcut_str)?;
    let normalized = shortcut_parts_to_string(&parts)?;
    Ok((normalized, Shortcut::new(Some(parts.modifiers), parts.key)))
}

fn normalize_shortcut_string(shortcut_str: &str) -> Result<String, String> {
    let parts = parse_shortcut_parts(shortcut_str)?;
    shortcut_parts_to_string(&parts)
}

fn parse_shortcut_parts(shortcut_str: &str) -> Result<ShortcutParts, String> {
    let parts: Vec<&str> = shortcut_str.split('+').collect();
    if parts.len() < 2 {
        return Err("Shortcut must have at least one modifier and one key".to_string());
    }

    let mut modifiers = Modifiers::empty();
    let mut key_code: Option<Code> = None;

    for part in parts {
        let token = part.trim();
        if token.is_empty() {
            return Err("Shortcut contains an empty token".to_string());
        }

        match token.to_lowercase().as_str() {
            "cmd" | "command" | "super" | "meta" => modifiers |= Modifiers::SUPER,
            "shift" => modifiers |= Modifiers::SHIFT,
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            _ => {
                if key_code.is_some() {
                    return Err("Shortcut must have only one main key".to_string());
                }
                key_code = Some(string_to_code(token)?);
            }
        }
    }

    let code = key_code.ok_or("No key specified in shortcut")?;
    if modifiers.is_empty() {
        return Err("At least one modifier required".to_string());
    }

    Ok(ShortcutParts { modifiers, key: code })
}

fn string_to_code(s: &str) -> Result<Code, String> {
    match s.to_lowercase().as_str() {
        "0" | "digit0" => Ok(Code::Digit0),
        "1" | "digit1" => Ok(Code::Digit1),
        "2" | "digit2" => Ok(Code::Digit2),
        "3" | "digit3" => Ok(Code::Digit3),
        "4" | "digit4" => Ok(Code::Digit4),
        "5" | "digit5" => Ok(Code::Digit5),
        "6" | "digit6" => Ok(Code::Digit6),
        "7" | "digit7" => Ok(Code::Digit7),
        "8" | "digit8" => Ok(Code::Digit8),
        "9" | "digit9" => Ok(Code::Digit9),
        "a" | "keya" => Ok(Code::KeyA),
        "b" | "keyb" => Ok(Code::KeyB),
        "c" | "keyc" => Ok(Code::KeyC),
        "d" | "keyd" => Ok(Code::KeyD),
        "e" | "keye" => Ok(Code::KeyE),
        "f" | "keyf" => Ok(Code::KeyF),
        "g" | "keyg" => Ok(Code::KeyG),
        "h" | "keyh" => Ok(Code::KeyH),
        "i" | "keyi" => Ok(Code::KeyI),
        "j" | "keyj" => Ok(Code::KeyJ),
        "k" | "keyk" => Ok(Code::KeyK),
        "l" | "keyl" => Ok(Code::KeyL),
        "m" | "keym" => Ok(Code::KeyM),
        "n" | "keyn" => Ok(Code::KeyN),
        "o" | "keyo" => Ok(Code::KeyO),
        "p" | "keyp" => Ok(Code::KeyP),
        "q" | "keyq" => Ok(Code::KeyQ),
        "r" | "keyr" => Ok(Code::KeyR),
        "s" | "keys" => Ok(Code::KeyS),
        "t" | "keyt" => Ok(Code::KeyT),
        "u" | "keyu" => Ok(Code::KeyU),
        "v" | "keyv" => Ok(Code::KeyV),
        "w" | "keyw" => Ok(Code::KeyW),
        "x" | "keyx" => Ok(Code::KeyX),
        "y" | "keyy" => Ok(Code::KeyY),
        "z" | "keyz" => Ok(Code::KeyZ),
        "f1" => Ok(Code::F1),
        "f2" => Ok(Code::F2),
        "f3" => Ok(Code::F3),
        "f4" => Ok(Code::F4),
        "f5" => Ok(Code::F5),
        "f6" => Ok(Code::F6),
        "f7" => Ok(Code::F7),
        "f8" => Ok(Code::F8),
        "f9" => Ok(Code::F9),
        "f10" => Ok(Code::F10),
        "f11" => Ok(Code::F11),
        "f12" => Ok(Code::F12),
        "space" => Ok(Code::Space),
        "enter" => Ok(Code::Enter),
        "tab" => Ok(Code::Tab),
        "escape" | "esc" => Ok(Code::Escape),
        "backspace" => Ok(Code::Backspace),
        "minus" | "-" => Ok(Code::Minus),
        "equal" | "equals" | "=" => Ok(Code::Equal),
        "bracketleft" | "lbracket" | "[" => Ok(Code::BracketLeft),
        "bracketright" | "rbracket" | "]" => Ok(Code::BracketRight),
        "semicolon" | ";" => Ok(Code::Semicolon),
        "quote" | "'" => Ok(Code::Quote),
        "comma" | "," => Ok(Code::Comma),
        "period" | "." => Ok(Code::Period),
        "slash" | "/" => Ok(Code::Slash),
        "backslash" | "\\" => Ok(Code::Backslash),
        "intlbackslash" => Ok(Code::IntlBackslash),
        "backquote" | "grave" | "`" => Ok(Code::Backquote),
        _ => Err(format!("Unknown key: {}", s)),
    }
}

fn code_to_string(code: Code) -> Result<String, String> {
    match code {
        Code::Digit0 => Ok("0".to_string()),
        Code::Digit1 => Ok("1".to_string()),
        Code::Digit2 => Ok("2".to_string()),
        Code::Digit3 => Ok("3".to_string()),
        Code::Digit4 => Ok("4".to_string()),
        Code::Digit5 => Ok("5".to_string()),
        Code::Digit6 => Ok("6".to_string()),
        Code::Digit7 => Ok("7".to_string()),
        Code::Digit8 => Ok("8".to_string()),
        Code::Digit9 => Ok("9".to_string()),
        Code::KeyA => Ok("A".to_string()),
        Code::KeyB => Ok("B".to_string()),
        Code::KeyC => Ok("C".to_string()),
        Code::KeyD => Ok("D".to_string()),
        Code::KeyE => Ok("E".to_string()),
        Code::KeyF => Ok("F".to_string()),
        Code::KeyG => Ok("G".to_string()),
        Code::KeyH => Ok("H".to_string()),
        Code::KeyI => Ok("I".to_string()),
        Code::KeyJ => Ok("J".to_string()),
        Code::KeyK => Ok("K".to_string()),
        Code::KeyL => Ok("L".to_string()),
        Code::KeyM => Ok("M".to_string()),
        Code::KeyN => Ok("N".to_string()),
        Code::KeyO => Ok("O".to_string()),
        Code::KeyP => Ok("P".to_string()),
        Code::KeyQ => Ok("Q".to_string()),
        Code::KeyR => Ok("R".to_string()),
        Code::KeyS => Ok("S".to_string()),
        Code::KeyT => Ok("T".to_string()),
        Code::KeyU => Ok("U".to_string()),
        Code::KeyV => Ok("V".to_string()),
        Code::KeyW => Ok("W".to_string()),
        Code::KeyX => Ok("X".to_string()),
        Code::KeyY => Ok("Y".to_string()),
        Code::KeyZ => Ok("Z".to_string()),
        Code::F1 => Ok("F1".to_string()),
        Code::F2 => Ok("F2".to_string()),
        Code::F3 => Ok("F3".to_string()),
        Code::F4 => Ok("F4".to_string()),
        Code::F5 => Ok("F5".to_string()),
        Code::F6 => Ok("F6".to_string()),
        Code::F7 => Ok("F7".to_string()),
        Code::F8 => Ok("F8".to_string()),
        Code::F9 => Ok("F9".to_string()),
        Code::F10 => Ok("F10".to_string()),
        Code::F11 => Ok("F11".to_string()),
        Code::F12 => Ok("F12".to_string()),
        Code::Space => Ok("Space".to_string()),
        Code::Enter => Ok("Enter".to_string()),
        Code::Tab => Ok("Tab".to_string()),
        Code::Escape => Ok("Escape".to_string()),
        Code::Backspace => Ok("Backspace".to_string()),
        Code::Minus => Ok("-".to_string()),
        Code::Equal => Ok("=".to_string()),
        Code::BracketLeft => Ok("[".to_string()),
        Code::BracketRight => Ok("]".to_string()),
        Code::Semicolon => Ok(";".to_string()),
        Code::Quote => Ok("'".to_string()),
        Code::Comma => Ok(",".to_string()),
        Code::Period => Ok(".".to_string()),
        Code::Slash => Ok("/".to_string()),
        Code::Backslash => Ok("\\".to_string()),
        Code::IntlBackslash => Ok("\\".to_string()),
        Code::Backquote => Ok("`".to_string()),
        _ => Err(format!("Unsupported key: {:?}", code)),
    }
}

fn shortcut_parts_to_string(parts: &ShortcutParts) -> Result<String, String> {
    let mut tokens = Vec::new();

    if parts.modifiers.contains(Modifiers::SUPER) {
        tokens.push("Cmd".to_string());
    }
    if parts.modifiers.contains(Modifiers::SHIFT) {
        tokens.push("Shift".to_string());
    }
    if parts.modifiers.contains(Modifiers::ALT) {
        tokens.push("Alt".to_string());
    }
    if parts.modifiers.contains(Modifiers::CONTROL) {
        tokens.push("Ctrl".to_string());
    }

    tokens.push(code_to_string(parts.key)?);
    Ok(tokens.join("+"))
}

fn shortcut_to_display(shortcut_str: &str) -> String {
    let normalized = normalize_shortcut_string(shortcut_str).unwrap_or_else(|_| shortcut_str.to_string());
    normalized
        .replace("Cmd", "⌘")
        .replace("Shift", "⇧")
        .replace("Alt", "⌥")
        .replace("Ctrl", "⌃")
        .replace("+", "")
}

fn build_tray_menu<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    settings: &Settings,
) -> tauri::Result<Menu<R>> {
    let full_display = shortcut_to_display(&settings.fullscreen_shortcut);
    let area_display = shortcut_to_display(&settings.area_shortcut);

    let screenshot_i = MenuItem::with_id(
        app,
        "screenshot",
        format!("Screenshot Area ({})", area_display),
        true,
        None::<&str>,
    )?;
    let fullscreen_i = MenuItem::with_id(
        app,
        "fullscreen",
        format!("Screenshot Full ({})", full_display),
        true,
        None::<&str>,
    )?;
    let show_i = MenuItem::with_id(app, "show", "Show App", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    Menu::with_items(app, &[&fullscreen_i, &screenshot_i, &show_i, &quit_i])
}

fn update_tray_labels(app: &tauri::AppHandle) -> Result<(), String> {
    let settings = app.state::<AppState>().settings.lock().unwrap().clone();
    let menu = build_tray_menu(app, &settings).map_err(|e| e.to_string())?;

    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut initial_settings = load_settings_from_file();
    let mut settings_changed = false;

    let (mut shortcut_full, mut shortcut_area);

    match normalize_and_parse(&initial_settings.fullscreen_shortcut) {
        Ok((normalized, shortcut)) => {
            if normalized != initial_settings.fullscreen_shortcut {
                initial_settings.fullscreen_shortcut = normalized;
                settings_changed = true;
            }
            shortcut_full = shortcut;
        }
        Err(_) => {
            initial_settings.fullscreen_shortcut = default_fullscreen_shortcut();
            settings_changed = true;
            let (normalized, shortcut) = normalize_and_parse(&initial_settings.fullscreen_shortcut)
                .unwrap_or_else(|_| {
                    (
                        default_fullscreen_shortcut(),
                        Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Digit3),
                    )
                });
            initial_settings.fullscreen_shortcut = normalized;
            shortcut_full = shortcut;
        }
    }

    match normalize_and_parse(&initial_settings.area_shortcut) {
        Ok((normalized, shortcut)) => {
            if normalized != initial_settings.area_shortcut {
                initial_settings.area_shortcut = normalized;
                settings_changed = true;
            }
            shortcut_area = shortcut;
        }
        Err(_) => {
            initial_settings.area_shortcut = default_area_shortcut();
            settings_changed = true;
            let (normalized, shortcut) = normalize_and_parse(&initial_settings.area_shortcut)
                .unwrap_or_else(|_| {
                    (
                        default_area_shortcut(),
                        Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Digit4),
                    )
                });
            initial_settings.area_shortcut = normalized;
            shortcut_area = shortcut;
        }
    }

    if shortcut_full.id() == shortcut_area.id() {
        initial_settings.fullscreen_shortcut = default_fullscreen_shortcut();
        initial_settings.area_shortcut = default_area_shortcut();
        settings_changed = true;

        let (normalized_full, full) = normalize_and_parse(&initial_settings.fullscreen_shortcut)
            .unwrap_or_else(|_| {
                (
                    default_fullscreen_shortcut(),
                    Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Digit3),
                )
            });
        let (normalized_area, area) = normalize_and_parse(&initial_settings.area_shortcut)
            .unwrap_or_else(|_| {
                (
                    default_area_shortcut(),
                    Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Digit4),
                )
            });
        initial_settings.fullscreen_shortcut = normalized_full;
        initial_settings.area_shortcut = normalized_area;
        shortcut_full = full;
        shortcut_area = area;
    }

    if settings_changed {
        let _ = save_settings_to_file(&initial_settings);
    }

    tauri::Builder::default()
        .manage(AppState {
            settings: Mutex::new(initial_settings),
            active_fullscreen_shortcut: Mutex::new(shortcut_full),
            active_area_shortcut: Mutex::new(shortcut_area),
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcuts([shortcut_area, shortcut_full])
                .unwrap()
                .with_handler(move |app, shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        if app.get_webview_window("shortcut-config").is_some() {
                            return;
                        }
                        let state = app.state::<AppState>();
                        let fullscreen_shortcut = *state.active_fullscreen_shortcut.lock().unwrap();
                        let area_shortcut = *state.active_area_shortcut.lock().unwrap();

                        if shortcut.id() == area_shortcut.id() {
                            // Cmd+Shift+4: Select area screenshot
                            let _ = app.emit("take-screenshot", ());
                        } else if shortcut.id() == fullscreen_shortcut.id() {
                            // Cmd+Shift+3: Full screen screenshot
                            let _ = app.emit("take-fullscreen-screenshot", ());
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            let settings = app.state::<AppState>().settings.lock().unwrap().clone();
            let menu = build_tray_menu(app.handle(), &settings)?;

            // Build the tray icon
            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "screenshot" => {
                        let _ = app.emit("take-screenshot", ());
                    }
                    "fullscreen" => {
                        let _ = app.emit("take-fullscreen-screenshot", ());
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![take_screenshot, take_fullscreen_screenshot, rename_screenshot, save_edited_screenshot, read_image_base64, open_rename_popup, close_rename_popup, delete_screenshot, open_editor_window, close_editor_and_open_rename, close_editor_window, copy_image_to_clipboard, copy_file_to_clipboard, get_settings, save_settings, update_shortcuts, open_shortcut_config, close_shortcut_config])
        .on_window_event(|window, event| {
            // Only prevent close for main window, let rename popup close normally
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    // Prevent the window from closing, just hide it instead
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
