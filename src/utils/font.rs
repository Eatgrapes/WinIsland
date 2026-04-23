use skia_safe::{Canvas, Font, FontMgr, FontStyle, Paint, Typeface};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;
use crate::core::persistence::load_config;

static GLOBAL_FONT_MANAGER: OnceLock<FontManager> = OnceLock::new();

pub struct FontManager {
    _marker: (),
}

thread_local! {
    static FONT_MGR: FontMgr = FontMgr::new();
    static FALLBACK_CACHE: RefCell<HashMap<(char, u32), Typeface>> = RefCell::new(HashMap::new());
    static TEXT_CACHE: RefCell<HashMap<String, (String, Vec<(String, Typeface, bool)>)>> = RefCell::new(HashMap::new());
    static CUSTOM_TYPEFACE: RefCell<Option<(String, Typeface)>> = RefCell::new(None);
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
            if let Some((ref cached_path, ref tf)) = *cache_mut {
                if cached_path == &path {
                    return Some(tf.clone());
                }
            }
            if let Ok(data) = std::fs::read(&path) {
                if let Some(tf) = FONT_MGR.with(|mgr| mgr.new_from_data(&data, None)) {
                    *cache_mut = Some((path, tf.clone()));
                    return Some(tf);
                }
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
        if cache.len() > 2000 { cache.clear(); }
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

        let tf = FONT_MGR.with(|mgr| mgr.match_family_style_character("", style, &["zh-CN", "ja-JP", "en-US"], c as i32))
            .unwrap_or_else(|| FONT_MGR.with(|mgr| mgr.legacy_make_typeface(None, style).unwrap()));
        let embolden = needs_synthetic_bold(&tf, style);
        cache.insert((c, s_key), tf.clone());
        (tf, embolden)
    })
}

