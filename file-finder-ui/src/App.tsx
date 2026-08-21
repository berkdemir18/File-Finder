import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { homeDir } from '@tauri-apps/api/path';
import { Minus, Square, X, Settings2, ArrowLeft } from 'lucide-react';

import { Logo, LensGlyph } from './components/Brand/Logo';
import { ResultsList } from './components/ResultsList/ResultsList';
import { PreviewPanel } from './components/Preview/PreviewPanel';
import { Spotlight, CommandHints } from './components/Spotlight/Spotlight';
import { ContextMenu } from './components/ContextMenu/ContextMenu';
import { ConfirmDialog } from './components/Confirm/ConfirmDialog';
import { SettingsWindow } from './components/Settings/SettingsWindow';

import { useSearchStore, filterResults, countByGroup } from './stores/searchStore';
import { useSettingsStore } from './stores/settingsStore';
import { useHistoryStore } from './stores/historyStore';
import { useKeyboardNav } from './hooks/useKeyboardNav';
import { FILTER_GROUPS, type FilterGroup } from './lib/fileKinds';
import { formatCount } from './lib/format';
import { openResult, revealResult, toggleResultFavorite } from './lib/actions';
import { ipcDeleteFile, ipcHideWindow } from './lib/ipc';
import {
  onServiceStatusChanged,
  onOpenSettings,
  onWindowShown,
  onIndexingProgress,
  onHotkeyRegistered,
} from './lib/ipc';
import type { SearchResult, IndexingProgress } from './types';

const appWindow = getCurrentWindow();

interface MenuState {
  visible: boolean;
  x: number;
  y: number;
  item: SearchResult | null;
}

