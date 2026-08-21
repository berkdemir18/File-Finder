import React, { useEffect, useState } from 'react';
import { Sliders, Database, Palette, Search, Info } from 'lucide-react';
import { useSettingsStore } from '../../stores/settingsStore';
import { ipcInstallService } from '../../lib/ipc';
import { formatCount } from '../../lib/format';
import type { AppSettings, IndexStats, ServiceStatus } from '../../types';
import { Logo } from '../Brand/Logo';

type Tab = 'general' | 'index' | 'appearance' | 'search' | 'about';

const TABS: Array<{ id: Tab; label: string; icon: React.ReactNode }> = [
  { id: 'general', label: 'Genel', icon: <Sliders size={15} /> },
  { id: 'index', label: 'İndeks', icon: <Database size={15} /> },
  { id: 'appearance', label: 'Görünüm', icon: <Palette size={15} /> },
  { id: 'search', label: 'Arama', icon: <Search size={15} /> },
  { id: 'about', label: 'Hakkında', icon: <Info size={15} /> },
];

const HOTKEY_OPTIONS = ['Ctrl+Space', 'Ctrl+Shift+Space', 'Alt+Space', 'Ctrl+Shift+F'];

const Row: React.FC<{ label: string; desc?: string; children: React.ReactNode }> = ({
  label,
  desc,
  children,
}) => (
  <div className="setting">
    <div>
      <div className="setting-label">{label}</div>
      {desc && <div className="setting-desc">{desc}</div>}
    </div>
    <div className="setting-control">{children}</div>
  </div>
);

const Toggle: React.FC<{ on: boolean; onChange: (v: boolean) => void }> = ({
  on,
  onChange,
}) => (
  <button className="switch" role="switch" aria-checked={on} onClick={() => onChange(!on)} />
);

