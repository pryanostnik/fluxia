//! Тонкая обёртка над крейтом `yandex-music`.

pub mod models;

use yandex_music::api::playlist::get_playlist::GetPlaylistOptions;
use yandex_music::api::track::add_liked_tracks::AddLikedTracksOptions;
use yandex_music::api::track::get_download_info::GetDownloadInfoOptions;
use yandex_music::api::track::remove_liked_tracks::RemoveLikedTracksOptions;
use yandex_music::model::playlist::PlaylistTracks;
use yandex_music::YandexMusicClient;

pub use models::*;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

pub fn new_client(token: &str) -> Result<YandexMusicClient, String> {
    YandexMusicClient::builder(token.trim().to_owned())
        .build()
        .map_err(|e| format!("Не удалось создать клиент: {e}"))
}

pub async fn account(client: &YandexMusicClient) -> Result<AccountInfo, String> {
    let status = client.get_account_status().await.map_err(err)?;
    let acc = &status.account;
    let name = acc
        .display_name
        .clone()
        .or_else(|| acc.first_name.clone())
        .or_else(|| acc.login.clone())
        .unwrap_or_else(|| "Аккаунт".to_owned());
    Ok(AccountInfo {
        uid: acc.uid.unwrap_or(0),
        name,
    })
}

/// GET и разбор ответа как JSON-значения (без строгих моделей крейта).
async fn get_json(
    client: &YandexMusicClient,
    url: String,
) -> Result<serde_json::Value, String> {
    let resp = client.inner.get(url).send().await.map_err(err)?;
    let status = resp.status();
    let text = resp.text().await.map_err(err)?;

    if !status.is_success() {
        return Err(format!("HTTP {status}: {}", preview(&text)));
    }
    serde_json::from_str(&text)
        .map_err(|e| format!("ответ не разобрался ({e}); начало: {}", preview(&text)))
}

fn playlist_item_from_json(value: &serde_json::Value) -> PlaylistItem {
    let cover = value["cover"]["uri"]
        .as_str()
        .filter(|s| !s.is_empty())
        .or_else(|| value["ogImage"].as_str().filter(|s| !s.is_empty()))
        .map(|u| cover_url(u, "120x120"));

    let owner = value["owner"]["displayName"]
        .as_str()
        .or_else(|| value["owner"]["name"].as_str())
        .or_else(|| value["owner"]["login"].as_str())
        .unwrap_or_default()
        .to_owned();

    PlaylistItem {
        kind: value["kind"].as_u64().unwrap_or(0) as u32,
        uid: value["uid"].as_u64().unwrap_or(0),
        title: value["title"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or("Без названия")
            .to_owned(),
        count: value["trackCount"].as_u64().unwrap_or(0) as u32,
        cover,
        owner,
    }
}

pub async fn playlists(
    client: &YandexMusicClient,
    uid: u64,
) -> Result<Vec<PlaylistItem>, String> {
    let url = format!("https://api.music.yandex.net/users/{uid}/playlists/list");
    let value = get_json(client, url).await?;

    let Some(list) = value["result"].as_array() else {
        return Err("в ответе нет списка плейлистов".to_owned());
    };
    Ok(list.iter().map(playlist_item_from_json).collect())
}

/// Полные треки по id через `POST /tracks` (как в yamusic-tui): id уходят в тело,
/// а не в путь запроса, — не упираемся в длину URL и не зависим от строгих моделей.
async fn fetch_tracks(
    client: &YandexMusicClient,
    ids: Vec<String>,
) -> Result<Vec<TrackItem>, String> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut found: std::collections::HashMap<String, TrackItem> =
        std::collections::HashMap::with_capacity(ids.len());

    for chunk in ids.chunks(200) {
        let mut form = String::from("with-positions=false");
        for id in chunk {
            form.push_str("&track-ids=");
            form.push_str(id);
        }

        let resp = client
            .inner
            .post("https://api.music.yandex.net/tracks")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(form)
            .send()
            .await
            .map_err(err)?;

        let status = resp.status();
        let text = resp.text().await.map_err(err)?;
        if !status.is_success() {
            return Err(format!("HTTP {status}: {}", preview(&text)));
        }

        let value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| format!("ответ не разобрался ({e}); начало: {}", preview(&text)))?;
        let Some(list) = value["result"].as_array() else {
            return Err(format!("в ответе нет result; начало: {}", preview(&text)));
        };

        for raw in list {
            let item = TrackItem::from_json(raw);
            if !item.id.is_empty() {
                found.insert(item.id.clone(), item);
            }
        }
    }

    // порядок — как просили, а не как ответил сервер
    Ok(ids.iter().filter_map(|id| found.remove(id)).collect())
}

