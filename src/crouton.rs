//! Reading a Crouton export (#69): the whole library, a zip of `.crumb` files,
//! turned into candidates the shared `import` machinery lands (ADR 0025).
//!
//! **This is the parser and nothing else.** Matching against the ledger,
//! making Lineages and writing the Import Report all belong to
//! [`crate::core::Core::import_each`]; what lives here is what a `.crumb`
//! means. `docs/research/crouton-real-export.md` measured the format against
//! a real library of 86 recipes, and every rule below comes from that measurement.
//!
//! - **The Crouton UUID is the foreign id, never the filename.** Crouton names
//!   a collision `Beef Bourguignon-1.crumb`, so a filename says nothing about
//!   which recipe a file is.
//! - **An Ingredient Line is rebuilt from Crouton's split fields, unmarked.**
//!   Crouton kept `{2, ITEM}` and a food named `cloves minced garlic` and threw
//!   the written line away; Kamosu writes `2 cloves minced garlic` and that is
//!   the line from then on (ADR 0025 declines the mark on purpose).
//! - **`sourceImage` is dropped.** It is the source site's favicon, not a
//!   photograph of the dish (ADR 0017); the Report says it was left out.
//! - **`neutritionalInfo` is dropped.** It is unparsed scrape debris, and a
//!   plausible wrong number is worse than an empty field (ADR 0025).
//! - **A tag is kept by its name alone** (#128). The 20 August 2026 export
//!   carried no tags; the 23 September one tags 64 of the 86 recipes. Each is
//!   `{uuid, name, color}`, and only the name comes in: Kamosu Tags have no
//!   colour, and a Kitchen's own list is keyed on the word, not Crouton's id.
//!   The names are handed over beside the candidate, never inside it, because
//!   a Tag is filing, not content (ADR 0035).
//! - **`defaultScale` and `folderIDs` are dropped**: Kamosu keeps no stored
//!   scaling factor, and no export yet has populated a folder.
//! - **Text is decoded** of the HTML entities Crouton scraped and kept.

use std::io::{Read, Seek};

use serde_json::{Value, json};

use crate::core::OpError;
use crate::entities::decode_entities;

/// The largest single `.crumb` read out of a zip. The corpus's largest is
/// 8.5 MB — one recipe with a camera-original photo inline — so this leaves
/// room for a library with bigger pictures while refusing to inflate an entry
/// that claims to be gigabytes.
const MAX_CRUMB_BYTES: u64 = 128 * 1024 * 1024;

/// One recipe read out of a `.crumb`: the candidate `import` lands, minus the
/// Main Photo, which the caller stores first because storing is Core's work.
#[derive(Debug)]
pub struct Crumb {
    /// The Crouton UUID — the ledger's key.
    pub foreign_id: String,
    pub title: String,
    /// Recipe content in the shape `create_recipe` accepts, `main_photo` absent.
    pub candidate: Value,
    /// Every picture in `images`, decoded from base64, in Crouton's order.
    /// The first becomes the Main Photo; any after it is left out and said so.
    pub photos: Vec<Vec<u8>>,
    /// Pictures in `images` that were not valid base64 at all.
    pub unreadable_photos: usize,
    /// The recipe's `sourceImage` — the source site's favicon, never stored as
    /// a Photograph, handed over only so the Report can show what was left out.
    pub site_icon: Option<Vec<u8>>,
    /// The names of the tags Crouton filed the recipe under, trimmed, each
    /// once, in Crouton's order. They are English words whatever the recipe's
    /// Language, and are filed as such (#128).
    pub tags: Vec<String>,
}

/// A readable Crouton export: one `.crumb`, or a zip of them. Entries are read
/// one at a time, so a library is never held in memory decoded all at once.
pub struct Export {
    entries: Entries,
}

enum Entries {
    Zip {
        archive: Box<zip::ZipArchive<Box<dyn ReadSeek>>>,
        /// The archive indices that hold a `.crumb`, in filename order.
        crumbs: Vec<usize>,
    },
    One(Option<Vec<u8>>),
}

trait ReadSeek: Read + Seek + Send {}
impl<T: Read + Seek + Send> ReadSeek for T {}

