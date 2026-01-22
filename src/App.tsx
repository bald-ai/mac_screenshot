import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import "./App.css";
import FilenameTemplateEditor from "./FilenameTemplate";
import { formatShortcutForDisplay } from "./shortcutFormat";

interface FilenameBlock {
  id: string;
  enabled: boolean;
  value?: string;
}

interface FilenameTemplate {
  blocks: FilenameBlock[];
  useCounter: boolean;
}

interface Settings {
  quality: number;
  maxWidth: number;
  notePrefixEnabled: boolean;
  notePrefix: string;
  filenameTemplate: FilenameTemplate;
  theme: "grey" | "dark" | "system";
  fullscreenShortcut: string;
  areaShortcut: string;
}

const DEFAULT_FILENAME_TEMPLATE: FilenameTemplate = {
  blocks: [
    { id: "prefix", enabled: true, value: "llm-scr" },
    { id: "date", enabled: true },
    { id: "time", enabled: true },
    { id: "quality", enabled: true },
    { id: "dimensions", enabled: true },
    { id: "counter", enabled: false },
  ],
  useCounter: false,
};

const SIZE_OPTIONS = [
  { label: "Original", value: 0 },
  { label: "1920px", value: 1920 },
  { label: "1440px", value: 1440 },
  { label: "1280px", value: 1280 },
  { label: "1024px", value: 1024 },
];

