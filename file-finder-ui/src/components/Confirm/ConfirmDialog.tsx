import React, { useEffect, useRef } from 'react';

interface ConfirmDialogProps {
  open: boolean;
  title: string;
  body: React.ReactNode;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
}

/**
 * Uygulama içi onay kutusu. `window.confirm` yerine kullanılır: tarayıcı
 * diyaloğu Tauri penceresinde sistem penceresi gibi görünüp uygulamanın
 * görsel dilinden kopuyor ve odağı arama kutusundan koparıyordu.
 */
export const ConfirmDialog: React.FC<ConfirmDialogProps> = ({
  open,
  title,
  body,
  confirmLabel,
  onConfirm,
  onCancel,
}) => {
  const confirmRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) return;
    confirmRef.current?.focus();

    const onKey = (e: KeyboardEvent) => {
      // Arkadaki arama kısayolları bu sırada devreye girmesin.
      e.stopPropagation();
      if (e.key === 'Escape') onCancel();
      if (e.key === 'Enter') onConfirm();
    };
    document.addEventListener('keydown', onKey, true);
    return () => document.removeEventListener('keydown', onKey, true);
  }, [open, onCancel, onConfirm]);

  if (!open) return null;

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 300,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'rgba(0,0,0,0.55)',
      }}
      onMouseDown={onCancel}
    >
      <div
        role="alertdialog"
        aria-modal="true"
        onMouseDown={(e) => e.stopPropagation()}
        style={{
          width: 380,
          padding: 22,
          borderRadius: 'var(--r-lg)',
          background: 'var(--surface)',
          border: '1px solid var(--border-strong)',
          boxShadow: 'var(--shadow-pop)',
          display: 'flex',
          flexDirection: 'column',
          gap: 12,
        }}
      >
        <div style={{ fontSize: 16, fontWeight: 600, letterSpacing: '-0.3px' }}>{title}</div>
        <div style={{ fontSize: 13, color: 'var(--text-3)', lineHeight: 1.6 }}>{body}</div>
        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end', marginTop: 6 }}>
          <button className="btn" onClick={onCancel}>
            Vazgeç
          </button>
          <button ref={confirmRef} className="btn primary" onClick={onConfirm}>
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
};
