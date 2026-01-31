import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import "./App.css";
import FilenameTemplateEditor from "./FilenameTemplate";
import { formatShortcutForDisplay } from "./shortcutFormat";
import { MAX_STITCH_IMAGES } from "./constants";


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
  fullscreenShortcut: string;
  areaShortcut: string;
  stitchShortcut: string;
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
  { label: "1920 x 1080", value: 1920 },
  { label: "1440 x 810", value: 1440 },
  { label: "1280 x 720", value: 1280 },
  { label: "1024 x 576", value: 1024 },
  { label: "800 x 450", value: 800 },
  { label: "640 x 360", value: 640 },
  { label: "512 x 288", value: 512 },
  { label: "400 x 225", value: 400 },
  { label: "320 x 180", value: 320 },
  { label: "256 x 144", value: 256 },
];

function App() {
  const [settings, setSettings] = useState<Settings>({
    quality: 70,
    maxWidth: 1024,
    notePrefixEnabled: false,
    notePrefix: "",
    filenameTemplate: DEFAULT_FILENAME_TEMPLATE,
    fullscreenShortcut: "Cmd+Shift+3",
    areaShortcut: "Cmd+Shift+4",
    stitchShortcut: "Cmd+Shift+2",
  });
  const [lastSavedSettings, setLastSavedSettings] = useState<Settings | null>(null);
  const [saveStatus, setSaveStatus] = useState<"idle" | "dirty" | "saving" | "saved" | "error">("idle");
  const saveStatusTimeoutRef = useRef<number | null>(null);
  const [showFilenameTemplate, setShowFilenameTemplate] = useState(false);
  const [shortcutError, setShortcutError] = useState<string | null>(null);
  const settingsPanelRef = useRef<HTMLDivElement | null>(null);

  // Load settings on mount
  useEffect(() => {
    invoke<Settings>("get_settings").then((s) => {
      setSettings(s);
      setLastSavedSettings(s);
    }).catch(console.error);
  }, []);

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
      setLastSavedSettings(settings);
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

    const unlistenStitch = listen("stitch-images", async () => {
      console.log("[stitch] event received");
      try {
        const paths = await invoke<string[]>("get_finder_selection");
        console.log("[stitch] finder selection count:", paths.length);
        if (paths.length < 2) {
          console.warn("[stitch] need at least 2 images");
          await invoke("clear_stitch_lock");
          return;
        }
        if (paths.length > MAX_STITCH_IMAGES) {
          throw new Error(`Too many images (max ${MAX_STITCH_IMAGES})`);
        }
        
        const { stitchImages } = await import("./stitch");
        const stitch = stitchImages;
        const result = await stitch(paths);
        console.log("[stitch] stitch result", { width: result.width, height: result.height });
        
        const tempPath = await invoke<string>("save_stitch_temp", { 
          base64Data: result.base64Data,
          maxSingleImageHeight: result.maxSingleImageHeight,
          maxSingleImageWidth: result.maxSingleImageWidth
        });
        console.log("[stitch] temp saved at", tempPath);
        
        await invoke("open_rename_popup", { filepath: tempPath });
      } catch (e) {
        console.error("Stitch failed:", e);
        const errorMessage = typeof e === "string" ? e : e instanceof Error ? e.message : "Stitch failed";
        if (errorMessage.includes("Too many images")) {
          await invoke("show_alert", { title: "Stitch Error", message: errorMessage });
        }
        await invoke("clear_stitch_lock");
      }
    });

    return () => {
      unlistenArea.then((fn) => fn());
      unlistenFull.then((fn) => fn());
      unlistenStitch.then((fn) => fn());
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
        } else if (target === "area") {
          newSettings.areaShortcut = shortcut;
        } else if (target === "stitch") {
          newSettings.stitchShortcut = shortcut;
        }

        try {
          await invoke("update_shortcuts", {
            fullscreenShortcut: newSettings.fullscreenShortcut,
            areaShortcut: newSettings.areaShortcut,
            stitchShortcut: newSettings.stitchShortcut,
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

  const handleTemplateBack = () => {
    if (lastSavedSettings) {
      setSettings(lastSavedSettings);
    }
    setSaveStatus("idle");
    if (saveStatusTimeoutRef.current !== null) {
      window.clearTimeout(saveStatusTimeoutRef.current);
      saveStatusTimeoutRef.current = null;
    }
    setShowFilenameTemplate(false);
  };

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
          />
          <div className="template-footer">
            <div className="template-footer-row">
              <button onClick={saveSettings} className="save-btn compact">Save</button>
              <button
                onClick={() => updateSettings({ ...settings, filenameTemplate: DEFAULT_FILENAME_TEMPLATE })}
                className="reset-btn compact"
              >
                Reset
              </button>
            </div>
            <div className="template-footer-row">
              <button onClick={handleTemplateBack} className="back-btn">◀ Back</button>
              <button onClick={() => getCurrentWindow().close()} className="exit-btn compact">Quit</button>
            </div>
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
          <label>Max Size:</label>
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
                otherShortcut: settings.areaShortcut + "," + settings.stitchShortcut,
              });
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                setShortcutError(null);
                invoke("open_shortcut_config", {
                  target: "fullscreen",
                  currentShortcut: settings.fullscreenShortcut,
                  otherShortcut: settings.areaShortcut + "," + settings.stitchShortcut,
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
                otherShortcut: settings.fullscreenShortcut + "," + settings.stitchShortcut,
              });
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                setShortcutError(null);
                invoke("open_shortcut_config", {
                  target: "area",
                  currentShortcut: settings.areaShortcut,
                  otherShortcut: settings.fullscreenShortcut + "," + settings.stitchShortcut,
                });
              }
            }}
          >
            <kbd>{formatShortcutForDisplay(settings.areaShortcut)}</kbd> area
          </span>
          {" · "}
          <span
            className="shortcut-link"
            role="button"
            tabIndex={0}
            onClick={() => {
              setShortcutError(null);
              invoke("open_shortcut_config", {
                target: "stitch",
                currentShortcut: settings.stitchShortcut,
                otherShortcut: settings.fullscreenShortcut + "," + settings.areaShortcut,
              });
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                setShortcutError(null);
                invoke("open_shortcut_config", {
                  target: "stitch",
                  currentShortcut: settings.stitchShortcut,
                  otherShortcut: settings.fullscreenShortcut + "," + settings.areaShortcut,
                });
              }
            }}
          >
            <kbd>{formatShortcutForDisplay(settings.stitchShortcut)}</kbd> stitch
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
