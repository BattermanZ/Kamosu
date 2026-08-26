//! Shared test plumbing. Everything here drives the real app — a real SQLite
//! file in a temporary directory, both real Doors, real HTTP over localhost.
//! No mocks, no stubs, no in-memory doubles.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use kamosu::core::Core;
use kamosu::{db, design_tokens, http_min, mcp_door, web_door};

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
    // The same assembly the binary runs: a Core with its Job lanes started, both
    // Doors plus the design-token assets — tests exercise exactly what is served.
    let core = Core::start(db);
    let app = web_door::router(core.clone())
        .merge(mcp_door::router(core.clone()))
        .merge(design_tokens::router());

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
        let response = http_min::post_json(self.addr, &format!("/api/op/{name}"), bearer, body)
            .expect("web door reachable");
        parse(response.status, &response.body)
    }

    /// POST JSON-RPC to the MCP door, as an agent would.
    pub fn post_mcp(&self, payload: &str, bearer: Option<&str>) -> (u16, Value) {
        self.wait_until_serving();
        let response =
            http_min::post_json(self.addr, "/mcp", bearer, payload).expect("mcp door reachable");
        parse(response.status, &response.body)
    }

    /// GET one path — an asset or the tokens page — as a browser would.
    /// Returns (status, content-type, body). Binary bodies are lossy text;
    /// assert on status and headers for those. Unused by parity tests.
    #[allow(dead_code)]
    pub fn get(&self, path: &str) -> (u16, String, String) {
        self.wait_until_serving();
        let response = http_min::get(self.addr, path).expect("server reachable");
        (
            response.status,
            response.content_type().unwrap_or_default().to_string(),
            response.body,
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
