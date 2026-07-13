use crate::core::config::{APP_HOMEPAGE, APP_VERSION};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock};

/// Check whether a search query is related to a song name.
fn query_matches_song(query: &str, song_name: &str) -> bool {
    let q = query.to_lowercase();
    let n = song_name.to_lowercase();
    if q.contains(&n) || n.contains(&q) {
        return true;
    }
    let words: Vec<&str> = q
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2)
        .collect();
    if words.is_empty() {
        return false;
    }
    words.iter().any(|w| n.contains(w))
}

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap()
});

#[derive(Clone, Default, Debug)]
pub struct LyricLine {
    pub time_ms: u64,
    pub text: String,
}

/// Try to read a matching `.lrc` file from `local_dir`.
///
/// Lookup order:
///   1. `{artist} - {title}.lrc`
///   2. `{title}.lrc`
///
/// Both are matched case-insensitively on the filesystem (Windows).
fn fetch_lyrics_local(title: &str, artist: &str, local_dir: &str) -> Option<Arc<Vec<LyricLine>>> {
    let san = |s: &str| -> String {
        s.chars()
            .filter(|c| !matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
            .collect()
    };
    let title_san = san(title);
    let artist_san = san(artist);

    let candidates = if artist_san.is_empty() {
        vec![format!("{}.lrc", title_san)]
    } else {
        vec![
            format!("{} - {}.lrc", artist_san, title_san),
            format!("{}.lrc", title_san),
        ]
    };

    for file_name in &candidates {
        let path = std::path::Path::new(local_dir).join(file_name);
        if path.exists()
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            let lines = parse_lyrics(&content, "");
            if !lines.is_empty() {
                return Some(Arc::new(lines));
            }
        }
    }
    None
}

pub async fn fetch_lyrics(
    title: &str,
    artist: &str,
    duration_secs: u64,
    source: &str,
    fallback: bool,
    local_dir: Option<&str>,
) -> Option<Arc<Vec<LyricLine>>> {
    if title.is_empty() {
        return None;
    }

    // 1. Local .lrc file takes priority
    if let Some(dir) = local_dir
        && !dir.is_empty()
        && let Some(lyrics) = fetch_lyrics_local(title, artist, dir)
    {
        return Some(lyrics);
    }

    // 2. Without an artist the online search is unreliable — it may match a
    //    totally unrelated song (e.g. a browser video title hitting a random
    //    NetEase hit). Only try the selected source once, skip fallback.
    if artist.trim().is_empty() {
        return match source {
            "lrclib" => fetch_lyrics_lrclib(title, "", duration_secs).await,
            _ => fetch_lyrics_163(title, "").await,
        };
    }

    // 3. Online sources
    let result = match source {
        "lrclib" => fetch_lyrics_lrclib(title, artist, duration_secs).await,
        _ => fetch_lyrics_163(title, artist).await,
    };
    if result.is_none() && fallback {
        match source {
            "lrclib" => fetch_lyrics_163(title, artist).await,
            _ => fetch_lyrics_lrclib(title, artist, duration_secs).await,
        }
    } else {
        result
    }
}

async fn fetch_lyrics_163(title: &str, artist: &str) -> Option<Arc<Vec<LyricLine>>> {
    if let Some(r) = fetch_lyrics_163_inner(title, artist).await {
        return Some(r);
    }
    // Only retry without artist when one was originally given, otherwise the
    // second call is identical to the first and we don't gain anything.
    if !artist.is_empty() {
        fetch_lyrics_163_inner(title, "").await
    } else {
        None
    }
}

