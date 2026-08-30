//! The picture a messaging app shows when a Share Link is sent (#65).
//!
//! It is **the page's own hero**, drawn again at card size: the photograph
//! under the indigo wash with the recipe's name standing on it, or — for the
//! roughly one recipe in three with no photograph — its Cover, carrying the
//! name bare, because a Cover's dye was chosen and needs no wash (#46, #81).
//!
//! Why the name is on the picture rather than left to the chat: every client
//! prints `og:title` as text beneath the image, so a name in both places is the
//! one thing that reads twice. Aurélien settled this on #65 — the name is set
//! once, here, and `og:title` says who shared it instead.
//!
//! Why it is drawn at all, rather than serving the photograph as it stands: a
//! recipe with no photograph would otherwise arrive as a naked URL, and the
//! recipe's name is the single most useful thing a card can carry.
//!
//! What this needs that nothing else in Kamosu did: a rasteriser. `tiny-skia`
//! fills the Cover's bezier shapes and `ab_glyph` turns the title into glyph
//! coverage, both pure Rust and both small. The alternative was a full SVG
//! engine, which is an order of magnitude more crates for a picture whose
//! entire vocabulary is *five path commands and one line of text*.

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, PixmapPaint, Point, Transform};

use crate::core::OpError;
use crate::cover::Cover;

/// 1.91:1, the shape every chat client crops a large link preview to. Drawing
/// at exactly that ratio is what stops a client cropping the title off.
pub const CARD_WIDTH: u32 = 1200;
pub const CARD_HEIGHT: u32 = 628;

/// The display face at the weight the identity uses for titles, in the form a
/// rasteriser reads. See `assets/fonts/README.md` for why both forms ship.
const TITLE_LATIN: &[u8] = include_bytes!("../assets/fonts/zen-old-mincho-600-latin.ttf");
const TITLE_LATIN_EXT: &[u8] = include_bytes!("../assets/fonts/zen-old-mincho-600-latin-ext.ttf");

/// The palette, read from the same tokens the stylesheet declares. Written as
/// numbers here because this is arithmetic on pixels, not CSS — the same
/// duplication `cover.rs` carries, and guarded the same way, by a test.
const ACCENT: (u8, u8, u8) = (0x1d, 0x2b, 0x4c); // --color-accent, ai · indigo
const ON_ACCENT: (u8, u8, u8) = (0xf4, 0xef, 0xe3); // --color-on-accent, kinari

/// The sizes a title is tried at, largest first, and how many lines it may run
/// to before it is set smaller instead.
const TITLE_SIZES: &[f32] = &[78.0, 66.0, 56.0, 48.0];
const MAX_TITLE_LINES: usize = 3;

/// Draw the card.
///
/// `photograph` is the recipe's Main Photo as stored bytes, or nothing — in
/// which case `cover` is drawn instead. One of the two always applies: a recipe
/// has a photograph or it has a Cover, and a Cover is never a placeholder for a
/// missing picture (#46).
pub fn draw(title: &str, photograph: Option<&[u8]>, cover: &Cover) -> Result<Vec<u8>, OpError> {
    let mut pixmap = Pixmap::new(CARD_WIDTH, CARD_HEIGHT)
        .ok_or_else(|| OpError::internal("cannot make the card's canvas"))?;

    let washed = match photograph {
        Some(bytes) => {
            paint_photograph(&mut pixmap, bytes)?;
            true
        }
        None => {
            paint_cover(&mut pixmap, cover)?;
            false
        }
    };

    // The wash goes over a photograph and never over a Cover.
    if washed {
        paint_wash(&mut pixmap);
    }
    paint_title(&mut pixmap, title)?;

    pixmap
        .encode_png()
        .map_err(|e| OpError::internal(format!("cannot encode the card: {e}")))
}

