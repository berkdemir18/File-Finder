use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// Global uygulama durumu
#[derive(Clone)]
pub struct AppState {
    /// Veritabanı bağlantısı
    pub db: Arc<Mutex<Option<crate::db::Database>>>,
    /// Servis bağlantı durumu
    pub service_connected: Arc<Mutex<bool>>,
    /// Indeks modu (MFT veya Fallback)
    pub index_mode: Arc<Mutex<crate::ipc::IndexMode>>,
    /// Ayarlar cache'i
    pub settings: Arc<Mutex<HashMap<String, String>>>,
    /// Pencere görünür mü?
    pub window_visible: Arc<Mutex<bool>>,
    /// Açılışta pencere bir kez gösterildi mi? Arayüzün `app_ready` sinyali
    /// ile emniyet zamanlayıcısı yarışır; kazanan bunu işaretler.
    pub startup_shown: Arc<AtomicBool>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            db: Arc::new(Mutex::new(None)),
            service_connected: Arc::new(Mutex::new(false)),
            index_mode: Arc::new(Mutex::new(crate::ipc::IndexMode::Fallback)),
            settings: Arc::new(Mutex::new(HashMap::new())),
            window_visible: Arc::new(Mutex::new(false)),
            startup_shown: Arc::new(AtomicBool::new(false)),
        }
    }
}
