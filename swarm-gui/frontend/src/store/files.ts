import { create } from "zustand";
import { FileNode, ReadFileResult, fsListDir, fsReadFile, fsWriteFile, fsSetWorkspace } from "@/lib/tauri";

export interface OpenTab {
  path: string;
  name: string;
  content: string;
  language: string;
  isDirty: boolean;
}

interface FilesState {
  workspace: string | null;
  tree: FileNode[];
  tabs: OpenTab[];
  activeTab: string | null;
  isLoading: boolean;
  setWorkspace: (path: string) => Promise<void>;
  refreshTree: () => Promise<void>;
  openFile: (path: string, name: string) => Promise<void>;
  closeTab: (path: string) => void;
  setActiveTab: (path: string) => void;
  saveActiveFile: () => Promise<void>;
  updateTabContent: (path: string, content: string) => void;
}

export const useFilesStore = create<FilesState>((set, get) => ({
  workspace: null,
  tree: [],
  tabs: [],
  activeTab: null,
  isLoading: false,

  setWorkspace: async (path) => {
    set({ isLoading: true });
    try {
      const tree = await fsSetWorkspace(path);
      set({ workspace: path, tree, isLoading: false });
    } catch (e) {
      console.error(e);
      set({ isLoading: false });
    }
  },

  refreshTree: async () => {
    const { workspace } = get();
    if (!workspace) return;
    const tree = await fsListDir(workspace);
    set({ tree });
  },

  openFile: async (path, name) => {
    const { tabs } = get();
    // If already open, just activate it
    if (tabs.find((t) => t.path === path)) {
      set({ activeTab: path });
      return;
    }
    try {
      const result: ReadFileResult = await fsReadFile(path);
      const tab: OpenTab = {
        path,
        name,
        content: result.content,
        language: result.language,
        isDirty: false,
      };
      set({ tabs: [...tabs, tab], activeTab: path });
    } catch (e) {
      console.error("Cannot open file:", e);
    }
  },

  closeTab: (path) => {
    const { tabs, activeTab } = get();
    const idx = tabs.findIndex((t) => t.path === path);
    const newTabs = tabs.filter((t) => t.path !== path);
    let newActive = activeTab;
    if (activeTab === path) {
      newActive = newTabs[Math.min(idx, newTabs.length - 1)]?.path ?? null;
    }
    set({ tabs: newTabs, activeTab: newActive });
  },

  setActiveTab: (path) => set({ activeTab: path }),

  saveActiveFile: async () => {
    const { tabs, activeTab } = get();
    const tab = tabs.find((t) => t.path === activeTab);
    if (!tab) return;
    await fsWriteFile(tab.path, tab.content);
    set({
      tabs: tabs.map((t) => (t.path === activeTab ? { ...t, isDirty: false } : t)),
    });
  },

  updateTabContent: (path, content) => {
    set((state) => ({
      tabs: state.tabs.map((t) =>
        t.path === path ? { ...t, content, isDirty: true } : t
      ),
    }));
  },
}));
