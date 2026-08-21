// ─── Dosya & Uygulama Tipleri ────────────────────────────────────────────────

export interface FileRecord {
  id: number;
  name: string;
  extension: string | null;
  parent_path: string;
  full_path: string;
  size: number | null;
  created_time: number | null;
  modified_time: number | null;
  is_directory: boolean;
  volume_id: string | null;
  frn: number | null;
  score: number;
}

export interface AppRecord {
  id: number;
  name: string;
  exe_path: string;
  icon_path: string | null;
  source: string;
  score: number;
}

// ─── Arama Tipleri ──────────────────────────────────────────────────────────

export type ResultType = 'File' | 'Directory' | 'App';

export interface SearchResult {
  id: number;
  name: string;
  extension: string | null;
  parent_path: string;
  full_path: string;
  size: number | null;
  modified_time: number | null;
  created_time: number | null;
  is_directory: boolean;
  score: number;
  match_positions: number[];
  result_type: ResultType;
}

export interface ParsedQuery {
  text: string;
  extensions: string[];
  size_min: number | null;
  size_max: number | null;
  modified_after: number | null;
  modified_before: number | null;
  created_after: number | null;
  created_before: number | null;
  path_prefix: string | null;
  is_regex: boolean;
  regex_pattern: string | null;
}

export interface SearchResponse {
  items: SearchResult[];
  /** Elde edilen eşleşme sayısı. `pool_full` true ise bu bir alt sınırdır. */
  total: number;
  /** Skorlanıp sıralanan aday sayısı. */
  ranked: number;
  /** Aday havuzu doldu mu? Doluysa gerçekte daha fazla eşleşme var. */
  pool_full: boolean;
  has_more: boolean;
  page: number;
  elapsed_ms: number;
  query_parsed: ParsedQuery;
}

// ─── Ayarlar Tipleri ────────────────────────────────────────────────────────

export type Theme = 'dark' | 'light' | 'system';
export type AnimationSpeed = 'fast' | 'normal' | 'slow' | 'off';

export interface AppSettings {
  theme: Theme;
  hotkey: string;
  startup_with_windows: string;
  hide_on_blur: string;
  show_preview: string;
  animation_speed: AnimationSpeed;
  fuzzy_threshold: string;
  excluded_paths: string;
  max_results: string;
}

// ─── İndeks İstatistikleri ──────────────────────────────────────────────────

export type IndexMode = 'MftFull' | 'Fallback';

export interface IndexStats {
  total_files: number;
  service_connected: boolean;
  mode: IndexMode;
  last_updated: number;
}

export interface ServiceStatus {
  connected: boolean;
  mode: IndexMode;
}

export interface IndexingProgress {
  current: number;
  total: number;
  volume: string;
  phase: string;
}
