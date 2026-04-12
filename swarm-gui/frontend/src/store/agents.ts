import { create } from "zustand";
import { HiveStatus, agentsList, agentsStartSwarm, agentsSendMessage } from "@/lib/tauri";

interface AgentsState {
  hive: HiveStatus | null;
  isLoading: boolean;
  swarmActive: boolean;
  load: () => Promise<void>;
  startSwarm: () => Promise<void>;
  sendMessage: (message: string, to?: string) => Promise<void>;
}

export const useAgentsStore = create<AgentsState>((set) => ({
  hive: null,
  isLoading: false,
  swarmActive: false,

  load: async () => {
    set({ isLoading: true });
    try {
      const hive = await agentsList();
      set({ hive, isLoading: false });
    } catch (e) {
      console.error(e);
      set({ isLoading: false });
    }
  },

  startSwarm: async () => {
    set({ isLoading: true });
    try {
      const hive = await agentsStartSwarm();
      set({ hive, swarmActive: true, isLoading: false });
    } catch (e) {
      console.error(e);
      set({ isLoading: false });
    }
  },

  sendMessage: async (message, to) => {
    await agentsSendMessage(message, to);
  },
}));
