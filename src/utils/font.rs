use crate::core::persistence::load_config;
use skia_safe::{Canvas, Font, FontMgr, FontStyle, Paint, Typeface};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

static GLOBAL_FONT_MANAGER: OnceLock<FontManager> = OnceLock::new();

type TextGroup = (String, Typeface, bool);
type TextGroups = Vec<TextGroup>;
type TextCacheValue = (f32, TextGroups);
type TextCacheMap = HashMap<u64, TextCacheValue>;

pub struct DrawTextInRectParams<'a> {
    pub canvas: &'a Canvas,
    pub text: &'a str,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub size: f32,
    pub bold: bool,
    pub paint: &'a Paint,
}

pub struct DrawTextCachedParams<'a> {
    pub canvas: &'a Canvas,
    pub text: &'a str,
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub bold: bool,
    pub paint: &'a Paint,
}

pub struct FontManager {
    _marker: (),
}

thread_local! {
    static FONT_MGR: FontMgr = FontMgr::new();
    static FALLBACK_CACHE: RefCell<HashMap<(char, u32), Typeface>> = RefCell::new(HashMap::new());
    static TEXT_CACHE: RefCell<TextCacheMap> = RefCell::new(HashMap::new());
    static CUSTOM_TYPEFACE: RefCell<Option<(String, Typeface)>> = const { RefCell::new(None) };
}

const FALLBACK_CACHE_LIMIT: usize = 2000;
const TEXT_CACHE_LIMIT: usize = 500;

fn evict_one_if_full<K, V>(cache: &mut HashMap<K, V>, limit: usize)
where
    K: Clone + std::cmp::Eq + std::hash::Hash,
{
    if cache.len() > limit
        && let Some(key) = cache.keys().next().cloned()
    {
        cache.remove(&key);
    }
}

fn hash_cache_key(text: &str, bold: u32, size_key: i32) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    bold.hash(&mut hasher);
    size_key.hash(&mut hasher);
    hasher.finish()
}

fn style_to_key(style: FontStyle) -> u32 {
    let weight = *style.weight() as u32;
    let width = *style.width() as u32;
    let slant = style.slant() as u32;
    (weight << 16) | (width << 8) | slant
}

fn needs_synthetic_bold(tf: &Typeface, style: FontStyle) -> bool {
    *style.weight() >= 600 && *tf.font_style().weight() < 600
}

fn make_font(tf: Typeface, size: f32, style: FontStyle) -> Font {
    let embolden = needs_synthetic_bold(&tf, style);
    let mut font = Font::from_typeface(tf, size);
    if embolden {
        font.set_embolden(true);
    }
    font
}

fn get_custom_typeface() -> Option<Typeface> {
    let config = load_config();
    if let Some(path) = config.custom_font_path {
        CUSTOM_TYPEFACE.with(|cache| {
            let mut cache_mut = cache.borrow_mut();
            if let Some((ref cached_path, ref tf)) = *cache_mut
                && cached_path == &path
            {
                return Some(tf.clone());
            }
            if let Ok(data) = std::fs::read(&path)
                && let Some(tf) = FONT_MGR.with(|mgr| mgr.new_from_data(&data, None))
            {
                *cache_mut = Some((path, tf.clone()));
                return Some(tf);
            }
            None
        })
    } else {
        None
    }
}

