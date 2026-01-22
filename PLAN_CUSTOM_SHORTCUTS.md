# Custom Keyboard Shortcuts Implementation Plan

## Objective

Add user-configurable global shortcuts for fullscreen (default ⌘⇧3) and area (default ⌘⇧4) screenshot actions via a popup editor window.

## Requirements

- Click shortcut box in settings → popup opens → record keys → Save/Cancel
- Duplicate shortcuts: show error, block save
- Reset to defaults button
- Persist across restarts
- On registration failure: show error, keep old shortcut

---

## Files to Modify

| File | Changes |
|------|---------|
| `src-tauri/src/lib.rs` | Settings struct, shortcut parsing, dynamic registration, update command, tray labels |
| `src/App.tsx` | Settings interface, clickable shortcut boxes, event listener |
| `src/App.css` | Shortcut box styles |
| `src-tauri/capabilities/default.json` | Add `"shortcut-config"` window |

## Files to Create

| File | Purpose |
|------|---------|
| `shortcut-config.html` | Popup for recording shortcuts (at repo root, like `rename.html`) |
| `shortcut-config.css` | Popup styling (in `src/` or root) |

---

## Step 1: Extend Settings Struct

**File:** `src-tauri/src/lib.rs` (lines 44-57)

Add two string fields with serde defaults for backward compatibility:

```rust
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

fn default_fullscreen_shortcut() -> String {
    "Cmd+Shift+3".to_string()
}

fn default_area_shortcut() -> String {
    "Cmd+Shift+4".to_string()
}
```

Update `Default` impl (lines 63-74):
```rust
impl Default for Settings {
    fn default() -> Self {
        Self {
            // ... existing fields ...
            fullscreen_shortcut: "Cmd+Shift+3".to_string(),
            area_shortcut: "Cmd+Shift+4".to_string(),
        }
    }
}
```

---

## Step 2: Add Shortcut Parsing Helpers

**File:** `src-tauri/src/lib.rs` (add before `run()` function, around line 710)

```rust
use std::collections::HashSet;

fn parse_shortcut(shortcut_str: &str) -> Result<Shortcut, String> {
    let parts: Vec<&str> = shortcut_str.split('+').collect();
    if parts.len() < 2 {
        return Err("Shortcut must have at least one modifier and one key".to_string());
    }

    let mut modifiers = Modifiers::empty();
    let mut key_code: Option<Code> = None;

    for part in parts {
        match part.to_lowercase().as_str() {
            "cmd" | "super" | "meta" => modifiers |= Modifiers::SUPER,
            "shift" => modifiers |= Modifiers::SHIFT,
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            _ => {
                // This is the key
                key_code = Some(string_to_code(part)?);
            }
        }
    }

    let code = key_code.ok_or("No key specified in shortcut")?;
    if modifiers.is_empty() {
        return Err("At least one modifier required".to_string());
    }

    Ok(Shortcut::new(Some(modifiers), code))
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
        _ => Err(format!("Unknown key: {}", s)),
    }
}

fn shortcut_to_display(shortcut_str: &str) -> String {
    shortcut_str
        .replace("Cmd", "⌘")
        .replace("Shift", "⇧")
        .replace("Alt", "⌥")
        .replace("Ctrl", "⌃")
        .replace("+", "")
}
```

---

## Step 3: Add AppState Fields for Active Shortcuts

**File:** `src-tauri/src/lib.rs` (line 76-78)

Extend `AppState` to track currently registered shortcuts for handler comparison:

```rust
pub struct AppState {
    pub settings: Mutex<Settings>,
    pub active_fullscreen_code: Mutex<Code>,
    pub active_area_code: Mutex<Code>,
}
```

---

## Step 4: Refactor run() for Dynamic Registration

**File:** `src-tauri/src/lib.rs` (lines 714-749)

Replace hardcoded shortcuts with settings-based registration:

