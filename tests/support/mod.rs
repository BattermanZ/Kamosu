//! Shared test plumbing. Everything here drives the real app — a real SQLite
//! file in a temporary directory, both real Doors, real HTTP over localhost.
//! No mocks, no stubs, no in-memory doubles.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use kamosu::core::Core;
use kamosu::{db, http_min};

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

    /// POST JSON-RPC to the MCP door, as an agent would.
    #[allow(dead_code)]
    pub fn post_mcp(&self, payload: &str, bearer: Option<&str>) -> (u16, Value) {
        self.wait_until_serving();
        let response = expect_reply(
            http_min::post_json(self.addr, "/mcp", bearer, payload),
            "POST",
            "/mcp",
        );
        parse(response.status, &response.text())
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

fn parse(status: u16, body: &str) -> (u16, Value) {
    let value = if body.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(body).unwrap_or(Value::Null)
    };
    (status, value)
}
