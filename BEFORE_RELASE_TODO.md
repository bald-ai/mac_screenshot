# Before Release TODO

## Finder Automation Permission Handling

**Context:** The stitch feature uses `osascript` to get Finder selection. First-time users will see macOS prompt: "App wants to control Finder."

**Issue:** If user denies, the feature silently fails.

**Required Fix:**
1. Catch AppleScript permission errors in `get_finder_selection()`
2. Detect the specific "not authorized" error
3. Show user-friendly notification: "Grant Finder access in System Preferences → Privacy & Security → Automation"
4. Optionally: add a button to open System Preferences directly

**Reference:** macOS error `-1743` = "Not authorized to send Apple events"
