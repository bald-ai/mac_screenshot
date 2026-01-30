# CLIPBOARD IMAGE SIZE ISSUE REPORT

## GOAL
Prevent images copied to clipboard from inflating in size when pasted into other applications (e.g., 1MB JPEG becoming 6MB when pasting into Claude/browsers).

## PROBLEM DISCOVERY

### Initial Observation
- User spliced 8 images into a 1MB JPEG file
- When copying the image and pasting into Claude, got error: "Image file too large: 6.0MB (max: 4.9MB)"
- File on disk was ~1MB, but clipboard data was ~6MB

### Root Cause Analysis
macOS clipboard behavior:
1. When copying an image, macOS stores **uncompressed pixel data** (TIFF/bitmap), not the compressed JPEG/PNG bytes
2. Original JPEG compression is lost when going through clipboard
3. `arboard` crate (previously used) converts images to raw RGBA pixels before putting on clipboard

### Verification
Running `osascript -e 'clipboard info'` after copying showed macOS auto-generates multiple representations:

| Format | Size |
|--------|------|
| JPEG | 1.0 MB (original) |
| TIFF | 16.4 MB |
| PNG | 4.7 MB |
| BMP | 16.4 MB |
| GIF | 1.5 MB |

Browsers typically request PNG or TIFF format, receiving the inflated version.

## APPROACHES TRIED

### Approach 1: Replace arboard with NSPasteboard + Image Bytes
- **What**: Use native `NSPasteboard` API to write compressed JPEG/PNG bytes directly with proper UTI (`public.jpeg`, `public.png`)
- **Result**: FAILED - macOS Image I/O framework still auto-generates all other representations from the image data

### Approach 2: Use NSPasteboardItem to Prevent Auto-Conversion
- **What**: Use `NSPasteboardItem` which supposedly prevents macOS from auto-generating other formats
- **Result**: FAILED - Auto-conversion still occurred; same multiple representations appeared

### Approach 3: Copy as File URL (SUCCESSFUL)
- **What**: Instead of putting image data on clipboard, put a **file URL reference** (like Finder does when copying files)
- **How**: Use `NSURL::fileURLWithPath` and write to pasteboard via `writeObjects`
- **Result**: SUCCESS

Clipboard after fix:
```
«class furl», 76
```
Only 76 bytes for the file path reference. Applications read the actual file from disk, preserving the original 991KB JPEG compression.

## FINAL SOLUTION

### Code Changes

1. **Cargo.toml**: Added NSPasteboard features, removed `arboard` dependency
```toml
objc2-app-kit = { version = "0.2", features = ["NSApplication", "NSWindow", "NSResponder", "NSPasteboard", "NSPasteboardItem"] }
objc2-foundation = { version = "0.2", features = ["NSData", "NSString", "NSArray"] }
```

2. **lib.rs**: New `write_file_url_to_clipboard` function
```rust
fn write_file_url_to_clipboard(filepath: &str) -> Result<(), String> {
    unsafe {
        let pb = NSPasteboard::generalPasteboard();
        pb.clearContents();
        
        let path_str = NSString::from_str(filepath);
        let url = NSURL::fileURLWithPath(&path_str);
        
        let urls = NSArray::from_id_slice(&[url]);
        let ok: bool = msg_send![&pb, writeObjects: &*urls];
        
        if !ok {
            return Err("NSPasteboard.writeObjects failed".to_string());
        }
        Ok(())
    }
}
```

3. **lib.rs**: Updated `copy_file_to_clipboard` to use file URL
```rust
fn copy_file_to_clipboard(filepath: String) -> Result<(), String> {
    write_file_url_to_clipboard(&filepath)
}
```

## AFFECTED FUNCTIONALITY

All copy-to-clipboard operations now use the file URL approach:
- **rename.html**: Cmd+Enter (save + copy), Cmd+Backspace (copy + delete)
- **editor.html**: Cmd+C (copy), Cmd+Enter (copy + save + close), Cmd+Backspace (copy + delete)

## VERIFICATION

1. Spliced 8 images → saved as 991KB JPEG
2. Copied via Cmd+Enter
3. `osascript -e 'clipboard info'` showed only: `«class furl», 76`
4. Successfully pasted into Claude without "too large" error
5. Image quality preserved, all content readable

## KEY INSIGHT

macOS clipboard has two fundamentally different modes:
1. **Image data mode**: macOS auto-converts between formats for compatibility (causes size inflation)
2. **File URL mode**: Apps read the actual file from disk (preserves original compression)

The solution is to always use file URL mode when the image exists as a file on disk.