```rust
pub fn run() {
    let initial_settings = load_settings_from_file();
    
    // Parse shortcuts from settings
    let shortcut_full = parse_shortcut(&initial_settings.fullscreen_shortcut)
        .unwrap_or_else(|_| Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Digit3));
    let shortcut_area = parse_shortcut(&initial_settings.area_shortcut)
        .unwrap_or_else(|_| Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Digit4));
    let shortcut_focus_rename = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyF);

    let fullscreen_code = shortcut_full.key;
    let area_code = shortcut_area.key;

    tauri::Builder::default()
        .manage(AppState {
            settings: Mutex::new(initial_settings),
            active_fullscreen_code: Mutex::new(fullscreen_code),
            active_area_code: Mutex::new(area_code),
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
                        let state = app.state::<AppState>();
                        let fullscreen_code = *state.active_fullscreen_code.lock().unwrap();
                        let area_code = *state.active_area_code.lock().unwrap();
                        
                        if shortcut.key == area_code {
                            let _ = app.emit("take-screenshot", ());
                        } else if shortcut.key == fullscreen_code {
                            let _ = app.emit("take-fullscreen-screenshot", ());
                        } else if shortcut.key == Code::KeyF {
                            if let Some(window) = app.get_webview_window("rename") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(),
        )
        // ... rest of setup
```

---

## Step 5: Add update_shortcuts Command

**File:** `src-tauri/src/lib.rs` (add near other commands, around line 114)

```rust
use tauri_plugin_global_shortcut::GlobalShortcutExt;

#[tauri::command]
async fn update_shortcuts(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    fullscreen_shortcut: String,
    area_shortcut: String,
) -> Result<(), String> {
    // Validate: not duplicates
    if fullscreen_shortcut == area_shortcut {
        return Err("Fullscreen and area shortcuts must be different".to_string());
    }

    // Parse new shortcuts
    let new_full = parse_shortcut(&fullscreen_shortcut)?;
    let new_area = parse_shortcut(&area_shortcut)?;

    // Get current shortcuts for rollback
    let mut settings = state.settings.lock().unwrap();
    let old_full_str = settings.fullscreen_shortcut.clone();
    let old_area_str = settings.area_shortcut.clone();
    let old_full = parse_shortcut(&old_full_str).ok();
    let old_area = parse_shortcut(&old_area_str).ok();

    let global_shortcut = app.global_shortcut();

    // Unregister old shortcuts
    if let Some(ref s) = old_full {
        let _ = global_shortcut.unregister(s.clone());
    }
    if let Some(ref s) = old_area {
        let _ = global_shortcut.unregister(s.clone());
    }

    // Try to register new shortcuts
    if let Err(e) = global_shortcut.register(new_full.clone()) {
        // Rollback: re-register old
        if let Some(ref s) = old_full {
            let _ = global_shortcut.register(s.clone());
        }
        if let Some(ref s) = old_area {
            let _ = global_shortcut.register(s.clone());
        }
        return Err(format!("Failed to register fullscreen shortcut: {}", e));
    }

    if let Err(e) = global_shortcut.register(new_area.clone()) {
        // Rollback: unregister new_full, re-register old
        let _ = global_shortcut.unregister(new_full);
        if let Some(ref s) = old_full {
            let _ = global_shortcut.register(s.clone());
        }
        if let Some(ref s) = old_area {
            let _ = global_shortcut.register(s.clone());
        }
        return Err(format!("Failed to register area shortcut: {}", e));
    }

    // Success: update state
    settings.fullscreen_shortcut = fullscreen_shortcut;
    settings.area_shortcut = area_shortcut;
    
    // Update active codes for handler
    *state.active_fullscreen_code.lock().unwrap() = new_full.key;
    *state.active_area_code.lock().unwrap() = new_area.key;

    // Persist to file
    save_settings_to_file(&settings)?;
    drop(settings);

    // Update tray menu labels
    update_tray_labels(&app)?;

    Ok(())
}
```

---

## Step 6: Add Tray Label Update Function

**File:** `src-tauri/src/lib.rs` (add near tray setup)

```rust
fn update_tray_labels(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let settings = state.settings.lock().unwrap();
    
    let full_display = shortcut_to_display(&settings.fullscreen_shortcut);
    let area_display = shortcut_to_display(&settings.area_shortcut);

    // Get tray and update menu items
    if let Some(tray) = app.tray_by_id("main") {
        if let Some(menu) = tray.get_menu() {
            if let Some(item) = menu.get("fullscreen") {
                if let Some(menu_item) = item.as_menuitem() {
                    let _ = menu_item.set_text(format!("Screenshot Full ({})", full_display));
                }
            }
            if let Some(item) = menu.get("screenshot") {
                if let Some(menu_item) = item.as_menuitem() {
                    let _ = menu_item.set_text(format!("Screenshot Area ({})", area_display));
                }
            }
        }
    }

    Ok(())
}
```

