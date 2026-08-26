# Kamosu development ritual. Run `just` with no argument to list everything.

# Every later ticket runs the app through these recipes — never by hand, never a
# leaking `cargo run &`. Dev and production share port 5266; no environment
# variable overrides it here.

default:
    @just --list

# ── Knobs set once ────────────────────────────────────────────────────────────

# Warn about the build cache at this many bytes. 20 GiB to start; raise it only
# with a reason written down.
build_cache_limit_bytes := "21474836480"

# Where the dev server keeps its pidfile, logfile and database. Disposable.
dev_dir := ".dev"
pid_file := dev_dir + "/kamosu.pid"
log_file := dev_dir + "/kamosu.log"

# The cargo target directory — respect CARGO_TARGET_DIR where a host sets one.
target_dir := env_var_or_default("CARGO_TARGET_DIR", "target")

# The binary, built once per invocation.
binary := target_dir + "/debug/kamosu"

# ── Running the app ───────────────────────────────────────────────────────────

# Start Kamosu in the background on 5266 (dev and production ports match), bound
# to 0.0.0.0 so it is reachable across the LAN. Writes a pidfile and a logfile
# under .dev/, prints the log path and the LAN URL. Always safe to re-run: it
# stops what it tracks, kills anything else bound to 5266 — including a server
# someone started by hand — and waits for the port to actually free first.
dev-start:
    #!/usr/bin/env bash
    set -euo pipefail
    # 1. Stop whatever a previous dev-start is tracking (whole process group).
    if [[ -f {{pid_file}} ]]; then
        pgid="$(cat {{pid_file}})"
        kill -TERM -- "-${pgid}" 2>/dev/null || true
        rm -f {{pid_file}}
    fi
    # 2. Kill anything else holding 5266.
    squatters="$(ss -ltnp 2>/dev/null | grep ':5266 ' | grep -oP 'pid=\K[0-9]+' | sort -u || true)"
    for pid in ${squatters}; do
        echo "killing stray server on 5266 (pid ${pid})"
        kill "${pid}" 2>/dev/null || true
    done
    # 3. Wait for the port to actually free before starting.
    for _ in $(seq 1 100); do
        if ! ss -ltn 2>/dev/null | grep -q ':5266 '; then break; fi
        sleep 0.1
    done
    if ss -ltn 2>/dev/null | grep -q ':5266 '; then
        echo "error: port 5266 is still busy after waiting" >&2
        exit 1
    fi
    # 4. Build, then start under its own session so the whole group is ours.
    mkdir -p "{{dev_dir}}"
    cargo build
    setsid env KAMOSU_DATA_DIR="{{dev_dir}}/data" \
        "{{binary}}" serve >>"{{log_file}}" 2>&1 &
    echo $! > "{{pid_file}}"
    # 5. Verify it actually came up — a silent failure must not look like success.
    up=false
    for _ in $(seq 1 50); do
        if ss -ltn 2>/dev/null | grep -q ':5266 '; then up=true; break; fi
        sleep 0.2
    done
    if [[ "${up}" != true ]]; then
        echo "error: server did not come up; last log lines:" >&2
        tail -n 10 "{{log_file}}" >&2 || true
        exit 1
    fi
    lan_ip="$(ip -4 addr show scope global 2>/dev/null | grep -oP '(?<=inet\s)\d+(\.\d+){3}' | head -1)"
    echo "started (pid $(cat {{pid_file}}))"
    echo "logs:   {{log_file}}   (just dev-logs to follow)"
    if [[ -n "${lan_ip}" ]]; then
        echo "LAN:    http://${lan_ip}:5266"
    else
        echo "LAN:    (no global IP found — try http://$(hostname -I | awk '{print $1}'):5266)"
    fi
    just _warn-cache-size

# Stop the tracked dev server — the whole process group, not just the top PID,
# so no child outlives the recipe.
dev-stop:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -f {{pid_file}} ]]; then
        pgid="$(cat {{pid_file}})"
        if kill -0 -- "-${pgid}" 2>/dev/null; then
            kill -TERM -- "-${pgid}"
            echo "stopped process group ${pgid}"
        else
            echo "was not running"
        fi
        rm -f {{pid_file}}
    else
        echo "not tracked (nothing to stop)"
    fi

