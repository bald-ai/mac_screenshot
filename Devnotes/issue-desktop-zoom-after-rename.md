# Desktop Zoom After Rename Window Confirmation

## Problem
When confirming the Rename window with Enter or Command+Enter, the screenshot saves/copies correctly, but then macOS "zooms" to the desktop (unintended behavior).

## Root Cause
When `close_rename_popup` closes the **only visible window**, macOS falls back to Finder/Desktop, causing the "zoom" effect. The rename window is borderless/transparent (`.decorations(false)` + `.transparent(true)`) which macOS treats as ephemeral UI.

`close_rename_popup` just calls `window.close()` with no explicit focus restoration afterward.

## Affected Code
- `src-tauri/src/lib.rs` lines 811-815 (`close_rename_popup`)
- `rename.html` - calls `invoke('close_rename_popup')` on Enter/Cmd+Enter

## Proposed Fixes

### Option 1: Hide instead of close
Use `window.hide()` instead of `window.close()` to avoid the "app has no windows" transition.

### Option 2: Restore focus to previous app
1. Capture the frontmost app before opening rename (via macOS APIs or AppleScript)
2. After closing rename, reactivate that app

### Option 3: Keep a hidden main window
Ensure there's always at least one window so closing rename doesn't trigger macOS fallback behavior.

## Effort
S-M (1-2 hours)