**Note:** Ensure tray is created with an ID. Update line ~760:
```rust
TrayIconBuilder::new()
    .id("main")  // ADD THIS
    .menu(&menu)
    // ...
```

---

## Step 7: Add open_shortcut_config Command

**File:** `src-tauri/src/lib.rs`

```rust
#[tauri::command]
fn open_shortcut_config(
    app: tauri::AppHandle,
    target: String,           // "fullscreen" or "area"
    current_shortcut: String,
    other_shortcut: String,   // for duplicate validation
) -> Result<(), String> {
    let url = format!(
        "/shortcut-config.html?target={}&current={}&other={}",
        urlencoding::encode(&target),
        urlencoding::encode(&current_shortcut),
        urlencoding::encode(&other_shortcut)
    );

    WebviewWindowBuilder::new(&app, "shortcut-config", tauri::WebviewUrl::App(url.into()))
        .title("Configure Shortcut")
        .inner_size(320.0, 180.0)
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
```

Register commands in `.invoke_handler()`:
```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    update_shortcuts,
    open_shortcut_config,
    close_shortcut_config,
])
```

---

## Step 8: Update Capabilities

**File:** `src-tauri/capabilities/default.json` (line 5)

```json
"windows": ["main", "rename", "shortcut-config"],
```

---

## Step 9: Create Shortcut Config Popup

**File:** `shortcut-config.html` (at repo root, same level as `rename.html`)

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Configure Shortcut</title>
  <link rel="stylesheet" href="/shortcut-config.css" />
  <script type="module">
    import { invoke } from '@tauri-apps/api/core';
    
    // Apply theme (same pattern as rename.html)
    invoke('get_settings').then(settings => {
      const isDark = settings.theme === 'dark' || 
        (settings.theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
      document.body.classList.add(isDark ? 'theme-dark' : 'theme-grey');
    }).catch(() => document.body.classList.add('theme-dark'));
  </script>
</head>
<body class="theme-dark">
  <div class="shortcut-config-frame">
    <div class="title">Press new shortcut</div>
    <div class="recording-area" id="recordingArea">
      <span id="shortcutDisplay">...</span>
    </div>
    <div class="error-message" id="errorMessage"></div>
    <div class="button-row">
      <button id="saveBtn" disabled>Save</button>
      <button id="cancelBtn">Cancel</button>
    </div>
  </div>

  <script type="module">
    import { invoke } from '@tauri-apps/api/core';
    import { emit } from '@tauri-apps/api/event';

    const params = new URLSearchParams(window.location.search);
    const target = params.get('target') || 'fullscreen';
    const currentShortcut = params.get('current') || '';
    const otherShortcut = params.get('other') || '';

    const display = document.getElementById('shortcutDisplay');
    const errorMsg = document.getElementById('errorMessage');
    const saveBtn = document.getElementById('saveBtn');
    const cancelBtn = document.getElementById('cancelBtn');

    let recordedShortcut = '';

    function formatForDisplay(shortcut) {
      return shortcut
        .replace(/Cmd/g, '⌘')
        .replace(/Shift/g, '⇧')
        .replace(/Alt/g, '⌥')
        .replace(/Ctrl/g, '⌃')
        .replace(/\+/g, '');
    }

    function keyEventToShortcut(e) {
      const parts = [];
      if (e.metaKey) parts.push('Cmd');
      if (e.ctrlKey) parts.push('Ctrl');
      if (e.altKey) parts.push('Alt');
      if (e.shiftKey) parts.push('Shift');

      // Ignore if only modifier keys pressed
      const modifierKeys = ['Meta', 'Control', 'Alt', 'Shift'];
      if (modifierKeys.includes(e.key)) return null;

      // Map key to backend format
      let key = e.code;
      if (key.startsWith('Digit')) key = key.replace('Digit', '');
      else if (key.startsWith('Key')) key = key.replace('Key', '');
      
      parts.push(key);
      return parts.join('+');
    }

    document.addEventListener('keydown', (e) => {
      e.preventDefault();
      const shortcut = keyEventToShortcut(e);
      if (!shortcut) return;

      recordedShortcut = shortcut;
      display.textContent = formatForDisplay(shortcut);

      // Validate: not duplicate
      if (shortcut === otherShortcut) {
        errorMsg.textContent = 'This shortcut is already used';
        saveBtn.disabled = true;
      } else {
        errorMsg.textContent = '';
        saveBtn.disabled = false;
      }
    });

    saveBtn.addEventListener('click', async () => {
      if (!recordedShortcut || saveBtn.disabled) return;
      
      // Emit event to main window with new shortcut
      await emit('shortcut-configured', { target, shortcut: recordedShortcut });
      await invoke('close_shortcut_config');
    });

    cancelBtn.addEventListener('click', async () => {
      await invoke('close_shortcut_config');
    });

    // ESC to cancel
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') {
        invoke('close_shortcut_config');
      }
    });
  </script>
