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

# Development is two processes side by side (ADR 0028): the binary serves the
# Operations on 5266, vite serves the interface on 5174 and proxies everything
# the binary owns to it. Both ports are fixed — 5173 is taken on the dev host,
# and vite runs with --strictPort so a silent fallback is impossible.
backend_port := "5266"
vite_port := "5174"
vite_pid_file := dev_dir + "/vite.pid"
vite_log_file := dev_dir + "/vite.log"

# The cargo target directory — respect CARGO_TARGET_DIR where a host sets one.
target_dir := env_var_or_default("CARGO_TARGET_DIR", "target")

# The binary, built once per invocation.
binary := target_dir + "/debug/kamosu"

# ── Running the app ───────────────────────────────────────────────────────────

# Bring the whole thing up: the binary on 5266 and vite on 5174, both bound to
# 0.0.0.0 so they are reachable across the LAN. Open the vite URL — it serves the
# interface and proxies everything the binary owns to it, so a browser only ever
# talks to one origin. Pidfiles and logfiles for both live under .dev/.
#
# Always safe to re-run, for both ports: whatever a previous run tracked is
# stopped, anything else holding either port is killed — including something
# someone started by hand — and each port is waited on until it is actually free.
dev-start:
    #!/usr/bin/env bash
    set -euo pipefail
    just _free-port {{backend_port}} {{pid_file}}
    just _free-port {{vite_port}} {{vite_pid_file}}

    mkdir -p "{{dev_dir}}"

    # 1. The binary. Started under its own session so the whole group is ours.
    cargo build
    setsid env KAMOSU_DATA_DIR="{{dev_dir}}/data" \
        "{{binary}}" serve >>"{{log_file}}" 2>&1 &
    echo $! > "{{pid_file}}"
    just _await-port {{backend_port}} "{{log_file}}" "the server"

    # 2. Vite. It proxies to the binary, so it comes up second.
    just _npm-deps
    setsid env sh -c 'cd ui && exec npm run --silent dev' >>"{{vite_log_file}}" 2>&1 &
    echo $! > "{{vite_pid_file}}"
    just _await-port {{vite_port}} "{{vite_log_file}}" "vite"

    lan_ip="$(ip -4 addr show scope global 2>/dev/null | grep -oP '(?<=inet\s)\d+(\.\d+){3}' | head -1)"
    [[ -n "${lan_ip}" ]] || lan_ip="$(hostname -I | awk '{print $1}')"
    echo "started: server pid $(cat {{pid_file}}), vite pid $(cat {{vite_pid_file}})"
    echo "logs:    {{log_file}} and {{vite_log_file}}   (just dev-logs to follow both)"
    echo "app:     http://${lan_ip}:{{vite_port}}      <- open this one"
    echo "api:     http://${lan_ip}:{{backend_port}}"
    just _warn-cache-size

# Internal: make a port free and keep it that way. Stops whatever a previous
# dev-start tracked there (the whole process group), kills anything else holding
# it — including something started by hand — and waits for the port to actually
# free. Always safe to re-run, which is the property the whole ritual rests on.
_free-port port pid_file:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -f "{{pid_file}}" ]]; then
        pgid="$(cat "{{pid_file}}")"
        kill -TERM -- "-${pgid}" 2>/dev/null || true
        rm -f "{{pid_file}}"
    fi
    squatters="$(ss -ltnp 2>/dev/null | grep ':{{port}} ' | grep -oP 'pid=\K[0-9]+' | sort -u || true)"
    for pid in ${squatters}; do
        echo "killing stray process on {{port}} (pid ${pid})"
        kill "${pid}" 2>/dev/null || true
    done
    for _ in $(seq 1 100); do
        if ! ss -ltn 2>/dev/null | grep -q ':{{port}} '; then break; fi
        sleep 0.1
    done
    if ss -ltn 2>/dev/null | grep -q ':{{port}} '; then
        echo "error: port {{port}} is still busy after waiting" >&2
        exit 1
    fi

