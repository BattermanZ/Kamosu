//! The Cover: what leads a recipe with no Main Photo (#46).
//!
//! This is the Rust half of a drawing that lives in two places, and it exists
//! because the Share Link page is rendered by the server as plain HTML (#65,
//! ADR 0012) while the app draws its own Covers in Svelte. The eight dyes and
//! eight shapes are **generated** into `cover_faces.rs` from the TypeScript, so
//! the artwork itself cannot drift. What is written twice is the arithmetic
//! below — a hash and four remainders — and the tests at the foot of this file
//! pin it against fixtures taken from the TypeScript, so the two cannot part
//! company without a failing build.
//!
//! Everything about how a Cover looks is derived from the recipe's **Lineage
//! id and nothing else**, which is what makes the promise hold: a rename never
//! changes a recipe's face, a Translation wears its source's face, and the same
//! Lineage wears the same face on every instance it ever reaches — including,
//! now, on a page served to somebody with no account at all.

use crate::cover_faces::{DYES, Dye, PASTA_SHAPES, PastaShape};

/// Kinari, the app's unbleached-paper ground, and sumi, its ink — the two ends
/// a dye is blended towards. These are the `--color-ground` and `--color-ink`
/// design tokens; `ui/src/lib/cover/cover.ts` carries the same two numbers and
/// its own spec test reads them back out of the stylesheet, so the stylesheet
/// remains the one place they are decided.
const GROUND: &str = "#f4efe3";
const INK: &str = "#1a1a1c";

/// FNV-1a with an avalanche finaliser.
///
/// The finaliser is not decoration: every choice below is a remainder of the
/// low bits, and plain FNV-1a leaves those poorly mixed. Written over UTF-16
/// code units rather than bytes because the TypeScript walks the string with
/// `charCodeAt`, and a Lineage id must hash identically in both languages.
fn hash(text: &str) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for unit in text.encode_utf16() {
        h ^= u32::from(unit);
        h = h.wrapping_mul(0x0100_0193);
    }
    h ^= h >> 16;
    h = h.wrapping_mul(0x7feb_352d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846c_a68b);
    h ^= h >> 16;
    h
}

/// One independent choice from the Lineage id: an index below `count`.
fn index(lineage_id: &str, aspect: &str, count: usize) -> usize {
    hash(&format!("{lineage_id}:{aspect}")) as usize % count
}

/// One independent choice from the Lineage id: a fraction in `[0, 1)`.
fn unit(lineage_id: &str, aspect: &str) -> f64 {
    f64::from(hash(&format!("{lineage_id}:{aspect}"))) / 4_294_967_296.0
}

/// Blend two hex colours. `t` of 0 is all `from`, 1 is all `to`.
fn blend(from: &str, to: &str, t: f64) -> String {
    let channel = |hex: &str, at: usize| -> f64 {
        f64::from(u8::from_str_radix(&hex[at..at + 2], 16).unwrap_or(0))
    };
    let mixed = |at: usize| -> String {
        let a = channel(from, at);
        let b = channel(to, at);
        format!("{:02x}", (a + (b - a) * t).round() as u8)
    };
    format!("#{}{}{}", mixed(1), mixed(3), mixed(5))
}

/// Everything needed to draw one Cover, and nothing that varies by screen.
pub struct Cover {
    pub dye: &'static Dye,
    /// The ground the whole Cover sits on.
    pub ground: &'static str,
    /// The shape's fill: the dye lifted towards kinari.
    pub tone: String,
    /// The hem band behind the title: the dye pushed towards sumi.
    pub band: String,
    pub shape: &'static PastaShape,
    /// The shape's size as a multiple of the Cover's height.
    pub scale: f64,
    /// Degrees, clockwise, about the shape's own centre.
    pub rotation: f64,
    /// The shape's centre, as fractions of the Cover's width and height.
    pub centre_x: f64,
    pub centre_y: f64,
}

/// The one function. Same Lineage id in, same Cover out, for ever.
pub fn cover_for(lineage_id: &str) -> Cover {
    let dye = &DYES[index(lineage_id, "dye", DYES.len())];
    Cover {
        dye,
        ground: dye.hex,
        tone: blend(dye.hex, GROUND, 0.19),
        band: blend(dye.hex, INK, 0.22),
        shape: &PASTA_SHAPES[index(lineage_id, "shape", PASTA_SHAPES.len())],
        scale: 0.66 + unit(lineage_id, "scale") * 0.36,
        rotation: unit(lineage_id, "rotation") * 360.0,
        centre_x: 0.34 + unit(lineage_id, "placeX") * 0.32,
        centre_y: 0.24 + unit(lineage_id, "placeY") * 0.2,
    }
}