async fn fetch_lyrics_163_inner(title: &str, artist: &str) -> Option<Arc<Vec<LyricLine>>> {
    let query = if artist.is_empty() {
        title.to_string()
    } else {
        format!("{} {}", title, artist)
    };
    let url = format!(
        "https://music.163.com/api/search/get/web?s={}&type=1&offset=0&total=true&limit=10",
        url_encode(&query)
    );

    let res = HTTP_CLIENT.get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .send()
        .await
        .ok()?;

    let json: Value = res.json().await.ok()?;

    let songs = json.get("result")?.get("songs")?.as_array()?;
    if songs.is_empty() {
        return None;
    }

    let artist_lower = artist.to_lowercase();
    let mut song_id: Option<i64> = None;

    if !artist_lower.is_empty() {
        for s in songs {
            if let Some(artists) = s.get("artists").and_then(|a| a.as_array()) {
                for a in artists {
                    if let Some(name) = a.get("name").and_then(|n| n.as_str())
                        && name.to_lowercase() == artist_lower
                    {
                        song_id = s.get("id").and_then(|id| id.as_i64());
                        break;
                    }
                }
            }
            if song_id.is_some() {
                break;
            }
        }
    }

    if song_id.is_none() {
        let first = songs.first()?;
        // Before blindly accepting the first result, verify it has at least
        // some relation to the original search query. Browser video titles
        // (e.g. "How to build a PC") would otherwise match a random unrelated
        // song on the platform.
        if let Some(name) = first.get("name").and_then(|n| n.as_str())
            && !query_matches_song(&query, name)
        {
            return None;
        }
        song_id = first.get("id")?.as_i64();
    }

    let id = song_id?;

    let lyric_url = format!(
        "https://music.163.com/api/song/lyric?id={}&lv=1&kv=1&tv=-1",
        id
    );

    let lyric_res = HTTP_CLIENT
        .get(&lyric_url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await
        .ok()?;

    let lyric_json: Value = lyric_res.json().await.ok()?;

    let lrc_str = lyric_json.get("lrc")?.get("lyric")?.as_str().unwrap_or("");
    let tlrc_str = lyric_json
        .get("tlyric")?
        .get("lyric")?
        .as_str()
        .unwrap_or("");

    Some(Arc::new(parse_lyrics(lrc_str, tlrc_str)))
}

async fn fetch_lyrics_lrclib(
    title: &str,
    artist: &str,
    duration_secs: u64,
) -> Option<Arc<Vec<LyricLine>>> {
    if let Some(r) = fetch_lyrics_lrclib_inner(title, artist, duration_secs).await {
        return Some(r);
    }
    fetch_lyrics_lrclib_search(title, artist).await
}

async fn fetch_lyrics_lrclib_inner(
    title: &str,
    artist: &str,
    duration_secs: u64,
) -> Option<Arc<Vec<LyricLine>>> {
    let url = format!(
        "https://lrclib.net/api/get?track_name={}&artist_name={}&duration={}",
        url_encode(title),
        url_encode(artist),
        duration_secs
    );

    let res = HTTP_CLIENT
        .get(&url)
        .header(
            "User-Agent",
            &format!("WinIsland/{} ({})", APP_VERSION, APP_HOMEPAGE),
        )
        .send()
        .await
        .ok()?;

    let json: Value = res.json().await.ok()?;
    let synced = json.get("syncedLyrics")?.as_str()?;

    let lines = parse_lyrics(synced, "");
    if lines.is_empty() {
        None
    } else {
        Some(Arc::new(lines))
    }
}

async fn fetch_lyrics_lrclib_search(title: &str, artist: &str) -> Option<Arc<Vec<LyricLine>>> {
    let query = if artist.is_empty() {
        title.to_string()
    } else {
        format!("{} {}", title, artist)
    };
    let url = format!("https://lrclib.net/api/search?q={}", url_encode(&query));

    let res = HTTP_CLIENT
        .get(&url)
        .header(
            "User-Agent",
            &format!("WinIsland/{} ({})", APP_VERSION, APP_HOMEPAGE),
        )
        .send()
        .await
        .ok()?;

    let json: Value = res.json().await.ok()?;
    let arr = json.as_array()?;

    for item in arr {
        if let Some(synced) = item.get("syncedLyrics").and_then(|s| s.as_str()) {
            // Skip if the result seems unrelated to the original query
            if let Some(name) = item.get("trackName").and_then(|n| n.as_str())
                && !query_matches_song(&query, name)
            {
                continue;
            }
            let lines = parse_lyrics(synced, "");
            if !lines.is_empty() {
                return Some(Arc::new(lines));
            }
        }
    }
    None
}

fn parse_lyrics(lrc: &str, tlrc: &str) -> Vec<LyricLine> {
    let mut map: BTreeMap<u64, String> = BTreeMap::new();

    let mut process_content = |content: &str| {
        for line in content.lines() {
            let line = line.trim();
            if !line.starts_with('[') {
                continue;
            }

            let parts: Vec<&str> = line.split(']').collect();
            if parts.len() < 2 {
                continue;
            }

            let text = parts[parts.len() - 1].trim().to_string();
            if text.is_empty() && content == lrc {
                // Keep empty lines from main lrc to allow clearing screen
            } else if text.is_empty() {
                continue;
            }

            for time_part in &parts[..parts.len() - 1] {
                let time_str = time_part.trim_start_matches('[');
                if let Some(ms) = parse_time(time_str) {
                    map.entry(ms)
                        .and_modify(|e| {
                            if e.is_empty() && !text.is_empty() {
                                *e = text.clone();
                            }
                        })
                        .or_insert(text.clone());
                }
            }
        }
    };

    process_content(lrc);
    process_content(tlrc);

    map.into_iter()
        .map(|(time_ms, text)| LyricLine { time_ms, text })
        .collect()
}

fn parse_time(time_str: &str) -> Option<u64> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() < 2 {
        return None;
    }

    let mins = parts[0].parse::<u64>().ok()?;

    let rest = parts[1];
    let (secs_str, ms_str) = if let Some(dot_idx) = rest.find('.') {
        (&rest[..dot_idx], Some(&rest[dot_idx + 1..]))
    } else if let Some(colon_idx) = rest.find(':') {
        (&rest[..colon_idx], Some(&rest[colon_idx + 1..]))
    } else if parts.len() > 2 {
        (parts[1], Some(parts[2]))
    } else {
        (rest, None)
    };

    let secs = secs_str.parse::<u64>().ok()?;
    let mut ms = 0;
    if let Some(ms_raw) = ms_str {
        let mut raw = ms_raw.to_string();
        raw.retain(|c| c.is_ascii_digit());
        if !raw.is_empty() {
            ms = raw.parse::<u64>().ok().unwrap_or(0);
            if raw.len() == 2 {
                ms *= 10;
            } else if raw.len() == 1 {
                ms *= 100;
            } else if raw.len() > 3 {
                ms /= 10u64.pow((raw.len() - 3) as u32);
            }
        }
    }

    Some(mins * 60000 + secs * 1000 + ms)
}

fn url_encode(input: &str) -> String {
    let mut output = String::new();
    for b in input.bytes() {
        match b {
            b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'-' | b'_' | b'.' | b'~' => {
                output.push(b as char);
            }
            b' ' => {
                output.push_str("%20");
            }
            _ => {
                output.push_str(&format!("%{:02X}", b));
            }
        }
    }
    output
}