/// The photograph, scaled to cover the card and centred — the same `object-fit:
/// cover` the page's hero uses, so the card is genuinely a crop of what the
/// reader is about to open.
fn paint_photograph(pixmap: &mut Pixmap, bytes: &[u8]) -> Result<(), OpError> {
    let decoded = image::load_from_memory(bytes)
        .map_err(|e| OpError::internal(format!("cannot read the Photograph: {e}")))?
        .to_rgba8();
    let (width, height) = decoded.dimensions();
    if width == 0 || height == 0 {
        return Err(OpError::internal("the Photograph has no pixels"));
    }

    let source = Pixmap::from_vec(
        premultiply(decoded.as_raw()),
        tiny_skia::IntSize::from_wh(width, height)
            .ok_or_else(|| OpError::internal("the Photograph has no size"))?,
    )
    .ok_or_else(|| OpError::internal("cannot read the Photograph's pixels"))?;

    let scale = (f32::from(u16::try_from(CARD_WIDTH).unwrap_or(u16::MAX)) / width as f32)
        .max(f32::from(u16::try_from(CARD_HEIGHT).unwrap_or(u16::MAX)) / height as f32);
    let left = (CARD_WIDTH as f32 - width as f32 * scale) / 2.0;
    let top = (CARD_HEIGHT as f32 - height as f32 * scale) / 2.0;

    pixmap.draw_pixmap(
        0,
        0,
        source.as_ref(),
        &PixmapPaint::default(),
        Transform::from_translate(left, top).pre_scale(scale, scale),
        None,
    );
    Ok(())
}

/// `image` gives straight alpha; tiny-skia wants it premultiplied.
fn premultiply(rgba: &[u8]) -> Vec<u8> {
    rgba.chunks_exact(4)
        .flat_map(|p| {
            let a = u32::from(p[3]);
            let scale = |c: u8| u8::try_from(u32::from(c) * a / 255).unwrap_or(255);
            [scale(p[0]), scale(p[1]), scale(p[2]), p[3]]
        })
        .collect()
}

/// The Cover, drawn from the same eight shapes the app draws (#46). Its ground,
/// then the one large pasta shape, placed and turned exactly as `cover_for`
/// says — so the card and the page show one face.
fn paint_cover(pixmap: &mut Pixmap, cover: &Cover) -> Result<(), OpError> {
    pixmap.fill(rgb(cover.ground)?);

    // The shape is a square whose side is a fraction of the card's HEIGHT,
    // centred on a point given as fractions of the card's own box — the same
    // arithmetic `Cover.svelte` does in CSS, which is what lets one set of
    // numbers serve a 320px hero and a 628px card.
    let side = CARD_HEIGHT as f32 * cover.scale as f32;
    let centre_x = CARD_WIDTH as f32 * cover.centre_x as f32;
    let centre_y = CARD_HEIGHT as f32 * cover.centre_y as f32;
    // The paths are authored in a 0 0 100 100 viewBox.
    let unit = side / 100.0;
    let placing = Transform::from_translate(centre_x - side / 2.0, centre_y - side / 2.0)
        .pre_scale(unit, unit)
        .pre_concat(Transform::from_rotate_at(cover.rotation as f32, 50.0, 50.0));

    for (paths, fill, alpha) in [
        (cover.shape.body, cover.tone.as_str(), 1.0_f32),
        // The detail is the dye itself at half opacity: ridges and grooves,
        // seasoning rather than what makes a shape legible.
        (cover.shape.detail, cover.ground, 0.5_f32),
    ] {
        let mut colour = rgb(fill)?;
        colour.set_alpha(alpha);
        let mut paint = Paint::default();
        paint.set_color(colour);
        paint.anti_alias = true;
        for data in paths {
            let Some(path) = parse_path(data) else {
                return Err(OpError::internal(format!(
                    "a Cover's shape is not drawable: {data}"
                )));
            };
            pixmap.fill_path(&path, &paint, FillRule::Winding, placing, None);
        }
    }
    Ok(())
}