fn get_typeface_for_char(c: char, style: FontStyle) -> (Typeface, bool) {
    let s_key = style_to_key(style);
    FALLBACK_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        evict_one_if_full(&mut cache, FALLBACK_CACHE_LIMIT);
        if let Some(tf) = cache.get(&(c, s_key)) {
            let embolden = needs_synthetic_bold(tf, style);
            return (tf.clone(), embolden);
        }

        if let Some(tf) = get_custom_typeface() {
            let mut glyphs = [0u16; 1];
            tf.unichars_to_glyphs(&[c as i32], &mut glyphs);
            if glyphs[0] != 0 {
                let embolden = needs_synthetic_bold(&tf, style);
                cache.insert((c, s_key), tf.clone());
                return (tf, embolden);
            }
        }

        let tf = FONT_MGR
            .with(|mgr| {
                mgr.match_family_style_character("", style, &["zh-CN", "ja-JP", "en-US"], c as i32)
            })
            .unwrap_or_else(|| FONT_MGR.with(|mgr| mgr.legacy_make_typeface(None, style).unwrap()));
        let embolden = needs_synthetic_bold(&tf, style);
        cache.insert((c, s_key), tf.clone());
        (tf, embolden)
    })
}

fn is_ascii_text(text: &str) -> bool {
    text.bytes().all(|b| b.is_ascii())
}

/// Compute text groups and total width.
/// Falls back to a single typeface for ASCII-only text to skip per-char lookups.
fn compute_text_groups(text: &str, size: f32, style: FontStyle) -> (f32, TextGroups) {
    let mut current_w = 0.0;
    let mut groups: TextGroups = Vec::new();

    if is_ascii_text(text) {
        let tf = get_custom_typeface().unwrap_or_else(|| {
            FONT_MGR.with(|mgr| {
                mgr.match_family_style("Microsoft YaHei", style)
                    .or_else(|| mgr.match_family_style("Segoe UI", style))
                    .unwrap_or_else(|| mgr.legacy_make_typeface(None, style).unwrap())
            })
        });
        let embolden = needs_synthetic_bold(&tf, style);
        let mut font = Font::from_typeface(tf.clone(), size);
        if embolden {
            font.set_embolden(true);
        }
        let (w, _) = font.measure_str(text, None);
        current_w += w;
        groups.push((text.to_string(), tf, embolden));
        return (current_w, groups);
    }

    let mut current_group = String::new();
    let mut last_tf: Option<Typeface> = None;
    let mut last_embolden = false;
    for c in text.chars() {
        let (tf, embolden) = get_typeface_for_char(c, style);
        if let Some(ref ltf) = last_tf
            && (ltf.unique_id() != tf.unique_id() || last_embolden != embolden)
        {
            groups.push((current_group.clone(), ltf.clone(), last_embolden));
            current_group.clear();
        }
        last_tf = Some(tf);
        last_embolden = embolden;
        current_group.push(c);
    }
    if let Some(ltf) = last_tf {
        groups.push((current_group, ltf, last_embolden));
    }

    for (s, tf, embolden) in &groups {
        let mut font = Font::from_typeface(tf.clone(), size);
        if *embolden {
            font.set_embolden(true);
        }
        let (w, _) = font.measure_str(s, None);
        current_w += w;
    }

    (current_w, groups)
}

