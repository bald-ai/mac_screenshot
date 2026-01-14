import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

function App() {
  const [status, setStatus] = useState("Ready");
  const [lastScreenshot, setLastScreenshot] = useState<string | null>(null);

  async function handleScreenshotTaken(filepath: string) {
    setLastScreenshot(filepath);
    setStatus("Screenshot saved! Opening rename...");
    // Open the rename popup window (always-on-top)
    try {
      await invoke("open_rename_popup", { filepath });
    } catch (e) {
      console.error("Failed to open rename popup:", e);
      setStatus("Screenshot saved!");
    }
  }

  async function takeScreenshot() {
    setStatus("Taking screenshot...");
    try {
      const filepath = await invoke<string>("take_screenshot");
      handleScreenshotTaken(filepath);
    } catch (e) {
      setStatus(`${e}`);
    }
  }

  async function takeFullscreenScreenshot() {
    setStatus("Taking fullscreen screenshot...");
    try {
      const filepath = await invoke<string>("take_fullscreen_screenshot");
      handleScreenshotTaken(filepath);
    } catch (e) {
      setStatus(`${e}`);
    }
  }

  useEffect(() => {
    // Listen for global shortcut triggers
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

  return (
    <>
      <main className="container">
        <h1>📸 Screenshot App</h1>

        <p style={{ color: "#888", marginBottom: "2rem" }}>
          <strong>Cmd + Shift + 3</strong> - Full screen<br />
          <strong>Cmd + Shift + 4</strong> - Select area
        </p>

        <div style={{ display: "flex", gap: "1rem", justifyContent: "center" }}>
          <button onClick={takeFullscreenScreenshot} style={{ fontSize: "1.2rem", padding: "1rem 2rem" }}>
            Full Screen
          </button>
          <button onClick={takeScreenshot} style={{ fontSize: "1.2rem", padding: "1rem 2rem" }}>
            Select Area
          </button>
        </div>

        <p style={{ marginTop: "2rem" }}>{status}</p>

        {lastScreenshot && (
          <p style={{ fontSize: "0.8rem", color: "#666", wordBreak: "break-all" }}>
            Saved: {lastScreenshot}
          </p>
        )}
      </main>
    </>
  );
}

export default App;
