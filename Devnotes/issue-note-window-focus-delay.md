# Note Window Focus Delay (~2 seconds)

## Problem
When switching from Rename to Note window (pressing Tab), it takes ~2 seconds for the Note window to receive focus. However, opening Editor from Note (quick tap tap) happens instantly.

## Root Cause
The issue is **window ordering**: `open_note_popup` closes the rename window *before* the note window becomes key, which triggers macOS activation arbitration.

Current flow:
1. Close rename (leaves app with no key window)
2. Build note with `.focused(true)` + `.always_on_top(true)`

macOS sees "reactivation from no key window" → ~2s delay due to focus-steal prevention.

**Editor is instant** because it's opened while the app is already active (user is interacting with Note), so no activation penalty.

## Affected Code
- `src-tauri/src/lib.rs` lines 818-841 (`open_note_popup`)
- `src-tauri/src/lib.rs` lines 851-876 (`close_note_and_open_rename`) - same pattern

## Proposed Fix
Change the order: **build the new window first, focus it, then close the old one**.

```rust
#[tauri::command]
fn open_note_popup(app: tauri::AppHandle, filepath: String, note: Option<String>, burned_note: Option<String>) -> Result<(), String> {
    // 1. Build note first (without .focused(true) or .always_on_top(true))
    let encoded_path = urlencoding::encode(&filepath);
    let encoded_note = urlencoding::encode(&note.unwrap_or_default());
    let encoded_burned_note = urlencoding::encode(&burned_note.unwrap_or_default());
    let url = format!("/note.html?path={}&note={}&burnedNote={}", encoded_path, encoded_note, encoded_burned_note);

    let note_window = WebviewWindowBuilder::new(&app, "note", tauri::WebviewUrl::App(url.into()))
        .title("Note")
        .inner_size(410.0, 120.0)
        .resizable(false)
        .center()
        .build()
        .map_err(|e| format!("Failed to open note window: {}", e))?;

    // 2. Focus + always-on-top AFTER build
    let _ = note_window.set_focus();
    let _ = note_window.set_always_on_top(true);

    // 3. NOW close rename
    if let Some(rename_window) = app.get_webview_window("rename") {
        let _ = rename_window.close();
    }

    Ok(())
}
```

Apply same pattern to `close_note_and_open_rename`.

## Effort
S (<1 hour)
