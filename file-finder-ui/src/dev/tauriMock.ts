/**
 * Tasarım önizlemesi — YALNIZ geliştirme içindir, uygulama paketine girmez
 * (vite yalnız index.html'i derler; bu giriş noktası dist'e çıkmaz).
 *
 * Tauri IPC'sini sınırından taklit eder: `window.__TAURI_INTERNALS__` sahte
 * veri döndürür, böylece App bileşeni hiç değiştirilmeden tarayıcıda çalışır
 * ve ekranların tasarımı gerçek bileşenler üzerinden görülebilir.
 *
 * Kullanım:  npm run dev  →  http://localhost:1420/preview.html
 */
// ─── Sahte veri ─────────────────────────────────────────────────────────────

const now = Math.floor(Date.now() / 1000);
const day = 86400;

const FILES = [
  {
    id: 1,
    name: 'FootballManager24',
    extension: null,
    parent_path: 'D:\\Oyunlar\\FootballManager24',
    full_path: 'D:\\Oyunlar\\FootballManager24',
    size: 19_756_051_005,
    modified_time: now - 3 * day,
    created_time: now - 300 * day,
    is_directory: true,
    score: 0.98,
    match_positions: [0, 1, 2, 3, 4, 5, 6, 7],
    result_type: 'Directory',
  },
  {
    id: 2,
    name: 'Football Manager 2024',
    extension: null,
    parent_path: 'C:\\Users\\Kullanici\\Belgeler\\Sports Interactive',
    full_path: 'C:\\Users\\Kullanici\\Belgeler\\Sports Interactive\\Football Manager 2024',
    size: null,
    modified_time: now - 2 * day,
    created_time: now - 200 * day,
    is_directory: true,
    score: 0.9,
    match_positions: [0, 1, 2, 3, 4, 5, 6, 7],
    result_type: 'Directory',
  },
  {
    id: 3,
    name: 'footer.ts',
    extension: 'ts',
    parent_path: 'D:\\projects\\modes\\components',
    full_path: 'D:\\projects\\modes\\components\\footer.ts',
    size: 10_240,
    modified_time: now - 9 * day,
    created_time: now - 90 * day,
    is_directory: false,
    score: 0.8,
    match_positions: [0, 1, 2, 3, 4, 5],
    result_type: 'File',
  },
  {
    id: 4,
    name: 'Footer.jsx',
    extension: 'jsx',
    parent_path: 'D:\\projects\\src\\components',
    full_path: 'D:\\projects\\src\\components\\Footer.jsx',
    size: 4_608,
    modified_time: now - 40 * day,
    created_time: now - 120 * day,
    is_directory: false,
    score: 0.78,
    match_positions: [0, 1, 2, 3, 4, 5],
    result_type: 'File',
  },
  {
    id: 5,
    name: 'fm24-manager-profil.png',
    extension: 'png',
    parent_path: 'C:\\Users\\Kullanici\\Resimler\\Screenshots',
    full_path: 'C:\\Users\\Kullanici\\Resimler\\Screenshots\\fm24-manager-profil.png',
    size: 1_258_291,
    modified_time: now - 12 * day,
    created_time: now - 12 * day,
    is_directory: false,
    score: 0.6,
    match_positions: [0, 1],
    result_type: 'File',
  },
  {
    id: 6,
    name: 'FM24-data-update.zip',
    extension: 'zip',
    parent_path: 'C:\\Users\\Kullanici\\Indirilenler',
    full_path: 'C:\\Users\\Kullanici\\Indirilenler\\FM24-data-update.zip',
    size: 882_900_992,
    modified_time: now - 70 * day,
    created_time: now - 70 * day,
    is_directory: false,
    score: 0.55,
    match_positions: [0, 1],
    result_type: 'File',
  },
  {
    id: 7,
    name: 'fm-taktik-notlari.pdf',
    extension: 'pdf',
    parent_path: 'C:\\Users\\Kullanici\\Belgeler',
    full_path: 'C:\\Users\\Kullanici\\Belgeler\\fm-taktik-notlari.pdf',
    size: 2_411_724,
    modified_time: now - 1 * day,
    created_time: now - 30 * day,
    is_directory: false,
    score: 0.5,
    match_positions: [0, 1],
    result_type: 'File',
  },
  {
    id: 8,
    name: 'fm24-highlights.mp4',
    extension: 'mp4',
    parent_path: 'D:\\Videolar\\Kayitlar',
    full_path: 'D:\\Videolar\\Kayitlar\\fm24-highlights.mp4',
    size: 734_003_200,
    modified_time: now - 5 * day,
    created_time: now - 5 * day,
    is_directory: false,
    score: 0.48,
    match_positions: [0, 1],
    result_type: 'File',
  },
];

