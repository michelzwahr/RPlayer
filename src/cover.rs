//! Cover-Bild per Kitty Graphics Protocol anzeigen.

use std::error::Error;
use std::io::{self, Cursor, Write};

use base64::{engine::general_purpose::STANDARD, Engine};
use crossterm::{cursor::MoveTo, queue};
use image::{imageops::FilterType, ImageFormat, DynamicImage};
use crossterm::style::Color;


/// Zeichnet das PNG an Zelle (col, row), skaliert auf `cols` x `rows` Zellen.
/// Sollte nur EINMAL aufgerufen werden, nicht in jedem Frame.
pub fn show(png: &[u8], col: u16, row: u16, cols: u16, rows: u16) -> io::Result<()> {
    queue!(io::stdout(), MoveTo(col, row))?;

    let data = STANDARD.encode(png);
    let chunks: Vec<&[u8]> = data.as_bytes().chunks(4096).collect();
    let last = chunks.len() - 1;

    for (i, chunk) in chunks.iter().enumerate() {
        let more = if i == last { 0 } else { 1 };
        let chunk = std::str::from_utf8(chunk).unwrap();
        if i == 0 {
            // a=T: übertragen + anzeigen, f=100: PNG, C=1: Cursor nicht bewegen,
            // q=2: keine Antworten vom Terminal
            print!("\x1b_Ga=T,f=100,c={cols},r={rows},C=1,q=2,m={more};{chunk}\x1b\\");
        } else {
            print!("\x1b_Gm={more};{chunk}\x1b\\");
        }
    }
    io::stdout().flush()
}

/// Entfernt alle Bilder wieder vom Terminal.
pub fn clear() -> io::Result<()> {
    print!("\x1b_Ga=d,q=2\x1b\\");
    io::stdout().flush()
}

pub struct Cover {
    pub png: Vec<u8>,
    pub accent: Option<Color>, // None bei Graustufen-Covern
}

pub async fn fetch_cover(url: &str) -> Result<Cover, Box<dyn Error>> {
    let bytes = reqwest::get(url).await?.error_for_status()?.bytes().await?;
    let img = image::load_from_memory(&bytes)?.resize(400, 400, FilterType::Triangle);

    let accent = accent_color(&img);

    let mut png = Vec::new();
    img.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)?;
    Ok(Cover { png, accent })
}

fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let max = r.max(g).max(b);
    let d = max - r.min(g).min(b);
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / d) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };
    let s = if max == 0.0 { 0.0 } else { d / max };
    (h, s, max)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let to_u8 = |f: f32| ((f + m) * 255.0).round() as u8;
    (to_u8(r), to_u8(g), to_u8(b))
}

/// Dominante, kräftige Farbe des Bildes, oder None bei (fast) grauen Bildern.
fn accent_color(img: &DynamicImage) -> Option<Color> {
    let small = img.resize(32, 32, FilterType::Triangle).to_rgb8();

    // je Bereich: (Gewicht, Σ r·w, Σ g·w, Σ b·w)
    let mut buckets = [(0.0f32, 0.0f32, 0.0f32, 0.0f32); 12];

    for p in small.pixels() {
        let [r, g, b] = p.0;
        let (h, s, v) = rgb_to_hsv(r, g, b);
        if s < 0.25 || v < 0.25 {
            continue; // grau, weiß oder fast schwarz ignorieren
        }
        let w = s * v;
        let e = &mut buckets[((h / 30.0) as usize).min(11)];
        e.0 += w;
        e.1 += r as f32 * w;
        e.2 += g as f32 * w;
        e.3 += b as f32 * w;
    }

    let &(w, r, g, b) = buckets
        .iter()
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .filter(|b| b.0 > 0.0)?;

    let (h, s, v) = rgb_to_hsv((r / w) as u8, (g / w) as u8, (b / w) as u8);
    let (r, g, b) = hsv_to_rgb(h, s.max(0.45), v.max(0.75)); // auf dunklem Grund gut lesbar
    Some(Color::Rgb { r, g, b })
}