</body>
</html>
```

---

## Step 10: Create Popup Styles

**File:** `shortcut-config.css` (at repo root or in `src/`)

```css
* { margin: 0; padding: 0; box-sizing: border-box; }

body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  -webkit-user-select: none;
  user-select: none;
}

body.theme-dark {
  --bg: #1a1a1a;
  --text: #e0e0e0;
  --border: #333;
  --accent: #4a9eff;
  --error: #ff6b6b;
}

body.theme-grey {
  --bg: #f5f5f5;
  --text: #333;
  --border: #ccc;
  --accent: #0066cc;
  --error: #cc3333;
}

.shortcut-config-frame {
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  align-items: center;
}

.title {
  color: var(--text);
  font-size: 14px;
  opacity: 0.7;
}

.recording-area {
  background: rgba(128, 128, 128, 0.1);
  border: 2px dashed var(--border);
  border-radius: 8px;
  padding: 24px 48px;
  min-width: 200px;
  text-align: center;
}

#shortcutDisplay {
  font-size: 28px;
  font-weight: 600;
  color: var(--text);
  letter-spacing: 2px;
}

.error-message {
  color: var(--error);
  font-size: 12px;
  min-height: 16px;
}

.button-row {
  display: flex;
  gap: 12px;
}

button {
  padding: 8px 20px;
  border-radius: 6px;
  border: none;
  font-size: 13px;
  cursor: pointer;
  transition: opacity 0.2s;
}

button:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

#saveBtn {
  background: var(--accent);
  color: white;
}

#cancelBtn {
  background: var(--border);
  color: var(--text);
}
```

---

## Step 11: Update Frontend Settings UI

**File:** `src/App.tsx`

### 11.1 Update Settings interface (line 19-26):

```typescript
interface Settings {
  quality: number;
  maxWidth: number;
  notePrefixEnabled: boolean;
  notePrefix: string;
  filenameTemplate: FilenameTemplate;
  theme: "grey" | "dark" | "system";
  fullscreenShortcut: string;
  areaShortcut: string;
}
```

### 11.2 Update initial state (line 49):

```typescript
const [settings, setSettings] = useState<Settings>({
  quality: 20,
  maxWidth: 1280,
  notePrefixEnabled: false,
  notePrefix: "",
  filenameTemplate: DEFAULT_FILENAME_TEMPLATE,
  theme: "system",
  fullscreenShortcut: "Cmd+Shift+3",
  areaShortcut: "Cmd+Shift+4",
});
```

### 11.3 Add event listener for shortcut-configured (in useEffect):

```typescript
useEffect(() => {
  // ... existing code ...
  
  const unlistenShortcut = listen<{ target: string; shortcut: string }>(
    "shortcut-configured",
    async (event) => {
      const { target, shortcut } = event.payload;
      const newSettings = { ...settings };
      
      if (target === "fullscreen") {
        newSettings.fullscreenShortcut = shortcut;
      } else {
        newSettings.areaShortcut = shortcut;
      }
      
      try {
        await invoke("update_shortcuts", {
          fullscreenShortcut: newSettings.fullscreenShortcut,
          areaShortcut: newSettings.areaShortcut,
        });
        setSettings(newSettings);
      } catch (e) {
        console.error("Failed to update shortcuts:", e);
        // Could show error toast here
      }
    }
  );

  return () => {
    unlistenShortcut.then((fn) => fn());
  };
}, [settings]);
```

### 11.4 Replace static shortcuts hint (lines 274-276):

```tsx
<div className="shortcuts-config">
  <div
    className="shortcut-box"
    onClick={() =>
      invoke("open_shortcut_config", {
        target: "fullscreen",
        currentShortcut: settings.fullscreenShortcut,
        otherShortcut: settings.areaShortcut,
      })
    }
  >
    <span className="shortcut-label">Fullscreen</span>
    <kbd>{formatShortcut(settings.fullscreenShortcut)}</kbd>
  </div>
  <div
    className="shortcut-box"
    onClick={() =>
      invoke("open_shortcut_config", {
        target: "area",
        currentShortcut: settings.areaShortcut,
        otherShortcut: settings.fullscreenShortcut,
      })
    }
  >
    <span className="shortcut-label">Area</span>
    <kbd>{formatShortcut(settings.areaShortcut)}</kbd>
  </div>
  <button
    className="reset-shortcuts-btn"
    onClick={async () => {
      await invoke("update_shortcuts", {
        fullscreenShortcut: "Cmd+Shift+3",
        areaShortcut: "Cmd+Shift+4",
      });
      setSettings((s) => ({
        ...s,
        fullscreenShortcut: "Cmd+Shift+3",
        areaShortcut: "Cmd+Shift+4",
      }));
    }}
  >
    Reset
  </button>