pub async fn playlist_tracks(
    client: &YandexMusicClient,
    uid: u64,
    kind: u32,
) -> Result<Vec<TrackItem>, String> {
    match playlist_tracks_crate(client, uid, kind).await {
        Ok(list) => Ok(list),
        Err(first) => match playlist_tracks_raw(client, uid, kind).await {
            Ok(list) => Ok(list),
            Err(second) => Err(format!("{first} | сырой разбор: {second}")),
        },
    }
}

async fn playlist_tracks_crate(
    client: &YandexMusicClient,
    uid: u64,
    kind: u32,
) -> Result<Vec<TrackItem>, String> {
    let playlist = client
        .get_playlist(&GetPlaylistOptions::new(uid, kind))
        .await
        .map_err(err)?;

    let tracks = match playlist.tracks {
        Some(PlaylistTracks::Full(list)) => list.iter().map(TrackItem::from).collect(),
        Some(PlaylistTracks::WithInfo(list)) => {
            list.iter().map(|t| TrackItem::from(&t.track)).collect()
        }
        Some(PlaylistTracks::Partial(list)) => {
            let ids: Vec<String> = list.iter().map(|p| p.id.clone()).collect();
            if ids.is_empty() {
                return Ok(Vec::new());
            }
            fetch_tracks(client, ids).await?
        }
        None => Vec::new(),
    };

    Ok(tracks)
}

/// Запасной путь: берём плейлист сырым JSON и достаём только id треков.
async fn playlist_tracks_raw(
    client: &YandexMusicClient,
    uid: u64,
    kind: u32,
) -> Result<Vec<TrackItem>, String> {
    let url = format!("https://api.music.yandex.net/users/{uid}/playlists/{kind}");
    let resp = client.inner.get(url).send().await.map_err(err)?;
    let status = resp.status();
    let text = resp.text().await.map_err(err)?;

    if !status.is_success() {
        return Err(format!("HTTP {status}: {}", preview(&text)));
    }

    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("ответ не разобрался ({e}); начало: {}", preview(&text)))?;
    let Some(list) = value["result"]["tracks"].as_array() else {
        return Err(format!("в ответе нет result.tracks; начало: {}", preview(&text)));
    };

    let ids: Vec<String> = list
        .iter()
        .filter_map(|item| id_of(&item["id"]).or_else(|| id_of(&item["track"]["id"])))
        .collect();

    fetch_tracks(client, ids).await
}

fn preview(text: &str) -> String {
    text.chars().take(180).collect()
}

fn id_of(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) if !s.is_empty() => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Достаём id лайкнутых треков, не трогая `timestamp` и прочие поля:
/// у крейта модель `PartialTrack` строгая, и одна битая дата ломает весь ответ.
fn library_ids(value: &serde_json::Value) -> Option<Vec<String>> {
    let library = &value["result"]["library"];

    if let Some(ids) = library["trackIds"].as_array() {
        return Some(ids.iter().filter_map(id_of).collect());
    }
    if let Some(list) = library["tracks"].as_array() {
        return Some(list.iter().filter_map(|t| id_of(&t["id"])).collect());
    }
    None
}

/// «Мне нравится»: забираем коллекцию своим запросом (терпимым к формату),
/// затем догружаем полные данные треков пачками.
pub async fn liked_tracks(
    client: &YandexMusicClient,
    uid: u64,
) -> Result<Vec<TrackItem>, String> {
    match liked_tracks_direct(client, uid).await {
        Ok(list) => Ok(list),
        // запасной путь: та же коллекция, но как обычный плейлист kind = 3
        Err(first) => match playlist_tracks(client, uid, 3).await {
            Ok(list) => Ok(list),
            Err(second) => Err(format!("{first} | запасной путь: {second}")),
        },
    }
}

