//! A dependency-free HTTP client, used by the self-run healthcheck and the tests.
//!
//! A distroless image has no shell and no curl (ADR 0028), so the binary checks
//! its own health with these ~40 lines instead of pulling in a client stack.

use std::io::{Read, Write};
use std::net::SocketAddr;

pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    /// The first Content-Type header, lowercased, if any.
    pub fn content_type(&self) -> Option<&str> {
        self.headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
            .map(|(_, value)| value.as_str())
    }

    /// The body decoded lossily — enough for tests asserting on JSON or text;
    /// never for the bytes of a picture themselves.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
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
        Some(("application/json", body.as_bytes())),
    )
}

/// POST raw bytes with the given Content-Type — a Photograph upload, tested
/// exactly as the out-of-band route (ADR 0001) receives it: no JSON envelope.
#[allow(dead_code)]
pub fn post_bytes(
    addr: SocketAddr,
    path: &str,
    bearer: Option<&str>,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<Response> {
    request(addr, "POST", path, bearer, Some((content_type, body)))
}

/// GET one resource and read one response, bytes intact.
pub fn get(addr: SocketAddr, path: &str) -> std::io::Result<Response> {
    request(addr, "GET", path, None, None)
}

/// GET one resource with a Credential — a Photograph download is
/// authenticated the same way an Operation is.
#[allow(dead_code)]
pub fn get_with_bearer(
    addr: SocketAddr,
    path: &str,
    bearer: Option<&str>,
) -> std::io::Result<Response> {
    request(addr, "GET", path, bearer, None)
}

fn request(
    addr: SocketAddr,
    method: &str,
    path: &str,
    bearer: Option<&str>,
    body: Option<(&str, &[u8])>,
) -> std::io::Result<Response> {
    let mut stream = std::net::TcpStream::connect(addr)?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;

    let mut head = format!("{method} {path} HTTP/1.1\r\nHost: {}\r\n", addr);
    if let Some((content_type, content)) = body {
        head.push_str(&format!(
            "Content-Type: {content_type}\r\nContent-Length: {}\r\n",
            content.len()
        ));
    }
    head.push_str("Connection: close\r\n");
    if let Some(secret) = bearer {
        head.push_str(&format!("Authorization: Bearer {secret}\r\n"));
    }
    head.push_str("\r\n");

    stream.write_all(head.as_bytes())?;
    if let Some((_, content)) = body {
        stream.write_all(content)?;
    }
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;

    let split_at = find_double_crlf(&raw).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "no header/body boundary")
    })?;
    let head_text = String::from_utf8_lossy(&raw[..split_at]);
    let body = raw[split_at + 4..].to_vec();

    let status = head_text
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "unreadable HTTP status line",
            )
        })?;
    let headers = head_text
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

fn find_double_crlf(raw: &[u8]) -> Option<usize> {
    raw.windows(4).position(|window| window == b"\r\n\r\n")
}
