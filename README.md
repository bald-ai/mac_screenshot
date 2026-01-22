# Screenshot App

Lightweight macOS screenshot tool optimized for fast capture, quick renaming, and AI‑friendly output sizes.

## What this app does (feature‑precise)
- **Capture modes:** global shortcuts for *area* and *fullscreen* screenshots. Defaults are ⌘⇧4 (area) and ⌘⇧3 (fullscreen). Shortcuts are user‑configurable.
- **Auto‑optimize:** screenshots are converted to JPEG (default), compressed by quality (default **50%**), and optionally resized to a max width (default **1280px**).
- **Rename popup:** appears immediately after capture with filename + optional note. Supports:
  - **Enter** = save
  - **⌘Enter** = copy to clipboard + save
  - **Esc** = delete
  - **Tab** = open editor (from note field)
- **Notes burned into image:** if a note is provided, it is rendered into a white bar below the image. Optional prefix can be added to every note.
- **Editor window:** annotate with pen, arrow, rectangle, ellipse, and text; color picker; undo; clear; copy to clipboard; save back to file.
- **Tray menu:** quick actions for area/fullscreen capture, show app, and quit. Closing the main window hides it instead of exiting.

## Key files / entrypoints (for future AI changes)
- **Backend commands & app wiring:** `src-tauri/src/lib.rs`
- **Main settings UI (React):** `src/App.tsx` + `src/App.css`
- **Rename popup:** `rename.html` + `public/rename.css`
- **Editor popup:** `editor.html` (pure HTML/JS)
- **Shortcut config popup:** `shortcut-config.html` + `public/shortcut-config.css`
- **Vite multi‑page setup:** `vite.config.ts`

## Settings & output
- **Settings file:** `~/.screenshot_app_settings.json`
- **Output location:** `~/Desktop/` by default
- **Filename template:** configurable in UI (template editor). If the template is empty, a safe fallback name is used.

## Quick dev commands
- `npm run dev` (frontend)
- `npm run tauri dev` (app)