# Internal: wait for something to actually answer on a port. A silent failure
# must not look like success, so the last log lines come with the error.
_await-port port log what:
    #!/usr/bin/env bash
    set -euo pipefail
    for _ in $(seq 1 100); do
        if ss -ltn 2>/dev/null | grep -q ':{{port}} '; then exit 0; fi
        sleep 0.2
    done
    echo "error: {{what}} did not come up on {{port}}; last log lines:" >&2
    tail -n 15 "{{log}}" >&2 || true
    exit 1

# Stop both — each as a whole process group, not just the top PID, so neither
# vite's workers nor the server's threads outlive the recipe.
dev-stop:
    #!/usr/bin/env bash
    set -euo pipefail
    for pair in "the server|{{pid_file}}" "vite|{{vite_pid_file}}"; do
        what="${pair%%|*}"; file="${pair#*|}"
        if [[ -f "${file}" ]]; then
            pgid="$(cat "${file}")"
            if kill -0 -- "-${pgid}" 2>/dev/null; then
                kill -TERM -- "-${pgid}"
                echo "stopped ${what} (process group ${pgid})"
            else
                echo "${what} was not running"
            fi
            rm -f "${file}"
        else
            echo "${what} is not tracked (nothing to stop)"
        fi
    done

# Report whether each half is running and since when, and how big the build
# cache is — always, warning or not.
dev-status:
    #!/usr/bin/env bash
    set -euo pipefail
    for pair in "server (:{{backend_port}})|{{pid_file}}" "vite   (:{{vite_port}})|{{vite_pid_file}}"; do
        what="${pair%%|*}"; file="${pair#*|}"
        if [[ -f "${file}" ]] && kill -0 -- "-$(cat "${file}")" 2>/dev/null; then
            pgid="$(cat "${file}")"
            elapsed="$(ps -o etime= --pid "${pgid}" 2>/dev/null | tr -d ' ' || true)"
            echo "${what}: running (process group ${pgid}, up ${elapsed:-unknown})"
        else
            echo "${what}: not running"
        fi
    done
    size="$(du -sb "{{target_dir}}" 2>/dev/null | cut -f1 || echo 0)"
    human="$(numfmt --to=iec --suffix=B "${size}" 2>/dev/null || echo "${size} bytes")"
    echo "cargo target directory: {{target_dir}} (${human})"
    just _warn-cache-size

# Follow both logs at once, each line under the name of the file it came from.
dev-logs:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "{{dev_dir}}"
    touch "{{log_file}}" "{{vite_log_file}}"
    tail -n 20 -f "{{log_file}}" "{{vite_log_file}}"

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

# Run everything: the behaviour suite, and the interface's screen-seam tests.
# The interface is built first because the behaviour suite serves it — the tests
# drive what is actually shipped, never a stand-in for it. The test-jobs feature
# adds `probe_job`, a demonstration Job, so slow work is provable end-to-end
# before real Jobs (Import, Sheets) arrive in later tickets.
test: ui-build && ui-test
    cargo test --features test-jobs

# The interface's own tests: Svelte screens against a Catalogue-derived stand-in
# (ADR 0012). They are the only thing between a bad-day component and a kitchen.
ui-test:
    #!/usr/bin/env bash
    set -euo pipefail
    just _npm-deps
    cd ui && npm run --silent test

# Check formatting and lints without changing anything. Also verifies the
# committed design-token stylesheet and icons are fresh against their sources:
# the build fails if what is committed has drifted from ui/.
check: _check-tokens-fresh _check-client-fresh ui-check
    cargo fmt --check && cargo clippy --all-targets -- -D warnings

# svelte-check in strict mode, with Svelte's accessibility warnings treated as
# failures (ADR 0012): the compiler is the reviewer this frontend does not have.
ui-check:
    #!/usr/bin/env bash
    set -euo pipefail
    just _npm-deps
    cd ui && npm run --silent check

# ── The interface ─────────────────────────────────────────────────────────────

