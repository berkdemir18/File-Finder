import React, { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { ExternalLink, FolderOpen, ShieldAlert, Copy, Star, Trash2 } from 'lucide-react';
import type { SearchResult } from '../../types';
import {
  copyResultPath,
  openResult,
  revealResult,
  runResultAsAdmin,
  toggleResultFavorite,
} from '../../lib/actions';

interface ContextMenuProps {
  visible: boolean;
  x: number;
  y: number;
  item: SearchResult | null;
  query: string;
  isFavorite: boolean;
  onClose: () => void;
  onRequestDelete: (item: SearchResult) => void;
  onFavoriteChanged: () => void;
}

export const ContextMenu: React.FC<ContextMenuProps> = ({
  visible,
  x,
  y,
  item,
  query,
  isFavorite,
  onClose,
  onRequestDelete,
  onFavoriteChanged,
}) => {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ x, y });

  // Menü ölçüsünü tahmin etmek yerine ölçüp konumlandır — ekran dışına taşmaz.
  useLayoutEffect(() => {
    if (!visible || !ref.current) return;
    const rect = ref.current.getBoundingClientRect();
    setPos({
      x: Math.max(8, Math.min(x, window.innerWidth - rect.width - 8)),
      y: Math.max(8, Math.min(y, window.innerHeight - rect.height - 8)),
    });
  }, [visible, x, y]);

  useEffect(() => {
    if (!visible) return;
    const onDown = (e: MouseEvent) => {
      if (!(e.target as HTMLElement).closest('[data-context-menu]')) onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        onClose();
      }
    };
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey, true);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey, true);
    };
  }, [visible, onClose]);

  if (!visible || !item) return null;

  const run = (fn: () => void) => () => {
    onClose();
    fn();
  };

  return (
    <div
      ref={ref}
      data-context-menu
      className="menu"
      style={{ left: pos.x, top: pos.y }}
      role="menu"
    >
      <div className="menu-head" title={item.full_path}>
        {item.name}
      </div>

      <div className="menu-item" role="menuitem" onClick={run(() => void openResult(item, query))}>
        <ExternalLink size={14} />
        <span>Aç</span>
        <span className="menu-shortcut">Enter</span>
      </div>

      <div className="menu-item" role="menuitem" onClick={run(() => void revealResult(item))}>
        <FolderOpen size={14} />
        <span>Dosya konumunu aç</span>
        <span className="menu-shortcut">Ctrl+O</span>
      </div>

      <div
        className="menu-item"
        role="menuitem"
        onClick={run(() => void runResultAsAdmin(item))}
      >
        <ShieldAlert size={14} />
        <span>Yönetici olarak çalıştır</span>
        <span className="menu-shortcut">Ctrl+Shift+Enter</span>
      </div>

      <div className="menu-sep" />

      <div className="menu-item" role="menuitem" onClick={run(() => void copyResultPath(item))}>
        <Copy size={14} />
        <span>Yolu kopyala</span>
        <span className="menu-shortcut">Ctrl+C</span>
      </div>

      <div
        className="menu-item"
        role="menuitem"
        onClick={run(() => {
          void toggleResultFavorite(item).then(onFavoriteChanged);
        })}
      >
        <Star size={14} fill={isFavorite ? 'currentColor' : 'none'} />
        <span>{isFavorite ? 'Favorilerden çıkar' : 'Favorilere ekle'}</span>
      </div>

      <div className="menu-sep" />

      <div
        className="menu-item danger"
        role="menuitem"
        onClick={run(() => onRequestDelete(item))}
      >
        <Trash2 size={14} />
        <span>Geri dönüşüme gönder</span>
        <span className="menu-shortcut">Del</span>
      </div>
    </div>
  );
};
