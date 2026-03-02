use std::sync::Arc;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Default, Debug)]
pub struct LyricLine {
    pub time_ms: u64,
    pub text: String,
}

pub fn fetch_lyrics(title: &str, artist: &str) -> Option<Arc<Vec<LyricLine>>> {
    if title.is_empty() {
        return None;
    }

    let query = format!("{} {}", title, artist);
    let url = format!("http://music.163.com/api/search/get/web?s={}&type=1&offset=0&total=true&limit=1", url_encode(&query));

    let res = ureq::get(&url)
        .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .call()
        .ok()?;
        
    let json: Value = res.into_json().ok()?;
    
    let song_id = json.get("result")?
        .get("songs")?
        .as_array()?
        .get(0)?
        .get("id")?
        .as_i64()?;

    let lyric_url = format!("http://music.163.com/api/song/lyric?id={}&lv=1&kv=1&tv=-1", song_id);
    
    let lyric_res = ureq::get(&lyric_url)
        .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .call()
        .ok()?;
        
    let lyric_json: Value = lyric_res.into_json().ok()?;
    
    let lrc_str = lyric_json.get("lrc")?.get("lyric")?.as_str().unwrap_or("");
    
    Some(Arc::new(parse_lyrics(lrc_str)))
}

fn parse_lyrics(lrc: &str) -> Vec<LyricLine> {
    let mut map: BTreeMap<u64, String> = BTreeMap::new();

    for line in lrc.lines() {
        let line = line.trim();
        if !line.starts_with('[') { continue; }
        
        let parts: Vec<&str> = line.split(']').collect();
        if parts.len() < 2 { continue; }
        
        let text = parts[parts.len() - 1].trim().to_string();
        for time_part in &parts[..parts.len() - 1] {
            let time_str = time_part.trim_start_matches('[');
            if let Some(ms) = parse_time(time_str) {
                // If same timestamp exists, we keep the first one encountered (or overwrite, map.insert does overwrite)
                // NetEase usually has original lyrics in 'lrc', so this is fine.
                map.entry(ms).or_insert(text.clone());
            }
        }
    }

    map.into_iter()
        .map(|(time_ms, text)| LyricLine { time_ms, text })
        .collect()
}

fn parse_time(time_str: &str) -> Option<u64> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() < 2 { return None; }
    
    let mins = parts[0].parse::<u64>().ok()?;
    
    let rest = parts[1];
    let (secs_str, ms_str) = if let Some(dot_idx) = rest.find('.') {
        (&rest[..dot_idx], Some(&rest[dot_idx+1..]))
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
        ms = raw.parse::<u64>().ok().unwrap_or(0);
        if raw.len() == 2 { ms *= 10; }
        else if raw.len() == 1 { ms *= 100; }
        else if raw.len() > 3 { ms /= 10u64.pow((raw.len() - 3) as u32); }
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
                output.push('+');
            }
            _ => {
                output.push_str(&format!("%{:02X}", b));
            }
        }
    }
    output
}
