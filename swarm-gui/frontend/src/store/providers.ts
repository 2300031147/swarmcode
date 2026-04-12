import { create } from "zustand";
import { ProviderEntry, providersList, providersTest, ProviderTestResult } from "@/lib/tauri";

interface ProvidersState {
  providers: ProviderEntry[];
  testResults: Record<string, ProviderTestResult>;
  isLoading: boolean;
  load: () => Promise<void>;
  test: (name: string) => Promise<void>;
}

export const useProvidersStore = create<ProvidersState>((set) => ({
  providers: [],
  testResults: {},
  isLoading: false,

  load: async () => {
    set({ isLoading: true });
    try {
      const providers = await providersList();
      set({ providers, isLoading: false });
    } catch (e) {
      console.error(e);
      set({ isLoading: false });
    }
  },

  test: async (name) => {
    try {
      const result = await providersTest(name);
      set((state) => ({
        testResults: { ...state.testResults, [name]: result },
      }));
    } catch (e) {
      console.error(e);
    }
  },
}));
