import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import type {
  SearchResponse,
  FileRecord,
  AppRecord,
  AppSettings,
  IndexStats,
  ServiceStatus,
  IndexingProgress,
} from '../types';

// ─── Arama ──────────────────────────────────────────────────────────────────

export const ipcSearch = (
  query: string,
  page = 0,
  pageSize = 50
): Promise<SearchResponse> => invoke('search', { query, page, pageSize });

export const ipcGetApps = (query: string): Promise<AppRecord[]> =>
  invoke('get_apps', { query });

// ─── Dosya İşlemleri ────────────────────────────────────────────────────────

export const ipcOpenFile = (path: string): Promise<void> =>
  invoke('open_file', { path });

export const ipcOpenFileLocation = (path: string): Promise<void> =>
  invoke('open_file_location', { path });

export const ipcRunAsAdmin = (path: string): Promise<void> =>
  invoke('run_as_admin', { path });

export const ipcDeleteFile = (path: string): Promise<void> =>
  invoke('delete_file', { path });

export const ipcRenameFile = (oldPath: string, newName: string): Promise<string> =>
  invoke('rename_file', { oldPath, newName });

export const ipcCopyToClipboard = (path: string): Promise<void> =>
  invoke('copy_to_clipboard', { path });

export const ipcMoveFile = (source: string, destination: string): Promise<void> =>
  invoke('move_file', { source, destination });

// ─── Son Kullanılanlar & Favoriler ──────────────────────────────────────────

export const ipcGetRecent = (limit = 20): Promise<FileRecord[]> =>
  invoke('get_recent', { limit });

export const ipcGetFavorites = (): Promise<FileRecord[]> => invoke('get_favorites');

export const ipcToggleFavorite = (path: string): Promise<boolean> =>
  invoke('toggle_favorite', { path });

// ─── Ayarlar ────────────────────────────────────────────────────────────────

export const ipcGetSettings = (): Promise<Partial<AppSettings>> =>
  invoke('get_settings');

export const ipcUpdateSetting = (key: string, value: string): Promise<void> =>
  invoke('update_setting', { key, value });

// ─── İndeks & Servis ────────────────────────────────────────────────────────

export const ipcGetIndexStats = (): Promise<IndexStats> => invoke('get_index_stats');

export const ipcGetServiceStatus = (): Promise<ServiceStatus> =>
  invoke('get_service_status');

export const ipcInstallService = (): Promise<void> => invoke('install_service');

// ─── Arama Geçmişi ──────────────────────────────────────────────────────────

export const ipcGetSearchHistory = (limit = 20): Promise<string[]> =>
  invoke('get_search_history', { limit });

export const ipcAddSearchHistory = (query: string): Promise<void> =>
  invoke('add_search_history', { query });

// ─── Pencere Yönetimi ───────────────────────────────────────────────────────

export const ipcHideWindow = (): Promise<void> => invoke('hide_window');

export const ipcShowWindow = (): Promise<void> => invoke('show_window');

// ─── Event Dinleyiciler ─────────────────────────────────────────────────────

export const onIndexUpdated = (
  cb: (payload: { path: string; event: string }) => void
): Promise<UnlistenFn> =>
  listen<{ path: string; event: string }>('index-updated', (e) => cb(e.payload));

export const onServiceStatusChanged = (
  cb: (payload: ServiceStatus) => void
): Promise<UnlistenFn> =>
  listen<ServiceStatus>('service-status-changed', (e) => cb(e.payload));

export const onWindowShown = (cb: () => void): Promise<UnlistenFn> =>
  listen('window-shown', () => cb());

export const onOpenSettings = (cb: () => void): Promise<UnlistenFn> =>
  listen('open-settings', () => cb());

export const onIndexingProgress = (
  cb: (payload: IndexingProgress) => void
): Promise<UnlistenFn> =>
  listen<IndexingProgress>('indexing-progress', (e) => cb(e.payload));

/** Global kısayolun gerçekte hangi kombinasyona bağlandığı (null: hiçbiri). */
export const onHotkeyRegistered = (
  cb: (combo: string | null) => void
): Promise<UnlistenFn> =>
  listen<string | null>('hotkey-registered', (e) => cb(e.payload));
