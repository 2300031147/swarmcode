import { useState, useRef } from "react";
import { terminalRun, terminalRunInWorkspace, TerminalOutput } from "@/lib/tauri";
import { useFilesStore } from "@/store/files";

interface BottomPanelProps {
  onClose: () => void;
}

interface TermLine {
  type: "info" | "stdout" | "stderr" | "cmd";
  text: string;
}

export default function BottomPanel({ onClose }: BottomPanelProps) {
  const [activeTab, setActiveTab] = useState<"terminal" | "output">("terminal");
  const [lines, setLines] = useState<TermLine[]>([
    { type: "info", text: "ClawSwarm Terminal  —  type a command and press Enter" },
  ]);
  const [cmd, setCmd] = useState("");
  const [running, setRunning] = useState(false);
  const { workspace } = useFilesStore();
  const outputRef = useRef<HTMLDivElement>(null);

  const append = (newLines: TermLine[]) => {
    setLines((prev) => {
      const next = [...prev, ...newLines];
      setTimeout(() => {
        if (outputRef.current) {
          outputRef.current.scrollTop = outputRef.current.scrollHeight;
        }
      }, 0);
      return next;
    });
  };

  const runCommand = async () => {
    const command = cmd.trim();
    if (!command || running) return;
    setCmd("");
    append([{ type: "cmd", text: `$ ${command}` }]);
    setRunning(true);
    try {
      let result: TerminalOutput;
      if (workspace) {
        result = await terminalRunInWorkspace(command, workspace);
      } else {
        result = await terminalRun(command);
      }
      const out: TermLine[] = [];
      if (result.stdout) {
        result.stdout.split("\n").filter(Boolean).forEach((l) =>
          out.push({ type: "stdout", text: l })
        );
      }
      if (result.stderr) {
        result.stderr.split("\n").filter(Boolean).forEach((l) =>
          out.push({ type: "stderr", text: l })
        );
      }
      if (!result.success) {
        out.push({ type: "stderr", text: `exit code ${result.exit_code}` });
      }
      append(out);
    } catch (e) {
      append([{ type: "stderr", text: String(e) }]);
    }
    setRunning(false);
  };

  return (
    <div className="bottom-panel" style={{ height: "100%" }}>
      {/* Tabs */}
      <div className="bottom-tabs">
        <div
          className={`bottom-tab ${activeTab === "terminal" ? "active" : ""}`}
          onClick={() => setActiveTab("terminal")}
        >
          ⌨ Terminal
        </div>
        <div
          className={`bottom-tab ${activeTab === "output" ? "active" : ""}`}
          onClick={() => setActiveTab("output")}
        >
          📋 Output
        </div>
        <div style={{ flex: 1 }} />
        <button
          onClick={onClose}
          style={{ background: "none", border: "none", color: "var(--text-2)", cursor: "pointer", padding: "0 12px", fontSize: 16 }}
          title="Close Panel"
        >
          ✕
        </button>
      </div>

      {/* Terminal output */}
      <div className="terminal-output" ref={outputRef} style={{ flex: 1 }}>
        {lines.map((line, i) => (
          <div key={i} className={`terminal-line ${line.type}`}>
            {line.type === "cmd" && (
              <span className="terminal-prompt">›</span>
            )}
            <span>{line.text}</span>
          </div>
        ))}
        {running && (
          <div className="terminal-line info">
            <div className="spinner" />
            <span>Running...</span>
          </div>
        )}
      </div>

      {/* Input */}
      <div className="terminal-input-row">
        <span className="terminal-prompt" style={{ fontSize: 14 }}>›</span>
        <input
          className="terminal-input"
          value={cmd}
          onChange={(e) => setCmd(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") runCommand();
          }}
          placeholder={workspace ? `${workspace.split(/[\\/]/).pop()} $` : "command..."}
          disabled={running}
        />
      </div>
    </div>
  );
}