export const SettingsWindow: React.FC = () => {
  const [tab, setTab] = useState<Tab>('general');
  const { settings, updateSetting, indexStats, loadIndexStats, serviceStatus, boundHotkey } =
    useSettingsStore();

  useEffect(() => {
    loadIndexStats();
  }, [loadIndexStats]);

  const set = (key: keyof AppSettings) => (value: string) => void updateSetting(key, value);
  const isOn = (key: keyof AppSettings) => settings[key] === 'true';

  return (
    <div className="settings">
      <nav className="settings-nav">
        {TABS.map((t) => (
          <button
            key={t.id}
            className={`settings-nav-item${tab === t.id ? ' active' : ''}`}
            onClick={() => setTab(t.id)}
          >
            {t.icon}
            <span>{t.label}</span>
          </button>
        ))}
      </nav>

      <div className="settings-body scroll">
        {tab === 'general' && (
          <>
            <div className="settings-group">
              <div className="eyebrow">Kısayol</div>
              <Row label="Global kısayol" desc="Uygulamayı her yerden çağır">
                <select
                  className="select"
                  value={settings.hotkey}
                  onChange={(e) => set('hotkey')(e.target.value)}
                >
                  {HOTKEY_OPTIONS.map((h) => (
                    <option key={h} value={h}>
                      {h}
                    </option>
                  ))}
                </select>
              </Row>

              {boundHotkey !== null && boundHotkey !== settings.hotkey && (
                <div className="banner">
                  <div className="banner-title">Kısayol farklı bağlandı</div>
                  <div>
                    <b className="mono">{settings.hotkey}</b> başka bir uygulamada olduğu için şu
                    an <b className="mono">{boundHotkey}</b> kullanılıyor. Seçtiğin kombinasyon
                    uygulama yeniden başladığında tekrar denenecek.
                  </div>
                </div>
              )}
              {boundHotkey === null && (
                <div className="banner">
                  <div className="banner-title">Kısayol bağlanamadı</div>
                  <div>
                    Denenen kombinasyonların hepsi başka uygulamalarda görünüyor. Uygulamayı
                    tepsi simgesinden açabilirsin.
                  </div>
                </div>
              )}
            </div>

            <div className="settings-group">
              <div className="eyebrow">Davranış</div>
              <Row
                label="Windows ile başlat"
                desc="Oturum açıldığında arka planda sessizce çalışır"
              >
                <Toggle
                  on={isOn('startup_with_windows')}
                  onChange={(v) => set('startup_with_windows')(String(v))}
                />
              </Row>
              <Row label="Odak kaybında gizle" desc="Başka pencereye geçince kendini toplar">
                <Toggle
                  on={isOn('hide_on_blur')}
                  onChange={(v) => set('hide_on_blur')(String(v))}
                />
              </Row>
              <Row label="Önizleme paneli" desc="Seçili sonucun ayrıntıları sağda görünür">
                <Toggle
                  on={isOn('show_preview')}
                  onChange={(v) => set('show_preview')(String(v))}
                />
              </Row>
            </div>
          </>
        )}

        {tab === 'index' && <IndexTab indexStats={indexStats} serviceStatus={serviceStatus} />}

        {tab === 'appearance' && (
          <div className="settings-group">
            <div className="eyebrow">Görünüm</div>
            <Row label="Renk modu">
              <select
                className="select"
                value={settings.theme}
                onChange={(e) => set('theme')(e.target.value)}
              >
                <option value="dark">Karanlık</option>
                <option value="light">Aydınlık</option>
                <option value="system">Sistemle uyumlu</option>
              </select>
            </Row>
            <Row label="Animasyon hızı" desc="Geçiş süreleri">
              <select
                className="select"
                value={settings.animation_speed}
                onChange={(e) => set('animation_speed')(e.target.value)}
              >
                <option value="fast">Hızlı</option>
                <option value="normal">Normal</option>
                <option value="slow">Yavaş</option>
                <option value="off">Kapalı</option>
              </select>
            </Row>
          </div>
        )}

        {tab === 'search' && (
          <div className="settings-group">
            <div className="eyebrow">Arama</div>
            <Row
              label="Eşleşme hassasiyeti"
              desc="Gevşek olan yazım hatasını affeder, sıkı olan yalnız yakın eşleşmeleri getirir"
            >
              <select
                className="select"
                value={settings.fuzzy_threshold}
                onChange={(e) => set('fuzzy_threshold')(e.target.value)}
              >
                <option value="0.1">Çok gevşek</option>
                <option value="0.3">Normal</option>
                <option value="0.5">Sıkı</option>
                <option value="0.7">Çok sıkı</option>
              </select>
            </Row>
            <Row label="Sayfa başına sonuç" desc="Liste sonuna gelince sonrakiler yüklenir">
              <select
                className="select"
                value={settings.max_results}
                onChange={(e) => set('max_results')(e.target.value)}
              >
                {['25', '50', '100', '200'].map((n) => (
                  <option key={n} value={n}>
                    {n}
                  </option>
                ))}
              </select>
            </Row>
          </div>
        )}

        {tab === 'about' && <AboutTab indexStats={indexStats} />}
      </div>
    </div>
  );
};

// ─── İndeks sekmesi ─────────────────────────────────────────────────────────

