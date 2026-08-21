import React, { useEffect, useRef } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { SearchResult } from '../../types';
import { ResultItem } from './ResultItem';

interface ResultsListProps {
  /** Filtre uygulanmış görünür sıra. 0. eleman "en iyi eşleşme". */
  visible: SearchResult[];
  selectedIndex: number;
  hasMore: boolean;
  isLoading: boolean;
  onSelect: (index: number) => void;
  onOpen: (item: SearchResult) => void;
  onContextMenu: (e: React.MouseEvent, item: SearchResult) => void;
  onLoadMore: () => void;
}

const ROW_HEIGHT = 54;

/**
 * En iyi eşleşme listenin dışında, sabit olarak üstte durur; kalanlar
 * sanallaştırılır. Böylece sanal listenin sayacı ile gerçekte çizilen
 * satırlar her zaman birebir örtüşür.
 */
export const ResultsList: React.FC<ResultsListProps> = ({
  visible,
  selectedIndex,
  hasMore,
  isLoading,
  onSelect,
  onOpen,
  onContextMenu,
  onLoadMore,
}) => {
  const scrollRef = useRef<HTMLDivElement>(null);
  const best = visible[0];
  const rest = visible.slice(1);

  const virtualizer = useVirtualizer({
    count: rest.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 8,
  });

  // Klavyeyle seçilen satırı görünür tut.
  useEffect(() => {
    if (selectedIndex <= 0) {
      scrollRef.current?.scrollTo({ top: 0 });
      return;
    }
    virtualizer.scrollToIndex(selectedIndex - 1, { align: 'auto' });
  }, [selectedIndex, virtualizer]);

  // Listenin sonuna gelindiğinde sıradaki sayfayı çek.
  const items = virtualizer.getVirtualItems();
  const lastRendered = items.length ? items[items.length - 1].index : 0;
  useEffect(() => {
    if (hasMore && !isLoading && rest.length > 0 && lastRendered >= rest.length - 5) {
      onLoadMore();
    }
  }, [hasMore, isLoading, lastRendered, rest.length, onLoadMore]);

  if (!best) return null;

  return (
    <>
      <div style={{ padding: '4px 16px 0' }}>
        <div className="section-label">En iyi eşleşme</div>
        <ResultItem
          item={best}
          index={0}
          isBest
          isSelected={selectedIndex === 0}
          onSelect={() => onSelect(0)}
          onOpen={() => onOpen(best)}
          onContextMenu={(e) => onContextMenu(e, best)}
        />
      </div>

      {rest.length > 0 && (
        <>
          <div className="section-label" style={{ padding: '10px 26px 2px' }}>
            Diğer sonuçlar
          </div>
          <div ref={scrollRef} className="results-scroll scroll">
            <div style={{ height: virtualizer.getTotalSize(), position: 'relative' }}>
              {items.map((virtualRow) => {
                const item = rest[virtualRow.index];
                if (!item) return null;
                const index = virtualRow.index + 1;

                return (
                  <div
                    key={`${item.result_type}-${item.id}-${item.full_path}`}
                    style={{
                      position: 'absolute',
                      top: 0,
                      left: 0,
                      right: 0,
                      height: virtualRow.size,
                      transform: `translateY(${virtualRow.start}px)`,
                    }}
                  >
                    <ResultItem
                      item={item}
                      index={index}
                      isSelected={selectedIndex === index}
                      onSelect={() => onSelect(index)}
                      onOpen={() => onOpen(item)}
                      onContextMenu={(e) => onContextMenu(e, item)}
                    />
                  </div>
                );
              })}
            </div>
          </div>
        </>
      )}
    </>
  );
};
