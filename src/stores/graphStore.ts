import { create } from 'zustand';
import type { GraphSettings, GraphPanelDataDto } from '../lib/types';
import * as api from '../lib/commands';

const DEFAULT_GRAPH_SETTINGS: GraphSettings = {
  show_connected: true,
  show_orphaned: true,
  show_tags: true,
  charge_strength: -4,
  link_distance: 40,
  alpha_decay: 0.15,
  velocity_decay: 0.4,
  link_curvature: 0.15,
  node_cap: 0,
};

export interface GraphState {
  settings: GraphSettings;
  data: GraphPanelDataDto | null;
  loading: boolean;
  error: string | null;
  setSettings: (settings: GraphSettings) => void;
  loadData: () => Promise<void>;
  saveSettings: () => Promise<void>;
}

export const useGraphStore = create<GraphState>((set, get) => ({
  settings: DEFAULT_GRAPH_SETTINGS,
  data: null,
  loading: false,
  error: null,

  setSettings: (settings) => set({ settings }),

  loadData: async () => {
    set({ loading: true, error: null });
    try {
      const graphData = await api.getGraphPanelData();
      set({ data: graphData, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  saveSettings: async () => {
    try {
      await api.saveGraphSettings(get().settings);
    } catch {
      // silently fail
    }
  },
}));
