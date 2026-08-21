import { create } from 'zustand';
import type {
  AppSettings,
  Theme,
  IndexStats,
  ServiceStatus,
  AnimationSpeed,
} from '../types';
import {
  ipcGetSettings,
  ipcUpdateSetting,
  ipcGetIndexStats,
  ipcGetServiceStatus,
} from '../lib/ipc';

const DEFAULTS: AppSettings = {
  theme: 'dark',
  hotkey: 'Ctrl+Space',
  startup_with_windows: 'false',
  hide_on_blur: 'false',
  show_preview: 'true',
  animation_speed: 'normal',
  fuzzy_threshold: '0.3',
  excluded_paths: '',
  max_results: '50',
};

const SPEED_SCALE: Record<AnimationSpeed, number> = {
  fast: 0.6,
  normal: 1,
  slow: 1.6,
  off: 0,
};

interface SettingsState {
  settings: AppSettings;
  indexStats: IndexStats | null;
  serviceStatus: ServiceStatus | null;
  /** Kısayolun gerçekte bağlandığı kombinasyon; null ise hiçbiri bağlanamadı. */
  boundHotkey: string | null;
  isLoaded: boolean;

  loadSettings: () => Promise<void>;
  updateSetting: (key: keyof AppSettings, value: string) => Promise<void>;
  loadIndexStats: () => Promise<void>;
  loadServiceStatus: () => Promise<void>;
  setServiceStatus: (status: ServiceStatus) => void;
  setBoundHotkey: (combo: string | null) => void;
}

const systemTheme = (): 'dark' | 'light' =>
  window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';

/** Tema ve animasyon hızını belgeye uygula — ayarlar kaydedilip unutulmasın. */
function applyToDocument(settings: AppSettings) {
  const theme: Theme = settings.theme || 'dark';
  const resolved = theme === 'system' ? systemTheme() : theme;
  document.documentElement.setAttribute('data-theme', resolved);

  const speed = SPEED_SCALE[settings.animation_speed] ?? 1;
  document.documentElement.style.setProperty('--speed', String(speed));
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: DEFAULTS,
  indexStats: null,
  serviceStatus: null,
  boundHotkey: null,
  isLoaded: false,

  loadSettings: async () => {
    try {
      const raw = await ipcGetSettings();
      const settings = { ...DEFAULTS, ...(raw as Partial<AppSettings>) };
      applyToDocument(settings);
      set({ settings, isLoaded: true });
    } catch {
      applyToDocument(DEFAULTS);
      set({ isLoaded: true });
    }
  },

  updateSetting: async (key, value) => {
    const settings = { ...get().settings, [key]: value } as AppSettings;
    // Önce arayüze uygula: ayar anında görünsün, kayıt arkada tamamlansın.
    applyToDocument(settings);
    set({ settings });
    try {
      await ipcUpdateSetting(key, value);
    } catch (e) {
      console.error('Ayar kaydedilemedi:', key, e);
    }
  },

  loadIndexStats: async () => {
    try {
      set({ indexStats: await ipcGetIndexStats() });
    } catch {
      /* indeks henüz hazır değil */
    }
  },

  loadServiceStatus: async () => {
    try {
      set({ serviceStatus: await ipcGetServiceStatus() });
    } catch {
      /* servis cevap vermiyor */
    }
  },

  setServiceStatus: (status) => set({ serviceStatus: status }),
  setBoundHotkey: (combo) => set({ boundHotkey: combo }),
}));

/** Ayarlarda yazan kısayol ile gerçekte bağlanan aynı mı? */
export function hotkeyMismatch(state: SettingsState): boolean {
  return state.boundHotkey !== null && state.boundHotkey !== state.settings.hotkey;
}
