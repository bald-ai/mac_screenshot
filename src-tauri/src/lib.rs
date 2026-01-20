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
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};

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
}

fn default_theme() -> String {
    "system".to_string()
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
        }
    }
}

pub struct AppState {
    pub settings: Mutex<Settings>,
}

fn get_settings_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(home).join(".screenshot_app_settings.json")
}

fn load_settings_from_file() -> Settings {
    let path = get_settings_path();
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(settings) = serde_json::from_str(&content) {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Define the shortcuts
    let shortcut_area = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Digit4);
    let shortcut_full = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Digit3);
    let shortcut_focus_rename = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyF);

    tauri::Builder::default()
        .manage(AppState {
            settings: Mutex::new(load_settings_from_file()),
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcuts([shortcut_area, shortcut_full, shortcut_focus_rename])
                .unwrap()
                .with_handler(move |app, shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        if shortcut.key == Code::Digit4 {
                            // Cmd+Shift+4: Select area screenshot
                            let _ = app.emit("take-screenshot", ());
                        } else if shortcut.key == Code::Digit3 {
                            // Cmd+Shift+3: Full screen screenshot
                            let _ = app.emit("take-fullscreen-screenshot", ());
                        } else if shortcut.key == Code::KeyF {
                            // Cmd+Shift+F: Focus rename window
                            if let Some(window) = app.get_webview_window("rename") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            // Create menu items
            let screenshot_i = MenuItem::with_id(app, "screenshot", "Screenshot Area (⌘⇧4)", true, None::<&str>)?;
            let fullscreen_i = MenuItem::with_id(app, "fullscreen", "Screenshot Full (⌘⇧3)", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Show App", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            // Create the menu
            let menu = Menu::with_items(app, &[&screenshot_i, &fullscreen_i, &show_i, &quit_i])?;

            // Build the tray icon
            TrayIconBuilder::new()
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
        .invoke_handler(tauri::generate_handler![take_screenshot, take_fullscreen_screenshot, rename_screenshot, save_edited_screenshot, read_image_base64, open_rename_popup, close_rename_popup, delete_screenshot, open_editor_window, close_editor_and_open_rename, close_editor_window, copy_image_to_clipboard, copy_file_to_clipboard, get_settings, save_settings])
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
