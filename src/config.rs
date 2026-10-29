//! Хранение настроек и токена.
//!
//! Основной путь — `%APPDATA%\fluxia\config.json`. Если запись туда невозможна
//! (антивирус/политики), пробуем `%LOCALAPPDATA%`, затем папку рядом с exe.

use std::path::PathBuf;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default = "default_volume")]
    pub volume: f32,
    /// Желаемый битрейт mp3: 64 / 128 / 192 / 320.
    #[serde(default = "default_bitrate")]
    pub bitrate: u32,
    /// Цвета интерфейса.
    #[serde(default)]
    pub theme: ThemeConfig,
    /// Автоматическая смена цветов.
    #[serde(default)]
    pub auto_color: AutoColor,
    /// Включать музыку сразу при запуске.
    #[serde(default)]
    pub autoplay: bool,
    /// Откуда играть при автопроигрывании.
    #[serde(default)]
    pub autoplay_source: AutoplaySource,
    /// Скрытые плейлисты (ключи вида "uid:kind").
    #[serde(default)]
    pub hidden_playlists: Vec<String>,
}

/// Ключ плейлиста для списка скрытых.
pub fn playlist_key(uid: u64, kind: u32) -> String {
    format!("{uid}:{kind}")
}

/// Откуда начинать воспроизведение при запуске.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoplaySource {
    Wave,
    Liked,
}

impl Default for AutoplaySource {
    fn default() -> Self {
        Self::Wave
    }
}

impl AutoplaySource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Wave => "Моя волна",
            Self::Liked => "Мне нравится",
        }
    }

    pub fn all() -> [Self; 2] {
        [Self::Wave, Self::Liked]
    }
}

/// Палитра в конфиге: цвета как [r, g, b].
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ThemeConfig {
    pub bg: [u8; 3],
    pub panel: [u8; 3],
    pub surface: [u8; 3],
    pub surface_hover: [u8; 3],
    pub border: [u8; 3],
    pub text: [u8; 3],
    pub text_dim: [u8; 3],
    pub text_faint: [u8; 3],
    pub accent: [u8; 3],
    pub accent_deep: [u8; 3],
    pub danger: [u8; 3],
    pub on_accent: [u8; 3],
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self::from_palette(crate::theme::Palette::dark())
    }
}

impl ThemeConfig {
    pub fn from_palette(p: crate::theme::Palette) -> Self {
        let rgb = |c: eframe::egui::Color32| [c.r(), c.g(), c.b()];
        Self {
            bg: rgb(p.bg),
            panel: rgb(p.panel),
            surface: rgb(p.surface),
            surface_hover: rgb(p.surface_hover),
            border: rgb(p.border),
            text: rgb(p.text),
            text_dim: rgb(p.text_dim),
            text_faint: rgb(p.text_faint),
            accent: rgb(p.accent),
            accent_deep: rgb(p.accent_deep),
            danger: rgb(p.danger),
            on_accent: rgb(p.on_accent),
        }
    }

    pub fn to_palette(self) -> crate::theme::Palette {
        let c = |v: [u8; 3]| eframe::egui::Color32::from_rgb(v[0], v[1], v[2]);
        crate::theme::Palette {
            bg: c(self.bg),
            panel: c(self.panel),
            surface: c(self.surface),
            surface_hover: c(self.surface_hover),
            border: c(self.border),
            text: c(self.text),
            text_dim: c(self.text_dim),
            text_faint: c(self.text_faint),
            accent: c(self.accent),
            accent_deep: c(self.accent_deep),
            danger: c(self.danger),
            on_accent: c(self.on_accent),
        }
    }
}

/// Что перекрашивает интерфейс автоматически.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoColor {
    /// Выключено: цвета только из настроек.
    Off,
    /// По обложке играющего трека.
    Cover,
    /// Днём — светлая, вечером и ночью — тёмная.
    TimeOfDay,
    /// Случайный акцент при запуске.
    Random,
}

impl Default for AutoColor {
    fn default() -> Self {
        Self::Off
    }
}

impl AutoColor {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Выключено",
            Self::Cover => "По обложке трека",
            Self::TimeOfDay => "По времени суток",
            Self::Random => "Случайно при запуске",
        }
    }

    pub fn all() -> [Self; 4] {
        [Self::Off, Self::Cover, Self::TimeOfDay, Self::Random]
    }
}

fn default_volume() -> f32 {
    0.7
}

fn default_bitrate() -> u32 {
    192
}

