# AGENTS.md

Project conventions for Kamosu. Host-specific conventions live in
`AGENTS.local.md`; vocabulary lives in `CONTEXT.md`; decisions live in
`docs/adr/`. **Where the spec and an ADR disagree, the ADR wins.**

Build, run and test instructions arrive with the walking skeleton
([#33](https://github.com/BattermanZ/Kamosu/issues/33)), which creates the
codebase this file describes.

## CodeGraph

**When a `.codegraph/` directory exists at the repository root, reach for
CodeGraph before grep, find, or reading source files** — both locating code and
understanding it go through it first.

```sh
codegraph explore "<symbols or question>"
```

Prefer the `codegraph_explore` MCP tool where it is available: it returns the
relevant source, the call paths between the symbols, and a blast-radius summary
in one query — a round trip a grep-and-read loop cannot match.

The index is machine-local and gitignored, so **a fresh clone has none.** When
`.codegraph/` is absent, use the ordinary search and read tools and do not
initialize an index automatically; `codegraph init` at the repo root is a
human's decision, not an agent's. Where the directory exists but neither the MCP
tool nor the CLI is reachable, fall back to normal search rather than blocking
the task.

## Dependencies

**The registry is the source of truth for every version number.** A version
you write from memory is a version that was current on your training cutoff,
which is not today.

Add a dependency by letting its package manager resolve it:

```sh
cargo add <crate>              # writes the current version into Cargo.toml
npm install <package>          # writes the current version into package.json
npx sv create                  # scaffolds SvelteKit at its current version
```

A version string typed by hand into `Cargo.toml` or `package.json` is a bug,
the same way a permission check inside a Door is a bug.

**Read the library's current documentation before writing against it.** Use the
**Context7** MCP server before writing code that calls a library API, even when
you believe you know the API. Getting the version number right and then calling
a function that was renamed two releases ago is the same failure wearing a
different hat.

Two calls, in order:

1. `context7_resolve-library-id` with `libraryName` (**and** `query` — both are
   required; the query says what you're trying to do and ranks the results).
2. `context7_query-docs` with the `libraryId` that returned (pin a version when
   one is offered, e.g. `/tokio-rs/axum/axum_v0_8_4`) and a `query` scoped to
   **one concept per call** — multi-concept queries return mush.

**Take the current version when a dependency is first added, then hold it.**
Chasing a major release mid-build is a rewrite, not an upgrade; upgrade
deliberately, as its own piece of work.

**Kamosu pins its compiler.** `rust-toolchain.toml` fixes the Rust version so
every machine and every CI run builds identically. Pinning the toolchain and
letting crates float are not in conflict: one is the kitchen, the other is the
ingredients.

## Building, testing and running

Everything runs through `just` at the repo root — never a hand-rolled
`cargo run &`, which leaks a process holding the port.

```sh
just check      # fmt + clippy + the tokens freshness gate (see Design tokens below)
just test       # the behaviour suite: real Operations, real Credential, real SQLite
just css        # regenerate assets/app.css and the icon PNGs from ui/ (their source)
just dev-start  # background server on 5266, bound to 0.0.0.0; prints log path + LAN URL
just dev-status # running? since when? build-cache size
just dev-logs   # follow .dev/kamosu.log
just dev-stop   # stops the whole process group
just docker-build && just docker-run   # the one-mount-one-port install, as a stranger runs it
```

`dev-clean` deletes `target/` to reclaim build cache; the next build is a full
rebuild. Dev state lives in gitignored `.dev/`. The dev server uses port 5266
with no environment variable set — dev and production match.

## Design tokens

What Kamosu looks like is **one generated stylesheet**, `assets/app.css`,
compiled from `ui/src/app.css` — Tailwind 4 theme tokens that replace Tailwind's
palette, type scale, spacing and radii wholesale (`just css` regenerates; the
current choice is recorded in `docs/design/2026-08-26-the-visual-identity.md`).
The committed output is what the binary embeds at compile time, so a cargo build
never needs Node — but `just check` regenerates it into a temp directory and
**fails if what is committed has drifted from `ui/`**. Never hand-edit
`assets/app.css`; change `ui/src/app.css` and run `just css`.

## Project layout

```
src/catalogue.rs    the Catalogue: every Operation declared once
src/core.rs         the Core: dispatch + the only authorisation check in Kamosu
src/db.rs           the SQLite file under /data — WAL on, migrations forward-only
src/jobs.rs         slow work: two lanes (one for strangers, ADR 0032), progress, results
src/operations.rs   the functions the Catalogue's declarations name
src/web_door.rs     Axum router generated by walking the Catalogue
src/mcp_door.rs     MCP endpoint generated by walking the Catalogue
src/config.rs       the four optional environment variables
src/http_min.rs     dependency-free HTTP client for the self-run healthcheck
src/design_tokens.rs embeds the generated stylesheet + fonts/icons, serves /tokens
tests/parity.rs     both Doors materialise every Operation — drift breaks the build
tests/behaviour.rs  behaviour through real Doors against a real database file
```

## How the program is shaped

Every Operation is declared once in the Catalogue — name, input, output,
required permission, the function that performs it — and **both Doors are built
by walking that declaration list at startup** (ADR 0001). There is no place to
hand-write a web-only route; adding an Operation means adding one entry to the
Catalogue and both Doors grow it in the same commit.

- The web API is deliberately RPC-shaped (`POST /api/op/<name>`). Rewriting it
  as REST destroys the parity guarantee; don't.
- **A permission check written inside a Door is a bug.** Authorisation happens
  once, in the Core, keyed on the Credential. Doors only carry the Secret.
- The MCP door speaks revision `2026-07-28`, stateless, no handshake.

## Where the truth lives

**The database is the truth; the Vault is a published copy** (ADR 0003). This
inverts Hatchdoor, this project's usual precedent — if you know Hatchdoor, do
not assume its Markdown-is-truth rule applies here. Deleting Kamosu's database
loses every account, cooking history and shopping list, and no Vault rebuilds
them.

Vocabulary lives in `CONTEXT.md`; decisions live in `docs/adr/`. Where the spec
and an ADR disagree, the ADR wins.
