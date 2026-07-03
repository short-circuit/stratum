import { create } from 'zustand';
import type { VaultInfo, PageDto } from '../lib/types';
import * as api from '../lib/commands';

export interface ThemeConfig {
  primaryHex: string;
  secondaryHex: string;
  dark: boolean;
  fontSize: number;
}

interface AppState {
  vault: VaultInfo | null;
  pages: PageDto[];
  currentPage: PageDto | null;
  loading: boolean;
  error: string | null;
  themeConfig: ThemeConfig;

  loadVault: () => Promise<void>;
  loadPages: () => Promise<void>;
  openPage: (path: string) => Promise<void>;
  createPage: (path: string, title?: string) => Promise<void>;
  deletePage: (path: string) => Promise<void>;
  pickVaultDirectory: () => Promise<void>;
  setThemeConfig: (config: ThemeConfig) => void;
}

export const useStore = create<AppState>((set, get) => ({
  vault: null,
  pages: [],
  currentPage: null,
  loading: false,
  error: null,
  themeConfig: { primaryHex: '#f97316', secondaryHex: '#6b7280', dark: true, fontSize: 16 },

  loadVault: async () => {
    try {
      set({ loading: true, error: null });
      const vault = await api.getVaultInfo();
      set({ vault });
    } catch (e) {
      set({ error: String(e) });
    } finally {
      set({ loading: false });
    }
  },

  loadPages: async () => {
    try {
      set({ loading: true });
      const { pages } = await api.listPages();
      set({ pages });
    } catch (e) {
      set({ error: String(e) });
    } finally {
      set({ loading: false });
    }
  },

  openPage: async (path: string) => {
    try {
      set({ loading: true });
      const page = await api.openPage(path);
      set({ currentPage: page });
    } catch (e) {
      set({ error: String(e) });
    } finally {
      set({ loading: false });
    }
  },

  createPage: async (path: string, title?: string) => {
    try {
      await api.createPage(path, title);
      await get().loadPages();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  deletePage: async (path: string) => {
    try {
      await api.deletePage(path);
      set({ currentPage: null });
      await get().loadPages();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  pickVaultDirectory: async () => {
    try {
      set({ loading: true, error: null });
      const vault = await api.pickVaultDirectory();
      set({ vault });
      await get().loadPages();
    } catch (e) {
      set({ error: String(e) });
    } finally {
      set({ loading: false });
    }
  },

  setThemeConfig: (config: ThemeConfig) => {
    set({ themeConfig: config });
  },
}));
