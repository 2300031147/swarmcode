import { useState, useEffect } from "react";
import { PanelGroup, Panel, PanelResizeHandle } from "react-resizable-panels";
import { open } from "@tauri-apps/plugin-dialog";
import TitleBar from "./components/layout/TitleBar";
import Sidebar from "./components/layout/Sidebar";
import EditorArea from "./components/layout/EditorArea";
import BottomPanel from "./components/layout/BottomPanel";
import ChatPanel from "./components/chat/ChatPanel";
import { useFilesStore } from "./store/files";
import { useChatStore } from "./store/chat";
import { appGetVersion } from "./lib/tauri";

type SidebarView = "explorer" | "providers" | "agents" | "search" | "senses";

export default function App() {
  const [sidebarView, setSidebarView] = useState<SidebarView>("explorer");
  const [bottomOpen, setBottomOpen] = useState(true);
  const [version, setVersion] = useState("");
  const { setWorkspace, workspace } = useFilesStore();
  const { loadModels } = useChatStore();

  useEffect(() => {
    loadModels();
    appGetVersion().then((v) => setVersion(v.version)).catch(() => {});
  }, []);

  const handleOpenFolder = async () => {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
      await setWorkspace(selected);
    }
  };

  return (
    <div className="app-shell">
      <TitleBar
        workspace={workspace}
        onOpenFolder={handleOpenFolder}
      />

      <div className="app-body">
        {/* Activity Bar */}
        <ActivityBar view={sidebarView} onChange={setSidebarView} />

        {/* Main PanelGroup: Sidebar | Editor+Bottom | Chat */}
        <PanelGroup direction="horizontal" style={{ flex: 1, overflow: "hidden" }}>
          {/* Sidebar */}
          <Panel defaultSize={18} minSize={12} maxSize={35}>
            <Sidebar view={sidebarView} onOpenFolder={handleOpenFolder} />
          </Panel>

          <PanelResizeHandle style={{ width: 3, background: "var(--border-0)", cursor: "col-resize" }} />

          {/* Editor + Bottom Panel */}
          <Panel defaultSize={57} minSize={30}>
            <PanelGroup direction="vertical">
              <Panel defaultSize={75} minSize={40}>
                <EditorArea />
              </Panel>
              {bottomOpen && (
                <>
                  <PanelResizeHandle style={{ height: 3, background: "var(--border-0)", cursor: "row-resize" }} />
                  <Panel defaultSize={25} minSize={15} maxSize={50}>
                    <BottomPanel onClose={() => setBottomOpen(false)} />
                  </Panel>
                </>
              )}
            </PanelGroup>
          </Panel>

          <PanelResizeHandle style={{ width: 3, background: "var(--border-0)", cursor: "col-resize" }} />

          {/* Chat Panel */}
          <Panel defaultSize={25} minSize={18} maxSize={45}>
            <ChatPanel />
          </Panel>
        </PanelGroup>
      </div>

      {/* Status Bar */}
      <div className="statusbar">
        <span className="statusbar-item">⚡ SwarmCode</span>
        {workspace && (
          <span className="statusbar-item" style={{ color: "rgba(255,255,255,0.6)", fontSize: 11 }}>
            {workspace.split(/[\\/]/).pop()}
          </span>
        )}
        {!bottomOpen && (
          <button
            className="statusbar-item"
            onClick={() => setBottomOpen(true)}
            style={{ background: "none", border: "none", color: "inherit", cursor: "pointer" }}
          >
            ⌨ Terminal
          </button>
        )}
        <div className="statusbar-right">
          <span className="statusbar-item">v{version}</span>
        </div>
      </div>
    </div>
  );
}

// ── Activity Bar ────────────────────────────────────────────────────────────
interface ActivityBarProps {
  view: SidebarView;
  onChange: (v: SidebarView) => void;
}

function ActivityBar({ view, onChange }: ActivityBarProps) {
  const items: { id: SidebarView; icon: string; label: string }[] = [
    { id: "explorer",  icon: "📁", label: "Explorer" },
    { id: "search",    icon: "🔍", label: "Search" },
    { id: "providers", icon: "🔌", label: "Providers" },
    { id: "agents",    icon: "🤖", label: "Agents" },
    { id: "senses",    icon: "🧠", label: "Senses" },
  ];

  return (
    <div className="activity-bar">
      {items.map((item) => (
        <button
          key={item.id}
          className={`activity-btn ${view === item.id ? "active" : ""}`}
          onClick={() => onChange(item.id)}
          data-tooltip={item.label}
          title={item.label}
        >
          <span style={{ fontSize: 18 }}>{item.icon}</span>
        </button>
      ))}
    </div>
  );
}
