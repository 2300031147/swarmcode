import FileTree from "@/components/filetree/FileTree";
import ProvidersPanel from "@/components/providers/ProvidersPanel";
import AgentsPanel from "@/components/agents/AgentsPanel";
import SensesPanel from "@/components/senses/SensesPanel";
import { useFilesStore } from "@/store/files";

type SidebarView = "explorer" | "providers" | "agents" | "search" | "senses";

interface SidebarProps {
  view: SidebarView;
  onOpenFolder: () => void;
}

export default function Sidebar({ view, onOpenFolder }: SidebarProps) {
  const { workspace } = useFilesStore();

  const title: Record<SidebarView, string> = {
    explorer: "Explorer",
    search: "Search",
    providers: "Providers",
    agents: "Agents",
    senses: "Senses",
  };

  return (
    <div className="sidebar" style={{ height: "100%" }}>
      <div className="sidebar-header">
        <span>{title[view]}</span>
        {view === "explorer" && (
          <button
            className="btn btn-ghost btn-sm"
            onClick={onOpenFolder}
            title="Open Folder"
            style={{ padding: "2px 6px", fontSize: 11 }}
          >
            📂
          </button>
        )}
      </div>

      <div className="sidebar-content">
        {view === "explorer" && (
          <>
            {!workspace ? (
              <div style={{ padding: "24px 16px", textAlign: "center", color: "var(--text-2)" }}>
                <div style={{ fontSize: 32, marginBottom: 12 }}>📁</div>
                <div style={{ fontSize: 13, marginBottom: 12 }}>No folder open</div>
                <button className="btn btn-primary btn-sm" onClick={onOpenFolder}>
                  Open Folder
                </button>
              </div>
            ) : (
              <FileTree />
            )}
          </>
        )}

        {view === "search" && <SearchPanel />}
        {view === "providers" && <ProvidersPanel />}
        {view === "agents" && <AgentsPanel />}
        {view === "senses" && <SensesPanel />}
      </div>
    </div>
  );
}

// Lightweight search panel — searches open file names
function SearchPanel() {
  return (
    <div style={{ padding: "8px" }}>
      <input
        className="chat-input"
        placeholder="Search files..."
        style={{ width: "100%", minHeight: 32, fontSize: 12 }}
      />
      <div style={{ padding: "8px 4px", fontSize: 12, color: "var(--text-2)" }}>
        Type to search open tabs or use the file tree
      </div>
    </div>
  );
}