# Internal: make sure the interface's dependencies are there. Every recipe that
# touches ui/ depends on this rather than carrying its own copy of the check.
_npm-deps:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ ! -d ui/node_modules ]]; then
        echo "installing ui dependencies (npm ci)"
        ( cd ui && npm ci --no-audit --no-fund >/dev/null )
    fi


# Build the Svelte app into ui/build, which the binary embeds (ADR 0028). A
# release build refuses to link without it; a debug build serves from disk, so
# development never needs this — `just dev-start` runs vite instead.
ui-build:
    #!/usr/bin/env bash
    set -euo pipefail
    just _npm-deps
    cd ui && npm run --silent build

# Regenerate the typed client from the Catalogue. Every Operation is declared
# once in src/catalogue.rs; this is the third thing built by walking that list,
# after the two Doors, so the interface cannot ask for a shape the Core does not
# serve. Safe to run any time.
client:
    #!/usr/bin/env bash
    set -euo pipefail
    just _npm-deps
    mkdir -p "{{dev_dir}}"
    cargo run --quiet -- catalogue > "{{dev_dir}}/catalogue.json"
    cd ui && node ./generate-client.mjs "../{{dev_dir}}/catalogue.json"

# Internal: regenerate the client into a temp directory and compare with what is
# committed. Fails without touching the tree, so a Catalogue change that was
# never carried into the interface cannot pass `just check` unnoticed.
_check-client-fresh:
    #!/usr/bin/env bash
    set -euo pipefail
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT
    just _npm-deps
    cargo run --quiet -- catalogue > "$tmp/catalogue.json"
    ( cd ui && OUT="$tmp/catalogue.ts" node ./generate-client.mjs "$tmp/catalogue.json" )
    if ! diff -u ui/src/lib/api/catalogue.ts "$tmp/catalogue.ts"; then
        echo "error: the committed typed client has drifted from the Catalogue —" >&2
        echo "       run 'just client' and commit the regenerated file." >&2
        exit 1
    fi

# ── Design tokens ─────────────────────────────────────────────────────────────

# Regenerate everything that comes from the design tokens: the one generated
# stylesheet (assets/app.css), the icon PNGs, and the web manifest — the last
# two read their colours back out of the stylesheet rather than holding copies.
# Safe to run any time.
css:
    #!/usr/bin/env bash
    set -euo pipefail
    just _npm-deps
    cd ui
    npm run --silent css
    npm run --silent icons
    npm run --silent manifest

# Internal: regenerate the stylesheet, the icons and the manifest into a temp
# directory and compare byte-for-byte with what is committed. Fails without touching the tree,
# so a stale generated file can never pass `just check` unnoticed.
_check-tokens-fresh:
    #!/usr/bin/env bash
    set -euo pipefail
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT
    mkdir -p "$tmp/assets/icons"
    just _npm-deps
    ( cd ui
      CSS_OUT="$tmp/assets/app.css" npm run --silent css >/dev/null
      node ./generate-icons.mjs "$tmp/assets/icons" "$tmp/assets/app.css" >/dev/null
      node ./generate-manifest.mjs "$tmp/manifest.webmanifest" "$tmp/assets/app.css" >/dev/null
    )
    if ! diff -r assets/app.css "$tmp/assets/app.css" \
        || ! diff -r assets/icons "$tmp/assets/icons" \
        || ! diff ui/static/manifest.webmanifest "$tmp/manifest.webmanifest"; then
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
docker-build: _check-tokens-fresh _check-client-fresh
    docker build --load -t kamosu .

# Run the image as the one-mount-one-port install: ./kamosu-data holds the truth,
# 5266 serves the world. Ctrl-C stops it.
docker-run:
    #!/usr/bin/env bash
    set -euo pipefail
    docker rm -f kamosu-dev >/dev/null 2>&1 || true
    exec docker run --name kamosu-dev -v ./kamosu-data:/data -p 5266:5266 kamosu