impl Export {
    /// Open what a person handed over. A zip is read as Crouton's whole-library
    /// export; anything else is taken to be a single `.crumb`, which is what
    /// Crouton shares one recipe as.
    pub fn open<R: Read + Seek + Send + 'static>(mut reader: R) -> Result<Export, OpError> {
        let mut magic = [0u8; 4];
        let read = read_up_to(&mut reader, &mut magic)
            .map_err(|e| OpError::bad_request(format!("cannot read the file: {e}")))?;
        reader
            .rewind()
            .map_err(|e| OpError::bad_request(format!("cannot read the file: {e}")))?;

        if read == 4 && magic == *b"PK\x03\x04" {
            let archive =
                zip::ZipArchive::new(Box::new(reader) as Box<dyn ReadSeek>).map_err(|_| {
                    OpError::bad_request(
                        "this zip could not be opened; export the library from Crouton again",
                    )
                })?;
            let mut crumbs: Vec<(String, usize)> = (0..archive.len())
                .filter_map(|index| {
                    let name = archive.name_for_index(index)?;
                    let lower = name.to_lowercase();
                    let file = lower.rsplit('/').next().unwrap_or(&lower);
                    // macOS zips carry `__MACOSX/._name.crumb` resource forks
                    // beside the real files; they are not recipes.
                    (lower.ends_with(".crumb") && !file.starts_with("._"))
                        .then(|| (name.to_string(), index))
                })
                .collect();
            if crumbs.is_empty() {
                return Err(OpError::bad_request(
                    "this zip holds no .crumb files, so it is not a Crouton export",
                ));
            }
            crumbs.sort();
            Ok(Export {
                entries: Entries::Zip {
                    archive: Box::new(archive),
                    crumbs: crumbs.into_iter().map(|(_, index)| index).collect(),
                },
            })
        } else {
            let mut bytes = Vec::new();
            reader
                .take(MAX_CRUMB_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| OpError::bad_request(format!("cannot read the file: {e}")))?;
            Ok(Export {
                entries: Entries::One(Some(bytes)),
            })
        }
    }

    /// How many recipes this export offers — the Job's total.
    pub fn len(&self) -> usize {
        match &self.entries {
            Entries::Zip { crumbs, .. } => crumbs.len(),
            Entries::One(_) => 1,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Read the `index`th recipe: the file's own name, for a Report that has
    /// to name a file it could not read, and what was in it.
    pub fn read(&mut self, index: usize) -> (Option<String>, Result<Crumb, String>) {
        match &mut self.entries {
            Entries::One(bytes) => {
                let bytes = bytes.take().unwrap_or_default();
                (None, read_bytes(&bytes))
            }
            Entries::Zip { archive, crumbs } => {
                let Some(&at) = crumbs.get(index) else {
                    return (None, Err("there is no such file in the export".to_string()));
                };
                let mut entry = match archive.by_index(at) {
                    Ok(entry) => entry,
                    Err(_) => return (None, Err(DAMAGED.to_string())),
                };
                let name = file_name(&entry);
                let mut bytes = Vec::new();
                if (&mut entry)
                    .take(MAX_CRUMB_BYTES + 1)
                    .read_to_end(&mut bytes)
                    .is_err()
                {
                    return (Some(name), Err(DAMAGED.to_string()));
                }
                (Some(name), read_bytes(&bytes))
            }
        }
    }
}

const DAMAGED: &str = "the file is damaged and none of it could be read; export the \
                       library from Crouton again and import that";

fn read_up_to(reader: &mut impl Read, buffer: &mut [u8]) -> std::io::Result<usize> {
    let mut filled = 0;
    while filled < buffer.len() {
        match reader.read(&mut buffer[filled..])? {
            0 => break,
            n => filled += n,
        }
    }
    Ok(filled)
}

/// The last path segment of an entry's name. A zip made on a Mac stores its
/// names as UTF-8 without saying so, so the raw bytes are tried as UTF-8 first
/// — otherwise `Îles Flottantes` reads back as `I╠Вles Flottantes`.
fn file_name(entry: &zip::read::ZipFile<'_, Box<dyn ReadSeek>>) -> String {
    let name = std::str::from_utf8(entry.name_raw())
        .map(str::to_string)
        .unwrap_or_else(|_| entry.name().to_string());
    name.rsplit('/').next().unwrap_or(&name).to_string()
}

fn read_bytes(bytes: &[u8]) -> Result<Crumb, String> {
    if bytes.len() as u64 > MAX_CRUMB_BYTES {
        return Err("the file is too large to be one recipe".to_string());
    }
    let crumb: Value = serde_json::from_slice(bytes).map_err(|_| {
        "the file is not a Crouton recipe Kamosu can read: it may be damaged, so \
         export it from Crouton again and import that"
            .to_string()
    })?;
    read_crumb(&crumb)
}

/// One parsed `.crumb`, as Kamosu's recipe content. The error is a sentence
/// for the Report — what went wrong, and what to do about it.
pub fn read_crumb(crumb: &Value) -> Result<Crumb, String> {
    if !crumb.is_object() {
        return Err("the file is not a Crouton recipe".to_string());
    }
    let foreign_id = text(crumb, "uuid").ok_or_else(|| {
        "the recipe carries no Crouton id, so Kamosu could not tell whether it had \
         brought it in before"
            .to_string()
    })?;
    let title = text(crumb, "name").unwrap_or_else(|| "Untitled recipe".to_string());

    let ingredients = ingredient_lines(crumb.get("ingredients"));
    let steps = step_list(crumb.get("steps"));
    let source = source(crumb);
    let note = text(crumb, "notes");

    let mut candidate = json!({
        "title": title,
        "prep_time_minutes": minutes(crumb.get("duration")),
        "cook_time_minutes": minutes(crumb.get("cookingDuration")),
        "note": note,
        "source": source,
        "nutrition": Value::Null,
        "ingredients": ingredients,
        "steps": steps,
    });
    // The Yield's noun is written in the recipe's own Language, which only the
    // rest of the recipe can say: Crouton stores a bare number.
    candidate["yield"] = match crumb.get("serves").and_then(Value::as_u64) {
        Some(serves) if serves > 0 => {
            let noun = match crate::language::detect_content(&candidate) {
                Some("fr") => "portions",
                Some("es") => "raciones",
                _ => "servings",
            };
            json!({ "amount": serves.to_string(), "noun": noun })
        }
        _ => Value::Null,
    };

    let mut photos = Vec::new();
    let mut unreadable_photos = 0;
    for image in crumb
        .get("images")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        use base64::Engine;
        match image
            .as_str()
            .and_then(|b64| base64::engine::general_purpose::STANDARD.decode(b64).ok())
        {
            Some(bytes) if !bytes.is_empty() => photos.push(bytes),
            _ => unreadable_photos += 1,
        }
    }
    let site_icon = crumb
        .get("sourceImage")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(|b64| {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD
                .decode(b64)
                .unwrap_or_default()
        });

    let mut tags: Vec<String> = Vec::new();
    for tag in crumb
        .get("tags")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(name) = text(tag, "name")
            && !tags.contains(&name)
        {
            tags.push(name);
        }
    }

    Ok(Crumb {
        foreign_id,
        title,
        candidate,
        photos,
        unreadable_photos,
        site_icon,
        tags,
    })
}