# Report whether the dev server is running, since when, and how big the build
# cache is — always, warning or not.
dev-status:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -f {{pid_file}} ]] && kill -0 -- "-$(cat {{pid_file}})" 2>/dev/null; then
        pgid="$(cat {{pid_file}})"
        elapsed="$(ps -o etime= --pid "${pgid}" 2>/dev/null | tr -d ' ' || true)"
        echo "running: yes (process group ${pgid}, up ${elapsed:-unknown})"
    else
        echo "running: no"
    fi
    size="$(du -sb "{{target_dir}}" 2>/dev/null | cut -f1 || echo 0)"
    human="$(numfmt --to=iec --suffix=B "${size}" 2>/dev/null || echo "${size} bytes")"
    echo "cargo target directory: {{target_dir}} (${human})"
    just _warn-cache-size

# Follow the dev server's log.
dev-logs:
    tail -n 40 -f {{log_file}}

# Reclaim the build cache by deleting the cargo target directory. The next build
# is a full rebuild — everything recompiles from scratch.
dev-clean:
    #!/usr/bin/env bash
    set -euo pipefail
    dir="${CARGO_TARGET_DIR:-target}"
    rm -rf "${dir}"
    echo "removed ${dir} — the next build is a full rebuild"

# Internal: warn when the cache crosses the threshold set at the top.
_warn-cache-size:
    #!/usr/bin/env bash
    set -euo pipefail
    size="$(du -sb "{{target_dir}}" 2>/dev/null | cut -f1 || echo 0)"
    if (( size >= {{build_cache_limit_bytes}} )); then
        human="$(numfmt --to=iec --suffix=B "${size}" 2>/dev/null || echo "${size} bytes")"
        limit_human="$(numfmt --to=iec --suffix=B {{build_cache_limit_bytes}} 2>/dev/null || echo '{{build_cache_limit_bytes}} bytes')"
        echo ""
        echo "WARNING: build cache target/ is ${human}, at or above the ${limit_human} limit."
        echo "         Reclaim it with 'just dev-clean' (the next build is a full rebuild)."
    fi

# ── Checking the code ─────────────────────────────────────────────────────────

# Run the behaviour suite (real Operations, real Credential, real SQLite file).
# The test-jobs feature adds `probe_job`, a demonstration Job, so slow work is
# provable end-to-end before real Jobs (Import, Sheets) arrive in later tickets.
test:
    cargo test --features test-jobs

# Check formatting and lints without changing anything. Also verifies the
# committed design-token stylesheet and icons are fresh against their sources:
# the build fails if what is committed has drifted from ui/.
check:
    cargo fmt --check && cargo clippy --all-targets -- -D warnings && just _check-tokens-fresh

# ── Design tokens ─────────────────────────────────────────────────────────────

# Regenerate the one generated stylesheet (assets/app.css) and the icon PNGs
# from their single source of truth in ui/. Safe to run any time.
css:
    #!/usr/bin/env bash
    set -euo pipefail
    cd ui
    if [[ ! -d node_modules ]]; then
        echo "installing ui dependencies (npm ci)"
        npm ci --no-audit --no-fund >/dev/null
    fi
    npm run --silent css
    npm run --silent icons

# Internal: regenerate the stylesheet and icons into a temp directory and
# compare byte-for-byte with what is committed. Fails without touching the tree,
# so a stale generated file can never pass `just check` unnoticed.
_check-tokens-fresh:
    #!/usr/bin/env bash
    set -euo pipefail
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT
    mkdir -p "$tmp/assets/icons"
    ( cd ui
      if [[ ! -d node_modules ]]; then
          npm ci --no-audit --no-fund >/dev/null
      fi
      CSS_OUT="$tmp/assets/app.css" npm run --silent css >/dev/null
      node ./generate-icons.mjs "$tmp/assets/icons" >/dev/null
    )
    if ! diff -r assets/app.css "$tmp/assets/app.css" \
        || ! diff -r assets/icons "$tmp/assets/icons"; then
        echo "error: committed design-token output has drifted from ui/ —" >&2
        echo "       run 'just css' and commit the regenerated files." >&2
        exit 1
    fi

tokens-fresh: _check-tokens-fresh

# ── Packaging ─────────────────────────────────────────────────────────────────

# Build the distroless image exactly as a stranger would. --load puts it in the
# local image store (a no-op on hosts where docker build already does that).
# Depends on the tokens freshness gate: the image embeds assets/app.css, so a
# stale committed stylesheet must fail here too, not only in `just check`.
docker-build: _check-tokens-fresh
    docker build --load -t kamosu .

# Run the image as the one-mount-one-port install: ./kamosu-data holds the truth,
# 5266 serves the world. Ctrl-C stops it.
docker-run:
    #!/usr/bin/env bash
    set -euo pipefail
    docker rm -f kamosu-dev >/dev/null 2>&1 || true
    exec docker run --name kamosu-dev -v ./kamosu-data:/data -p 5266:5266 kamosu
