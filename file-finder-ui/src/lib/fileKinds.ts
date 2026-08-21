/**
 * Dosya türü sınıflandırması — ikon seti ve filtre şeridi bunu paylaşır.
 * Tasarım sistemindeki sekiz aile: klasör · metin · belge · kod · görsel ·
 * arşiv · video · uygulama (+ ses ve tanımsız için nötr aile).
 */

export type FileKind =
  | 'folder'
  | 'app'
  | 'text'
  | 'doc'
  | 'code'
  | 'image'
  | 'video'
  | 'audio'
  | 'archive'
  | 'exe'
  | 'other';

/** Filtre şeridindeki gruplar — birden çok türü tek pile toplar. */
export type FilterGroup = 'all' | 'folder' | 'doc' | 'code' | 'media' | 'app';

const EXT_KIND: Record<string, FileKind> = {
  // Metin
  txt: 'text', md: 'text', rtf: 'text', log: 'text', csv: 'text', ini: 'text',
  // Belge
  pdf: 'doc', doc: 'doc', docx: 'doc', odt: 'doc',
  xls: 'doc', xlsx: 'doc', ods: 'doc',
  ppt: 'doc', pptx: 'doc', odp: 'doc',
  // Kod
  js: 'code', mjs: 'code', cjs: 'code', ts: 'code', jsx: 'code', tsx: 'code',
  py: 'code', rs: 'code', go: 'code', java: 'code', kt: 'code', rb: 'code',
  php: 'code', c: 'code', h: 'code', cpp: 'code', hpp: 'code', cs: 'code',
  swift: 'code', sql: 'code', sh: 'code', ps1: 'code',
  html: 'code', css: 'code', scss: 'code', json: 'code', xml: 'code',
  yaml: 'code', yml: 'code', toml: 'code',
  // Görsel
  jpg: 'image', jpeg: 'image', png: 'image', gif: 'image', bmp: 'image',
  svg: 'image', webp: 'image', ico: 'image', tif: 'image', tiff: 'image',
  heic: 'image', psd: 'image', ai: 'image',
  // Video
  mp4: 'video', mkv: 'video', avi: 'video', mov: 'video', wmv: 'video',
  webm: 'video', flv: 'video', m4v: 'video',
  // Ses
  mp3: 'audio', wav: 'audio', flac: 'audio', aac: 'audio', ogg: 'audio',
  m4a: 'audio', wma: 'audio',
  // Arşiv
  zip: 'archive', rar: 'archive', '7z': 'archive', tar: 'archive',
  gz: 'archive', bz2: 'archive', iso: 'archive', xz: 'archive',
  // Çalıştırılabilir
  exe: 'exe', msi: 'exe', bat: 'exe', cmd: 'exe', com: 'exe', appx: 'exe',
};

export function getFileKind(
  extension: string | null | undefined,
  isDirectory: boolean,
  isApp = false
): FileKind {
  if (isApp) return 'app';
  if (isDirectory) return 'folder';
  const ext = (extension || '').toLowerCase().replace(/^\./, '');
  return EXT_KIND[ext] || 'other';
}

/** İkon karosunun rengi — CSS token'ı döner. */
export function kindColorVar(kind: FileKind): string {
  switch (kind) {
    case 'folder':
      return 'var(--t-folder)';
    case 'text':
      return 'var(--t-text)';
    case 'doc':
      return 'var(--t-doc)';
    case 'code':
      return 'var(--t-code)';
    case 'image':
      return 'var(--t-image)';
    case 'video':
    case 'audio':
      return 'var(--t-video)';
    case 'archive':
      return 'var(--t-archive)';
    default:
      return 'var(--t-app)';
  }
}

/** İkon karosunun içindeki üç harflik etiket. */
export function kindLabel(
  kind: FileKind,
  extension: string | null | undefined
): string {
  if (kind === 'folder') return '';
  if (kind === 'app') return 'APP';
  const ext = (extension || '').toLowerCase().replace(/^\./, '');
  if (!ext) return 'DOSYA';
  return ext.length > 4 ? ext.slice(0, 4).toUpperCase() : ext.toUpperCase();
}

const GROUP_OF: Record<FileKind, FilterGroup> = {
  folder: 'folder',
  app: 'app',
  exe: 'app',
  text: 'doc',
  doc: 'doc',
  code: 'code',
  image: 'media',
  video: 'media',
  audio: 'media',
  archive: 'all',
  other: 'all',
};

export function groupOf(kind: FileKind): FilterGroup {
  return GROUP_OF[kind];
}

export const FILTER_GROUPS: Array<{ id: FilterGroup; label: string }> = [
  { id: 'all', label: 'Tümü' },
  { id: 'folder', label: 'Klasör' },
  { id: 'doc', label: 'Belge' },
  { id: 'code', label: 'Kod' },
  { id: 'media', label: 'Medya' },
  { id: 'app', label: 'Uygulama' },
];
