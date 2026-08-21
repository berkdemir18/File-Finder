/**
 * Metin biçimlendirme yardımcıları.
 * Sayılar ve tarihler her yerde tr-TR ile yazılır; yollar mono tipografiye
 * gider, o yüzden kısaltma karakter sayısına göre yapılır.
 */

/** Arama sonucu adında eşleşen harfleri işaretle. HTML string döner. */
export function highlightMatches(name: string, positions: number[]): string {
  if (!positions.length) return escapeHtml(name);

  const hit = new Set(positions);
  let out = '';
  let i = 0;

  while (i < name.length) {
    if (hit.has(i)) {
      let j = i;
      while (j < name.length && hit.has(j)) j++;
      out += `<mark>${escapeHtml(name.slice(i, j))}</mark>`;
      i = j;
    } else {
      out += escapeHtml(name[i]);
      i++;
    }
  }
  return out;
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'];

/** 19740139520 → "18,4 GB" */
export function formatFileSize(bytes: number | null | undefined): string {
  if (bytes == null) return '—';
  if (bytes === 0) return '0 B';

  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), UNITS.length - 1);
  const value = bytes / Math.pow(1024, i);
  const decimals = i === 0 ? 0 : value < 10 ? 1 : value < 100 ? 1 : 0;

  return `${value.toLocaleString('tr-TR', {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  })} ${UNITS[i]}`;
}

/** Göreli tarih: "Bugün", "3 gün önce", "17.08.2026" */
export function formatDate(timestamp: number | null | undefined): string {
  if (!timestamp) return '—';

  const date = new Date(timestamp * 1000);
  if (Number.isNaN(date.getTime())) return '—';

  const days = Math.floor((Date.now() - date.getTime()) / 86400000);

  if (days < 0) return date.toLocaleDateString('tr-TR');
  if (days === 0) return 'Bugün';
  if (days === 1) return 'Dün';
  if (days < 7) return `${days} gün önce`;
  if (days < 30) return `${Math.floor(days / 7)} hafta önce`;
  if (days < 365) return `${Math.floor(days / 30)} ay önce`;
  return date.toLocaleDateString('tr-TR');
}

/** Tam tarih ve saat — önizleme panelinde. */
export function formatDateTime(timestamp: number | null | undefined): string {
  if (!timestamp) return '—';
  const date = new Date(timestamp * 1000);
  if (Number.isNaN(date.getTime())) return '—';
  return date.toLocaleString('tr-TR', {
    day: '2-digit',
    month: '2-digit',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

/**
 * Uzun yolu ortadan kısalt; sürücü harfi ve son iki klasör hep görünür kalır.
 * C:\Users\Berk\OneDrive\Belgeler\Sports Interactive
 *   → C:\…\Belgeler\Sports Interactive
 */
export function shortenPath(path: string, maxLen = 58): string {
  if (!path) return '';
  if (path.length <= maxLen) return path;

  const parts = path.split(/[\\/]/).filter(Boolean);
  if (parts.length <= 3) return path;

  const head = parts[0];
  for (let keep = 2; keep >= 1; keep--) {
    const candidate = [head, '…', ...parts.slice(-keep)].join('\\');
    if (candidate.length <= maxLen) return candidate;
  }

  const tail = parts[parts.length - 1];
  return `${head}\\…\\${tail.slice(-Math.max(8, maxLen - head.length - 4))}`;
}

/** 1284902 → "1.284.902" */
export function formatCount(n: number): string {
  return n.toLocaleString('tr-TR');
}

/** Sürücü harfi: "C:\Users\..." → "C:" */
export function driveOf(path: string): string {
  const m = /^([a-zA-Z]:)/.exec(path);
  return m ? m[1].toUpperCase() : '—';
}
