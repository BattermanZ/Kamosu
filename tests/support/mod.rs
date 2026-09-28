//! Shared test plumbing. Everything here drives the real app — a real SQLite
//! file in a temporary directory, both real Doors, real HTTP over localhost.
//! No mocks, no stubs, no in-memory doubles.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use kamosu::core::Core;
use kamosu::{MCP_PROTOCOL_VERSION, db, http_min};

pub struct TestApp {
    pub addr: SocketAddr,
    /// Used by behaviour tests; unused by parity tests.
    #[allow(dead_code)]
    pub core: Arc<Core>,
    /// Holds the temporary data directory open for the life of the test. None
    /// when the caller owns the directory itself (migration tests do).
    dir: Option<tempfile::TempDir>,
}

impl TestApp {
    /// The one data directory this instance was given, when this helper created
    /// it. Tests that brought their own directory hold it themselves.
    #[allow(dead_code)]
    pub fn data_dir(&self) -> Option<&std::path::Path> {
        self.dir.as_ref().map(|d| d.path())
    }
}

/// Start a real Kamosu on an ephemeral port, with its database in a fresh
/// temporary directory. Call from a multi-threaded tokio runtime so the server
/// task keeps serving while the test blocks on ordinary HTTP calls.
pub fn spawn_app() -> TestApp {
    let dir = tempfile::tempdir().expect("temp dir");
    spawn_app_in(dir.path()).hold_dir(dir)
}

/// Start a real Kamosu in a data directory the caller chose and keeps open —
/// used where the test builds the database's *prior* state first, as the
/// migration tests do.
pub fn spawn_app_in(data_dir: &std::path::Path) -> TestApp {
    let db = Arc::new(db::Db::open(data_dir).expect("database"));
    // Literally the application the binary serves — `kamosu::app` is the only
    // place it is assembled, so a test can never drive an arrangement nobody
    // runs. A Core with its Job lanes started sits underneath it.
    let core = Core::start(db);
    let app = kamosu::app(core.clone());

    let std_listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    std_listener.set_nonblocking(true).unwrap();
    let listener = tokio::net::TcpListener::from_std(std_listener).expect("async listener");
    let addr = listener.local_addr().expect("local addr");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    TestApp {
        addr,
        core,
        dir: None,
    }
}

impl TestApp {
    /// Attach a temporary directory this app should keep open for its lifetime.
    fn hold_dir(mut self, dir: tempfile::TempDir) -> TestApp {
        self.dir = Some(dir);
        self
    }
}

