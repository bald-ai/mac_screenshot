# Screenshot Editor Documentation

## Responsibility Boundaries

Core behavior must keep working even if the webview is unresponsive or unmounted.

**Rust (src-tauri) owns:**
- Global shortcuts, tray menu actions, app lifecycle.
- Screenshot capture (`screencapture`), file IO, clipboard writes.
- Any action that must work when the UI is broken or closed.

**Webview / JS owns:**
- UI rendering, settings panels, and editor interactions.
- Canvas drawing, annotations, and other visual edits.
- Calling Rust commands for non-critical actions and updating UI state.

## UI Layout Constraints

### Editor Toolbar Minimum Width

The edit screenshot window has an **absolute minimum width of 580px**. This is an inviolable constraint to ensure all toolbar controls remain visible regardless of window dimensions.

When the screenshot being edited is tall and narrow (portrait orientation), the window itself cannot be resized smaller than 580px width - the OS enforces this constraint.

**Implementation:**
- `lib.rs`: `min_inner_size(580.0, 250.0)` in `open_editor_window`
- `lib.rs`: `MIN_WIDTH = 580.0` in `calculate_editor_window_size`

This ensures the toolbar never becomes too small to display all editing controls fully. The constraint is enforced at the window level by Tauri/the OS, not via CSS.

## Stitching Constraints

Stitching is limited to 8 images per run to avoid oversized outputs and excessive memory use.

Stitching can be triggered from the tray menu via “Stitch Images”.

When editing a stitched image, the editor window width is based on the largest source image so the stitched view matches the single-screenshot width.