async fn liked_tracks_direct(
    client: &YandexMusicClient,
    uid: u64,
) -> Result<Vec<TrackItem>, String> {
    let url = format!("https://api.music.yandex.net/users/{uid}/likes/tracks");
    let resp = client.inner.get(url).send().await.map_err(err)?;
    let status = resp.status();
    let text = resp.text().await.map_err(err)?;

    if !status.is_success() {
        return Err(format!("HTTP {status}: {}", preview(&text)));
    }

    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("ответ не разобрался ({e}); начало: {}", preview(&text)))?;

    let ids = library_ids(&value)
        .ok_or_else(|| format!("в ответе нет library; начало: {}", preview(&text)))?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    fetch_tracks(client, ids).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_from_track_ids_mixed_types() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"result":{"library":{"trackIds":["111",222,null],"tracks":[]}}}"#,
        )
        .unwrap();
        assert_eq!(
            library_ids(&v).unwrap(),
            vec!["111".to_owned(), "222".to_owned()]
        );
    }

    #[test]
    fn ids_from_tracks_with_broken_timestamp() {
        // именно на таком ответе падал крейт: битый timestamp не должен мешать
        let v: serde_json::Value = serde_json::from_str(
            r#"{"result":{"library":{"tracks":[{"id":"42","timestamp":""},{"id":77,"timestamp":"0001-01-01T00:00:00Z"}]}}}"#,
        )
        .unwrap();
        assert_eq!(
            library_ids(&v).unwrap(),
            vec!["42".to_owned(), "77".to_owned()]
        );
    }

    #[test]
    fn empty_library_is_ok() {
        let v: serde_json::Value =
            serde_json::from_str(r#"{"result":{"library":{"trackIds":[]}}}"#).unwrap();
        assert!(library_ids(&v).unwrap().is_empty());
    }

    #[test]
    fn unknown_shape_is_detected() {
        let v: serde_json::Value = serde_json::from_str(r#"{"result":{"other":1}}"#).unwrap();
        assert!(library_ids(&v).is_none());
    }
}

pub async fn like_track(
    client: &YandexMusicClient,
    uid: u64,
    track_id: &str,
    like: bool,
) -> Result<(), String> {
    let ids = vec![track_id.to_owned()];
    if like {
        client
            .add_liked_tracks(&AddLikedTracksOptions::new(uid, ids))
            .await
            .map_err(err)?;
    } else {
        client
            .remove_liked_tracks(&RemoveLikedTracksOptions::new(uid, ids))
            .await
            .map_err(err)?;
    }
    Ok(())
}

/// Процентное кодирование запроса для URL.
fn enc(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    for byte in text.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub async fn search_tracks(
    client: &YandexMusicClient,
    text: &str,
) -> Result<Vec<TrackItem>, String> {
    let url = format!(
        "https://api.music.yandex.net/search?text={}&type=track&page=0",
        enc(text)
    );
    let value = get_json(client, url).await?;

    let Some(list) = value["result"]["tracks"]["results"].as_array() else {
        return Ok(Vec::new());
    };

    Ok(list
        .iter()
        .map(|raw| {
            let mut item = TrackItem::from_json(raw);
            // у подменённых треков настоящий id лежит в realId
            if let Some(real) = id_of(&raw["realId"]) {
                item.id = real;
            }
            item
        })
        .filter(|t| !t.id.is_empty())
        .collect())
}

/// «Моя волна» — rotor-станция `user:onyourwave`.
pub async fn wave_tracks(client: &YandexMusicClient) -> Result<Vec<TrackItem>, String> {
    let url =
        "https://api.music.yandex.net/rotor/station/user:onyourwave/tracks?settings2=true"
            .to_owned();
    let value = get_json(client, url).await?;

    let Some(list) = value["result"]["sequence"].as_array() else {
        return Ok(Vec::new());
    };

    Ok(list
        .iter()
        .map(|s| TrackItem::from_json(&s["track"]))
        .filter(|t| !t.id.is_empty())
        .collect())
}

/// Скачивает mp3 трека целиком (несколько мегабайт) и отдаёт байты —
/// так декодер умеет искать по позиции без сетевого стриминга.
pub async fn stream(
    client: &YandexMusicClient,
    track_id: &str,
    want_kbps: u32,
) -> Result<Vec<u8>, String> {
    let infos = client
        .get_download_info(&GetDownloadInfoOptions::new(track_id))
        .await
        .map_err(err)?;

    if infos.is_empty() {
        return Err("Для этого трека нет доступных форматов".to_owned());
    }

    let want = want_kbps as i64;
    let score = |codec: &str, kbps: u32| -> i64 {
        let codec_penalty = if codec == "mp3" { 0 } else { 500 };
        codec_penalty + (kbps as i64 - want).abs()
    };

    let best = infos
        .iter()
        .filter(|i| !i.preview)
        .min_by_key(|i| score(&i.codec, i.bitrate_in_kbps))
        .or_else(|| infos.iter().min_by_key(|i| score(&i.codec, i.bitrate_in_kbps)))
        .ok_or_else(|| "Не нашёл подходящий формат".to_owned())?;

    let direct = best.get_direct_link(&client.inner).await.map_err(err)?;
    let resp = client
        .inner
        .get(direct)
        .send()
        .await
        .map_err(err)?
        .error_for_status()
        .map_err(err)?;
    let bytes = resp.bytes().await.map_err(err)?;

    if bytes.is_empty() {
        return Err("Пустой ответ от сервера аудио".to_owned());
    }
    Ok(bytes.to_vec())
}

pub async fn cover_bytes(client: &YandexMusicClient, url: &str) -> Result<Vec<u8>, String> {
    let resp = client
        .inner
        .get(url)
        .send()
        .await
        .map_err(err)?
        .error_for_status()
        .map_err(err)?;
    Ok(resp.bytes().await.map_err(err)?.to_vec())
}