impl FontManager {
    pub fn global() -> &'static FontManager {
        GLOBAL_FONT_MANAGER.get_or_init(|| FontManager { _marker: () })
    }

    pub fn get_font(&self, size: f32, bold: bool) -> Font {
        let style = if bold { FontStyle::bold() } else { FontStyle::normal() };
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

    pub fn measure(&self, text: &str, size: f32, bold: bool) -> (f32, skia_safe::Rect) {
        let font = self.get_font(size, bold);
        font.measure_str(text, None)
    }

    pub fn draw_text(&self, canvas: &Canvas, text: &str, pos: (f32, f32), size: f32, bold: bool, paint: &Paint) {
        let font = self.get_font(size, bold);
        canvas.draw_str(text, pos, &font, paint);
    }

    pub fn draw_text_with_custom_font(&self, canvas: &Canvas, text: &str, pos: (f32, f32), size: f32, bold: bool, paint: &Paint) {
        let style = if bold { FontStyle::bold() } else { FontStyle::normal() };
        if let Some(tf) = get_custom_typeface() {
            let font = make_font(tf, size, style);
            canvas.draw_str(text, pos, &font, paint);
        } else {
            let font = self.get_font(size, bold);
            canvas.draw_str(text, pos, &font, paint);
        }
    }

    pub fn draw_text_with_default_font(&self, canvas: &Canvas, text: &str, pos: (f32, f32), size: f32, bold: bool, paint: &Paint) {
        let style = if bold { FontStyle::bold() } else { FontStyle::normal() };
        let typeface = FONT_MGR.with(|mgr| {
            mgr.match_family_style("Microsoft YaHei", style)
                .or_else(|| mgr.match_family_style("Segoe UI", style))
                .unwrap_or_else(|| mgr.legacy_make_typeface(None, style).unwrap())
        });
        let font = make_font(typeface, size, style);
        canvas.draw_str(text, pos, &font, paint);
    }

    pub fn draw_text_centered(&self, canvas: &Canvas, text: &str, center_x: f32, y: f32, size: f32, bold: bool, paint: &Paint) {
        let font = self.get_font(size, bold);
        let (_, rect) = font.measure_str(text, None);
        canvas.draw_str(text, (center_x - rect.width() / 2.0, y), &font, paint);
    }

    pub fn draw_text_in_rect(&self, canvas: &Canvas, text: &str, x: f32, y: f32, w: f32, size: f32, bold: bool, paint: &Paint) {
        let font = self.get_font(size, bold);
        let (_, rect) = font.measure_str(text, None);
        if rect.width() <= w {
            canvas.draw_str(text, (x + (w - rect.width()) / 2.0, y), &font, paint);
        } else {
            let mut truncated = String::new();
            let mut current_w = 0.0;
            let (ellipsis_w, _) = font.measure_str("...", None);
            let max_w = w - ellipsis_w;
            for c in text.chars() {
                let (cw, _) = font.measure_str(&c.to_string(), None);
                if current_w + cw > max_w { break; }
                current_w += cw;
                truncated.push(c);
            }
            truncated.push_str("...");
            canvas.draw_str(&truncated, (x, y), &font, paint);
        }
    }

    pub fn measure_text_cached(&self, text: &str, size: f32, style: FontStyle) -> f32 {
        let cache_key = format!("measure-{}-{:?}-{}", text, style, size as i32);
        TEXT_CACHE.with(|cache| {
            let mut cache_mut = cache.borrow_mut();
            if cache_mut.len() > 500 { cache_mut.clear(); }
            if !cache_mut.contains_key(&cache_key) {
                let mut current_w = 0.0;
                let mut groups: Vec<(String, Typeface, bool)> = Vec::new();
                let mut current_group = String::new();
                let mut last_tf: Option<Typeface> = None;
                let mut last_embolden = false;
                for c in text.chars() {
                    let (tf, embolden) = get_typeface_for_char(c, style);
                    if let Some(ref ltf) = last_tf {
                        if ltf.unique_id() != tf.unique_id() || last_embolden != embolden {
                            groups.push((current_group.clone(), ltf.clone(), last_embolden));
                            current_group.clear();
                        }
                    }
                    last_tf = Some(tf);
                    last_embolden = embolden;
                    current_group.push(c);
                }
                if let Some(ltf) = last_tf { groups.push((current_group, ltf, last_embolden)); }

                for (s, tf, embolden) in &groups {
                    let mut font = Font::from_typeface(tf.clone(), size);
                    if *embolden { font.set_embolden(true); }
                    let (w, _) = font.measure_str(s, None);
                    current_w += w;
                }
                
                cache_mut.insert(cache_key.clone(), (current_w.to_string(), groups));
                return current_w;
            }
            let (w_str, _) = cache_mut.get(&cache_key).unwrap();
            w_str.parse::<f32>().unwrap_or(0.0)
        })
    }

    pub fn draw_text_cached(&self, canvas: &Canvas, text: &str, pos: (f32, f32), size: f32, style: FontStyle, paint: &Paint, align_center: bool, max_w: f32) {
        let cache_key = format!("{}-{}-{:?}-{}", text, max_w as i32, style, size as i32);
        TEXT_CACHE.with(|cache| {
            let mut cache_mut = cache.borrow_mut();
            if cache_mut.len() > 500 { cache_mut.clear(); }
            if !cache_mut.contains_key(&cache_key) {
                let mut current_w = 0.0;
                let mut truncated = String::new();
                for c in text.chars() {
                    let (tf, embolden) = get_typeface_for_char(c, style);
                    let mut font = Font::from_typeface(tf, size);
                    if embolden { font.set_embolden(true); }
                    let (w, _) = font.measure_str(&c.to_string(), None);
                    if current_w + w > max_w { truncated.push_str("..."); break; }
                    current_w += w; truncated.push(c);
                }
                let mut groups: Vec<(String, Typeface, bool)> = Vec::new();
                let mut current_group = String::new();
                let mut last_tf: Option<Typeface> = None;
                let mut last_embolden = false;
                for c in truncated.chars() {
                    let (tf, embolden) = get_typeface_for_char(c, style);
                    if let Some(ref ltf) = last_tf {
                        if ltf.unique_id() != tf.unique_id() || last_embolden != embolden {
                            groups.push((current_group.clone(), ltf.clone(), last_embolden));
                            current_group.clear();
                        }
                    }
                    last_tf = Some(tf);
                    last_embolden = embolden;
                    current_group.push(c);
                }
                if let Some(ltf) = last_tf { groups.push((current_group, ltf, last_embolden)); }
                cache_mut.insert(cache_key.clone(), (truncated, groups));
            }
            let (_, groups) = cache_mut.get(&cache_key).unwrap();
            let mut total_width = 0.0;
            if align_center {
                for (s, tf, embolden) in groups {
                    let mut font = Font::from_typeface(tf.clone(), size);
                    if *embolden { font.set_embolden(true); }
                    let (w, _) = font.measure_str(s, None);
                    total_width += w;
                }
            }
            let mut x = if align_center { pos.0 - total_width / 2.0 } else { pos.0 };
            let y = pos.1.round();
            for (s, tf, embolden) in groups {
                let mut font = Font::from_typeface(tf.clone(), size);
                if *embolden { font.set_embolden(true); }
                canvas.draw_str(s, (x.round(), y), &font, paint);
                let (w, _) = font.measure_str(s, None);
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
