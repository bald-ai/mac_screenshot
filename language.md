# Language Glossary

This file defines the shared language for the project so humans and AI refer to the same things.
Use canonical names here and list the aliases you commonly say out loud. Keep it updated only when requested or after a confirmed misunderstanding.

## How to add entries
- Use a stable, descriptive canonical name.
- Add aliases (what you tend to call it).
- Add code anchors so it is easy to find (route, file, component, hook).

## Screens / Pages

| Canonical Name | Aliases | Code Anchors |
|----------------|---------|--------------|
| Rename window | rename popup, rename screen | `rename.html`, `rename_popup`, `close_rename_popup`, `open_rename_popup` |
| Settings | settings window, settings panel | `src/App.tsx`, `settings-panel` |
| Editor | edit window, editor window | `editor.html`, `editor_window`, `open_editor_window`, `close_editor_window` |

**Rename window** - The popup that appears right after taking a screenshot. User can rename the file or add a note.

**Settings** - The main app window where you change app settings (quality, shortcuts, filename template, etc.).

**Editor** - The window that opens when you tap Tab from rename window. Allows drawing, adding text overlays, arrows, shapes on the screenshot.

## Flows

## Components

## Data / Concepts

| Canonical Name | Aliases | Code Anchors |
|----------------|---------|--------------|
| Stitching | stitch, image stitching, combine screenshots | `src/stitch.ts`, `stitchImages`, `stitch-images` event, `save_stitch_temp` |

**Stitching** - Feature that combines multiple screenshots into a single vertical image. Images are stacked top-to-bottom with 8px gray dividers between them. Narrower images are centered horizontally on a light gray background.
