import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import "./App.css";
import FilenameTemplateEditor from "./FilenameTemplate";

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
  theme: "light" | "dark" | "system";
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
  const [settings, setSettings] = useState<Settings>({ quality: 20, maxWidth: 1280, notePrefixEnabled: false, notePrefix: "", filenameTemplate: DEFAULT_FILENAME_TEMPLATE, theme: "system" });
  const [saveStatus, setSaveStatus] = useState<"idle" | "dirty" | "saving" | "saved" | "error">("idle");
  const saveStatusTimeoutRef = useRef<number | null>(null);
  const [showFilenameTemplate, setShowFilenameTemplate] = useState(false);

  // Apply theme to body element
  const applyTheme = (theme: "light" | "dark" | "system") => {
    let isDark: boolean;
    if (theme === "system") {
      isDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    } else {
      isDark = theme === "dark";
    }
    document.body.classList.remove("theme-light", "theme-dark");
    document.body.classList.add(isDark ? "theme-dark" : "theme-light");
  };

  // Load settings on mount
  useEffect(() => {
    invoke<Settings>("get_settings").then((s) => {
      setSettings(s);
      applyTheme(s.theme);
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
    return () => {
      if (saveStatusTimeoutRef.current !== null) {
        window.clearTimeout(saveStatusTimeoutRef.current);
      }
    };
  }, []);

  if (showFilenameTemplate) {
    return (
      <main className="container">
        <div className="settings-panel template-view">
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
      <div className="settings-panel">
        <div className="settings-row">
          <label>Quality: {settings.quality}%</label>
          <input
            type="range"
            min="10"
            max="100"
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
          <label>
            <input
              type="checkbox"
              checked={settings.notePrefixEnabled}
              onChange={(e) => updateSettings({ ...settings, notePrefixEnabled: e.target.checked })}
            />
            Note Prefix
          </label>
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
            Filename Template ▶
          </button>
        </div>
        <div className="settings-row">
          <label>Theme:</label>
          <div className="theme-toggle">
            <button
              className={settings.theme === "light" ? "active" : ""}
              onClick={() => updateSettings({ ...settings, theme: "light" })}
            >
              Light
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
          <kbd>⌘⇧4</kbd> area · <kbd>⌘⇧3</kbd> fullscreen
        </div>
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
