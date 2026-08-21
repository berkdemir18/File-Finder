import type { SearchResult } from '../types';
import {
  ipcOpenFile,
  ipcOpenFileLocation,
  ipcRunAsAdmin,
  ipcCopyToClipboard,
  ipcToggleFavorite,
  ipcAddSearchHistory,
  ipcHideWindow,
} from './ipc';

/**
 * Sonucu aç. Arama geçmişine yalnız BURADA yazılır: kullanıcı bir sonucu
 * gerçekten açtıysa o sorgu işe yaramıştır. Her tuş vuruşunda yazsaydık
 * geçmiş "f", "fo", "foo" gibi yarım sorgularla dolardı.
 */
export async function openResult(item: SearchResult, committedQuery: string) {
  const q = committedQuery.trim();
  if (q.length > 1) {
    ipcAddSearchHistory(q).catch(() => {});
  }
  try {
    await ipcOpenFile(item.full_path);
    await ipcHideWindow();
  } catch (e) {
    console.error('Açılamadı:', item.full_path, e);
  }
}

export async function revealResult(item: SearchResult) {
  try {
    await ipcOpenFileLocation(item.full_path);
    await ipcHideWindow();
  } catch (e) {
    console.error('Konum açılamadı:', item.full_path, e);
  }
}

export async function runResultAsAdmin(item: SearchResult) {
  try {
    await ipcRunAsAdmin(item.full_path);
    await ipcHideWindow();
  } catch (e) {
    console.error('Yönetici olarak çalıştırılamadı:', item.full_path, e);
  }
}

export async function copyResultPath(item: SearchResult) {
  try {
    await ipcCopyToClipboard(item.full_path);
  } catch (e) {
    console.error('Panoya kopyalanamadı:', item.full_path, e);
  }
}

export async function toggleResultFavorite(item: SearchResult): Promise<boolean> {
  try {
    return await ipcToggleFavorite(item.full_path);
  } catch (e) {
    console.error('Favori değiştirilemedi:', item.full_path, e);
    return false;
  }
}