impl Default for Config {
    fn default() -> Self {
        Self {
            token: None,
            volume: default_volume(),
            bitrate: default_bitrate(),
            theme: ThemeConfig::default(),
            auto_color: AutoColor::Off,
            autoplay: false,
            autoplay_source: AutoplaySource::Wave,
            hidden_playlists: Vec::new(),
        }
    }
}

fn env_dir(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from)
}

/// Куда пробуем писать, по порядку.
pub fn candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(base) = env_dir("APPDATA") {
        paths.push(base.join("fluxia").join("config.json"));
    }
    if let Some(base) = env_dir("LOCALAPPDATA") {
        paths.push(base.join("fluxia").join("config.json"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join("fluxia-config.json"));
        }
    }
    paths
}

/// Самый свежий существующий файл из списка.
fn newest(paths: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    paths
        .into_iter()
        .filter_map(|p| {
            let t = std::fs::metadata(&p).ok()?.modified().ok()?;
            Some((t, p))
        })
        .max_by_key(|(t, _)| *t)
        .map(|(_, p)| p)
}

/// Куда писать, если ещё не писали: туда, где уже лежит самый свежий конфиг.
fn default_save_path() -> PathBuf {
    newest(candidate_paths())
        .or_else(|| candidate_paths().into_iter().next())
        .unwrap_or_else(|| PathBuf::from("fluxia-config.json"))
}

/// Путь, по которому реально лежит конфиг (самый свежий).
pub fn config_path() -> Option<PathBuf> {
    if let Some(p) = saved_path() {
        return Some(p);
    }
    newest(candidate_paths())
}

/// Последний файл, в который успешно писали (чтобы писать всегда в него же).
static SAVED_PATH: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

pub fn saved_path() -> Option<PathBuf> {
    SAVED_PATH.lock().ok().and_then(|g| g.clone())
}

fn remember_path(path: &std::path::Path) {
    if let Ok(mut g) = SAVED_PATH.lock() {
        *g = Some(path.to_path_buf());
    }
}

fn legacy_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(base) = env_dir("APPDATA") {
        paths.push(base.join("yamp").join("config.json"));
    }
    paths
}

/// Диагностика: `fluxia.exe --config-info` — куда пишется и что читается.
/// Пишет отчёт рядом с exe в `config-info.txt` (в release нет консоли).
pub fn dump_info() {
    let mut out = String::new();
    macro_rules! line {
        ($($arg:tt)*) => {{
            let s = format!($($arg)*);
            println!("{s}");
            out.push_str(&s);
            out.push('\n');
        }};
    }

    line!("== кандидаты ==");
    for p in candidate_paths() {
        match std::fs::metadata(&p) {
            Ok(m) => line!(
                "  {} (есть, {} байт, изменён {:?})",
                p.display(),
                m.len(),
                m.modified().ok()
            ),
            Err(e) => line!("  {} (нет: {e})", p.display()),
        }
    }
    for p in legacy_paths() {
        if p.exists() {
            line!("  старый конфиг: {}", p.display());
        }
    }

    let cfg = Config::load();
    line!("== прочитано ==");
    line!("  токен: {}", if cfg.token.is_some() { "есть" } else { "нет" });
    line!("  акцент: {:?}", cfg.theme.accent);
    line!("  автоцвет: {:?}", cfg.auto_color);
    line!("  автозапуск: {} ({:?})", cfg.autoplay, cfg.autoplay_source);
    line!("  скрытых плейлистов: {}", cfg.hidden_playlists.len());

    // проверка записи: меняем акцент и смотрим, что прочитается обратно
    let mut probe = cfg.clone();
    probe.theme.accent = [1, 2, 3];
    match probe.save() {
        Ok(p) => line!("== запись: ok -> {} ==", p.display()),
        Err(e) => line!("== запись: ОШИБКА: {e} =="),
    }
    for p in candidate_paths() {
        if p.exists() {
            line!(
                "  после записи: {} -> {:?}",
                p.display(),
                std::fs::metadata(&p).ok().and_then(|m| m.modified().ok())
            );
        }
    }
    let back = Config::load().theme.accent;
    line!("  перечитано: {back:?}");
    line!(
        "  итог: {}",
        if back == [1, 2, 3] {
            "запись и чтение совпадают"
        } else {
            "ВНИМАНИЕ: читается другой файл"
        }
    );

    // возвращаем прежний акцент
    let _ = cfg.save();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let _ = std::fs::write(dir.join("config-info.txt"), &out);
        }
    }
}