const IndexTab: React.FC<{
  indexStats: IndexStats | null;
  serviceStatus: ServiceStatus | null;
}> = ({ indexStats, serviceStatus }) => {
  const [installing, setInstalling] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const connected = Boolean(serviceStatus?.connected);

  const install = async () => {
    setInstalling(true);
    setMessage(null);
    try {
      await ipcInstallService();
      setMessage('Servis kurulumu başlatıldı. Yönetici onayı istenebilir.');
    } catch (e) {
      setMessage(`Servis kurulumu başarısız: ${e}`);
    }
    setInstalling(false);
  };

  return (
    <>
      <div className="settings-group">
        <div className="eyebrow">Durum</div>
        <div className={`banner${connected ? ' ok' : ''}`} style={{ marginTop: 8 }}>
          <div className="banner-title">
            <span className={`dot ${connected ? 'live' : 'warn'}`} />
            {connected ? 'MFT servisi bağlı — canlı izleme' : 'Yedek mod — dizin tarama'}
          </div>
          <div>
            {connected
              ? 'NTFS ana dosya tablosu okunuyor. İndeks anlık; değişiklikler saniyeler içinde yansır.'
              : 'Servis bulunamadı, klasörler tek tek taranıyor. Bu mod daha yavaştır ve tüm sürücüleri kapsamayabilir.'}
          </div>
        </div>
      </div>

      <div className="settings-group">
        <div className="eyebrow">İndeks</div>
        <div className="volumes" style={{ marginTop: 8 }}>
          <div className="volume">
            <div>
              <div className="volume-name">Toplam</div>
              <div className="volume-sub">İndekslenmiş kayıt sayısı</div>
            </div>
            <div className="volume-value">
              {indexStats ? `${formatCount(indexStats.total_files)} dosya` : '—'}
            </div>
          </div>
          <div className="volume">
            <div>
              <div className="volume-name">Mod</div>
              <div className="volume-sub">Çalışma biçimi</div>
            </div>
            <div className="volume-value">
              {indexStats?.mode === 'MftFull' ? 'NTFS · MFT' : 'Yedek tarama'}
            </div>
          </div>
        </div>
      </div>

      {!connected && (
        <div className="settings-group">
          <div className="eyebrow">Servis kurulumu</div>
          <p
            style={{
              fontSize: 12.5,
              color: 'var(--text-3)',
              lineHeight: 1.6,
              margin: '8px 0 12px',
            }}
          >
            Kurulumda tek seferlik yönetici (UAC) onayı istenir. Sonrasında servis Windows ile
            birlikte başlar ve bir daha onay gerekmez.
          </p>
          <button className="btn primary" onClick={install} disabled={installing}>
            {installing ? 'Kuruluyor…' : 'FileFinderService kur'}
          </button>
          {message && (
            <div style={{ fontSize: 12, color: 'var(--text-3)', marginTop: 10 }}>{message}</div>
          )}
        </div>
      )}
    </>
  );
};

// ─── Hakkında ───────────────────────────────────────────────────────────────

const AboutTab: React.FC<{ indexStats: IndexStats | null }> = ({ indexStats }) => (
  <>
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 16,
        paddingBottom: 20,
        borderBottom: '1px solid var(--border)',
      }}
    >
      <Logo size={56} />
      <div>
        <div style={{ fontSize: 22, fontWeight: 600, letterSpacing: '-0.6px' }}>File Finder</div>
        <div className="eyebrow" style={{ marginTop: 4 }}>
          Sürüm 0.1.0 · Windows
        </div>
      </div>
    </div>

    <div className="settings-group">
      <div className="eyebrow">Teknoloji</div>
      <Row label="Çerçeve">
        <span className="mono" style={{ fontSize: 12 }}>
          Tauri 2 · Rust
        </span>
      </Row>
      <Row label="Arayüz">
        <span className="mono" style={{ fontSize: 12 }}>
          React 19 · TypeScript
        </span>
      </Row>
      <Row label="Veritabanı">
        <span className="mono" style={{ fontSize: 12 }}>
          SQLite (WAL)
        </span>
      </Row>
      <Row label="İndeksleme">
        <span className="mono" style={{ fontSize: 12 }}>
          {indexStats?.mode === 'MftFull' ? 'NTFS MFT' : 'Yedek tarama'}
        </span>
      </Row>
    </div>

    <div className="settings-group">
      <div className="eyebrow">Tipografi</div>
      <div style={{ fontSize: 12.5, color: 'var(--text-3)', lineHeight: 1.6, paddingTop: 8 }}>
        Space Grotesk (arayüz) ve JetBrains Mono (yol, kısayol, sayı) uygulamayla birlikte
        paketlenir; çevrimdışıyken de aynı görünür.
      </div>
    </div>
  </>
);
