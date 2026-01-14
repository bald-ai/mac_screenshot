use std::process::Command;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    webview::WebviewWindowBuilder,
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};

fn generate_screenshot_path(extension: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    format!("{}/Desktop/screenshot_{}.{}", home, timestamp, extension)
}

fn optimize_screenshot(filepath: &str) -> Result<String, String> {
    // Convert to JPEG with 85% quality and resize to max 1920px width
    let jpeg_path = filepath.replace(".png", ".jpg");

    // Use sips to resize (if wider than 1920px) and convert to JPEG
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

    // Resize if wider than 1920px
    if width > 1920 {
        Command::new("sips")
            .args(["--resampleWidth", "1920", filepath])
            .output()
            .map_err(|e| format!("Failed to resize: {}", e))?;
    }

    // Convert to JPEG with quality 85
    let output = Command::new("sips")
        .args(["-s", "format", "jpeg", "-s", "formatOptions", "85", filepath, "--out", &jpeg_path])
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
fn take_screenshot(app: tauri::AppHandle) -> Result<String, String> {
    // Block if rename popup is open
    if app.get_webview_window("rename").is_some() {
        return Err("Please finish renaming the current screenshot first".to_string());
    }

    let filepath = generate_screenshot_path("png");

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
            optimize_screenshot(&filepath)
        } else {
            Err("Screenshot cancelled".to_string())
        }
    } else {
        Err("Screenshot cancelled or failed".to_string())
    }
}

#[tauri::command]
fn take_fullscreen_screenshot(app: tauri::AppHandle) -> Result<String, String> {
    // Block if rename popup is open
    if app.get_webview_window("rename").is_some() {
        return Err("Please finish renaming the current screenshot first".to_string());
    }

    let filepath = generate_screenshot_path("png");

    // Use macOS screencapture command
    // -x = no sound (full screen capture, no -i flag)
    let output = Command::new("screencapture")
        .args(["-x", &filepath])
        .output()
        .map_err(|e| format!("Failed to run screencapture: {}", e))?;

    if output.status.success() {
        optimize_screenshot(&filepath)
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
    Ok(format!("data:image/png;base64,{}", base64_data))
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

#[tauri::command]
fn open_rename_popup(app: tauri::AppHandle, filepath: String) -> Result<(), String> {
    // URL encode the filepath for the query param
    let encoded_path = urlencoding::encode(&filepath);
    let url = format!("/rename.html?path={}", encoded_path);

    // Create compact popup window for renaming with preview
    WebviewWindowBuilder::new(&app, "rename", tauri::WebviewUrl::App(url.into()))
        .title("Screenshot")
        .inner_size(340.0, 72.0)
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

#[tauri::command]
fn open_editor_window(app: tauri::AppHandle, filepath: String) -> Result<(), String> {
    // Close rename popup first
    if let Some(rename_window) = app.get_webview_window("rename") {
        let _ = rename_window.close();
    }

    let encoded_path = urlencoding::encode(&filepath);
    let url = format!("/editor.html?path={}", encoded_path);

    WebviewWindowBuilder::new(&app, "editor", tauri::WebviewUrl::App(url.into()))
        .title("Edit Screenshot")
        .inner_size(1200.0, 800.0)
        .resizable(true)
        .center()
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn close_editor_and_open_rename(app: tauri::AppHandle, filepath: String) -> Result<(), String> {
    // Close editor window first
    if let Some(editor_window) = app.get_webview_window("editor") {
        let _ = editor_window.close();
    }

    // Open rename popup
    let encoded_path = urlencoding::encode(&filepath);
    let url = format!("/rename.html?path={}", encoded_path);

    WebviewWindowBuilder::new(&app, "rename", tauri::WebviewUrl::App(url.into()))
        .title("Screenshot")
        .inner_size(340.0, 72.0)
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Define the shortcuts
    let shortcut_area = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Digit4);
    let shortcut_full = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Digit3);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcuts([shortcut_area, shortcut_full])
                .unwrap()
                .with_handler(move |app, shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        if shortcut.key == Code::Digit4 {
                            // Cmd+Shift+4: Select area screenshot
                            let _ = app.emit("take-screenshot", ());
                        } else if shortcut.key == Code::Digit3 {
                            // Cmd+Shift+3: Full screen screenshot
                            let _ = app.emit("take-fullscreen-screenshot", ());
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
        .invoke_handler(tauri::generate_handler![take_screenshot, take_fullscreen_screenshot, rename_screenshot, save_edited_screenshot, read_image_base64, open_rename_popup, close_rename_popup, delete_screenshot, open_editor_window, close_editor_and_open_rename, close_editor_window, copy_image_to_clipboard])
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