impl Config {
    /// Читает самый свежий конфиг и запоминает, откуда он — писать будем туда же.
    pub fn load() -> Self {
        let mut all = candidate_paths();
        all.extend(legacy_paths());

        // порядок: сначала самый свежий, потом остальные
        let mut ordered: Vec<PathBuf> = Vec::new();
        if let Some(fresh) = newest(all.clone()) {
            ordered.push(fresh);
        }
        ordered.extend(all);

        for path in ordered {
            if let Ok(raw) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<Self>(&raw) {
                    remember_path(&path);
                    return cfg;
                }
            }
        }
        Self::default()
    }

    /// Пишет конфиг в конкретный файл (атомарно: tmp + переименование).
    pub fn save_to(&self, path: &std::path::Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        if let Some(dir) = path.parent() {
            if !dir.as_os_str().is_empty() {
                std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            }
        }

        let tmp = path.with_extension("json.tmp");
        match std::fs::write(&tmp, &json) {
            Ok(()) => {
                if std::fs::rename(&tmp, path).is_err() {
                    // не дало переименовать — пишем напрямую
                    let _ = std::fs::remove_file(&tmp);
                    std::fs::write(path, &json)
                        .map_err(|e| format!("{}: {e}", path.display()))?;
                }
                Ok(())
            }
            // tmp рядом не создать — пробуем прямо
            Err(_) => std::fs::write(path, &json).map_err(|e| format!("{}: {e}", path.display())),
        }
    }

    /// Пишет конфиг в тот же файл, откуда читали. Возвращает путь — его же показываем в настройках.
    pub fn save(&self) -> Result<PathBuf, String> {
        let mut errors = Vec::new();

        let mut order: Vec<PathBuf> = Vec::new();
        if let Some(p) = saved_path() {
            order.push(p);
        }
        let fallback = default_save_path();
        order.push(fallback);
        for p in candidate_paths() {
            if !order.contains(&p) {
                order.push(p);
            }
        }

        for path in order {
            match self.save_to(&path) {
                Ok(()) => {
                    remember_path(&path);
                    return Ok(path);
                }
                Err(e) => errors.push(e),
            }
        }

        Err(errors.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_to_and_read_back() {
        let dir = std::env::temp_dir().join("fluxia-config-test");
        let path = dir.join("config.json");
        let _ = std::fs::remove_file(&path);

        let cfg = Config {
            token: Some("test-token-42".to_owned()),
            volume: 0.42,
            bitrate: 320,
            theme: ThemeConfig::from_palette(crate::theme::Palette::light()),
            auto_color: AutoColor::Cover,
            autoplay: true,
            autoplay_source: AutoplaySource::Liked,
            hidden_playlists: vec!["1:2".to_owned()],
        };
        cfg.save_to(&path).expect("save_to must write the file");
        assert!(path.exists(), "файл не создан");

        let raw = std::fs::read_to_string(&path).expect("read back");
        let loaded: Config = serde_json::from_str(&raw).expect("parse back");
        assert_eq!(loaded.token.as_deref(), Some("test-token-42"));
        assert!((loaded.volume - 0.42).abs() < 1e-6);
        assert_eq!(loaded.bitrate, 320);
        assert_eq!(loaded.auto_color, AutoColor::Cover);
        assert_eq!(loaded.theme, ThemeConfig::from_palette(crate::theme::Palette::light()));
        assert!(loaded.autoplay);
        assert_eq!(loaded.autoplay_source, AutoplaySource::Liked);
        assert_eq!(loaded.hidden_playlists, vec!["1:2".to_owned()]);
    }

    /// Главное свойство: что записали — то и прочитаем при следующем запуске.
    #[test]
    fn change_then_load_sees_the_same_values() {
        let before = Config::load();

        let mut cfg = before.clone();
        cfg.theme.accent = [7, 8, 9];
        cfg.autoplay = true;
        cfg.autoplay_source = AutoplaySource::Liked;
        cfg.hidden_playlists = vec!["5:7".to_owned()];
        let written = cfg.save().expect("конфиг должен записываться");

        let after = Config::load();
        assert_eq!(after.theme.accent, [7, 8, 9], "акцент не сохранился в {}", written.display());
        assert!(after.autoplay);
        assert_eq!(after.autoplay_source, AutoplaySource::Liked);
        assert_eq!(after.hidden_playlists, vec!["5:7".to_owned()]);

        // возвращаем как было, чтобы не портить рабочий конфиг
        let _ = before.save();
    }

    #[test]
    fn old_config_without_theme_still_loads() {
        let json = r#"{"token":"abc","volume":0.5,"bitrate":128}"#;
        let cfg: Config = serde_json::from_str(json).expect("старый конфиг должен читаться");
        assert_eq!(cfg.token.as_deref(), Some("abc"));
        assert_eq!(cfg.auto_color, AutoColor::Off);
        assert_eq!(cfg.theme, ThemeConfig::default());
    }
}
