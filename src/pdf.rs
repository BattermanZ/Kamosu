//! Reading the text out of a recipe PDF (#176).
//!
//! **Only the text.** A PDF that a browser or a word processor printed holds
//! real text, and what comes out of it goes to the paste reader
//! ([`crate::pasting::read_printed`]) like anything pasted. A scan or a
//! photograph of a page holds a picture of text and no text at all, and is
//! refused with a sentence saying so. Reading a picture (OCR) is a separate
//! and much larger piece of work, and none of it is here.
//!
//! **The file is a stranger's.** `pdf-extract` answers most damaged files with
//! an error, and panics on some others. A panic is caught here and becomes
//! the same refusal as the error, so a bad file costs its sender one
//! sentence and nobody else anything. The size bound keeps it to a recipe's
//! worth of work.

use crate::core::OpError;

/// Past this many bytes a file is not one recipe. A two-page recipe printed
/// from a web page is 41 KB; a cookbook's worth of photographs is not what
/// this reads.
pub const LARGEST_PDF: u64 = 20 * 1024 * 1024;

/// The `reason` a PDF with no text is refused with, which the interface says
/// in its own words and three Languages (#176).
pub const NO_TEXT: &str = "pdf_has_no_text";

/// The refusal a scanned PDF gets. The wording was chosen on #176: it
/// says why, and what to do instead.
const NO_TEXT_SENTENCE: &str = "This PDF is a scan or a photo of a page, so it has no text to \
                                read. If it came from a website, use From a link instead, or copy \
                                the recipe's text and use From pasted text.";

/// **The text a PDF holds, or why it holds none.**
pub fn text_of(bytes: &[u8]) -> Result<String, OpError> {
    // A PDF says so in its first kilobyte, where the format allows some
    // leading junk before the marker. Anything else is refused before the
    // parser sees it.
    let head = &bytes[..bytes.len().min(1024)];
    if !head.windows(5).any(|window| window == b"%PDF-") {
        return Err(OpError::bad_request("that file is not a PDF"));
    }

    let read = std::panic::catch_unwind(|| pdf_extract::extract_text_from_mem(bytes));
    let text = match read {
        Ok(Ok(text)) => text,
        Ok(Err(error)) => {
            return Err(OpError::bad_request(format!(
                "that PDF could not be read: {error}"
            )));
        }
        Err(_) => {
            return Err(OpError::bad_request(
                "that PDF could not be read: it looks damaged",
            ));
        }
    };
    if text.trim().is_empty() {
        return Err(no_text());
    }
    Ok(text)
}

/// The refusal for a PDF with no text to read: a scan, a photograph, or a
/// page whose only marks are list bullets.
pub fn no_text() -> OpError {
    OpError::refused_because(NO_TEXT, NO_TEXT_SENTENCE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_that_is_not_a_pdf_is_refused_before_it_is_read() {
        let refused = text_of(b"PK\x03\x04 a zip file").unwrap_err();
        assert_eq!(refused.message, "that file is not a PDF");
        assert_eq!(refused.reason, None);
    }

    #[test]
    fn a_damaged_pdf_is_refused_rather_than_taking_anything_down() {
        let refused = text_of(b"%PDF-1.4\n1 0 obj << /Type /Catalog >>\n%%EOF").unwrap_err();
        assert!(refused.message.starts_with("that PDF could not be read"));
    }
}