/// The indigo wash, over a photograph only.
///
/// The same gradient as `@utility wash` in `ui/src/app.css` — transparent at
/// the top, 55% at four tenths, 95% at the hem — scaled to the card. Written
/// here as pixel arithmetic because there is no browser to read the CSS, and
/// the stops are copied from there rather than chosen again.
fn paint_wash(pixmap: &mut Pixmap) {
    // The wash is 200px over a 320px hero in the app; the same proportion of
    // this card is what keeps the title as readable here as it is there.
    let height = (CARD_HEIGHT as f32 * (200.0 / 320.0)) as u32;
    let top = CARD_HEIGHT.saturating_sub(height);
    let width = CARD_WIDTH;
    let data = pixmap.pixels_mut();
    for y in top..CARD_HEIGHT {
        let t = (y - top) as f32 / height.max(1) as f32;
        // 0% at 0, 55% at 0.4, 95% at 1 — linear between the stops, as CSS
        // interpolates them.
        let alpha = if t < 0.4 {
            0.55 * (t / 0.4)
        } else {
            0.55 + (0.95 - 0.55) * ((t - 0.4) / 0.6)
        };
        for x in 0..width {
            let index = (y * width + x) as usize;
            let under = data[index];
            let over = |a: u8, b: u8| -> u8 {
                (f32::from(b).mul_add(alpha, f32::from(a) * (1.0 - alpha))) as u8
            };
            data[index] = tiny_skia::PremultipliedColorU8::from_rgba(
                over(under.red(), ACCENT.0),
                over(under.green(), ACCENT.1),
                over(under.blue(), ACCENT.2),
                255,
            )
            .unwrap_or(under);
        }
    }
}

/// The recipe's name, in the display face, standing on the picture.
///
/// Set once and only here. Wrapped to at most three lines and shrunk to fit
/// rather than truncated: a title cut mid-word in a chat is worse than a title
/// set small, and a recipe called *Aurélien's Creamy Miso Shin Ramyun with
/// Mushrooms and Cheddar* is a real row in the corpus, not a stress test.
/// The display face at one size: the two subsets the stylesheet declares, and
/// the size the title is being set at.
///
/// They exist as one thing because they are one thing — measuring a character,
/// breaking a line and drawing a glyph all need exactly this trio, and passing
/// it as three loose arguments is what had `draw_line` reaching for
/// `#[allow(clippy::too_many_arguments)]`.
struct TitleFace<'a> {
    latin: FontRef<'a>,
    ext: FontRef<'a>,
    size: f32,
}

impl<'a> TitleFace<'a> {
    fn load(size: f32) -> Result<Self, OpError> {
        let read = |bytes: &'a [u8]| {
            FontRef::try_from_slice(bytes)
                .map_err(|e| OpError::internal(format!("the display face is unreadable: {e}")))
        };
        Ok(Self {
            latin: read(TITLE_LATIN)?,
            ext: read(TITLE_LATIN_EXT)?,
            size,
        })
    }

    /// The same face at another size. `FontRef` is a borrowed view over bytes
    /// that live for the program, so cloning one copies a reference, not a font.
    fn at(&self, size: f32) -> Self {
        Self {
            latin: self.latin.clone(),
            ext: self.ext.clone(),
            size,
        }
    }

    /// Which subset actually has this character. The two are exactly the two
    /// the stylesheet declares, so the server draws what a browser would.
    fn font_for(&self, ch: char) -> Option<&FontRef<'a>> {
        if self.latin.glyph_id(ch).0 != 0 {
            Some(&self.latin)
        } else if self.ext.glyph_id(ch).0 != 0 {
            Some(&self.ext)
        } else {
            None
        }
    }

    /// How far the pen moves over this character. A character in neither
    /// subset still takes room: dropping it would silently reflow the line.
    fn advance(&self, ch: char) -> f32 {
        self.font_for(ch).map_or(self.size * 0.5, |font| {
            font.as_scaled(PxScale::from(self.size))
                .h_advance(font.glyph_id(ch))
        })
    }

    fn width_of(&self, text: &str) -> f32 {
        text.chars().map(|ch| self.advance(ch)).sum()
    }
}