/// A trimmed, decoded, non-empty text field.
fn text(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(|s| decode_entities(s.trim()))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Crouton's `duration` and `cookingDuration` are whole minutes. Zero is how it
/// stores a time nobody filled in, so it is no time at all rather than a claim
/// the dish takes none.
fn minutes(value: Option<&Value>) -> Value {
    match value.and_then(Value::as_u64) {
        Some(minutes) if minutes > 0 => json!(minutes),
        _ => Value::Null,
    }
}

/// A Source: the site's own name with its link, the link's host where Crouton
/// kept only the link, or nothing where it kept neither.
fn source(crumb: &Value) -> Value {
    let link = text(crumb, "webLink");
    let name = text(crumb, "sourceName").or_else(|| link.as_deref().and_then(host));
    match name {
        Some(name) => json!({ "text": name, "link": link }),
        None => Value::Null,
    }
}

fn host(link: &str) -> Option<String> {
    let rest = link.split_once("://").map_or(link, |(_, rest)| rest);
    let host = rest.split(['/', '?', '#']).next()?;
    let host = host.strip_prefix("www.").unwrap_or(host);
    (!host.is_empty()).then(|| host.to_string())
}

/// Items of a Crouton list, in the order Crouton says rather than the order
/// the JSON happens to hold them.
fn ordered(list: Option<&Value>) -> Vec<&Value> {
    let mut items: Vec<&Value> = list
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default();
    items.sort_by_key(|item| {
        item.get("order")
            .and_then(Value::as_i64)
            .unwrap_or(i64::MAX)
    });
    items
}

/// Ingredient rows as Kamosu's list: a `SECTION` row is a real Section, and
/// every other row an Ingredient Line rebuilt from its parts.
fn ingredient_lines(list: Option<&Value>) -> Vec<Value> {
    ordered(list)
        .into_iter()
        .filter_map(|row| {
            let name = row
                .get("ingredient")
                .and_then(|ingredient| text(ingredient, "name"))
                .unwrap_or_default();
            let quantity = row.get("quantity").filter(|q| q.is_object());
            let kind = quantity
                .and_then(|q| q.get("quantityType"))
                .and_then(Value::as_str);
            if kind == Some("SECTION") {
                return (!name.is_empty()).then(|| json!({ "kind": "section", "text": name }));
            }
            let line = rebuild_line(quantity, &name);
            (!line.is_empty()).then(|| json!({ "kind": "ingredient", "text": line }))
        })
        .collect()
}

/// Write the line Crouton did not keep: amount, Unit, then the food as Crouton
/// named it — `2 cloves minced garlic`, `½ cup soy sauce`, `380-400 ml
/// vegetable stock/water`. The food name is kept exactly as Crouton had it,
/// debris included; an odd line is obvious at a glance and fixed when seen
/// (ADR 0025), which is not true of one Kamosu quietly rewrote.
pub fn rebuild_line(quantity: Option<&Value>, name: &str) -> String {
    let amount = quantity
        .and_then(|q| q.get("amount"))
        .and_then(Value::as_f64)
        .filter(|a| a.is_finite() && *a > 0.0);
    let upper = quantity
        .and_then(|q| q.get("secondaryAmount"))
        .and_then(Value::as_f64)
        .filter(|a| a.is_finite() && *a > 0.0);
    let kind = quantity
        .and_then(|q| q.get("quantityType"))
        .and_then(Value::as_str);

    let metric = matches!(kind, Some("GRAMS" | "KGS" | "MILLS" | "LITRES"));
    let mut parts: Vec<String> = Vec::new();
    if let Some(amount) = amount {
        parts.push(match upper {
            Some(upper) if upper > amount => format!(
                "{}-{}",
                amount_text(amount, metric),
                amount_text(upper, metric)
            ),
            _ => amount_text(amount, metric),
        });
        let many = upper.unwrap_or(amount) > 1.0;
        if let Some(unit) = kind.and_then(|kind| unit_word(kind, many)) {
            parts.push(unit.to_string());
        }
    }
    if !name.is_empty() {
        parts.push(name.to_string());
    }
    parts.join(" ")
}

/// Crouton's closed unit enum as words Kamosu reads back as the same Unit
/// (ADR 0016's mapping; cups and spoons are the US measures, which is what
/// `units.rs` means by them). `ITEM` has no word: Crouton put the noun inside
/// the food name (`cloves minced garlic`), so the line needs none.
fn unit_word(kind: &str, many: bool) -> Option<&'static str> {
    Some(match (kind, many) {
        ("CUP", false) => "cup",
        ("CUP", true) => "cups",
        ("TABLESPOON", _) => "tbsp",
        ("TEASPOON", _) => "tsp",
        ("GRAMS", _) => "g",
        ("KGS", _) => "kg",
        ("MILLS", _) => "ml",
        ("LITRES", _) => "l",
        ("OUNCE", _) => "oz",
        ("POUND", _) => "lb",
        ("CAN", false) => "can",
        ("CAN", true) => "cans",
        ("BOTTLE", false) => "bottle",
        ("BOTTLE", true) => "bottles",
        ("PACKET", false) => "packet",
        ("PACKET", true) => "packets",
        ("PINCH", false) => "pinch",
        ("PINCH", true) => "pinches",
        _ => return None,
    })
}

