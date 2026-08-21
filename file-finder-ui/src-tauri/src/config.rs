use std::path::PathBuf;

/// Uygulama yapılandırma yolları
pub struct Config;

impl Config {
    /// %APPDATA%\FileFinder\
    pub fn app_data_dir() -> PathBuf {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "C:\\Users\\Public".to_string());
        PathBuf::from(appdata).join("FileFinder")
    }

    /// Veritabanı dosyası yolu
    pub fn db_path() -> PathBuf {
        Self::app_data_dir().join("index.db")
    }

    /// Log dosyası yolu
    pub fn log_path() -> PathBuf {
        Self::app_data_dir().join("ui.log")
    }

    /// Named Pipe adı
    pub const PIPE_NAME: &'static str = r"\\.\pipe\FileFinderService";

    /// Kalıcı ayar anahtarları — hem başlangıç cache'i hem `get_settings` okur.
    pub const SETTING_KEYS: &'static [&'static str] = &[
        "theme",
        "hotkey",
        "startup_with_windows",
        "hide_on_blur",
        "show_preview",
        "animation_speed",
        "fuzzy_threshold",
        "excluded_paths",
        "max_results",
    ];

    /// Varsayılan ayarlar
    pub const DEFAULT_HOTKEY: &'static str = "Ctrl+Space";
    pub const DEFAULT_THEME: &'static str = "dark";
    pub const DEFAULT_MAX_RESULTS: u32 = 50;
    pub const DEFAULT_FUZZY_THRESHOLD: f64 = 0.3;
}
