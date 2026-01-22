# Code Review (Framework + Findings)
Date: 2026-01-22
Scope: repository source/config files (excluded build output + `node_modules/`).
Goal: remove obvious mistakes, reduce redundancy, improve naming consistency, and make future AI edits safer.

## Review Framework
1. Build/packaging entrypoints and route coverage (Vite inputs, Tauri asset routes).
2. Window lifecycle and permissions (Tauri capabilities per window).
3. Settings schema consistency across Rust + TS + HTML.
4. Screenshot pipeline correctness (naming, files, resizing, format).
5. UX flows + keyboard interactions.
6. Dependency and permission hygiene (unused plugins, excess capabilities).
7. Code/style/asset redundancy.
8. Test coverage gaps for critical logic.

## Findings (ordered by severity)

### Critical
1) Editor window is not part of the Vite multi-page build inputs, so production builds can omit `editor.html`.
   - Impact: `open_editor_window` opens `/editor.html`, which may 404 in `dist/`, breaking the editor window in production.
   - Evidence: `vite.config.ts:26-33` only lists `index.html`, `rename.html`, `shortcut-config.html` while `open_editor_window` targets `/editor.html` (`src-tauri/src/lib.rs:641-669`).

2) The editor window is missing from the default capabilities list.
   - Impact: `editor.html` calls `invoke(...)` (save/copy/read). In Tauri v2, window capabilities gate APIs; missing the window label can block these calls in production.
   - Evidence: `src-tauri/capabilities/default.json:4-6` lists `main`, `rename`, `shortcut-config` but not `editor`, while the editor window label is `editor` (`src-tauri/src/lib.rs:645-669`).

### Major
3) Filename template can generate hidden/invalid filenames when only the counter block is enabled.
   - Impact: `base_name` becomes empty and filenames become `/Desktop/.jpg` (or `/_2.jpg`), which is surprising and hard to find.
   - Evidence: backend drops the `counter` block from `base_name` (`src-tauri/src/lib.rs:237-284`), and the UI allows disabling all other blocks as long as time or counter is enabled (`src/FilenameTemplate.tsx:61-95`).

4) Frontend defaults diverge from backend defaults, creating inconsistent first-run behavior and future drift.
   - Impact: If `get_settings` fails or settings are reset, the UI starts from `quality: 20` while backend defaults to `70`, leading to surprising output sizes.
   - Evidence: `src/App.tsx:50-59` vs `src-tauri/src/lib.rs:69-79`.

5) Excess permissions + unused plugins/deps increase attack surface and maintenance cost.
   - Impact: `shell` and `fs` permissions are enabled without a clear need, and plugins are initialized even though no JS APIs use them; this makes future reviews harder and widens the app’s permission footprint.
   - Evidence: permissions include `shell:allow-execute` and `shell:allow-spawn` (`src-tauri/capabilities/default.json:11-15`), while the builder initializes `tauri_plugin_shell` and `tauri_plugin_fs` (`src-tauri/src/lib.rs:1208-1211`). JS dependencies include `@tauri-apps/plugin-autostart` but there is no Rust or JS usage (`package.json:12-17`).

### Minor
6) UI preview shows `.webp` while actual outputs are `.jpg`/`.png`.
   - Impact: Preview mismatches the real filename/format and can confuse users.
   - Evidence: `src/FilenameTemplate.tsx:48-59` hardcodes `.webp`.

7) Template/default assets and styles from Vite remain, creating noise for future edits.
   - Impact: Adds dead CSS and files that are easy to mistakenly edit or keep synced.
   - Evidence: unused template styles and IDs like `.logo`, `.row`, `#greet-input` in `src/App.css:1-76, 122-124` and unused asset `src/assets/react.svg` (no references in code).

8) Documentation still refers to the starter template and does not describe this app’s architecture or commands.
   - Impact: Slows down onboarding (including AI-driven changes) and makes it harder to reason about the screenshot pipeline.
   - Evidence: `README.md:1-7` and `index.html:7` still use template branding.

### Tests / Observability Gaps
- No automated tests for filename generation, shortcut normalization, or editor save/copy flows. These are high-value, low-effort units to lock down regressions.

## Open Questions / Assumptions
- Should the editor window ship in production? If yes, it needs to be in the Vite input list and capabilities.
- Is the “counter-only” filename template intended behavior? If not, the backend should guard against an empty `base_name`.
- Are `tauri_plugin_shell`, `tauri_plugin_fs`, and `@tauri-apps/plugin-autostart` required for near-term roadmap? If not, remove them and the related permissions.

## Change Summary (No code changes made)
- Identified 2 critical packaging/capability issues, 3 major correctness/consistency issues, and multiple cleanup/documentation gaps.
- Main focus areas to address first: build inputs, editor capabilities, filename generation guardrails, and permission/dependency cleanup.
