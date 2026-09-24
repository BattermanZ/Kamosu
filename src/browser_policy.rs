//! What every answer tells the browser the page may do (#139).
//!
//! Three headers, stamped on every response by one layer over the whole app,
//! so no route can be added without them. ADR 0033 holds that Kamosu on its
//! bare port is as safe as Kamosu behind a proxy, which puts these in Kamosu
//! rather than in a proxy config each Operator would have to write.
//!
//! Each is the second line behind a defence that already holds: the
//! Content-Security-Policy behind "the app never inserts raw HTML", the frame
//! ban behind the `SameSite` Session cookie, `nosniff` behind re-encoded
//! Photographs, and the Referrer-Policy behind the browser's own default,
//! which already keeps a Share Link's Secret off other sites.

use std::sync::OnceLock;

use axum::http::{HeaderValue, header};
use axum::response::Response;

use crate::interface;

/// The layer's whole work: the same three headers on whatever was answered.
pub async fn stamp(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_SECURITY_POLICY, content_security_policy());
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    // A link followed from a Share Link, an Invite or a recovery page carries
    // nothing of the address to another site, the Secret in its path included.
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("same-origin"),
    );
    response
}

/// One policy for every answer. It names the shell's inline bootstrap by hash
/// rather than allowing inline script at all, so a script that got into a page
/// any other way does not run.
///
/// A release build embeds the shell, so its hash is worked out once. A debug
/// build reads `ui/build` from disk and `just ui-build` can replace it under a
/// running server, so there it is worked out on every answer.
fn content_security_policy() -> HeaderValue {
    static POLICY: OnceLock<HeaderValue> = OnceLock::new();
    if cfg!(debug_assertions) {
        policy_for(&interface::shell_script_hashes())
    } else {
        POLICY
            .get_or_init(|| policy_for(&interface::shell_script_hashes()))
            .clone()
    }
}

fn policy_for(script_hashes: &[String]) -> HeaderValue {
    let mut scripts = String::from("'self'");
    for hash in script_hashes {
        scripts.push(' ');
        scripts.push_str(hash);
    }
    let policy = [
        "default-src 'self'".to_string(),
        format!("script-src {scripts}"),
        // Inline style is allowed, and taken on purpose: the shell wraps the
        // app in `<div style="display: contents">`, a dozen Svelte templates
        // set `style=` (the Cover, the tiles, the hero), and so does the Share
        // Link page's Cover. A style cannot run code; script is what this
        // policy is for.
        "style-src 'self' 'unsafe-inline'".to_string(),
        // `blob:` is a Photograph taken offline, shown from the outbox before
        // it reaches the server. `data:` is a Crouton import's favicon.
        "img-src 'self' blob: data:".to_string(),
        "object-src 'none'".to_string(),
        "base-uri 'self'".to_string(),
        "form-action 'self'".to_string(),
        // Only a header can say this; a `<meta>` policy ignores it.
        "frame-ancestors 'none'".to_string(),
    ]
    .join("; ");
    HeaderValue::from_str(&policy).expect("a policy is plain ASCII")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_script_is_allowed_only_by_hash() {
        let policy = policy_for(&["'sha256-abc='".to_string()]);
        let policy = policy.to_str().unwrap();
        assert!(
            policy.contains("script-src 'self' 'sha256-abc='"),
            "{policy}"
        );
        let scripts = policy
            .split(';')
            .find(|d| d.trim().starts_with("script-src"))
            .unwrap();
        assert!(!scripts.contains("unsafe"), "{policy}");
    }
}
