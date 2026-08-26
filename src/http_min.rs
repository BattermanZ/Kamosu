//! A dependency-free HTTP client, used by the self-run healthcheck and the tests.
//!
//! A distroless image has no shell and no curl (ADR 0028), so the binary checks
//! its own health with these ~40 lines instead of pulling in a client stack.

use std::io::{Read, Write};
use std::net::SocketAddr;

pub struct Response {
    pub status: u16,
    #[allow(dead_code)]
    pub body: String,
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
    let mut stream = std::net::TcpStream::connect(addr)?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;

    let mut request = format!(
        "POST {path} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n",
        addr,
        body.len()
    );
    if let Some(secret) = bearer {
        request.push_str(&format!("Authorization: Bearer {secret}\r\n"));
    }
    request.push_str("\r\n");
    request.push_str(body);

    stream.write_all(request.as_bytes())?;
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
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, b)| b.to_string())
        .unwrap_or_default();
    Ok(Response { status, body })
}
