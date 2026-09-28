//! One guard, run before the crate compiles: a release build must carry a built
//! interface.
//!
//! Kamosu is one executable (ADR 0028), so a release binary without `ui/build`
//! is not a smaller Kamosu — it is a Kamosu with no screens, and it would only
//! be discovered by whoever opened the port. `rust-embed` is happy to embed
//! nothing, so the loudness has to be added here.
//!
//! Debug builds are exempt on purpose: development runs vite on 5174 beside the
//! binary on 5266, and `cargo run` must not require a frontend build first.

use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=ui/build");

    let built = Path::new("ui/build");
    let release = std::env::var("PROFILE").as_deref() == Ok("release");

    if release && !built.join("index.html").exists() {
        panic!(
            "ui/build/index.html is missing, so this release binary would ship no \
             interface.\n       Build it first: `just ui-build` (or `npm run build` in ui/)."
        );
    }

    // `rust-embed` refuses to compile against a folder that is not there, and a
    // fresh clone has never run the frontend build. An empty one is honest in a
    // debug build — the app is served by vite on 5174 — and the check above has
    // already ruled it out for a release.
    std::fs::create_dir_all(built).expect("ui/build must be creatable");
}
