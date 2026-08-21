import React from 'react';

/**
 * File Finder işareti — kıvrılmış dosya köşesi + mercek.
 * Tek çizgi kalınlığı, tek yarıçap ailesi; 16px'e kadar okunur.
 * Geometri src-tauri/icons ile aynı: 88px karo üzerinden ölçeklenir.
 */
interface LogoProps {
  size?: number;
  /** Karo zemini; `none` verilirse yalnız işaret çizilir (tek renk kullanım). */
  tile?: string;
  mark?: string;
  className?: string;
}

export const Logo: React.FC<LogoProps> = ({
  size = 22,
  tile = '#f0c26a',
  mark = '#13151a',
  className,
}) => {
  const clipId = React.useId();

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 100 100"
      className={className}
      aria-hidden="true"
      focusable="false"
    >
      <defs>
        <clipPath id={clipId}>
          <rect width="100" height="100" rx="27.27" />
        </clipPath>
      </defs>
      <g clipPath={`url(#${clipId})`}>
        {tile !== 'none' && <rect width="100" height="100" fill={tile} />}
        {tile !== 'none' && <path d="M72.73 0 H100 V27.27 Z" fill={mark} />}
        <circle
          cx="35.75"
          cy="35.75"
          r="18.43"
          fill="none"
          stroke={mark}
          strokeWidth="6.03"
        />
        <line
          x1="50.92"
          y1="50.92"
          x2="68.94"
          y2="68.94"
          stroke={mark}
          strokeWidth="6.03"
          strokeLinecap="round"
        />
      </g>
    </svg>
  );
};

/** Arama alanındaki mercek — karo yok, tek çizgi. */
export const LensGlyph: React.FC<{ size?: number }> = ({ size = 18 }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    aria-hidden="true"
  >
    <circle cx="10.5" cy="10.5" r="6.5" />
    <line x1="15.5" y1="15.5" x2="20" y2="20" />
  </svg>
);
