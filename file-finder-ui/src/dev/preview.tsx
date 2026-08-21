/**
 * Tasarım önizlemesi girişi — YALNIZ geliştirme içindir, uygulama paketine
 * girmez (vite yalnız index.html'i derler).
 *
 * `./tauriMock` İLK sırada import edilir: ES modülleri import sırasına göre
 * çalıştığı için sahte IPC, App modülü `getCurrentWindow()` çağırmadan önce
 * kurulmuş olur.
 *
 * Kullanım:  npm run dev  →  http://localhost:1420/preview.html
 */
import './tauriMock';

import React from 'react';
import ReactDOM from 'react-dom/client';
import App from '../App';
import '../styles/global.css';

const Frame: React.FC = () => (
  <div
    style={{
      width: '100vw',
      height: '100vh',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      background: '#07080a',
      padding: 24,
    }}
  >
    <div style={{ width: 940, height: 640 }}>
      <App />
    </div>
  </div>
);

ReactDOM.createRoot(document.getElementById('root')!).render(<Frame />);
