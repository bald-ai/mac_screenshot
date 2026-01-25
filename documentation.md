# Screenshot Editor Documentation

## UI Layout Constraints

### Editor Toolbar Minimum Width

The edit screenshot window has an **absolute minimum width of 580px**. This is an inviolable constraint to ensure all toolbar controls remain visible regardless of window dimensions.

When the screenshot being edited is tall and narrow (portrait orientation), the window itself cannot be resized smaller than 580px width - the OS enforces this constraint.

**Implementation:**
- `lib.rs`: `min_inner_size(580.0, 250.0)` in `open_editor_window`
- `lib.rs`: `MIN_WIDTH = 580.0` in `calculate_editor_window_size`

This ensures the toolbar never becomes too small to display all editing controls fully. The constraint is enforced at the window level by Tauri/the OS, not via CSS.
