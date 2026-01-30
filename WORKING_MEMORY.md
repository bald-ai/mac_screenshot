# Working Memory

Active issues and tasks to address.

## Stitching Feature

- [x] Size difference: Get Info shows ~900KB but app blocks at 5MB limit - fixed by applying optimize_jpeg to save_edited_screenshot
- [ ] Note is hard to read on large stitched images without using specialized tools (e.g., look_at) - AI inline image reading may hallucinate text
- [ ] Whether AI can read the note also depends on which file it received - clipboard or file. Need to test.
- [ ] Clipboard size is much larger than file size - arboard library decodes JPEG to raw pixels (~6MB) instead of keeping compressed bytes (~1MB). Need to switch to native macOS NSPasteboard to copy raw JPEG/PNG bytes with correct UTType.
- [ ] Different resizing/background rules for stitched images vs normal screenshots
  - Note: Background still does not match between solo and stitched. Will probably require full redesign of logic.
- [x] Gaps between images too small (separators need to be larger) - set to 30px
- [ ] When I stitch 8 full screen it looks fine, then when I stitch 2x 2(8) stitched full screen it looks smaller suddenly
- [ ] Move note to top for stitched images (hard to scroll down to it)
