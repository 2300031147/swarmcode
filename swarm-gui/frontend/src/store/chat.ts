import { create } from "zustand";
import {
  ChatMessage, ChatResponse, CostReport, ModelEntry,
  chatSendMessage, chatGetModels, chatClearHistory,
  chatCompactHistory, chatGetCost,
} from "@/lib/tauri";
import { uid } from "@/lib/utils";

interface ChatState {
  messages: ChatMessage[];
  model: string;
  models: ModelEntry[];
  isLoading: boolean;
  cost: CostReport | null;
  sendMessage: (text: string) => Promise<void>;
  loadModels: () => Promise<void>;
  setModel: (model: string) => void;
  clearHistory: () => Promise<void>;
  compactHistory: () => Promise<string>;
  refreshCost: () => Promise<void>;
}

export const useChatStore = create<ChatState>((set, get) => ({
  messages: [],
  model: "swarm-model-sonnet",
  models: [],
  isLoading: false,
  cost: null,

  sendMessage: async (text) => {
    const { model, messages } = get();

    // Optimistically add the user message
    const userMsg: ChatMessage = {
      id: uid(),
      role: "user",
      content: text,
      model: null,
      tokens_in: null,
      tokens_out: null,
      timestamp_ms: Date.now(),
    };
    set({ messages: [...messages, userMsg], isLoading: true });

    try {
      const response: ChatResponse = await chatSendMessage(text, model);
      set((state) => ({
        messages: [...state.messages, response.message],
        isLoading: false,
        cost: {
          total_input_tokens: response.total_input_tokens,
          total_output_tokens: response.total_output_tokens,
          estimated_cost_usd: response.total_cost_usd,
          session_messages: state.messages.length + 1,
        },
      }));
    } catch (e) {
      const errMsg: ChatMessage = {
        id: uid(),
        role: "assistant",
        content: `⚠️ Error: ${e instanceof Error ? e.message : String(e)}`,
        model: null,
        tokens_in: null,
        tokens_out: null,
        timestamp_ms: Date.now(),
      };
      set((state) => ({
        messages: [...state.messages, errMsg],
        isLoading: false,
      }));
    }
  },

  loadModels: async () => {
    try {
      const models = await chatGetModels();
      set({ models });
      if (models.length > 0 && !get().model) {
        set({ model: models[0].id });
      }
    } catch (e) {
      console.error("Failed to load models:", e);
    }
  },

  setModel: (model) => set({ model }),

  clearHistory: async () => {
    await chatClearHistory();
    set({ messages: [], cost: null });
  },

  compactHistory: async () => {
    const result = await chatCompactHistory();
    // Reload to reflect compacted state
    set({ messages: [] });
    return result;
  },

  refreshCost: async () => {
    try {
      const cost = await chatGetCost();
      set({ cost });
    } catch (_) {}
  },
}));
