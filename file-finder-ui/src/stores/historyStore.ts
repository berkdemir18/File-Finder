import { create } from 'zustand';
import type { FileRecord } from '../types';
import { ipcGetRecent, ipcGetFavorites, ipcGetSearchHistory } from '../lib/ipc';

interface HistoryState {
  recentFiles: FileRecord[];
  favorites: FileRecord[];
  searchHistory: string[];
  historyIndex: number; // Yukarı/aşağı ok navigasyonu için

  loadRecent: () => Promise<void>;
  loadFavorites: () => Promise<void>;
  loadSearchHistory: () => Promise<void>;
  setHistoryIndex: (i: number) => void;
  getHistoryQuery: (index: number) => string | null;
}

export const useHistoryStore = create<HistoryState>((set, get) => ({
  recentFiles: [],
  favorites: [],
  searchHistory: [],
  historyIndex: -1,

  loadRecent: async () => {
    try {
      const files = await ipcGetRecent(20);
      set({ recentFiles: files });
    } catch {}
  },

  loadFavorites: async () => {
    try {
      const files = await ipcGetFavorites();
      set({ favorites: files });
    } catch {}
  },

  loadSearchHistory: async () => {
    try {
      const history = await ipcGetSearchHistory(30);
      set({ searchHistory: history, historyIndex: -1 });
    } catch {}
  },

  setHistoryIndex: (i) => set({ historyIndex: i }),

  getHistoryQuery: (index) => {
    const { searchHistory } = get();
    if (index < 0 || index >= searchHistory.length) return null;
    return searchHistory[index];
  },
}));
