import Editor from "@monaco-editor/react";
import { useFilesStore } from "@/store/files";
import { fileIcon } from "@/lib/utils";

export default function EditorArea() {
  const { tabs, activeTab, closeTab, setActiveTab, updateTabContent, saveActiveFile } = useFilesStore();

  const activeTabData = tabs.find((t) => t.path === activeTab);

  return (
    <div className="editor-area">
      {/* Tab bar */}
      {tabs.length > 0 && (
        <div className="tab-bar">
          {tabs.map((tab) => (
            <div
              key={tab.path}
              className={`tab ${tab.path === activeTab ? "active" : ""}`}
              onClick={() => setActiveTab(tab.path)}
            >
              <span style={{ fontSize: 13 }}>
                {fileIcon(tab.name.split(".").pop() ?? null, false)}
              </span>
              <span>
                {tab.name}
                {tab.isDirty && (
                  <span style={{ color: "var(--accent)", marginLeft: 3 }}>●</span>
                )}
              </span>
              <button
                className="tab-close"
                onClick={(e) => {
                  e.stopPropagation();
                  closeTab(tab.path);
                }}
                title="Close"
              >
                ✕
              </button>
            </div>
          ))}
        </div>
      )}

      {/* Monaco Editor */}
      {activeTabData ? (
        <Editor
          height="100%"
          language={activeTabData.language}
          value={activeTabData.content}
          theme="vs-dark"
          onChange={(value) => {
            if (value !== undefined) {
              updateTabContent(activeTabData.path, value);
            }
          }}
          onMount={(editor, monaco) => {
            // Ctrl+S saves
            editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
              saveActiveFile();
            });
            // Customize Monaco theme to match ClawSwarm design
            monaco.editor.defineTheme("clawswarm", {
              base: "vs-dark",
              inherit: true,
              rules: [],
              colors: {
                "editor.background": "#161820",
                "editor.lineHighlightBackground": "#1e2130",
                "editorLineNumber.foreground": "#3a4060",
                "editorLineNumber.activeForeground": "#6c8ef7",
                "editor.selectionBackground": "#252840",
                "editor.inactiveSelectionBackground": "#1e2130",
              },
            });
            monaco.editor.setTheme("clawswarm");
          }}
          options={{
            fontSize: 14,
            fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
            fontLigatures: true,
            lineNumbers: "on",
            minimap: { enabled: true, scale: 1 },
            scrollBeyondLastLine: false,
            wordWrap: "off",
            renderWhitespace: "selection",
            smoothScrolling: true,
            cursorBlinking: "phase",
            cursorSmoothCaretAnimation: "on",
            renderLineHighlight: "gutter",
            bracketPairColorization: { enabled: true },
            guides: { bracketPairs: true },
            padding: { top: 12 },
          }}
        />
      ) : (
        <div className="editor-empty">
          <div className="editor-empty-logo">ClawSwarm</div>
          <p>Open a file from the explorer or ask the AI to create one</p>
          <div style={{ display: "flex", gap: 12, fontSize: 12, color: "var(--text-2)" }}>
            <span>Ctrl+S — Save</span>
            <span>·</span>
            <span>Ctrl+Shift+P — Command Palette</span>
          </div>
        </div>
      )}
    </div>
  );
}