function App() {
  const [settings, setSettings] = useState<Settings>({
    quality: 20,
    maxWidth: 1280,
    notePrefixEnabled: false,
    notePrefix: "",
    filenameTemplate: DEFAULT_FILENAME_TEMPLATE,
    theme: "system",
    fullscreenShortcut: "Cmd+Shift+3",
    areaShortcut: "Cmd+Shift+4",
  });
  const [saveStatus, setSaveStatus] = useState<"idle" | "dirty" | "saving" | "saved" | "error">("idle");
  const saveStatusTimeoutRef = useRef<number | null>(null);
  const [showFilenameTemplate, setShowFilenameTemplate] = useState(false);
  const [shortcutError, setShortcutError] = useState<string | null>(null);
  const settingsPanelRef = useRef<HTMLDivElement | null>(null);

  // Apply theme to body element
  const applyTheme = (theme: "grey" | "dark" | "system") => {
    let isDark: boolean;
    if (theme === "system") {
      isDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    } else {
      isDark = theme === "dark";
    }
    document.body.classList.remove("theme-grey", "theme-dark");
    document.body.classList.add(isDark ? "theme-dark" : "theme-grey");
  };

  // Load settings on mount
  useEffect(() => {
    invoke<Settings>("get_settings").then((s) => {
      const themeFromBackend = s.theme as string;
      const normalizedTheme = themeFromBackend === "light" ? "grey" : s.theme;
      const normalizedSettings = { ...s, theme: normalizedTheme as "grey" | "dark" | "system" };
      setSettings(normalizedSettings);
      applyTheme(normalizedSettings.theme);
      if (themeFromBackend === "light") {
        invoke("save_settings", { settings: normalizedSettings }).catch(console.error);
      }
    }).catch(console.error);
  }, []);

  // Apply theme when settings.theme changes
  useEffect(() => {
    applyTheme(settings.theme);
    
    // Listen for system theme changes when using system preference
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handleChange = () => {
      if (settings.theme === "system") {
        applyTheme("system");
      }
    };
    mediaQuery.addEventListener("change", handleChange);
    return () => mediaQuery.removeEventListener("change", handleChange);
  }, [settings.theme]);

  // Save settings when they change
  const updateSettings = (newSettings: Settings) => {
    setSettings(newSettings);
    setSaveStatus("dirty");
    if (saveStatusTimeoutRef.current !== null) {
      window.clearTimeout(saveStatusTimeoutRef.current);
      saveStatusTimeoutRef.current = null;
    }
  };

  const saveSettings = async () => {
    try {
      setSaveStatus("saving");
      await invoke("save_settings", { settings });
      setSaveStatus("saved");
      if (saveStatusTimeoutRef.current !== null) {
        window.clearTimeout(saveStatusTimeoutRef.current);
      }
      saveStatusTimeoutRef.current = window.setTimeout(() => {
        setSaveStatus("idle");
        saveStatusTimeoutRef.current = null;
      }, 1500);
    } catch (e) {
      console.error("Failed to save settings:", e);
      setSaveStatus("error");
    }
  };

  // Screenshot handlers
  async function handleScreenshotTaken(filepath: string) {
    try {
      await invoke("open_rename_popup", { filepath });
    } catch (e) {
      console.error("Failed to open rename popup:", e);
    }
  }

  async function takeScreenshot() {
    try {
      const filepath = await invoke<string>("take_screenshot");
      handleScreenshotTaken(filepath);
    } catch (e) {
      console.error(e);
    }
  }

  async function takeFullscreenScreenshot() {
    try {
      const filepath = await invoke<string>("take_fullscreen_screenshot");
      handleScreenshotTaken(filepath);
    } catch (e) {
      console.error(e);
    }
  }

  // Listen for global shortcut triggers
  useEffect(() => {
    const unlistenArea = listen("take-screenshot", () => {
      takeScreenshot();
    });

    const unlistenFull = listen("take-fullscreen-screenshot", () => {
      takeFullscreenScreenshot();
    });

    return () => {
      unlistenArea.then((fn) => fn());
      unlistenFull.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    const unlistenShortcut = listen<{ target: string; shortcut: string }>(
      "shortcut-configured",
      async (event) => {
        const { target, shortcut } = event.payload;
        const newSettings = { ...settings };

        if (target === "fullscreen") {
          newSettings.fullscreenShortcut = shortcut;
        } else {
          newSettings.areaShortcut = shortcut;
        }

        try {
          await invoke("update_shortcuts", {
            fullscreenShortcut: newSettings.fullscreenShortcut,
            areaShortcut: newSettings.areaShortcut,
          });
          setSettings(newSettings);
          setShortcutError(null);
        } catch (e) {
          console.error("Failed to update shortcuts:", e);
          const message = typeof e === "string" ? e : e instanceof Error ? e.message : "Failed to update shortcuts";
          setShortcutError(message);
        }
      }
    );

    return () => {
      unlistenShortcut.then((fn) => fn());
    };
  }, [settings]);

  useEffect(() => {
    return () => {
      if (saveStatusTimeoutRef.current !== null) {
        window.clearTimeout(saveStatusTimeoutRef.current);
      }
    };
  }, []);

  useEffect(() => {
    const panel = settingsPanelRef.current;
    if (!panel) return;

    const handlePointerDown = (event: PointerEvent) => {
      if (event.button !== 0) return;
      const target = event.target as HTMLElement | null;
      if (!target) return;
      if (target.closest("input, textarea, select, button, option, a, label, [role='button'], .shortcut-link")) {
        return;
      }
      if (target.isContentEditable || target.closest("[data-no-drag='true']")) {
        return;
      }
      getCurrentWindow().startDragging().catch(() => {});
    };

    panel.addEventListener("pointerdown", handlePointerDown);
    return () => panel.removeEventListener("pointerdown", handlePointerDown);
  }, [showFilenameTemplate]);

  if (showFilenameTemplate) {
    return (
      <main className="container">
        <div className="settings-panel template-view" ref={settingsPanelRef}>
          <FilenameTemplateEditor
            template={settings.filenameTemplate}
            onTemplateChange={(template) => updateSettings({ ...settings, filenameTemplate: template })}
            onBack={() => setShowFilenameTemplate(false)}
            onReset={() => updateSettings({ ...settings, filenameTemplate: DEFAULT_FILENAME_TEMPLATE })}
          />
          <div className="template-footer">
            <button onClick={() => setShowFilenameTemplate(false)} className="back-btn">◀ Back</button>
            <button onClick={saveSettings} className="save-btn compact">Save</button>
          </div>
          <div className={`save-status compact ${saveStatus}`}>
            {saveStatus === "dirty" && "•"}
            {saveStatus === "saving" && "..."}
            {saveStatus === "saved" && "✓"}
            {saveStatus === "error" && "✗"}
          </div>
        </div>
      </main>
    );
  }

  return (
    <main className="container">
      <div className="settings-panel" ref={settingsPanelRef}>
        <div className="settings-row">
          <label>Quality: {settings.quality}%</label>
          <input
            type="range"
            min="10"
            max="100"
            step="5"
            value={settings.quality}
            onChange={(e) => updateSettings({ ...settings, quality: parseInt(e.target.value) })}
            className="quality-slider"
          />
        </div>
        <div className="settings-row">
          <label>Max Width:</label>
          <select
            value={settings.maxWidth}
            onChange={(e) => updateSettings({ ...settings, maxWidth: parseInt(e.target.value) })}
            className="size-select"
          >
            {SIZE_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </div>
        <div className="settings-row prefix-row">
          <div className="prefix-toggle">
            <input
              type="checkbox"
              aria-label="Toggle note prefix"
              checked={settings.notePrefixEnabled}
              onChange={(e) => updateSettings({ ...settings, notePrefixEnabled: e.target.checked })}
            />
            <span>Note Prefix</span>
          </div>
          <input
            type="text"
            value={settings.notePrefix}
            onChange={(e) => updateSettings({ ...settings, notePrefix: e.target.value.slice(0, 50) })}
            placeholder="Prompt for AI:"
            maxLength={50}
            disabled={!settings.notePrefixEnabled}
            className="prefix-input"
          />
        </div>
        <div className="settings-row">
          <button onClick={() => setShowFilenameTemplate(true)} className="template-btn">
            Change Filename Template
          </button>
        </div>
        <div className="settings-row">
          <label>Theme:</label>
          <div className="theme-toggle">
            <button
              className={settings.theme === "grey" ? "active" : ""}
              onClick={() => updateSettings({ ...settings, theme: "grey" })}
            >
              Grey
            </button>
            <button
              className={settings.theme === "dark" ? "active" : ""}
              onClick={() => updateSettings({ ...settings, theme: "dark" })}
            >
              Dark
            </button>
            <button
              className={settings.theme === "system" ? "active" : ""}
              onClick={() => updateSettings({ ...settings, theme: "system" })}
            >
              System
            </button>
          </div>
        </div>
        <div className="shortcuts-hint">
          <span
            className="shortcut-link"
            role="button"
            tabIndex={0}
            onClick={() => {
              setShortcutError(null);
              invoke("open_shortcut_config", {
                target: "fullscreen",
                currentShortcut: settings.fullscreenShortcut,
                otherShortcut: settings.areaShortcut,
              });
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                setShortcutError(null);
                invoke("open_shortcut_config", {
                  target: "fullscreen",
                  currentShortcut: settings.fullscreenShortcut,
                  otherShortcut: settings.areaShortcut,
                });
              }
            }}
          >
            <kbd>{formatShortcutForDisplay(settings.fullscreenShortcut)}</kbd> fullscreen
          </span>
          {" · "}
          <span
            className="shortcut-link"
            role="button"
            tabIndex={0}
            onClick={() => {
              setShortcutError(null);
              invoke("open_shortcut_config", {
                target: "area",
                currentShortcut: settings.areaShortcut,
                otherShortcut: settings.fullscreenShortcut,
              });
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                setShortcutError(null);
                invoke("open_shortcut_config", {
                  target: "area",
                  currentShortcut: settings.areaShortcut,
                  otherShortcut: settings.fullscreenShortcut,
                });
              }
            }}
          >
            <kbd>{formatShortcutForDisplay(settings.areaShortcut)}</kbd> area
          </span>
        </div>
        {shortcutError && <div className="shortcut-error">{shortcutError}</div>}
        <div className="button-row">
          <button onClick={saveSettings} className="save-btn">
            Save
          </button>
          <button onClick={() => getCurrentWindow().close()} className="exit-btn">
            Exit
          </button>
        </div>
        <div className={`save-status ${saveStatus}`}>
          {saveStatus === "dirty" && "Unsaved changes"}
          {saveStatus === "saving" && "Saving..."}
          {saveStatus === "saved" && "Saved"}
          {saveStatus === "error" && "Save failed"}
        </div>
      </div>
    </main>
  );
}

export default App;
