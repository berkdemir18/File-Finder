import React from 'react';
import { formatCount } from '../../lib/format';

interface SpotlightProps {
  indexedFiles: number | null;
  hotkey: string | null;
  quickFilters: Array<{ label: string; query: string }>;
  onPickFilter: (query: string) => void;
}

/** Sorgu kutusu boşken görünen ekran: hızlı filtreler + indeks durumu. */
export const Spotlight: React.FC<SpotlightProps> = ({
  indexedFiles,
  hotkey,
  quickFilters,
  onPickFilter,
}) => (
  <div className="spotlight">
    <div className="quick-row">
      {quickFilters.map((f) => (
        <button key={f.label} className="pill" onClick={() => onPickFilter(f.query)}>
          {f.label}
        </button>
      ))}
    </div>

    <div className="spotlight-center">
      <div className="spotlight-title">Aramaya başlamak için yaz</div>
      <div className="spotlight-sub">
        {indexedFiles != null ? `${formatCount(indexedFiles)} dosya indekslendi` : 'İndeks hazırlanıyor'}
        {hotkey ? ` · her yerden ${hotkey}` : ''}
      </div>
    </div>
  </div>
);

const COMMANDS: Array<{ token: string; desc: string }> = [
  { token: 'ext:pdf', desc: 'Dosya türüne göre filtrele' },
  { token: 'size:>1gb', desc: 'Boyuta göre sınırla' },
  { token: 'modified:today', desc: 'Tarihe göre filtrele' },
  { token: 'path:C:\Projeler', desc: 'Tek klasörle sınırla' },
];

interface CommandHintsProps {
  onPick: (token: string) => void;
}

/**
 * Komut modu ipuçları. Yalnız sorgu ayrıştırıcının GERÇEKTEN desteklediği
 * anahtarlar listelenir — çalışmayan bir filtreyi önermek işe yaramaz.
 */
export const CommandHints: React.FC<CommandHintsProps> = ({ onPick }) => (
  <div className="spotlight">
    <span className="eyebrow">Komut modu</span>
    <div className="cmd-hints">
      {COMMANDS.map((c) => (
        <div key={c.token} className="cmd-hint" onClick={() => onPick(c.token)}>
          <code>{c.token}</code>
          <span>{c.desc}</span>
        </div>
      ))}
    </div>
    <div className="spotlight-sub">
      Filtreler yazarken birleşir. Regex için sorguyu <b>/desen/</b> biçiminde yaz.
    </div>
  </div>
);
