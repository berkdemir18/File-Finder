import { create } from 'zustand';
import type { SearchResult, ParsedQuery } from '../types';
import { ipcSearch, ipcGetApps } from '../lib/ipc';
import { getFileKind, groupOf, type FilterGroup } from '../lib/fileKinds';

interface SearchState {
  query: string;
  results: SearchResult[];
  /** Eşleşen gerçek dosya sayısı — gösterilen satır sayısıyla aynı değil. */
  total: number;
  ranked: number;
  /** Aday havuzu doldu mu — sayaç "2000+" diye yazılır. */
  poolFull: boolean;
  hasMore: boolean;
  elapsedMs: number;
  parsedQuery: ParsedQuery | null;
  page: number;
  pageSize: number;
  typeFilter: FilterGroup;
  selectedIndex: number;
  isLoading: boolean;
  error: string | null;

  setQuery: (q: string) => void;
  setTypeFilter: (f: FilterGroup) => void;
  setSelectedIndex: (i: number) => void;
  setPageSize: (n: number) => void;
  moveSelection: (delta: number, visibleCount: number) => void;
  loadMore: () => Promise<void>;
  clear: () => void;
}

/**
 * Her istek bir sıra numarası taşır; yalnız en güncel isteğin cevabı yazılır.
 * Aksi hâlde yavaş dönen eski bir arama, yeni sonuçların üstüne biner.
 */
let requestSeq = 0;
let debounceTimer: ReturnType<typeof setTimeout> | null = null;

// Arama tam tarama gerektiriyor (~1 sn, 5M kayıt). Kısa debounce her harfte
// yeni bir tarama kuyruğa sokup DB kilidini tıkıyordu.
const DEBOUNCE_MS = 220;

export const useSearchStore = create<SearchState>((set, get) => ({
  query: '',
  results: [],
  total: 0,
  ranked: 0,
  poolFull: false,
  hasMore: false,
  elapsedMs: 0,
  parsedQuery: null,
  page: 0,
  pageSize: 50,
  typeFilter: 'all',
  selectedIndex: 0,
  isLoading: false,
  error: null,

  setQuery: (q) => {
    set({ query: q, selectedIndex: 0, page: 0 });

    if (debounceTimer) clearTimeout(debounceTimer);

    if (!q.trim()) {
      requestSeq++; // uçuştaki cevapları geçersiz kıl
      set({
        results: [],
        total: 0,
        ranked: 0,
        poolFull: false,
        hasMore: false,
        parsedQuery: null,
        elapsedMs: 0,
        isLoading: false,
        error: null,
      });
      return;
    }

    debounceTimer = setTimeout(() => void runSearch(set, get, q, 0), DEBOUNCE_MS);
  },

  setTypeFilter: (f) => set({ typeFilter: f, selectedIndex: 0 }),

  setSelectedIndex: (i) => set({ selectedIndex: Math.max(0, i) }),

  setPageSize: (n) => set({ pageSize: Math.max(10, n) }),

  moveSelection: (delta, visibleCount) => {
    if (visibleCount === 0) return;
    const next = get().selectedIndex + delta;
    set({ selectedIndex: Math.min(Math.max(next, 0), visibleCount - 1) });
  },

  loadMore: async () => {
    const { query, page, hasMore, isLoading } = get();
    if (!hasMore || isLoading || !query.trim()) return;
    await runSearch(set, get, query, page + 1, true);
  },

  clear: () => {
    requestSeq++;
    if (debounceTimer) clearTimeout(debounceTimer);
    set({
      query: '',
      results: [],
      total: 0,
      ranked: 0,
      poolFull: false,
      hasMore: false,
      parsedQuery: null,
      elapsedMs: 0,
      selectedIndex: 0,
      page: 0,
      isLoading: false,
      error: null,
    });
  },
}));

type Setter = (partial: Partial<SearchState>) => void;

async function runSearch(
  set: Setter,
  get: () => SearchState,
  query: string,
  page: number,
  append = false
) {
  const seq = ++requestSeq;
  const { pageSize } = get();
  set({ isLoading: true, error: null });

  try {
    // Uygulamalar yalnız ilk sayfada aranır; sonraki sayfalar dosya listesini uzatır.
    const [fileResponse, appResults] = await Promise.all([
      ipcSearch(query, page, pageSize),
      page === 0 ? ipcGetApps(query) : Promise.resolve([]),
    ]);

    if (seq !== requestSeq) return; // daha yeni bir arama var

    const apps: SearchResult[] = appResults.slice(0, 5).map((app) => ({
      id: app.id,
      name: app.name,
      extension: null,
      parent_path: app.exe_path,
      full_path: app.exe_path,
      size: null,
      modified_time: null,
      created_time: null,
      is_directory: false,
      score: Math.min(app.score + 0.1, 1),
      match_positions: [],
      result_type: 'App' as const,
    }));

    const incoming = [...apps, ...fileResponse.items];
    const results = append ? [...get().results, ...fileResponse.items] : incoming;

    set({
      results,
      total: fileResponse.total + apps.length,
      ranked: fileResponse.ranked,
      poolFull: fileResponse.pool_full,
      hasMore: fileResponse.has_more,
      elapsedMs: fileResponse.elapsed_ms,
      parsedQuery: fileResponse.query_parsed,
      page: fileResponse.page,
      isLoading: false,
    });
  } catch (err) {
    if (seq !== requestSeq) return;
    set({ isLoading: false, error: String(err), results: append ? get().results : [] });
  }
}

// ─── Türetilmiş seçiciler ───────────────────────────────────────────────────

export function filterResults(
  results: SearchResult[],
  filter: FilterGroup
): SearchResult[] {
  if (filter === 'all') return results;
  return results.filter((r) => {
    const kind = getFileKind(r.extension, r.is_directory, r.result_type === 'App');
    return groupOf(kind) === filter;
  });
}

/** Filtre şeridindeki sayaçlar — yüklenmiş sonuçlar içinde sayar. */
export function countByGroup(results: SearchResult[]): Record<FilterGroup, number> {
  const counts: Record<FilterGroup, number> = {
    all: results.length,
    folder: 0,
    doc: 0,
    code: 0,
    media: 0,
    app: 0,
  };
  for (const r of results) {
    const group = groupOf(
      getFileKind(r.extension, r.is_directory, r.result_type === 'App')
    );
    if (group !== 'all') counts[group]++;
  }
  return counts;
}
