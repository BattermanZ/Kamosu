//! The few small helpers several areas of the Core call and no one area owns.

use super::*;

pub(super) fn is_sixteen_hex(hex: &str) -> bool {
    hex.len() == 16
        && hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// **When a write was made**, in the form every stored time in Kamosu takes
/// (#77, ADR 0013).
///
/// A phone with no signal keeps the writes it may make, the Attempt's and the
/// Shopping List's, and sends them once it can. Each then says when it was
/// really made, as `written_at`, so an Attempt started at your parents' on
/// Saturday is dated Saturday, and a change made yesterday cannot undo one
/// made this morning on the iPad. Absent is now, which is every write that
/// reached the server when it was made.
///
/// The phone's clock is trusted to say *when*, since it is the cook's own
/// record, but never to say *later than now*: a clock running fast would
/// otherwise win every argument against every other device until the real
/// time caught up.
pub(super) fn written_moment(
    conn: &Connection,
    written_at: Option<&str>,
) -> Result<String, OpError> {
    let (moment, now): (Option<String>, String) = conn
        .query_row(
            "SELECT strftime('%Y-%m-%dT%H:%M:%fZ', ?1), strftime('%Y-%m-%dT%H:%M:%fZ','now')",
            params![written_at.unwrap_or("now")],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| OpError::internal(format!("cannot read the time: {e}")))?;
    let moment = moment.ok_or_else(|| {
        OpError::bad_request("written_at must be a time, such as 2026-09-19T14:05:00.000Z")
    })?;
    Ok(if moment > now { now } else { moment })
}

/// A Branch id that names nothing here — and, by ADR 0040, a Branch held by a
/// Kitchen the caller does not cook in.
pub(super) fn no_such_branch() -> OpError {
    OpError::not_found("no such Branch")
}

/// One word reduced to the form every spelling of it shares, so a Kitchen can
/// hold one word once. This is Unicode canonical caseless matching — decompose,
/// case-fold, decompose again — written out as a value rather than a comparison
/// so the fold can be stored and indexed.
///
/// It settles both ways one word arrives looking like two. Case: *Été* and
/// *été* are one word, which SQLite's own NOCASE cannot say, folding ASCII
/// alone. And shape: an *é* typed as one character and an *e* followed by a
/// combining accent look identical on screen and are different bytes — a French
/// cookbook meets both, depending on the keyboard.
///
/// What it does not fold is what the Unicode default fold leaves alone: *straße*
/// and *strasse* stay two words. That is the standard's own line, not one drawn
/// here.
pub(super) fn folded_word(name: &str) -> String {
    use caseless::Caseless;
    use unicode_normalization::UnicodeNormalization;
    name.chars().nfd().default_case_fold().nfd().collect()
}

pub(crate) fn random_bytes(n: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; n];
    SysRng
        .try_fill_bytes(&mut bytes)
        .expect("the operating system random source is unavailable");
    bytes
}

pub(super) fn required_text<'a>(value: &'a str, field: &str) -> Result<&'a str, OpError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(OpError::bad_request(format!("{field} is required")));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::folded_word;

    /// The fold is what holds one word to one Tag, so it is checked directly
    /// rather than only through the Operations that lean on it.
    #[test]
    fn one_word_folds_to_one_form_however_it_was_typed() {
        // Case, ASCII and beyond it.
        assert_eq!(folded_word("Dessert"), folded_word("dessert"));
        assert_eq!(folded_word("ÉTÉ"), folded_word("été"));
        assert_eq!(folded_word("RÁPIDO"), folded_word("rápido"));

        // Shape: one character, or a letter and a combining accent.
        assert_eq!(folded_word("crème"), folded_word("cre\u{0300}me"));
        assert_eq!(folded_word("Crème"), folded_word("CRE\u{0300}ME"));

        // And words that really are different stay different.
        assert_ne!(folded_word("dessert"), folded_word("desert"));
        assert_ne!(folded_word("été"), folded_word("ete"));
    }
}
