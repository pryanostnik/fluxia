//! Упрощённые модели: из ответов API берём только то, что рисуем.

use std::time::Duration;

use yandex_music::model::{playlist::Playlist as YmPlaylist, track::Track};

#[derive(Debug, Clone)]
pub struct TrackItem {
    pub id: String,
    pub title: String,
    pub artists: String,
    pub duration: Duration,
    pub cover: Option<String>,
    pub available: bool,
}

impl TrackItem {
    /// Разбор «сырого» JSON трека, минуя строгие модели крейта.
    ///
    /// Зачем: в `yandex_music::model::track::Track` альбомы тянут за собой
    /// `release_date: DateTime<Utc>`, и если у трека дата пустая — падает разбор
    /// всего ответа («premature end of input»). Нам от альбома нужна только
    /// обложка, поэтому читаем поля руками.
    pub fn from_json(value: &serde_json::Value) -> Self {
        let mut names: Vec<String> = value["artists"]
            .as_array()
            .map(|list| {
                list.iter()
                    .filter_map(|a| a["name"].as_str())
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        if names.is_empty() {
            names.push("Неизвестный исполнитель".to_owned());
        }

        // важно: API отдаёт camelCase (coverUri, durationMs) — не snake_case
        let cover = value["coverUri"]
            .as_str()
            .filter(|s| !s.is_empty())
            .or_else(|| value["albums"][0]["coverUri"].as_str().filter(|s| !s.is_empty()))
            .map(|u| cover_url(u, "200x200"));

        Self {
            id: match &value["id"] {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                _ => String::new(),
            },
            title: value["title"]
                .as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| "Без названия".to_owned()),
            artists: names.join(", "),
            duration: Duration::from_millis(value["durationMs"].as_u64().unwrap_or(0)),
            cover,
            available: value["available"].as_bool().unwrap_or(true),
        }
    }
}

impl From<&Track> for TrackItem {
    fn from(t: &Track) -> Self {
        let mut names: Vec<String> = t
            .artists
            .iter()
            .filter_map(|a| a.name.clone())
            .filter(|n| !n.is_empty())
            .collect();
        if names.is_empty() {
            names.push("Неизвестный исполнитель".to_owned());
        }
        let artists = names.join(", ");

        let cover = t
            .cover_uri
            .clone()
            .or_else(|| t.albums.first().and_then(|a| a.cover_uri.clone()))
            .map(|u| cover_url(&u, "200x200"));

        Self {
            id: t.id.clone(),
            title: t.title.clone().unwrap_or_else(|| "Без названия".to_owned()),
            artists,
            duration: t.duration.unwrap_or_default(),
            cover,
            available: t.available.unwrap_or(true),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlaylistItem {
    pub kind: u32,
    pub uid: u64,
    pub title: String,
    pub count: u32,
    pub cover: Option<String>,
    pub owner: String,
}

impl From<&YmPlaylist> for PlaylistItem {
    fn from(p: &YmPlaylist) -> Self {
        let cover = p
            .cover
            .uri
            .clone()
            .or_else(|| Some(p.og_image.clone()).filter(|s| !s.is_empty()))
            .map(|u| cover_url(&u, "120x120"));

        Self {
            kind: p.kind,
            uid: p.uid,
            title: if p.title.is_empty() {
                "Без названия".to_owned()
            } else {
                p.title.clone()
            },
            count: p.track_count,
            cover,
            owner: p
                .owner
                .display_name
                .clone()
                .or_else(|| p.owner.name.clone())
                .unwrap_or_else(|| p.owner.login.clone()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub uid: u64,
    pub name: String,
}

/// `avatars.yandex.net/get-.../%%` -> полный https-URL нужного размера.
pub fn cover_url(uri: &str, size: &str) -> String {
    let url = uri.replace("%%", size);
    if url.starts_with("http://") || url.starts_with("https://") {
        url
    } else {
        format!("https://{url}")
    }
}

pub fn fmt_duration(d: Duration) -> String {
    let secs = d.as_secs();
    format!("{}:{:02}", secs / 60, secs % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_track_with_broken_album_dates() {
        // на таком ответе падал строгий крейт: release_date = "" -> chrono error
        let raw = serde_json::json!({
            "id": "12345",
            "title": "  Ночной город ",
            "durationMs": 187000,
            "available": false,
            "coverUri": "avatars.yandex.net/get-music-content/1/%%",
            "artists": [{"name": "Kavinsky"}, {"name": ""}, {"id": 3}],
            "albums": [{"id": 9, "releaseDate": "", "title": "Neon"}]
        });

        let t = TrackItem::from_json(&raw);
        assert_eq!(t.id, "12345");
        assert_eq!(t.title, "Ночной город");
        assert_eq!(t.artists, "Kavinsky");
        assert_eq!(t.duration, Duration::from_millis(187000));
        assert!(!t.available);
        assert_eq!(
            t.cover.as_deref(),
            Some("https://avatars.yandex.net/get-music-content/1/200x200")
        );
    }

    #[test]
    fn cover_falls_back_to_album() {
        let raw = serde_json::json!({
            "id": 7,
            "title": "Трек",
            "artists": [{"name": "Кто-то"}],
            "albums": [{"coverUri": "avatars.yandex.net/get-music-content/2/%%"}]
        });

        let t = TrackItem::from_json(&raw);
        assert_eq!(t.id, "7");
        assert_eq!(
            t.cover.as_deref(),
            Some("https://avatars.yandex.net/get-music-content/2/200x200")
        );
    }

    #[test]
    fn fallbacks_for_bare_track() {
        let raw = serde_json::json!({"id": 7, "artists": []});
        let t = TrackItem::from_json(&raw);
        assert_eq!(t.id, "7");
        assert_eq!(t.title, "Без названия");
        assert_eq!(t.artists, "Неизвестный исполнитель");
        assert!(t.available);
        assert!(t.cover.is_none());
    }
}
