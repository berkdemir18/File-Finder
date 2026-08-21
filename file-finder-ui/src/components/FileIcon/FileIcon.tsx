import React, { memo } from 'react';
import { getFileKind, kindColorVar, kindLabel, type FileKind } from '../../lib/fileKinds';

interface FileIconProps {
  extension: string | null | undefined;
  isDirectory: boolean;
  isApp?: boolean;
  /** 1 = liste satırı, 1.6 = önizleme kapağı */
  scale?: number;
}

/**
 * Tasarım sistemindeki dosya türü karosu: tek yarıçap, tek grid,
 * rengi türden gelir, içinde uzantının kendisi yazar.
 */
export const FileIcon: React.FC<FileIconProps> = memo(
  ({ extension, isDirectory, isApp = false, scale = 1 }) => {
    const kind: FileKind = getFileKind(extension, isDirectory, isApp);
    const label = kindLabel(kind, extension);

    const style = {
      '--c': kindColorVar(kind),
      ...(scale !== 1 ? { transform: `scale(${scale})` } : null),
    } as React.CSSProperties;

    const cls = ['ficon', kind === 'folder' ? 'folder' : '', kind === 'app' ? 'app' : '']
      .filter(Boolean)
      .join(' ');

    return (
      <div className={cls} style={style} aria-hidden="true">
        {label}
      </div>
    );
  }
);

FileIcon.displayName = 'FileIcon';
