import React from "react";
import ReactDOM from "react-dom/client";
import "./playground.css";

function VariantC() {
  return (
    <div className="rename-frame variant-c">
      <div className="group">
        <input
          type="text"
          className="name-input"
          defaultValue="llm-scr_01-20_14-17-19_50%_1024x640"
        />
        <input
          type="text"
          className="note-input"
          placeholder="Annotate image with text."
        />
        <div className="tooltip">
          <span>↵ save</span>
          <span className="sep">·</span>
          <span>⌘↵ copy+save</span>
          <span className="sep">·</span>
          <span>esc delete</span>
          <span className="sep">·</span>
          <span>tab save+edit</span>
        </div>
      </div>
    </div>
  );
}

function VariantD() {
  return (
    <div className="rename-frame variant-d">
      <div className="group">
        <input
          type="text"
          className="name-input"
          defaultValue="llm-scr_01-20_14-17-19_50%_1024x640"
        />
        <input
          type="text"
          className="note-input"
          placeholder="Annotate image with text."
        />
        <div className="tooltip">
          <span>↵ save</span>
          <span className="sep">·</span>
          <span>⌘↵ copy+save</span>
          <span className="sep">·</span>
          <span>esc delete</span>
          <span className="sep">·</span>
          <span>tab save+edit</span>
        </div>
      </div>
    </div>
  );
}

function Playground() {
  return (
    <div className="playground">
      <h1>🎨 UI Playground</h1>

      <div className="variants-grid">
        <section className="section">
          <h2>C: "Ink & Paper" — light mode B&W editorial</h2>
          <div className="component-container ink-paper">
            <VariantC />
          </div>
        </section>

        <section className="section">
          <h2>D: "Midnight Editorial" — dark mode B&W editorial</h2>
          <div className="component-container midnight">
            <VariantD />
          </div>
        </section>
      </div>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Playground />
  </React.StrictMode>
);
