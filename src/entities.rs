//! HTML entities left inside text that was never HTML — the one repair every
//! importer makes to what it reads, and so made in one place (#69).
//!
//! Crouton scrapes a page and stores what it scraped without decoding it, so a
//! `.crumb` can hold `Noodles &amp; choi sum:`; a WordPress plugin that escaped
//! a field before writing its JSON-LD leaves the same thing on a web page
//! (#70). Two importers decoding the same text two ways would disagree about
//! the same recipe, which is why neither owns this.

/// Decode the five XML-predefined entities and numeric character references
/// — the only kinds a JSON string value plausibly carries, since it was
/// never HTML to begin with. A run with no `&` at all (the overwhelming
/// majority) costs one scan and no allocation beyond the final `String`.
pub fn decode_entities(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let tail = &rest[amp..];
        match decode_one_entity(tail) {
            Some((decoded, consumed)) => {
                out.push(decoded);
                rest = &tail[consumed..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// `text` starts with `&`. Decodes one entity and reports how many bytes of
/// `text` it consumed, or `None` if `text` doesn't start with a recognised
/// one — in which case the `&` is kept literally rather than guessed at.
fn decode_one_entity(text: &str) -> Option<(char, usize)> {
    let end = text.find(';').filter(|&end| end <= 10)?;
    let body = &text[1..end];
    let decoded = match body {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        _ if body.len() > 2 && (body.starts_with("#x") || body.starts_with("#X")) => {
            char::from_u32(u32::from_str_radix(&body[2..], 16).ok()?)?
        }
        _ if body.len() > 1 && body.starts_with('#') => char::from_u32(body[1..].parse().ok()?)?,
        _ => return None,
    };
    Some((decoded, end + 1))
}

#[cfg(test)]
mod tests {
    use super::decode_entities;

    #[test]
    fn named_and_numeric_entities_decode() {
        assert_eq!(
            decode_entities("Noodles &amp; choi sum:"),
            "Noodles & choi sum:"
        );
        assert_eq!(decode_entities("&#39;00&#39; &#x27;x&#X27;"), "'00' 'x'");
        assert_eq!(
            decode_entities("&lt;b&gt; &quot;q&quot; &apos;"),
            "<b> \"q\" '"
        );
    }

    #[test]
    fn a_bare_ampersand_is_kept_rather_than_guessed_at() {
        assert_eq!(decode_entities("salt & pepper"), "salt & pepper");
        assert_eq!(decode_entities("Truffle&Egg"), "Truffle&Egg");
        assert_eq!(decode_entities("&nbsp;"), "&nbsp;");
    }
}