const APPS = [
  {
    id: 101,
    name: 'Football Manager 2024',
    exe_path: 'C:\\Program Files (x86)\\Steam\\steamapps\\common\\FM24\\fm.exe',
    icon_path: null,
    source: 'start_menu',
    score: 0.95,
  },
];

const SETTINGS: Record<string, string> = {
  theme: 'dark',
  hotkey: 'Ctrl+Space',
  startup_with_windows: 'true',
  hide_on_blur: 'false',
  show_preview: 'true',
  animation_speed: 'normal',
  fuzzy_threshold: '0.3',
  excluded_paths: '',
  max_results: '50',
};

// ─── IPC taklidi ────────────────────────────────────────────────────────────

const listeners = new Map<number, (payload: unknown) => void>();
let callbackId = 0;

function mockInvoke(cmd: string, args: any): Promise<unknown> {
  switch (cmd) {
    case 'plugin:event|listen':
      return Promise.resolve(0);
    case 'plugin:event|unlisten':
      return Promise.resolve();
    case 'plugin:path|resolve_directory':
      return Promise.resolve('C:\\Users\\Kullanici');
    case 'search': {
      const q: string = (args?.query ?? '').toLowerCase();
      const terms = q.split(/\s+/).filter(Boolean);
      const items = FILES.filter((f) =>
        terms.every((t) => f.name.toLowerCase().includes(t.replace(/^[a-z]+:/, '')))
      );
      return Promise.resolve({
        items,
        total: items.length + 142,
        ranked: items.length,
        has_more: false,
        page: 0,
        elapsed_ms: 42,
        query_parsed: {
          text: q,
          extensions: [],
          size_min: null,
          size_max: null,
          modified_after: null,
          modified_before: null,
          created_after: null,
          created_before: null,
          path_prefix: null,
          is_regex: false,
          regex_pattern: null,
        },
      });
    }
    case 'get_apps': {
      const q: string = (args?.query ?? '').toLowerCase();
      return Promise.resolve(
        q.length > 1 ? APPS.filter((a) => a.name.toLowerCase().includes(q.slice(0, 3))) : []
      );
    }
    case 'get_settings':
      return Promise.resolve(SETTINGS);
    case 'update_setting':
      SETTINGS[args.key] = args.value;
      return Promise.resolve();
    case 'get_index_stats':
      return Promise.resolve({
        total_files: 1_284_902,
        service_connected: true,
        mode: 'MftFull',
        last_updated: now,
      });
    case 'get_service_status':
      return Promise.resolve({ connected: true, mode: 'MftFull' });
    case 'get_favorites':
      return Promise.resolve([{ ...FILES[0] }]);
    case 'get_recent':
      return Promise.resolve([]);
    case 'get_search_history':
      return Promise.resolve(['football manager', 'footer', 'ext:pdf']);
    default:
      return Promise.resolve(null);
  }
}

(window as any).__TAURI_INTERNALS__ = {
  invoke: mockInvoke,
  transformCallback(cb: (payload: unknown) => void) {
    const id = ++callbackId;
    listeners.set(id, cb);
    (window as any)[`_${id}`] = cb;
    return id;
  },
  metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
  plugins: {},
};
