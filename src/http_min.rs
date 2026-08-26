//! A dependency-free HTTP client, used by the self-run healthcheck and the tests.
//!
//! A distroless image has no shell and no curl (ADR 0028), so the binary checks
//! its own health with these ~40 lines instead of pulling in a client stack.

use std::io::{Read, Write};
use std::net::SocketAddr;

pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    #[allow(dead_code)]
    pub body: String,
}

impl Response {
    /// The first Content-Type header, lowercased, if any.
    pub fn content_type(&self) -> Option<&str> {
        self.headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
            .map(|(_, value)| value.as_str())
    }
}

/// POST JSON to `http://<addr><path>` and read one response. Sends
/// `Connection: close` so reading to EOF ends the exchange — fine for a
/// healthcheck against itself, never used as a general client.
pub fn post_json(
    addr: SocketAddr,
    path: &str,
    bearer: Option<&str>,
    body: &str,
) -> std::io::Result<Response> {
    request(
        addr,
        "POST",
        path,
        bearer,
        Some(("Content-Type: application/json\r\n", body)),
    )
}

/// GET one resource and read one response. Binary bodies are decoded lossily —
/// enough for tests that assert status and headers, never for bytes themselves.
pub fn get(addr: SocketAddr, path: &str) -> std::io::Result<Response> {
    request(addr, "GET", path, None, None)
}

fn request(
    addr: SocketAddr,
    method: &str,
    path: &str,
    bearer: Option<&str>,
    body: Option<(&str, &str)>,
) -> std::io::Result<Response> {
    let mut stream = std::net::TcpStream::connect(addr)?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;

    let mut r = format!("{method} {path} HTTP/1.1\r\nHost: {}\r\n", addr);
    if let Some((content_header, content)) = body {
        r.push_str(&format!(
            "{content_header}Content-Length: {}\r\n",
            content.len()
        ));
    }
    r.push_str("Connection: close\r\n");
    if let Some(secret) = bearer {
        r.push_str(&format!("Authorization: Bearer {secret}\r\n"));
    }
    r.push_str("\r\n");
    if let Some((_, content)) = body {
        r.push_str(content);
    }
    stream.write_all(r.as_bytes())?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;

    let text = String::from_utf8_lossy(&raw);
    let status = text
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "unreadable HTTP status line",
            )
        })?;
    let (head, body) = text
        .split_once("\r\n\r\n")
        .map(|(h, b)| (h.to_string(), b.to_string()))
        .unwrap_or_default();
    let headers = head
        .lines()
        .skip(1)
        .filter_map(|line| line.split_once(": "))
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
    Ok(Response {
        status,
        headers,
        body,
    })
}