impl Cover {
    /// The Cover as one block of HTML, sized entirely in CSS so the same markup
    /// serves a page hero and a card image without being told which it is.
    ///
    /// It carries no title: whatever is showing a Cover sets the recipe's title
    /// as real text over it, so this is decoration and is hidden from assistive
    /// technology rather than announced twice — exactly as `Cover.svelte` does.
    pub fn html(&self, height: &str) -> String {
        let paths: String = self
            .shape
            .body
            .iter()
            .map(|d| format!(r#"<path d="{d}" fill="{}"/>"#, self.tone))
            .chain(self.shape.detail.iter().map(|d| {
                format!(
                    r#"<path d="{d}" fill="{}" fill-opacity="0.5"/>"#,
                    self.ground
                )
            }))
            .collect();
        format!(
            r#"<div class="relative overflow-hidden" style="height:{height};background:{ground}" aria-hidden="true">
  <div class="absolute aspect-square -translate-x-1/2 -translate-y-1/2" style="left:{x:.2}%;top:{y:.2}%;height:{scale:.2}%">
    <svg width="100%" height="100%" viewBox="0 0 100 100" class="block">
      <g transform="rotate({rotation:.1} 50 50)">{paths}</g>
    </svg>
  </div>
</div>"#,
            ground = self.ground,
            x = self.centre_x * 100.0,
            y = self.centre_y * 100.0,
            scale = self.scale * 100.0,
            rotation = self.rotation,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixtures taken from the TypeScript `coverFor`, which is where #46's
    /// choice lives. If this fails, the two halves of one drawing have parted
    /// company and every photograph-less recipe now looks different depending
    /// on which one drew it.
    ///
    /// Regenerate with:
    ///   node --input-type=module -e "import {coverFor} from
    ///     './ui/src/lib/cover/cover.ts'; …"
    #[test]
    fn a_lineage_wears_the_same_face_the_typescript_gives_it() {
        // (lineage id, dye key, shape key, scale, rotation, centreX, centreY)
        let fixtures: &[(&str, &str, &str, f64, f64, f64, f64)] = &[
            (
                "l_663dbc0e0c771e1a",
                "ai",
                "conchiglie",
                0.857_992_589,
                316.866_892_073,
                0.495_633_609,
                0.353_459_566,
            ),
            (
                "l_1848cb7653cb9d93",
                "beni",
                "fusilli",
                0.846_889_982,
                337.891_862_355,
                0.507_407_451,
                0.398_279_739,
            ),
            (
                "l_da439448fd264658",
                "beni",
                "conchiglie",
                0.780_669_037,
                159.821_321_480,
                0.469_262_661,
                0.379_818_631,
            ),
        ];
        for (lineage, dye, shape, scale, rotation, x, y) in fixtures {
            let cover = cover_for(lineage);
            assert_eq!(cover.dye.key, *dye, "dye for {lineage}");
            assert_eq!(cover.shape.key, *shape, "shape for {lineage}");
            for (got, want, what) in [
                (cover.scale, *scale, "scale"),
                (cover.rotation, *rotation, "rotation"),
                (cover.centre_x, *x, "centreX"),
                (cover.centre_y, *y, "centreY"),
            ] {
                assert!(
                    (got - want).abs() < 1e-6,
                    "{what} for {lineage}: {got} is not {want}"
                );
            }
        }
    }

    #[test]
    fn a_dye_blends_the_way_the_typescript_blends_it() {
        // The values `cover.ts` produces for ai · indigo.
        let cover = cover_for("l_663dbc0e0c771e1a");
        assert_eq!(cover.ground, "#1d2b4c");
        assert_eq!(cover.tone, "#465069");
        assert_eq!(cover.band, "#1c2741");
    }

    /// Every dye and every shape must be reachable, or a Lineage could resolve
    /// to an index that is never drawn.
    #[test]
    fn the_two_lists_are_not_empty() {
        assert_eq!(DYES.len(), 8);
        assert_eq!(PASTA_SHAPES.len(), 8);
        for shape in PASTA_SHAPES {
            assert!(!shape.body.is_empty(), "{} has no body", shape.key);
        }
    }
}
