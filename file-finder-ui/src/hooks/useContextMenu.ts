import { useEffect, useState, useCallback, useRef } from 'react';
import type { SearchResult } from '../types';

interface ContextMenuState {
  visible: boolean;
  x: number;
  y: number;
  item: SearchResult | null;
}

export function useContextMenu() {
  const [menuState, setMenuState] = useState<ContextMenuState>({
    visible: false,
    x: 0,
    y: 0,
    item: null,
  });

  const openMenu = useCallback((e: React.MouseEvent, item: SearchResult) => {
    e.preventDefault();
    e.stopPropagation();

    // Ekran sınırları içinde tut
    const menuWidth = 220;
    const menuHeight = 320;
    let x = e.clientX;
    let y = e.clientY;

    if (x + menuWidth > window.innerWidth) x = window.innerWidth - menuWidth - 8;
    if (y + menuHeight > window.innerHeight) y = window.innerHeight - menuHeight - 8;

    setMenuState({ visible: true, x, y, item });
  }, []);

  const closeMenu = useCallback(() => {
    setMenuState((prev) => ({ ...prev, visible: false, item: null }));
  }, []);

  const openMenuAt = useCallback((index: number, item: SearchResult) => {
    // Klavye kısayoluyla açıldığında, seçili öğenin konumunu kullan
    const el = document.querySelector(`[data-result-index="${index}"]`);
    if (el) {
      const rect = el.getBoundingClientRect();
      setMenuState({
        visible: true,
        x: rect.left + 20,
        y: rect.bottom,
        item,
      });
    }
  }, []);

  // Dışarı tıklayınca kapat
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (!target.closest('[data-context-menu]')) {
        closeMenu();
      }
    };

    if (menuState.visible) {
      document.addEventListener('mousedown', handler);
    }
    return () => document.removeEventListener('mousedown', handler);
  }, [menuState.visible, closeMenu]);

  // Esc ile kapat
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && menuState.visible) {
        closeMenu();
      }
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [menuState.visible, closeMenu]);

  return { menuState, openMenu, closeMenu, openMenuAt };
}