</div>
```

### 11.5 Add helper function:

```typescript
function formatShortcut(shortcut: string): string {
  return shortcut
    .replace(/Cmd/g, "⌘")
    .replace(/Shift/g, "⇧")
    .replace(/Alt/g, "⌥")
    .replace(/Ctrl/g, "⌃")
    .replace(/\+/g, "");
}
```

---

## Step 12: Add Frontend Styles

**File:** `src/App.css` (add near existing `.shortcuts-hint` styles)

```css
.shortcuts-config {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  margin: 12px 0;
}

.shortcut-box {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 8px 16px;
  background: rgba(128, 128, 128, 0.1);
  border: 1px solid rgba(128, 128, 128, 0.2);
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
}

.shortcut-box:hover {
  background: rgba(128, 128, 128, 0.2);
  border-color: rgba(128, 128, 128, 0.3);
}

.shortcut-label {
  font-size: 11px;
  opacity: 0.6;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.shortcut-box kbd {
  font-size: 18px;
  font-weight: 600;
  letter-spacing: 1px;
}

.reset-shortcuts-btn {
  padding: 6px 12px;
  font-size: 11px;
  background: transparent;
  border: 1px solid rgba(128, 128, 128, 0.3);
  border-radius: 6px;
  color: inherit;
  cursor: pointer;
  opacity: 0.6;
  transition: opacity 0.2s;
}

.reset-shortcuts-btn:hover {
  opacity: 1;
}
```

---

## Verification Checklist

| # | Test | Expected Result |
|---|------|-----------------|
| 1 | App starts with existing settings file (no shortcut fields) | Defaults to ⌘⇧3/⌘⇧4, no crash |
| 2 | Click fullscreen shortcut box | Popup opens centered |
| 3 | Press ⌘⇧A in popup | Shows "⌘⇧A", Save enabled |
| 4 | Try to set same shortcut as other action | Error shown, Save disabled |
| 5 | Click Save | Popup closes, UI updates, tray updates |
| 6 | Press new shortcut (⌘⇧A) | Fullscreen screenshot triggers |
| 7 | Press ESC during recording | Popup closes, no change |
| 8 | Click Reset | Both revert to ⌘⇧3/⌘⇧4 |
| 9 | Restart app | Custom shortcuts persist |
| 10 | Try reserved macOS shortcut | Registration fails, error shown, old shortcut remains |

---

## Error Handling Summary

| Scenario | Behavior |
|----------|----------|
| Invalid shortcut format | `parse_shortcut()` returns Err, shown in popup |
| Duplicate shortcuts | Blocked in popup before save |
| Registration failure (system conflict) | Rollback to old shortcuts, return error |
| Missing settings fields | `#[serde(default)]` provides defaults |