impl TestApp {
    /// Wait until the server actually accepts connections, instead of hoping a
    /// fixed sleep was long enough.
    fn wait_until_serving(&self) {
        for _ in 0..100 {
            if std::net::TcpStream::connect(self.addr).is_ok() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!(
            "server at {} never started accepting connections",
            self.addr
        );
    }

    /// POST through the web door, as a browser would.
    pub fn post_op(&self, name: &str, bearer: Option<&str>, body: &str) -> (u16, Value) {
        self.wait_until_serving();
        let path = format!("/api/op/{name}");
        let response = expect_reply(
            http_min::post_json(self.addr, &path, bearer, body),
            "POST",
            &path,
        );
        parse(response.status, &response.text())
    }

    /// POST at the browser authentication boundary: credential minting precedes
    /// Operations, then its Session cookie becomes the Credential.
    #[allow(dead_code)]
    pub fn post_auth(&self, path: &str, body: &str) -> (u16, Value) {
        let response = self.post_auth_response(path, body);
        parse(response.status, &response.text())
    }

    #[allow(dead_code)]
    pub fn post_auth_response(&self, path: &str, body: &str) -> http_min::Response {
        self.wait_until_serving();
        expect_reply(
            http_min::post_json(self.addr, path, None, body),
            "POST",
            path,
        )
    }

    /// POST JSON-RPC to the MCP door, as an agent would: carrying the headers
    /// and `_meta` fields revision `2026-07-28` requires, read off the body by
    /// `complete_mcp_request` so a test writes only what it is about (#144).
    #[allow(dead_code)]
    pub fn post_mcp(&self, payload: &str, bearer: Option<&str>) -> (u16, Value) {
        let authorization = bearer.map(|secret| format!("Bearer {secret}"));
        let headers: Vec<(&str, &str)> = authorization
            .iter()
            .map(|value| ("Authorization", value.as_str()))
            .collect();
        self.post_mcp_with_headers(payload, &headers)
    }

    /// GET one path — an asset or the tokens page — as a browser would.
    /// Returns (status, content-type, body). Binary bodies are lossy text;
    /// assert on status and headers for those. Unused by parity tests.
    #[allow(dead_code)]
    pub fn get(&self, path: &str) -> (u16, String, String) {
        self.wait_until_serving();
        let response = expect_reply(http_min::get(self.addr, path), "GET", path);
        (
            response.status,
            response.content_type().unwrap_or_default().to_string(),
            response.text(),
        )
    }

    /// GET one path with a Credential and read the bytes back intact — a
    /// Photograph or Display Copy download, never lossily decoded.
    #[allow(dead_code)]
    pub fn get_bytes(&self, path: &str, bearer: Option<&str>) -> (u16, String, Vec<u8>) {
        self.wait_until_serving();
        let response = expect_reply(
            http_min::get_with_bearer(self.addr, path, bearer),
            "GET",
            path,
        );
        (
            response.status,
            response.content_type().unwrap_or_default().to_string(),
            response.body,
        )
    }

    /// POST an Operation with extra headers of the test's choosing, and no
    /// Credential unless one of those headers carries it.
    #[allow(dead_code)]
    pub fn post_op_with_headers(
        &self,
        name: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> (u16, Value) {
        let (status, _, body) = self.post_op_with_headers_reply(name, headers, body);
        (status, body)
    }

    /// The same, with the answer's own headers — for the one question that is
    /// about a header rather than about a result: whether a refusal takes back
    /// the Session cookie this Door set (#91).
    #[allow(dead_code)]
    pub fn post_op_with_headers_reply(
        &self,
        name: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> (u16, Vec<(String, String)>, Value) {
        self.post_with_headers(&format!("/api/op/{name}"), headers, body)
    }

    /// The same at the MCP door, which is built by walking the same Catalogue
    /// and so has to answer the same way. The request is completed as
    /// `post_mcp` completes it, and a header the test names replaces the one
    /// that would have been read off the body.
    #[allow(dead_code)]
    pub fn post_mcp_with_headers(&self, payload: &str, headers: &[(&str, &str)]) -> (u16, Value) {
        let (payload, derived) = complete_mcp_request(payload);
        let mut sent: Vec<(&str, &str)> = derived
            .iter()
            .filter(|(name, _)| {
                !headers
                    .iter()
                    .any(|(given, _)| given.eq_ignore_ascii_case(name))
            })
            .map(|(name, value)| (*name, value.as_str()))
            .collect();
        sent.extend_from_slice(headers);
        self.post_mcp_bare(&payload, &sent)
    }

    /// POST to the MCP door exactly what the test wrote, and nothing it did
    /// not: for the refusals of a request missing what the revision requires.
    #[allow(dead_code)]
    pub fn post_mcp_bare(&self, payload: &str, headers: &[(&str, &str)]) -> (u16, Value) {
        let (status, _, body) = self.post_with_headers("/mcp", headers, payload);
        (status, body)
    }

    /// POST to one path with headers of the test's choosing.
    ///
    /// This exists for three questions. Whether anything Kamosu reads off a
    /// request can stand in for authorisation (ADR 0033); whether a refusal
    /// takes back the Session cookie this Door set (#91), since the answer's
    /// own headers come back with it; and whether the MCP door gets the
    /// headers revision `2026-07-28` requires of every request (#144), which
    /// is why every MCP test comes this way. All three need a request written
    /// out by hand rather than sent through `http_min`, because that client
    /// serves the binary's own healthcheck, and widening it so a test can
    /// forge a header would put the forgery in the shipped program.
    #[allow(dead_code)]
    fn post_with_headers(
        &self,
        path: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> (u16, Vec<(String, String)>, Value) {
        use std::io::{Read, Write};

        self.wait_until_serving();
        let mut request = format!(
            "POST {path} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\n\
             Content-Length: {}\r\nConnection: close\r\n",
            self.addr,
            body.len()
        );
        for (field, value) in headers {
            request.push_str(&format!("{field}: {value}\r\n"));
        }
        request.push_str("\r\n");
        request.push_str(body);

        let mut stream = std::net::TcpStream::connect(self.addr).expect("connect to the web door");
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("set a read timeout");
        stream
            .write_all(request.as_bytes())
            .expect("write the request");
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).expect("read the reply");

        let text = String::from_utf8_lossy(&raw).into_owned();
        let status: u16 = text
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse().ok())
            .unwrap_or_else(|| panic!("no status line in the reply to {path}: {text}"));
        let (reply_head, reply_body) = text.split_once("\r\n\r\n").unwrap_or((text.as_str(), ""));
        let reply_headers = reply_head
            .lines()
            .skip(1)
            .filter_map(|line| line.split_once(": "))
            .map(|(field, value)| (field.to_string(), value.to_string()))
            .collect();
        let (status, parsed) = parse(status, reply_body);
        (status, reply_headers, parsed)
    }

    /// POST raw bytes — a Photograph upload through the out-of-band route
    /// (ADR 0001), never wrapped in a JSON envelope.
    #[allow(dead_code)]
    pub fn post_bytes(
        &self,
        path: &str,
        bearer: Option<&str>,
        content_type: &str,
        body: &[u8],
    ) -> (u16, Value) {
        self.wait_until_serving();
        let response = expect_reply(
            http_min::post_bytes(self.addr, path, bearer, content_type, body),
            "POST",
            path,
        );
        parse(response.status, &response.text())
    }
}

/// The Session Secret a browser would keep from one answer: the value of the
/// `kamosu_session` cookie the Door set, with its attributes stripped off.
///
/// Every test that signs in through `/auth/…` and then acts as that browser
/// needs this, so it lives here rather than being written out again in each.
#[allow(dead_code)]
pub fn session_cookie_secret(reply: &http_min::Response) -> String {
    reply
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("set-cookie"))
        .and_then(|(_, value)| value.split(';').next())
        .and_then(|pair| pair.strip_prefix("kamosu_session="))
        .expect("an HttpOnly Session cookie")
        .to_string()
}

/// One request's answer, or a panic that says what actually went wrong.
///
/// Written because the message it replaces did not. Every one of these used to
/// read `.expect("web door reachable")`, which asserts the one thing that was
/// never in doubt — `wait_until_serving` has already connected by the time any
/// of them runs. When `tests/photographs_corpus.rs` failed, it therefore
/// announced an unreachable door for what was really a reply that took longer
/// than the client would wait, and reading it cost an afternoon.
///
/// The `WouldBlock` case is the whole reason this exists: on Linux a read
/// timeout on a blocking socket comes back as "Resource temporarily
/// unavailable", which names an operating-system condition and not a thing a
/// person did. Said plainly, it is the server taking its time.
fn expect_reply(
    result: std::io::Result<http_min::Response>,
    method: &str,
    path: &str,
) -> http_min::Response {
    result.unwrap_or_else(|error| match error.kind() {
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => panic!(
            "{method} {path} got no reply before the client gave up waiting. \
             The server is answering — it is taking longer than http_min's \
             timeout allows. Real work behind the route (re-encoding a \
             Photograph, say) is the usual reason."
        ),
        _ => panic!("{method} {path} failed: {error}"),
    })
}

/// Poll `get_job` through the web Door until the Job reaches an end state, and
/// answer the record. Every suite that drives a Job needs this, so it lives
/// here rather than once per test binary.
#[allow(dead_code)]
pub fn wait_terminal(app: &TestApp, bearer: Option<&str>, job_id: &str) -> Value {
    let body = serde_json::json!({ "job_id": job_id }).to_string();
    for _ in 0..400 {
        let (_, answered) = app.post_op("get_job", bearer, &body);
        let record = answered["result"].clone();
        if ["completed", "failed", "cancelled"].contains(&record["status"].as_str().unwrap_or("")) {
            return record;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("job {job_id} never reached an end state");
}

/// A JSON-RPC body made whole, as revision `2026-07-28` has a client send it,
/// with the headers that mirror it. Only what is missing is filled in: a field
/// or header a test set on purpose, a foreign version or a tasks capability,
/// is left as written, and the headers mirror it. A body that is not JSON, or
/// names no method, goes as it came. So does `initialize`, which only a legacy
/// client sends, and a legacy client sends none of this.
fn complete_mcp_request(payload: &str) -> (String, Vec<(&'static str, String)>) {
    let Ok(mut request) = serde_json::from_str::<Value>(payload) else {
        return (payload.to_string(), Vec::new());
    };
    let method = match request.get("method").and_then(Value::as_str) {
        Some(method) if method != "initialize" => method.to_string(),
        _ => return (payload.to_string(), Vec::new()),
    };

    // A notification carries no id and the revision asks no `_meta` of it.
    if request.get("id").is_some() {
        if request.get("params").is_none() {
            request["params"] = json!({});
        }
        if let Some(params) = request["params"].as_object_mut()
            && let Some(meta) = params
                .entry("_meta")
                .or_insert_with(|| json!({}))
                .as_object_mut()
        {
            meta.entry("io.modelcontextprotocol/protocolVersion")
                .or_insert_with(|| json!(MCP_PROTOCOL_VERSION));
            meta.entry("io.modelcontextprotocol/clientCapabilities")
                .or_insert_with(|| json!({}));
        }
    }

    let version = request
        .pointer("/params/_meta/io.modelcontextprotocol~1protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(MCP_PROTOCOL_VERSION)
        .to_string();
    // The door's `name_field` answers the same question. This copy is kept
    // apart on purpose, as a client's own reading of the revision would be.
    let named_by = match method.as_str() {
        "tools/call" => Some("name"),
        "tasks/get" | "tasks/update" | "tasks/cancel" => Some("taskId"),
        _ => None,
    };
    let name = named_by
        .and_then(|field| request["params"].get(field))
        .and_then(Value::as_str)
        .map(header_safe);

    let mut headers = vec![("MCP-Protocol-Version", version), ("Mcp-Method", method)];
    if let Some(name) = name {
        headers.push(("Mcp-Name", name));
    }
    (request.to_string(), headers)
}

/// A value as a header can carry it: as sent when it is plain ASCII, and in
/// the revision's `=?base64?…?=` sentinel otherwise.
fn header_safe(value: &str) -> String {
    use base64::Engine;
    let plain = value
        .bytes()
        .all(|byte| byte.is_ascii_graphic() || byte == b' ')
        && value.trim() == value
        && !value.starts_with("=?base64?");
    if plain {
        value.to_string()
    } else {
        format!(
            "=?base64?{}?=",
            base64::engine::general_purpose::STANDARD.encode(value)
        )
    }
}

fn parse(status: u16, body: &str) -> (u16, Value) {
    let value = if body.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(body).unwrap_or(Value::Null)
    };
    (status, value)
}