/// The recipe's name, in the display face, standing on the picture.
///
/// Set once and only here. Wrapped to at most three lines and shrunk to fit
/// rather than truncated: a title cut mid-word in a chat is worse than a title
/// set small, and a recipe called *Aurélien's Creamy Miso Shin Ramyun with
/// Mushrooms and Cheddar* is a real row in the corpus, not a stress test.
///
/// The ink is kinari either way — on a photograph it sits on the wash, on a
/// Cover on the dye itself, and both were chosen to carry it.
fn paint_title(pixmap: &mut Pixmap, title: &str) -> Result<(), OpError> {
    let face = TitleFace::load(TITLE_SIZES[0])?;
    let margin = 48.0_f32;
    let usable = CARD_WIDTH as f32 - margin * 2.0;

    // Come down through the sizes until the title fits three lines. The largest
    // is the page title's 27px scaled to this card; nothing smaller than the
    // last would be legible as a thumbnail in a chat list.
    let (face, lines) = TITLE_SIZES
        .iter()
        .map(|size| face.at(*size))
        .find_map(|face| {
            let lines = wrap(title, &face, usable);
            (lines.len() <= MAX_TITLE_LINES).then_some((face, lines))
        })
        .unwrap_or_else(|| {
            let smallest = face.at(TITLE_SIZES[TITLE_SIZES.len() - 1]);
            let mut lines = wrap(title, &smallest, usable);
            lines.truncate(MAX_TITLE_LINES);
            (smallest, lines)
        });

    let line_height = face.size * 1.16;
    let baseline_of_last = CARD_HEIGHT as f32 - margin;
    for (index, line) in lines.iter().enumerate() {
        let from_bottom = (lines.len() - 1 - index) as f32;
        let baseline = baseline_of_last - from_bottom * line_height;
        draw_line(pixmap, line, &face, margin, baseline, ON_ACCENT);
    }
    Ok(())
}

/// Break the title on spaces, and only on spaces: a recipe's name is somebody's
/// words and hyphenating it would invent a spelling.
fn wrap(title: &str, face: &TitleFace, usable: f32) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    let mut width = 0.0_f32;
    for word in title.split_whitespace() {
        let word_width = face.width_of(word);
        let space = if line.is_empty() {
            0.0
        } else {
            face.advance(' ')
        };
        if !line.is_empty() && width + space + word_width > usable {
            lines.push(std::mem::take(&mut line));
            width = 0.0;
        }
        if !line.is_empty() {
            line.push(' ');
            width += space;
        }
        line.push_str(word);
        width += word_width;
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

fn draw_line(
    pixmap: &mut Pixmap,
    line: &str,
    face: &TitleFace,
    left: f32,
    baseline: f32,
    ink: (u8, u8, u8),
) {
    let mut pen = left;
    for ch in line.chars() {
        let Some(font) = face.font_for(ch) else {
            pen += face.advance(ch);
            continue;
        };
        let glyph = font
            .glyph_id(ch)
            .with_scale_and_position(PxScale::from(face.size), ab_glyph::point(pen, baseline));
        if let Some(outline) = font.outline_glyph(glyph) {
            let bounds = outline.px_bounds();
            outline.draw(|x, y, coverage| {
                let px = bounds.min.x + x as f32;
                let py = bounds.min.y + y as f32;
                if px < 0.0 || py < 0.0 {
                    return;
                }
                let (px, py) = (px as u32, py as u32);
                if px >= CARD_WIDTH || py >= CARD_HEIGHT {
                    return;
                }
                let index = (py * CARD_WIDTH + px) as usize;
                let data = pixmap.pixels_mut();
                let under = data[index];
                let over = |a: u8, b: u8| -> u8 {
                    (f32::from(b).mul_add(coverage, f32::from(a) * (1.0 - coverage))) as u8
                };
                data[index] = tiny_skia::PremultipliedColorU8::from_rgba(
                    over(under.red(), ink.0),
                    over(under.green(), ink.1),
                    over(under.blue(), ink.2),
                    255,
                )
                .unwrap_or(under);
            });
        }
        pen += face.advance(ch);
    }
}

/// A `#rrggbb` token as a colour.
fn rgb(hex: &str) -> Result<Color, OpError> {
    let bad = || OpError::internal(format!("not a colour: {hex}"));
    if hex.len() != 7 || !hex.starts_with('#') {
        return Err(bad());
    }
    let channel = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).map_err(|_| bad());
    Ok(Color::from_rgba8(
        channel(1)?,
        channel(3)?,
        channel(5)?,
        255,
    ))
}

