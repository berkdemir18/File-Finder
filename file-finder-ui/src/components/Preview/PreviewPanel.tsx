import React from 'react';
import { FolderOpen, ExternalLink, Star } from 'lucide-react';
import type { SearchResult } from '../../types';
import { FileIcon } from '../FileIcon/FileIcon';
import { formatFileSize, formatDateTime, driveOf } from '../../lib/format';
import { getFileKind } from '../../lib/fileKinds';

interface PreviewPanelProps {
  item: SearchResult | null;
  isFavorite: boolean;
  onOpen: () => void;
  onReveal: () => void;
  onToggleFavorite: () => void;
}

const KIND_LABEL: Record<string, string> = {
  folder: 'Klasör',
  app: 'Uygulama',
  text: 'Metin',
  doc: 'Belge',
  code: 'Kod',
  image: 'Görsel',
  video: 'Video',
  audio: 'Ses',
  archive: 'Arşiv',
  exe: 'Çalıştırılabilir',
  other: 'Dosya',
};

export const PreviewPanel: React.FC<PreviewPanelProps> = ({
  item,
  isFavorite,
  onOpen,
  onReveal,
  onToggleFavorite,
}) => {
  if (!item) {
    return (
      <aside className="preview">
        <div className="spotlight-center">
          <span className="eyebrow">Önizleme</span>
          <div className="spotlight-sub">Bir sonuç seç</div>
        </div>
      </aside>
    );
  }

  const isApp = item.result_type === 'App';
  const kind = getFileKind(item.extension, item.is_directory, isApp);

  return (
    <aside className="preview">
      <div className="preview-cover">
        <FileIcon
          extension={item.extension}
          isDirectory={item.is_directory}
          isApp={isApp}
          scale={1.5}
        />
      </div>

      <div>
        <div className="preview-name">{item.name}</div>
        <div className="preview-path" style={{ marginTop: 6 }}>
          {item.full_path}
        </div>
      </div>

      <dl className="preview-facts">
        <div className="preview-fact">
          <dt>Tür</dt>
          <dd>{KIND_LABEL[kind] ?? 'Dosya'}</dd>
        </div>
        {!item.is_directory && item.size != null && (
          <div className="preview-fact">
            <dt>Boyut</dt>
            <dd>{formatFileSize(item.size)}</dd>
          </div>
        )}
        {item.modified_time != null && (
          <div className="preview-fact">
            <dt>Değiştirilme</dt>
            <dd>{formatDateTime(item.modified_time)}</dd>
          </div>
        )}
        <div className="preview-fact">
          <dt>Sürücü</dt>
          <dd>{driveOf(item.full_path)}</dd>
        </div>
      </dl>

      <div className="preview-actions">
        <button className="btn primary" onClick={onOpen}>
          <ExternalLink size={14} /> Aç
        </button>
        <button className="btn icon" onClick={onReveal} title="Dosya konumunu aç (Ctrl+O)">
          <FolderOpen size={14} />
        </button>
        <button
          className="btn icon"
          onClick={onToggleFavorite}
          title={isFavorite ? 'Favorilerden çıkar' : 'Favorilere ekle'}
          style={isFavorite ? { color: 'var(--accent)', borderColor: 'var(--accent)' } : undefined}
        >
          <Star size={14} fill={isFavorite ? 'currentColor' : 'none'} />
        </button>
      </div>
    </aside>
  );
};
