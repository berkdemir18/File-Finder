import React, { memo } from 'react';
import type { SearchResult } from '../../types';
import { FileIcon } from '../FileIcon/FileIcon';
import { highlightMatches, formatFileSize, formatDate, shortenPath } from '../../lib/format';

interface ResultItemProps {
  item: SearchResult;
  index: number;
  isSelected: boolean;
  /** En iyi eşleşme satırı biraz daha iri çizilir. */
  isBest?: boolean;
  onSelect: () => void;
  onOpen: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
}

export const ResultItem: React.FC<ResultItemProps> = memo(
  ({ item, index, isSelected, isBest = false, onSelect, onOpen, onContextMenu }) => {
    const isApp = item.result_type === 'App';

    return (
      <div
        className={`row${isSelected ? ' selected' : ''}${isBest ? ' best' : ''}`}
        data-result-index={index}
        role="option"
        aria-selected={isSelected}
        onMouseDown={onSelect}
        onDoubleClick={onOpen}
        onContextMenu={onContextMenu}
        title={item.full_path}
      >
        <FileIcon
          extension={item.extension}
          isDirectory={item.is_directory}
          isApp={isApp}
        />

        <div className="row-body">
          <div
            className="row-name"
            dangerouslySetInnerHTML={{
              __html: highlightMatches(item.name, item.match_positions),
            }}
          />
          <div className="row-path">{shortenPath(item.parent_path)}</div>
        </div>

        <div className="row-meta">
          {!item.is_directory && item.size != null && (
            <span>{formatFileSize(item.size)}</span>
          )}
          {item.modified_time != null && <span>{formatDate(item.modified_time)}</span>}
        </div>

        {/* Alt+1..9 ile hızlı açma. Kısayol yazıyla gösterilir: ⌥ gibi
            modifier sembolleri paketlenen font altkümesinde yok. */}
        <span className="row-index">{index < 9 ? `Alt ${index + 1}` : ''}</span>
      </div>
    );
  }
);

ResultItem.displayName = 'ResultItem';