const App: React.FC = () => {
  const inputRef = useRef<HTMLInputElement>(null);

  const {
    query,
    results,
    total,
    poolFull,
    hasMore,
    elapsedMs,
    isLoading,
    error,
    typeFilter,
    selectedIndex,
    setQuery,
    setTypeFilter,
    setSelectedIndex,
    setPageSize,
    loadMore,
    clear,
  } = useSearchStore();

  const {
    settings,
    indexStats,
    serviceStatus,
    boundHotkey,
    loadSettings,
    loadIndexStats,
    loadServiceStatus,
    setServiceStatus,
    setBoundHotkey,
  } = useSettingsStore();

  const { favorites, loadFavorites } = useHistoryStore();

  const [view, setView] = useState<'search' | 'settings'>('search');
  const [menu, setMenu] = useState<MenuState>({ visible: false, x: 0, y: 0, item: null });
  const [pendingDelete, setPendingDelete] = useState<SearchResult | null>(null);
  const [indexing, setIndexing] = useState<IndexingProgress | null>(null);
  const [home, setHome] = useState<string | null>(null);

  // ─── Kurulum ──────────────────────────────────────────────────────────────

  useEffect(() => {
    loadSettings();
    loadIndexStats();
    loadServiceStatus();
    loadFavorites();
    homeDir().then(setHome).catch(() => setHome(null));
  }, [loadSettings, loadIndexStats, loadServiceStatus, loadFavorites]);

  // Sayfa boyutu ayardan gelir.
  useEffect(() => {
    const n = Number(settings.max_results);
    if (Number.isFinite(n) && n > 0) setPageSize(n);
  }, [settings.max_results, setPageSize]);

  useEffect(() => {
    const cleanups: Array<() => void> = [];
    const track = (p: Promise<() => void>) => p.then((fn) => cleanups.push(fn));

    track(onServiceStatusChanged((status) => {
      setServiceStatus(status);
      loadIndexStats();
    }));
    track(onOpenSettings(() => setView('settings')));
    track(onWindowShown(() => {
      setView('search');
      setTimeout(() => inputRef.current?.focus(), 40);
    }));
    track(onHotkeyRegistered(setBoundHotkey));
    track(onIndexingProgress((p) => {
      setIndexing(p);
      if (p.phase === 'Complete') {
        loadIndexStats();
        setTimeout(() => setIndexing(null), 1500);
      }
    }));

    return () => cleanups.forEach((fn) => fn());
  }, [setServiceStatus, setBoundHotkey, loadIndexStats]);

  // ─── Türetilmiş veri ──────────────────────────────────────────────────────

  const visible = useMemo(() => filterResults(results, typeFilter), [results, typeFilter]);
  const counts = useMemo(() => countByGroup(results), [results]);
  const selected = visible[selectedIndex] ?? null;

  const favoritePaths = useMemo(
    () => new Set(favorites.map((f) => f.full_path)),
    [favorites]
  );

  const isCommandMode = /(^\/)|(\b(ext|size|modified|created|path):)/.test(query);
  const showPreview = settings.show_preview !== 'false';
  const hotkeyLabel = boundHotkey ?? settings.hotkey;

  const quickFilters = useMemo(() => {
    const list = [
      { label: 'Bugün değişenler', query: 'modified:today' },
      { label: 'Son 7 gün', query: 'modified:<7d' },
      { label: '50 MB üstü', query: 'size:>50mb' },
    ];
    // Yol filtresi boşluk içeremediği için yalnız boşluksuz klasörler önerilir.
    if (home) {
      const downloads = `${home.replace(/[\\/]+$/, '')}\\Downloads`;
      if (!downloads.includes(' ')) {
        list.push({ label: 'İndirilenler', query: `path:${downloads}` });
      }
    }
    return list;
  }, [home]);

  // ─── Eylemler ─────────────────────────────────────────────────────────────

  const handleOpen = useCallback(
    (item: SearchResult) => void openResult(item, query),
    [query]
  );

  const handleContextMenu = useCallback(
    (e: React.MouseEvent, item: SearchResult) => {
      e.preventDefault();
      const index = visible.indexOf(item);
      if (index >= 0) setSelectedIndex(index);
      setMenu({ visible: true, x: e.clientX, y: e.clientY, item });
    },
    [visible, setSelectedIndex]
  );

  const openMenuForIndex = useCallback((index: number, item: SearchResult) => {
    const el = document.querySelector(`[data-result-index="${index}"]`);
    const rect = el?.getBoundingClientRect();
    setMenu({
      visible: true,
      x: rect ? rect.left + 24 : 120,
      y: rect ? rect.bottom : 120,
      item,
    });
  }, []);

  const confirmDelete = useCallback(async () => {
    const item = pendingDelete;
    setPendingDelete(null);
    if (!item) return;
    try {
      await ipcDeleteFile(item.full_path);
      // Silinen satır listede kalmasın.
      useSearchStore.setState((s) => ({
        results: s.results.filter((r) => r.full_path !== item.full_path),
      }));
    } catch (e) {
      console.error('Silinemedi:', item.full_path, e);
    }
  }, [pendingDelete]);

  useKeyboardNav({
    visible,
    enabled: view === 'search' && !menu.visible && pendingDelete === null,
    onRequestDelete: setPendingDelete,
    onOpenContextMenu: openMenuForIndex,
  });

  // ─── Görünüm ──────────────────────────────────────────────────────────────

  const hasQuery = query.trim().length > 0;
  const showResults = visible.length > 0;
  const showNoMatch = hasQuery && !isLoading && results.length === 0;
  const showFilteredEmpty = hasQuery && results.length > 0 && visible.length === 0;

  return (
    <div className="app">
      <div className="titlebar" data-tauri-drag-region>
        <div className="titlebar-brand" data-tauri-drag-region>
          <Logo size={22} />
          <span data-tauri-drag-region>File Finder</span>
        </div>

        <div className="titlebar-actions">
          <button
            className={`titlebar-btn${view === 'settings' ? ' active' : ''}`}
            title={view === 'settings' ? 'Aramaya dön' : 'Ayarlar'}
            onClick={() => setView(view === 'settings' ? 'search' : 'settings')}
          >
            {view === 'settings' ? <ArrowLeft size={16} /> : <Settings2 size={16} />}
          </button>
          <button className="titlebar-btn" title="Küçült" onClick={() => appWindow.minimize()}>
            <Minus size={16} />
          </button>
          <button
            className="titlebar-btn"
            title="Büyüt"
            onClick={() => appWindow.toggleMaximize()}
          >
            <Square size={13} />
          </button>
          <button className="titlebar-btn close" title="Gizle" onClick={() => void ipcHideWindow()}>
            <X size={16} />
          </button>
        </div>
      </div>

      {view === 'settings' ? (
        <SettingsWindow />
      ) : (
        <>
          <div className="search-row">
            <div className={`search-field${isCommandMode ? ' command-mode' : ''}`}>
              <span className="search-glyph">
                {isCommandMode ? '>' : <LensGlyph size={18} />}
              </span>
              <input
                ref={inputRef}
                className="search-input"
                placeholder="Dosya, klasör veya uygulama ara…"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                spellCheck={false}
                autoComplete="off"
                autoFocus
              />
              {hasQuery && (
                <button className="search-clear" title="Temizle (Esc)" onClick={clear}>
                  <X size={14} />
                </button>
              )}
            </div>

            <div className="index-chip">
              <span className="index-chip-label">
                <span
                  className={`dot ${
                    indexing ? 'warn' : serviceStatus?.connected ? 'live' : 'warn'
                  }`}
                />
                {indexing ? 'İndeksleniyor' : serviceStatus?.connected ? 'İndeks hazır' : 'Yedek mod'}
              </span>
              <span className="index-chip-value">
                {indexing
                  ? `${indexing.volume} · ${formatCount(indexing.current)}`
                  : indexStats
                  ? `${formatCount(indexStats.total_files)} dosya`
                  : '—'}
              </span>
            </div>
          </div>

          {hasQuery && showResults && (
            <div className="filter-row">
              {FILTER_GROUPS.map((g) => {
                const count = counts[g.id];
                if (g.id !== 'all' && count === 0) return null;
                return (
                  <button
                    key={g.id}
                    className={`pill${typeFilter === g.id ? ' active' : ''}`}
                    onClick={() => setTypeFilter(g.id as FilterGroup)}
                  >
                    {g.label}
                    <span className="pill-count">{count}</span>
                  </button>
                );
              })}

              <span className="search-stats">
                {formatCount(total)}
                {poolFull ? '+' : ''} eşleşme
                <span className="muted"> · {elapsedMs} ms</span>
              </span>
            </div>
          )}

          {isLoading && <div className="loading-line" style={{ marginTop: 12 }} />}

          <div className="body">
            <div className="results-pane">
              {!hasQuery && isCommandMode === false && (
                <Spotlight
                  indexedFiles={indexStats?.total_files ?? null}
                  hotkey={hotkeyLabel}
                  quickFilters={quickFilters}
                  onPickFilter={(q) => {
                    setQuery(q);
                    inputRef.current?.focus();
                  }}
                />
              )}

              {hasQuery && isCommandMode && !showResults && !isLoading && (
                <CommandHints
                  onPick={(token) => {
                    setQuery(`${query.trim()} ${token}`.trim());
                    inputRef.current?.focus();
                  }}
                />
              )}

              {showResults && (
                <ResultsList
                  visible={visible}
                  selectedIndex={selectedIndex}
                  hasMore={hasMore}
                  isLoading={isLoading}
                  onSelect={setSelectedIndex}
                  onOpen={handleOpen}
                  onContextMenu={handleContextMenu}
                  onLoadMore={loadMore}
                />
              )}

              {showFilteredEmpty && (
                <div className="empty">
                  <div className="empty-title">Bu filtrede sonuç yok</div>
                  <div className="empty-sub">
                    {formatCount(results.length)} sonuç var ama hiçbiri bu türde değil.
                  </div>
                  <button className="btn" onClick={() => setTypeFilter('all')}>
                    Tümünü göster
                  </button>
                </div>
              )}

              {showNoMatch && !isCommandMode && (
                <div className="empty">
                  <div className="empty-title">Sonuç bulunamadı</div>
                  <div className="empty-sub">
                    <b>{query}</b> ile eşleşen bir şey yok. Yazımı kontrol et ya da
                    filtreleri gevşet — hassasiyeti Ayarlar › Arama'dan değiştirebilirsin.
                  </div>
                </div>
              )}

              {error && (
                <div className="empty">
                  <div className="empty-title">Arama yapılamadı</div>
                  <div className="empty-sub">{error}</div>
                </div>
              )}
            </div>

            {showPreview && showResults && (
              <PreviewPanel
                item={selected}
                isFavorite={selected ? favoritePaths.has(selected.full_path) : false}
                onOpen={() => selected && handleOpen(selected)}
                onReveal={() => selected && void revealResult(selected)}
                onToggleFavorite={() => {
                  if (selected) void toggleResultFavorite(selected).then(loadFavorites);
                }}
              />
            )}
          </div>

          <div className="footer">
            <div className="hints">
              <span className="hint">
                <span className="kbd">↑↓</span> Gezin
              </span>
              <span className="hint">
                <span className="kbd">Enter</span> Aç
              </span>
              <span className="hint">
                <span className="kbd">Ctrl+O</span> Konumu aç
              </span>
              <span className="hint">
                <span className="kbd">Ctrl+C</span> Yolu kopyala
              </span>
              <span className="hint">
                <span className="kbd">Esc</span> Kapat
              </span>
            </div>

            <div className="footer-status">
              <span className={`dot ${serviceStatus?.connected ? 'live' : 'warn'}`} />
              {serviceStatus?.connected ? 'NTFS · Canlı izleme' : 'Yedek tarama'}
            </div>
          </div>
        </>
      )}

      <ContextMenu
        visible={menu.visible}
        x={menu.x}
        y={menu.y}
        item={menu.item}
        query={query}
        isFavorite={menu.item ? favoritePaths.has(menu.item.full_path) : false}
        onClose={() => setMenu((m) => ({ ...m, visible: false, item: null }))}
        onRequestDelete={setPendingDelete}
        onFavoriteChanged={loadFavorites}
      />

      <ConfirmDialog
        open={pendingDelete !== null}
        title="Geri dönüşüme gönderilsin mi?"
        body={
          <>
            <b style={{ color: 'var(--text)' }}>{pendingDelete?.name}</b> geri dönüşüm kutusuna
            taşınacak. Oradan geri alabilirsin.
          </>
        }
        confirmLabel="Gönder"
        onConfirm={confirmDelete}
        onCancel={() => setPendingDelete(null)}
      />
    </div>
  );
};

export default App;
