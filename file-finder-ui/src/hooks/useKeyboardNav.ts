import { useCallback, useEffect, useRef } from 'react';
import type { SearchResult } from '../types';
import { useSearchStore } from '../stores/searchStore';
import { useHistoryStore } from '../stores/historyStore';
import { ipcHideWindow } from '../lib/ipc';
import { copyResultPath, openResult, revealResult, runResultAsAdmin } from '../lib/actions';

interface Options {
  /** Filtre uygulanmış, ekranda görünen sıra. */
  visible: SearchResult[];
  enabled: boolean;
  onRequestDelete?: (item: SearchResult) => void;
  onOpenContextMenu?: (index: number, item: SearchResult) => void;
}

/**
 * Klavye gezinme. Odak her zaman arama kutusunda kaldığı için kısayollar
 * "input'ta mıyız" testine göre değil, değiştirici tuşlara göre ayrılır —
 * eski sürümde Ctrl+C hiç çalışmıyordu, çünkü odak hep input'taydı.
 */
export function useKeyboardNav({
  visible,
  enabled,
  onRequestDelete,
  onOpenContextMenu,
}: Options) {
  const { selectedIndex, setSelectedIndex, moveSelection, query, setQuery } =
    useSearchStore();
  const { searchHistory, historyIndex, setHistoryIndex, loadSearchHistory } =
    useHistoryStore();

  // Dinleyici bir kez bağlanır; güncel değerleri ref'ten okur.
  const ref = useRef({
    visible,
    selectedIndex,
    query,
    searchHistory,
    historyIndex,
    enabled,
  });
  ref.current = { visible, selectedIndex, query, searchHistory, historyIndex, enabled };

  useEffect(() => {
    loadSearchHistory();
  }, [loadSearchHistory]);

  const handler = useCallback(
    (e: KeyboardEvent) => {
      const s = ref.current;
      if (!s.enabled) return;

      const selected = s.visible[s.selectedIndex];
      const mod = e.ctrlKey || e.metaKey;

      switch (e.key) {
        case 'ArrowDown': {
          e.preventDefault();
          // Sorgu boşken aşağı ok geçmişte gezinir, sonuç listesinde değil.
          if (!s.query && s.searchHistory.length) {
            const next = Math.min(s.historyIndex + 1, s.searchHistory.length - 1);
            setHistoryIndex(next);
            setQuery(s.searchHistory[next] ?? '');
          } else {
            moveSelection(1, s.visible.length);
          }
          break;
        }

        case 'ArrowUp': {
          e.preventDefault();
          if (!s.query && s.historyIndex >= 0) {
            const next = s.historyIndex - 1;
            setHistoryIndex(next);
            setQuery(next < 0 ? '' : s.searchHistory[next] ?? '');
          } else {
            moveSelection(-1, s.visible.length);
          }
          break;
        }

        case 'PageDown':
          e.preventDefault();
          moveSelection(8, s.visible.length);
          break;

        case 'PageUp':
          e.preventDefault();
          moveSelection(-8, s.visible.length);
          break;

        case 'Home':
          if (mod) {
            e.preventDefault();
            setSelectedIndex(0);
          }
          break;

        case 'End':
          if (mod) {
            e.preventDefault();
            setSelectedIndex(s.visible.length - 1);
          }
          break;

        case 'Enter':
          if (!selected) break;
          e.preventDefault();
          if (mod && e.shiftKey) void runResultAsAdmin(selected);
          else void openResult(selected, s.query);
          break;

        case 'Escape':
          e.preventDefault();
          if (s.query) {
            // Önce aramayı temizle, ikinci Esc pencereyi kapatsın.
            useSearchStore.getState().clear();
          } else {
            void ipcHideWindow();
          }
          break;

        case 'c':
        case 'C':
          if (mod && selected && window.getSelection()?.toString() === '') {
            e.preventDefault();
            void copyResultPath(selected);
          }
          break;

        case 'o':
        case 'O':
          if (mod && selected) {
            e.preventDefault();
            void revealResult(selected);
          }
          break;

        case 'Delete':
          if (selected) {
            e.preventDefault();
            onRequestDelete?.(selected);
          }
          break;

        case 'ContextMenu':
          if (selected) {
            e.preventDefault();
            onOpenContextMenu?.(s.selectedIndex, selected);
          }
          break;

        default:
          // Alt+1..9 ile hızlı açma
          if (e.altKey && /^[1-9]$/.test(e.key)) {
            const item = s.visible[Number(e.key) - 1];
            if (item) {
              e.preventDefault();
              setSelectedIndex(Number(e.key) - 1);
              void openResult(item, s.query);
            }
          }
      }
    },
    [moveSelection, setSelectedIndex, setHistoryIndex, setQuery, onRequestDelete, onOpenContextMenu]
  );

  useEffect(() => {
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [handler]);
}
