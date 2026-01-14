
Let me first explore what you've already built to understand the current state, then I can help you organize this into a proper planning document.

Got it. You've got a solid foundation - a Tauri app (React + Rust) that currently does basic screenshot capture with a global shortcut (Cmd+Shift+4). Let me organize your vision into a proper product document.

---

## 📋 Product Planning Document: **ScreenshotApp for AI/Vibe Coders**

### The Problem
Mac's native screenshot workflow sucks for AI-assisted development:
- **Bloated files** - Default screenshots are 3+ MB, murdering your token budget
- **No quick renaming** - Can't tag "padding shit" and move on
- **Editing is ass** - Mac's markup tools are painfully slow for quick annotations
- **No grouping** - Can't bundle related screenshots into one shareable artifact
- **Video gap** - Record quick video → AI can't eat it → need transcription/description

### The Goal
Let vibe coders grab, annotate, organize, and compress screenshots (and video) optimized for feeding to AI models.

---

### Current State ✅
| Feature | Status |
|---------|--------|
| Basic screenshot capture | ✅ Done |
| Global shortcut (Cmd+Shift+4) | ✅ Done |
| Saves to Desktop | ✅ Done |

---

### Feature Roadmap

#### **F1: Image Compression/Optimization** ✅ DONE
| Problem | Solution |
|---------|----------|
| 3+ MB full-screen shots kill tokens | Auto-compress to <500KB (configurable) |
| Some AI apps have hard file size limits | Support quality presets: "Token Saver", "Balanced", "High Quality" |

**Implementation ideas:**
- Use `image` crate in Rust for JPEG/WebP compression
- Configurable quality slider (10-100)
- Auto-resize large dimensions (e.g., cap at 1920px width)
- Show before/after file size

---

#### **F2: Quick Rename After Capture** ✅ DONE
| Problem | Solution |
|---------|----------|
| Screenshots dump as `screenshot_1736512345.png` | Popup immediately after capture with rename field |
| "padding shit" should take 2 seconds | Pre-focus text input, Enter to confirm |

**Implementation ideas:**
- Small floating window appears post-capture
- Keyboard-first: type name → Enter → done
- Optional: tag system (e.g., #bug, #ui, #padding)
- Save to organized folder structure based on tags/date

---

#### **F3: Quick Annotation/Edit Tools** 🔥 High Priority
| Problem | Solution |
|---------|----------|
| Mac Preview editing is clunky | Built-in canvas with arrows, rectangles, text, highlight |
| Need to point at specific shit fast | One-click tools, keyboard shortcuts |

**Implementation ideas:**
- Post-capture opens in edit mode
- Tools: Arrow (A), Rectangle (R), Text (T), Highlight (H), Freehand (F)
- Color picker (red default for visibility)
- Undo/Redo
- Libraries: Canvas-based (React) or Rust image manipulation

---

#### **F4: Screenshot Grouping & Export** 🟡 Medium Priority
| Problem | Solution |
|---------|----------|
| 4 screenshots = 4 separate files to juggle | Group into a "collection" |
| Want to send as one artifact | Export as: PDF, ZIP, or single combined image |
| Space optimization | Compress all at once |

**Implementation ideas:**
- "New Session" creates a group
- Drag/reorder screenshots within session
- Export options:
  - **PDF**: All images in one doc, compressed
  - **ZIP**: Folder with all optimized images
  - **Combined image**: Vertical/horizontal stitch
- Add session notes/description

---

#### **F5: Quick Video Recording + AI Description** 🟡 Medium Priority
| Problem | Solution |
|---------|----------|
| Video is rarely accepted by LLMs | Record short clip → AI summarizes → text description |
| Need quick demos for bugs | Screen recording with simple start/stop |

**Implementation ideas:**
- Record button or hotkey (e.g., Cmd+Shift+5)
- Capture window/region
- After recording:
  - Compress video
  - Call AI API (OpenAI Vision, Claude, etc.) to describe
  - Output: markdown summary + key frames extracted
- Option to extract key frames as screenshots

---

### Priority Matrix

| Feature | Value | Complexity | Priority |
|---------|-------|------------|----------|
| F1: Compression | ✅ Done | Low-Medium | ~~P0~~ |
| F2: Quick Rename | ✅ Done | Low | ~~P0~~ |
| F3: Annotation Tools | 🔥🔥 | High | P1 |
| F4: Grouping/Export | 🔥 | Medium | P2 |
| F5: Video + AI | 🔥 | High | P3 |

---

### Suggested Implementation Order

1. **Phase 1** (Quick Wins): ~~F1 + F2 - Compression + Quick Rename~~ ✅ COMPLETE
2. **Phase 2** (Core Editor): F3 - Annotation Tools  
3. **Phase 3** (Power Features): F4 + F5 - Grouping & Video

---

### Technical Considerations

- **Image processing**: Rust `image` crate or `imagemagick` via shell
- **Annotation canvas**: Could use HTML5 Canvas, Fabric.js, or Konva.js on React side
- **Video recording**: macOS `screencapture -v` or AVFoundation
- **AI integration**: OpenAI Vision API / Claude API for video-to-text
- **Storage**: Local SQLite for metadata, organized folder structure for files

---

Want me to help you create this as an actual `PRODUCT.md` or `ROADMAP.md` file in your repo? I can also help you start on any of these features - F1 (compression) and F2 (quick rename) would be the fastest wins to implement given your current codebase.


Ad-Hoc ideas:
- need quick copy from same space we are renaming