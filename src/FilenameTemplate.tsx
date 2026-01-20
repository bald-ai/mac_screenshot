import { useState, useEffect } from "react";

interface FilenameBlock {
  id: string;
  enabled: boolean;
  value?: string;
}

interface FilenameTemplate {
  blocks: FilenameBlock[];
  useCounter: boolean;
}

interface Props {
  template: FilenameTemplate;
  onTemplateChange: (template: FilenameTemplate) => void;
  onBack: () => void;
  onReset: () => void;
}

const BLOCK_LABELS: Record<string, string> = {
  prefix: "Prefix",
  date: "Date (MM-DD)",
  time: "Time (HH-MM-SS)",
  quality: "Quality (%)",
  dimensions: "Dimensions (WxH)",
  counter: "Counter (#001)",
};

const PLACEHOLDER_VALUES: Record<string, string> = {
  prefix: "",
  date: "01-20",
  time: "14-35-42",
  quality: "85",
  dimensions: "1920x1080",
  counter: "001",
};

const LOCKED_BLOCKS = ["time", "counter"];

function FilenameTemplate({ template, onTemplateChange, onReset }: Props) {
  const [blocks, setBlocks] = useState<FilenameBlock[]>(template.blocks);

  useEffect(() => {
    setBlocks(template.blocks);
  }, [template.blocks]);

  const generatePreview = (): string => {
    const parts: string[] = [];
    for (const block of blocks) {
      if (!block.enabled) continue;
      if (block.id === "prefix") {
        if (block.value) parts.push(block.value);
      } else {
        parts.push(PLACEHOLDER_VALUES[block.id]);
      }
    }
    return parts.length > 0 ? parts.join("_") + ".webp" : ".webp";
  };

  const isTimeEnabled = blocks.find((b) => b.id === "time")?.enabled ?? false;
  const isCounterEnabled = blocks.find((b) => b.id === "counter")?.enabled ?? false;
  const showWarning = !isTimeEnabled && !isCounterEnabled;

  const updateBlocks = (newBlocks: FilenameBlock[]) => {
    setBlocks(newBlocks);
    onTemplateChange({ ...template, blocks: newBlocks });
  };

  const handleToggle = (id: string) => {
    const newBlocks = blocks.map((b) => {
      if (b.id !== id) return b;
      const newEnabled = !b.enabled;
      if (!newEnabled && (id === "time" || id === "counter")) {
        const otherId = id === "time" ? "counter" : "time";
        const otherBlock = blocks.find((x) => x.id === otherId);
        if (!otherBlock?.enabled) {
          return b;
        }
      }
      return { ...b, enabled: newEnabled };
    });

    if (id === "time" || id === "counter") {
      const timeBlock = newBlocks.find((b) => b.id === "time");
      const counterBlock = newBlocks.find((b) => b.id === "counter");
      if (!timeBlock?.enabled && !counterBlock?.enabled) {
        const otherIdx = newBlocks.findIndex((b) => b.id === (id === "time" ? "counter" : "time"));
        if (otherIdx !== -1) {
          newBlocks[otherIdx] = { ...newBlocks[otherIdx], enabled: true };
        }
      }
    }

    updateBlocks(newBlocks);
  };

  const handlePrefixChange = (value: string) => {
    const newBlocks = blocks.map((b) => (b.id === "prefix" ? { ...b, value } : b));
    updateBlocks(newBlocks);
  };

  const moveBlock = (index: number, direction: "up" | "down") => {
    const newIndex = direction === "up" ? index - 1 : index + 1;
    if (newIndex < 0 || newIndex >= blocks.length) return;
    const newBlocks = [...blocks];
    [newBlocks[index], newBlocks[newIndex]] = [newBlocks[newIndex], newBlocks[index]];
    updateBlocks(newBlocks);
  };

  return (
    <div className="filename-template">
      <div className="preview-section">
        <label>Preview:</label>
        <div className="preview-filename">{generatePreview()}</div>
      </div>

      <div className="template-header">
        {showWarning && (
          <div className="warning-message">⚠️ Time OR Counter required</div>
        )}
        <button onClick={onReset} className="reset-btn" title="Reset to default template">↺</button>
      </div>

      <div className="blocks-list">
        {blocks.map((block, index) => (
          <div key={block.id} className="block-row">
            <div className="block-controls">
              <button
                onClick={() => moveBlock(index, "up")}
                disabled={index === 0}
                className="move-btn"
              >
                ▲
              </button>
              <button
                onClick={() => moveBlock(index, "down")}
                disabled={index === blocks.length - 1}
                className="move-btn"
              >
                ▼
              </button>
            </div>
            <label className="block-checkbox">
              <input
                type="checkbox"
                checked={block.enabled}
                onChange={() => handleToggle(block.id)}
              />
              {BLOCK_LABELS[block.id]}
              {LOCKED_BLOCKS.includes(block.id) && <span className="lock-icon" title="Required for unique filenames. Either Time or Counter must be enabled.">🔒</span>}
            </label>
            {block.id === "prefix" && (
              <input
                type="text"
                value={block.value || ""}
                onChange={(e) => handlePrefixChange(e.target.value)}
                placeholder="screenshot"
                className="prefix-input"
              />
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

export default FilenameTemplate;