/// A number as a cook writes it, by the rule `units.rs` writes one by: metric
/// as a decimal (`1.4 kg`), anything else as the fraction on the cup (`⅓ cup`,
/// `1½` eggs). Crouton stores `0.3333333333333333`; nobody wrote that.
fn amount_text(amount: f64, metric: bool) -> String {
    if metric {
        crate::units::as_decimal(amount)
    } else {
        crate::units::as_fraction(amount)
    }
}

/// Steps as Kamosu's list: `isSection` rows become real Sections. Crouton
/// steps carry text only — no photo, in all 599 rows of the corpus.
fn step_list(list: Option<&Value>) -> Vec<Value> {
    ordered(list)
        .into_iter()
        .filter_map(|row| {
            let text = text(row, "step")?;
            let kind = if row.get("isSection").and_then(Value::as_bool) == Some(true) {
                "section"
            } else {
                "step"
            };
            Some(json!({ "kind": kind, "text": text, "photo": null }))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(amount: f64, kind: &str) -> Value {
        json!({ "amount": amount, "quantityType": kind })
    }

    #[test]
    fn a_line_is_rebuilt_from_its_parts_as_a_cook_would_write_it() {
        assert_eq!(
            rebuild_line(Some(&q(2.0, "ITEM")), "cloves minced garlic"),
            "2 cloves minced garlic"
        );
        assert_eq!(
            rebuild_line(Some(&q(16.0, "OUNCE")), "block soft or silken tofu"),
            "16 oz block soft or silken tofu"
        );
        assert_eq!(
            rebuild_line(Some(&q(0.25, "CUP")), "soy sauce"),
            "¼ cup soy sauce"
        );
        assert_eq!(rebuild_line(Some(&q(1.5, "CUP")), "milk"), "1½ cups milk");
        assert_eq!(
            rebuild_line(Some(&q(1.0 / 3.0, "TEASPOON")), "salt"),
            "⅓ tsp salt"
        );
        assert_eq!(
            rebuild_line(Some(&q(1.4, "KGS")), "whole chicken"),
            "1.4 kg whole chicken"
        );
        assert_eq!(
            rebuild_line(Some(&q(1.0, "BOTTLE")), "red wine"),
            "1 bottle red wine"
        );
        assert_eq!(
            rebuild_line(Some(&q(2.0, "CAN")), "Tin tomatoes"),
            "2 cans Tin tomatoes"
        );
    }

    #[test]
    fn a_line_with_no_quantity_is_the_food_alone() {
        assert_eq!(
            rebuild_line(None, "to taste Freshly ground black pepper"),
            "to taste Freshly ground black pepper"
        );
    }

    #[test]
    fn a_range_keeps_both_ends_and_counts_as_many() {
        let range = json!({ "amount": 0.5, "secondaryAmount": 1, "quantityType": "CUP" });
        assert_eq!(
            rebuild_line(Some(&range), "cheese shredded"),
            "½-1 cup cheese shredded"
        );
        let range = json!({ "amount": 380, "secondaryAmount": 400, "quantityType": "MILLS" });
        assert_eq!(
            rebuild_line(Some(&range), "vegetable stock/water"),
            "380-400 ml vegetable stock/water"
        );
        let range = json!({ "amount": 2, "secondaryAmount": 3, "quantityType": "CUP" });
        assert_eq!(rebuild_line(Some(&range), "Sugar"), "2-3 cups Sugar");
    }

    #[test]
    fn a_rebuilt_line_reads_back_as_the_amount_and_unit_it_was_built_from() {
        for (quantity, name, amount, unit) in [
            (q(0.5, "CUP"), "honey", 0.5, "cup"),
            (q(1.25, "TABLESPOON"), "sugar", 1.25, "tbsp"),
            (q(200.0, "GRAMS"), "plain flour", 200.0, "g"),
            (q(2.0 / 3.0, "CUP"), "flour", 2.0 / 3.0, "cup"),
            (q(1.0, "LITRES"), "milk", 1.0, "l"),
        ] {
            let line = rebuild_line(Some(&quantity), name);
            let reading = crate::reading::read_line(&line).expect("a reading");
            let read = crate::units::parse_amount(reading.amount.as_deref().unwrap()).unwrap();
            assert!((read - amount).abs() < 1e-9, "{line}: read {read}");
            assert_eq!(reading.unit.as_deref(), Some(unit), "{line}");
            assert_eq!(
                crate::units::recognise(unit).map(|u| u.id),
                crate::units::recognise(reading.unit.as_deref().unwrap()).map(|u| u.id)
            );
        }
    }

    fn crumb() -> Value {
        json!({
            "uuid": "89D0E4FE-0000",
            "name": "Dan Dan Noodles",
            "serves": 2,
            "duration": 10,
            "cookingDuration": 0,
            "sourceName": "recipetineats.com",
            "webLink": "https://www.recipetineats.com/dan-dan-noodles/",
            "sourceImage": "/9j/AAAA",
            "neutritionalInfo": "Calories: 610 kcal",
            "defaultScale": 1,
            "tags": [
                { "uuid": "A9179A12-0000", "name": "Vegan", "color": "#FFCC00" },
                { "uuid": "B0000000-0000", "name": " Weeknight dinner ", "color": "#00FF00" },
                { "uuid": "C0000000-0000", "name": "  ", "color": "#000000" },
                { "uuid": "A9179A12-0000", "name": "Vegan", "color": "#FFCC00" },
            ],
            "images": ["aGVsbG8=", "d29ybGQ=", "not base64!"],
            "ingredients": [
                { "order": 2, "ingredient": { "name": "Sauce" }, "quantity": { "quantityType": "SECTION" } },
                { "order": 1, "ingredient": { "name": "salt" } },
                { "order": 0, "ingredient": { "name": "noodles" }, "quantity": { "amount": 200, "quantityType": "GRAMS" } },
                { "order": 3, "ingredient": { "name": "  " } },
            ],
            "steps": [
                { "order": 1, "isSection": false, "step": "Boil the noodles.\n" },
                { "order": 0, "isSection": true, "step": "Noodles &amp; choi sum:" },
            ],
        })
    }

    #[test]
    fn a_crumb_reads_as_ordinary_recipe_content() {
        let crumb = read_crumb(&crumb()).expect("readable");
        assert_eq!(crumb.foreign_id, "89D0E4FE-0000");
        assert_eq!(crumb.title, "Dan Dan Noodles");
        let c = &crumb.candidate;
        assert_eq!(c["yield"], json!({ "amount": "2", "noun": "servings" }));
        assert_eq!(c["prep_time_minutes"], json!(10));
        assert_eq!(
            c["cook_time_minutes"],
            Value::Null,
            "zero is no time filled in"
        );
        assert_eq!(c["nutrition"], Value::Null, "Crouton's blob is dropped");
        assert_eq!(
            c["source"],
            json!({ "text": "recipetineats.com", "link": "https://www.recipetineats.com/dan-dan-noodles/" })
        );
        assert_eq!(
            c["ingredients"],
            json!([
                { "kind": "ingredient", "text": "200 g noodles" },
                { "kind": "ingredient", "text": "salt" },
                { "kind": "section", "text": "Sauce" },
            ])
        );
        assert_eq!(
            c["steps"],
            json!([
                { "kind": "section", "text": "Noodles & choi sum:", "photo": null },
                { "kind": "step", "text": "Boil the noodles.", "photo": null },
            ])
        );
        assert!(c.get("main_photo").is_none());
    }

    #[test]
    fn the_site_icon_is_dropped_and_every_picture_accounted_for() {
        let crumb = read_crumb(&crumb()).expect("readable");
        // "/9j/AAAA" is the start of a JPEG: handed over to be shown, never stored.
        assert_eq!(
            crumb.site_icon.as_deref(),
            Some(&[0xff, 0xd8, 0xff, 0, 0, 0][..])
        );
        assert_eq!(crumb.photos, vec![b"hello".to_vec(), b"world".to_vec()]);
        assert_eq!(crumb.unreadable_photos, 1);
    }

    #[test]
    fn a_tag_is_kept_by_its_name_alone() {
        let crumb = read_crumb(&crumb()).expect("readable");
        // The colour and Crouton's uuid go; a blank name is no tag; a name
        // written twice is one tag.
        assert_eq!(crumb.tags, vec!["Vegan", "Weeknight dinner"]);
        assert!(
            crumb.candidate.get("tags").is_none(),
            "tags are not content"
        );
        assert_eq!(
            read_crumb(&json!({ "uuid": "x", "name": "y" }))
                .unwrap()
                .tags,
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_link_alone_names_its_host_as_the_source() {
        let mut bare = json!({ "uuid": "x", "name": "Gochujang Pasta",
            "webLink": "https://youtube.com/shorts/E9omFgkaCTA?si=IUN" });
        let crumb = read_crumb(&bare).unwrap();
        assert_eq!(
            crumb.candidate["source"],
            json!({ "text": "youtube.com", "link": "https://youtube.com/shorts/E9omFgkaCTA?si=IUN" })
        );
        bare["webLink"] = json!("https://www.marionskitchen.com/creamy/");
        assert_eq!(
            read_crumb(&bare).unwrap().candidate["source"]["text"],
            "marionskitchen.com"
        );
        assert_eq!(
            read_crumb(&json!({ "uuid": "x", "name": "y" }))
                .unwrap()
                .candidate["source"],
            Value::Null
        );
    }

    #[test]
    fn a_french_recipe_counts_portions() {
        let crumb = read_crumb(&json!({
            "uuid": "x", "name": "Purée de pommes de terre", "serves": 4,
            "ingredients": [{ "order": 0, "ingredient": { "name": "pommes de terre et du beurre pour la purée" } }],
            "steps": [{ "order": 0, "isSection": false, "step": "Faire cuire les pommes de terre dans l'eau salée pendant vingt minutes, puis les écraser avec le beurre et le lait." }],
        }))
        .unwrap();
        assert_eq!(crumb.candidate["yield"]["noun"], "portions");
    }

    #[test]
    fn a_crumb_without_its_crouton_id_cannot_be_matched_and_says_so() {
        let error = read_crumb(&json!({ "name": "No id" })).unwrap_err();
        assert!(error.contains("no Crouton id"), "{error}");
        assert!(read_bytes(b"{ \"uuid\": ").unwrap_err().contains("damaged"));
    }

    #[test]
    fn an_export_is_a_zip_of_crumbs_read_in_filename_order() {
        use std::io::Write;
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buffer);
            let options = zip::write::SimpleFileOptions::default();
            zip.start_file("b.crumb", options).unwrap();
            zip.write_all(br#"{"uuid":"B","name":"Bee"}"#).unwrap();
            zip.start_file("__MACOSX/._b.crumb", options).unwrap();
            zip.write_all(b"junk").unwrap();
            zip.start_file("a.crumb", options).unwrap();
            zip.write_all(b"not json").unwrap();
            zip.start_file("readme.txt", options).unwrap();
            zip.write_all(b"hi").unwrap();
            zip.finish().unwrap();
        }
        buffer.set_position(0);
        let mut export = Export::open(buffer).expect("opens");
        assert_eq!(export.len(), 2);
        let (name, first) = export.read(0);
        assert_eq!(name.as_deref(), Some("a.crumb"));
        assert!(first.is_err());
        let (name, second) = export.read(1);
        assert_eq!(name.as_deref(), Some("b.crumb"));
        assert_eq!(second.unwrap().title, "Bee");
    }

    #[test]
    fn a_single_crumb_is_an_export_of_one() {
        let mut export = Export::open(std::io::Cursor::new(
            br#"{"uuid":"A","name":"Ay"}"#.to_vec(),
        ))
        .unwrap();
        assert_eq!(export.len(), 1);
        assert_eq!(export.read(0).1.unwrap().foreign_id, "A");
    }

    #[test]
    fn a_zip_with_no_crumbs_is_not_a_crouton_export() {
        use std::io::Write;
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buffer);
            zip.start_file("photo.jpg", zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"x").unwrap();
            zip.finish().unwrap();
        }
        buffer.set_position(0);
        let error = Export::open(buffer).err().expect("refused");
        assert!(error.to_sentence().contains("not a Crouton export"));
    }
}