/// The five path commands the Cover's shapes are authored in — `M`, `L`, `C`,
/// `Q` and `Z`, all absolute — and deliberately nothing else.
///
/// Anything richer returns `None` rather than being approximated, so a shape
/// that grows an arc is a loud failure here instead of a quietly wrong drawing
/// on the one surface strangers see. `ui/generate-cover.mjs` and this parser are
/// the two ends of the same contract.
fn parse_path(data: &str) -> Option<tiny_skia::Path> {
    let mut builder = PathBuilder::new();
    let mut numbers: Vec<f32> = Vec::new();
    let mut command: Option<char> = None;
    let mut start = Point::from_xy(0.0, 0.0);
    let mut at = Point::from_xy(0.0, 0.0);

    let flush = |builder: &mut PathBuilder,
                 command: Option<char>,
                 numbers: &mut Vec<f32>,
                 start: &mut Point,
                 at: &mut Point|
     -> Option<()> {
        match command {
            None => (numbers.is_empty()).then_some(()),
            Some('M') => {
                if numbers.len() != 2 {
                    return None;
                }
                *at = Point::from_xy(numbers[0], numbers[1]);
                *start = *at;
                builder.move_to(at.x, at.y);
                numbers.clear();
                Some(())
            }
            Some('L') => {
                if numbers.len() != 2 {
                    return None;
                }
                *at = Point::from_xy(numbers[0], numbers[1]);
                builder.line_to(at.x, at.y);
                numbers.clear();
                Some(())
            }
            Some('C') => {
                if !numbers.len().is_multiple_of(6) || numbers.is_empty() {
                    return None;
                }
                for six in numbers.chunks_exact(6) {
                    builder.cubic_to(six[0], six[1], six[2], six[3], six[4], six[5]);
                    *at = Point::from_xy(six[4], six[5]);
                }
                numbers.clear();
                Some(())
            }
            Some('Q') => {
                if !numbers.len().is_multiple_of(4) || numbers.is_empty() {
                    return None;
                }
                for four in numbers.chunks_exact(4) {
                    builder.quad_to(four[0], four[1], four[2], four[3]);
                    *at = Point::from_xy(four[2], four[3]);
                }
                numbers.clear();
                Some(())
            }
            Some('Z') => {
                if !numbers.is_empty() {
                    return None;
                }
                builder.close();
                *at = *start;
                Some(())
            }
            Some(_) => None,
        }
    };

    for token in data.split_whitespace() {
        if let Ok(number) = token.parse::<f32>() {
            numbers.push(number);
            continue;
        }
        let mut chars = token.chars();
        let letter = chars.next()?;
        if chars.next().is_some() || !"MLCQZ".contains(letter) {
            return None;
        }
        flush(&mut builder, command, &mut numbers, &mut start, &mut at)?;
        command = Some(letter);
    }
    flush(&mut builder, command, &mut numbers, &mut start, &mut at)?;
    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cover::cover_for;
    use crate::cover_faces::PASTA_SHAPES;

    /// Every shape Kamosu ships must be drawable by the parser above. This is
    /// the guard on the contract between `ui/generate-cover.mjs` and this file:
    /// a shape authored with a command the server cannot draw would show up
    /// only as a broken card in somebody's chat.
    #[test]
    fn every_pasta_shape_is_drawable() {
        for shape in PASTA_SHAPES {
            for data in shape.body.iter().chain(shape.detail.iter()) {
                assert!(
                    parse_path(data).is_some(),
                    "{} has a path the card cannot draw: {data}",
                    shape.key
                );
            }
        }
    }

    #[test]
    fn a_richer_path_is_refused_rather_than_approximated() {
        // An elliptical arc: legal SVG, and not something this parser guesses at.
        assert!(parse_path("M 0 0 A 10 10 0 0 1 20 20 Z").is_none());
        // A relative command, likewise.
        assert!(parse_path("M 0 0 c 1 1 2 2 3 3 Z").is_none());
    }

    #[test]
    fn a_card_is_drawn_for_a_recipe_with_no_photograph() {
        let cover = cover_for("l_1848cb7653cb9d93");
        let png = draw("Braised Chicken in Red Wine", None, &cover).expect("a card");
        // A PNG, and a real one: the magic number, and enough bytes to be a
        // picture rather than an empty canvas.
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert!(png.len() > 5_000, "the card is suspiciously small");
    }

    #[test]
    fn a_long_title_wraps_rather_than_being_cut() {
        let face = TitleFace::load(78.0).expect("the display face");
        let lines = wrap(
            "Aurelien's Creamy Miso Shin Ramyun with Mushrooms and Cheddar",
            &face,
            1104.0,
        );
        assert!(lines.len() > 1, "a long title should wrap");
        assert!(
            lines.iter().all(|line| !line.is_empty()),
            "no line is empty"
        );
        // Every word survives: wrapping never drops one.
        let rejoined = lines.join(" ");
        assert_eq!(
            rejoined,
            "Aurelien's Creamy Miso Shin Ramyun with Mushrooms and Cheddar"
        );
    }

    /// The `.ttf` the server draws with must stay in step with the `.woff2` a
    /// browser loads: they are one subset in two forms (`assets/fonts/README.md`),
    /// and a card whose title is set in a different face from the page's would
    /// be a quiet, permanent mismatch on the one surface strangers see.
    ///
    /// A byte-for-byte gate is not available without a woff2 decoder, which is
    /// a dependency this earns nowhere else. What is checked instead is the
    /// thing that would actually break: the coverage the stylesheet promises.
    /// `ui/src/app.css` declares a `unicode-range` for each face, and a `.ttf`
    /// missing a character in it would drop that glyph from a title silently.
    #[test]
    fn the_face_the_server_draws_with_covers_what_the_stylesheet_promises() {
        let face = TitleFace::load(78.0).expect("the display face loads");
        // Drawn from the two declared ranges: ASCII, the Latin-1 letters a
        // French or Spanish title needs, the ligature and the punctuation the
        // identity actually sets.
        for ch in "AZaz09 ÀÉÈÊÇÔÙàéèêçôùÑñÜü½¾·—’()&,.:".chars() {
            assert!(
                face.font_for(ch).is_some(),
                "the display face has no glyph for {ch:?} — the .ttf has drifted \
                 from the .woff2 the browser loads, or was regenerated wrongly"
            );
        }
        // And a character in neither subset is handled rather than panicking:
        // a title may contain anything somebody can type.
        assert!(face.font_for('漢').is_none());
        assert!(
            face.advance('漢') > 0.0,
            "an unknown character still takes room"
        );
    }

    /// The two colours this file works in are design tokens, written here as
    /// numbers because pixels are arithmetic. They must not drift from the
    /// stylesheet that ships.
    #[test]
    fn the_cards_colours_are_the_tokens() {
        let css = crate::design_tokens::STYLESHEET;
        for (token, (r, g, b)) in [("--color-accent", ACCENT), ("--color-on-accent", ON_ACCENT)] {
            let declared = css
                .split(&format!("{token}:"))
                .nth(1)
                .and_then(|rest| rest.split(';').next())
                .map(str::trim)
                .unwrap_or_default()
                .to_ascii_lowercase();
            assert_eq!(
                declared,
                format!("#{r:02x}{g:02x}{b:02x}"),
                "{token} has moved in ui/src/app.css and this file did not follow"
            );
        }
    }
}