impl FontManager {
    pub fn global() -> &'static FontManager {
        GLOBAL_FONT_MANAGER.get_or_init(|| FontManager { _marker: () })
    }

    pub fn get_font(&self, size: f32, bold: bool) -> Font {
        let style = if bold {
            FontStyle::bold()
        } else {
            FontStyle::normal()
        };
        if let Some(tf) = get_custom_typeface() {
            return make_font(tf, size, style);
        }
        let typeface = FONT_MGR.with(|mgr| {
            mgr.match_family_style("Microsoft YaHei", style)
                .or_else(|| mgr.match_family_style("Segoe UI", style))
                .unwrap_or_else(|| mgr.legacy_make_typeface(None, style).unwrap())
        });
        make_font(typeface, size, style)
    }

    pub fn draw_text_with_custom_font(
        &self,
        canvas: &Canvas,
        text: &str,
        pos: (f32, f32),
        size: f32,
        bold: bool,
        paint: &Paint,
    ) {
        let style = if bold {
            FontStyle::bold()
        } else {
            FontStyle::normal()
        };
        if let Some(tf) = get_custom_typeface() {
            let font = make_font(tf, size, style);
            canvas.draw_str(text, pos, &font, paint);
        } else {
            let font = self.get_font(size, bold);
            canvas.draw_str(text, pos, &font, paint);
        }
    }

    pub fn draw_text_with_default_font(
        &self,
        canvas: &Canvas,
        text: &str,
        pos: (f32, f32),
        size: f32,
        bold: bool,
        paint: &Paint,
    ) {
        let style = if bold {
            FontStyle::bold()
        } else {
            FontStyle::normal()
        };
        let typeface = FONT_MGR.with(|mgr| {
            mgr.match_family_style("Microsoft YaHei", style)
                .or_else(|| mgr.match_family_style("Segoe UI", style))
                .unwrap_or_else(|| mgr.legacy_make_typeface(None, style).unwrap())
        });
        let font = make_font(typeface, size, style);
        canvas.draw_str(text, pos, &font, paint);
    }

    pub fn draw_text_in_rect(&self, params: DrawTextInRectParams<'_>) {
        let font = self.get_font(params.size, params.bold);
        let (_, rect) = font.measure_str(params.text, None);
        if rect.width() <= params.w {
            params.canvas.draw_str(
                params.text,
                (params.x + (params.w - rect.width()) / 2.0, params.y),
                &font,
                params.paint,
            );
        } else {
            let mut truncated = String::new();
            let mut current_w = 0.0;
            let (ellipsis_w, _) = font.measure_str("...", None);
            let max_w = params.w - ellipsis_w;
            for c in params.text.chars() {
                let (cw, _) = font.measure_str(c.to_string(), None);
                if current_w + cw > max_w {
                    break;
                }
                current_w += cw;
                truncated.push(c);
            }
            truncated.push_str("...");
            params
                .canvas
                .draw_str(&truncated, (params.x, params.y), &font, params.paint);
        }
    }

    pub fn measure_text_cached(&self, text: &str, size: f32, style: FontStyle) -> f32 {
        let cache_key = hash_cache_key(text, style_to_key(style), (size * 100.0).round() as i32);
        TEXT_CACHE.with(|cache| {
            let mut cache_mut = cache.borrow_mut();
            evict_one_if_full(&mut cache_mut, TEXT_CACHE_LIMIT);
            let entry = cache_mut.entry(cache_key).or_insert_with(|| {
                let (width, groups) = compute_text_groups(text, size, style);
                (width, groups)
            });
            entry.0
        })
    }

    pub fn draw_text_cached(&self, params: DrawTextCachedParams<'_>) {
        let style = if params.bold {
            FontStyle::bold()
        } else {
            FontStyle::normal()
        };
        let cache_key = hash_cache_key(params.text, params.bold as u32, params.size as i32);
        TEXT_CACHE.with(|cache| {
            let mut cache_mut = cache.borrow_mut();
            evict_one_if_full(&mut cache_mut, TEXT_CACHE_LIMIT);
            let entry = cache_mut.entry(cache_key).or_insert_with(|| {
                let (_, groups) = compute_text_groups(params.text, params.size, style);
                (0.0, groups)
            });
            let (_, groups) = entry;
            let mut x = params.x;
            let y = params.y.round();
            for (s, tf, embolden) in groups {
                let mut font = Font::from_typeface(tf.clone(), params.size);
                if *embolden {
                    font.set_embolden(true);
                }
                params
                    .canvas
                    .draw_str(&**s, (x.round(), y), &font, params.paint);
                let (w, _) = font.measure_str(&**s, None);
                x += w;
            }
        });
    }

    pub fn refresh_custom_font(&self) {
        CUSTOM_TYPEFACE.with(|cache| {
            *cache.borrow_mut() = None;
        });
        TEXT_CACHE.with(|cache| {
            cache.borrow_mut().clear();
        });
        FALLBACK_CACHE.with(|cache| {
            cache.borrow_mut().clear();
        });
    }
}
