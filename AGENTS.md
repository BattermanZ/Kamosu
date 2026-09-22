# AGENTS.md

Project conventions for Kamosu. Host-specific conventions live in
`AGENTS.local.md`; vocabulary lives in `CONTEXT.md`; decisions live in
`docs/adr/`. **Where the spec and an ADR disagree, the ADR wins.**

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

**Svelte's own documentation for models is committed to this repo.** Read
`docs/svelte/llms-small.txt` before writing any Svelte or SvelteKit code, and
search `docs/svelte/llms-full.txt` for what the small file omits. This is not
optional: Svelte was chosen over React on the argument that a model reaching for
Svelte 4 habits is a risk closable at build time, and these files are what closes
it (ADR 0012). Both are pinned; `docs/svelte/README.md` says to which version and
how to refresh them.

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

**Holding a version is not the same as ignoring an advisory.** `just audit`
checks both trees — cargo-deny against RustSec, `npm audit` against GitHub's
database — and it is the one reason to move a version outside a deliberate
upgrade. A published vulnerability or a yanked release is a fact about the
world, not a new feature you are chasing. This is why no Dependabot or Renovate
runs here: automated version-bump pull requests would fight the paragraph above
every week, and `just audit` gives the security half without the churn. Read
what it reports rather than obeying it — `npm audit`'s suggested remedy is
routinely a downgrade that breaks more than it fixes. Advisories judged not to
apply are recorded, with the reasoning, in `deny.toml`.

**Kamosu pins its compiler.** `rust-toolchain.toml` fixes the Rust version so
every machine and every CI run builds identically. Pinning the toolchain and
letting crates float are not in conflict: one is the kitchen, the other is the
ingredients.

## Building, testing and running

Everything runs through `just` at the repo root — never a hand-rolled
`cargo run &`, which leaks a process holding the port.

```sh
just check      # fmt + clippy + prettier + eslint + svelte-check + the two freshness gates (below)
just test       # the behaviour suite (real Operations, real SQLite) + the screen tests
just format     # apply Prettier to ui/ — the Svelte half of `cargo fmt`
just audit      # both dependency trees against their advisory databases (needs network)
just css        # regenerate assets/app.css and the icon PNGs from ui/ (their source)
just client     # regenerate the typed client from the Catalogue (see The interface)
just ui-build   # build the Svelte app into ui/build, which the binary embeds
just dev-start  # server on 5266 AND vite on 5174, both on 0.0.0.0; prints both URLs
just dev-status # each half running? since when? build-cache size
just dev-logs   # follow .dev/kamosu.log and .dev/vite.log together
just dev-stop   # stops both process groups
just docker-build && just docker-run   # the one-mount-one-port install, as a stranger runs it
```

**Development is two processes side by side** (ADR 0028): the binary serves the
Operations on **5266**, vite serves the interface on **5174** and proxies
everything the binary owns to it, so a browser only ever talks to one origin.
Open the 5174 URL. Both ports are fixed with no environment variable — 5173 is
taken on the dev host, and vite runs with `--strictPort` so a silent fallback to
another port cannot happen. `dev-start` is always safe to re-run for both.

`dev-clean` deletes `target/` to reclaim build cache; the next build is a full
rebuild. Dev state lives in gitignored `.dev/`.

**Layout is settled by a formatter in both languages** — `cargo fmt` for Rust,
Prettier for `ui/` — and `just check` fails on either. Never argue with them and
never hand-format; run `just format` and move on. Two things `ui/.prettierignore`
excludes are worth knowing before you try to "fix" them: the generated files,
because `just check` compares them byte-for-byte against a fresh regeneration,
and `ui/src/app.css`, because Tailwind passes that file's indentation through
into the compiled stylesheet that ships.

**Lints are held to the same standard in both languages** — `cargo clippy -D
warnings` for Rust, `eslint --max-warnings 0` for `ui/`. The ESLint net is
deliberately narrow, because svelte-check and a strict `tsconfig` already catch
most of what it would: what is left is the part a type checker cannot see.
`ui/eslint.config.js` explains the two Svelte rules Kamosu turns off and what
each was guarding — read the reasoning before turning either back on.

## Design tokens

What Kamosu looks like is **one generated stylesheet**, `assets/app.css`,
compiled from `ui/src/app.css` — Tailwind 4 theme tokens that replace Tailwind's
palette, type scale, spacing and radii wholesale (`just css` regenerates).

**`ui/src/app.css` is the current value of every token; the dated design records
under `docs/design/` are not.** A record says what was chosen on one day and is
never updated, so a token it names may since have moved — `--hero-h` did, when
#81 put the recipe's title on its hero. Read the record for *why* a direction
was taken and the stylesheet for what is true now, and when you move a token,
say in a comment beside it which decision moved it.
The committed output is what the binary embeds at compile time, so a cargo build
never needs Node — but `just check` regenerates it into a temp directory and
**fails if what is committed has drifted from `ui/`**. Never hand-edit
`assets/app.css`; change `ui/src/app.css` and run `just css`.

**Tailwind reads `ui/src/` and nothing else**, declared with `source(none)` and
an explicit `@source` at the top of `ui/src/app.css`. The automatic detection
that replaced sweeps every non-gitignored file under `ui/` — including config
files and the prose inside their comments — and a bare utility word written in a
comment emits a real rule into the stylesheet that ships. If a new place ever
holds markup (the Rust-rendered Share Link page will), add an `@source` line for
it rather than removing the scoping.

## The interface

The frontend is **Svelte 5 + SvelteKit on `adapter-static` + TypeScript +
Tailwind 4 + Paraglide**, and it is **compiled into the binary** (ADR 0028) —
never copied beside it. `rust-embed` reads `ui/build` from disk in a debug build
and embeds it in a release build, so a frontend change recompiles no Rust and a
release binary that would ship no interface fails in `build.rs` instead.

**The interface calls Kamosu through a client generated from the Catalogue.**
`kamosu catalogue` prints every Operation's declaration as JSON; `just client`
turns that into `ui/src/lib/api/catalogue.ts`. It is the third thing built by
walking the one declaration list, after the two Doors, so the frontend and the
Core cannot disagree about an Operation's shape. Never edit the generated file:
change `src/catalogue.rs`, run `just client`, and commit both. `just check`
fails if what is committed has drifted.

**Screens are tested against a Catalogue-derived stand-in.** `standIn({ … })` in
`ui/src/lib/api/stand-in.ts` answers what a test says — and checks every answer
against the output schema the Catalogue declares first. A test can lie about the
values; it cannot lie about the shape. Screens take their client from context,
never from `fetch`.

**`svelte-check` runs strict and its warnings are failures**, Svelte's
accessibility warnings included (ADR 0012). **Paraglide compiles every phrase to
a function**, so a misspelt key is a build error; messages live in
`ui/messages/{en,fr,es}.json` and all three are present from the first day.

## Project layout

```
build.rs            refuses a release build that would ship no interface
src/backups.rs      a Backup: one archive of the database and the
                    Photographs, three kept at three distances (ADR 0039)
src/bundles.rs      a Bundle: one recipe's worth of Vault, written as a zip of
                    readable notes and a .kamosu/ sidecar (ADR 0020)
src/catalogue.rs    the Catalogue: every Operation declared once
src/core/           the Core, one file per area, all adding to one `Core`:
  mod.rs            the Core itself, startup, `execute` with the only
                    authorisation check in Kamosu, and Credential resolution
  accounts.rs       people, logins, Sessions, Access Keys, invites, recovery,
                    a Person's Reading Language, Operator
  kitchens.rs       Kitchens, their members, and the membership checks
  recipes.rs        creating, saving, reading a recipe; threads and divergence;
                    unfolding components; `parse_recipe_content`
  tags.rs           Tags, and putting one on a recipe
  related.rs        Related Recipes (#105)
  shelves.rs        the home shelves, and the shelf entry every list shows
  translation.rs    translation and a Branch's Language
  readings.rs       Ingredient Lines, Readings, Foods and Food merges
  cooking.rs        Attempts and promotion
  keeping_photographs.rs
                    storing, reading and sweeping Photographs
  imports.rs        Crouton, staged uploads, the import ledger
  sharing.rs        Share Links, Sheets, Bundles
  shopping_list.rs  the shopping list
  search.rs         word search and Meaning Search
  keeping_backups.rs
                    taking and listing Backups
  shared.rs         the few helpers several areas call and none owns
src/crouton.rs      reading a Crouton export into import candidates: lines rebuilt
                    from split fields, favicons and nutrition text left out (#69)
src/db.rs           the SQLite file under /data — WAL on, migrations forward-only
src/entities.rs     the HTML entities importers undo in text that was never HTML
src/jobs.rs         slow work: two lanes (one for strangers, ADR 0032), progress, results
src/language.rs     a Branch's Language, read off the recipe's own text (ADR 0006)
src/meaning.rs      Meaning Search: the model Kamosu does not ship, and the
                    index it builds from the recipes (ADR 0029)
src/operations.rs   the functions the Catalogue's declarations name
src/schema.rs       the Catalogue's declared input, compiled into the thing
                    that checks it — shape only, in the Core at dispatch, so
                    both Doors inherit it (#85)
src/reading.rs      reading an Ingredient Line into a Reading — where the
                    number ends, the Unit ends and the Food begins. Its
                    vocabulary is units.rs's, not its own (ADR 0036)
src/web_door.rs     Axum router generated by walking the Catalogue
src/mcp_door.rs     MCP endpoint generated by walking the Catalogue
src/config.rs       the four optional environment variables
src/http_min.rs     dependency-free HTTP client for the self-run healthcheck
src/design_tokens.rs embeds and serves the generated stylesheet, fonts and icons
src/interface.rs    embeds and serves the built Svelte app; owns the fallback
tests/parity.rs     both Doors materialise every Operation — drift breaks the build
tests/behaviour.rs  behaviour through real Doors against a real database file
tests/*_corpus.rs   the same, against the real 86-recipe Crouton export —
                    #[ignore]d, since samples/crouton/ is personal and gitignored:
                    run with `cargo test --test <name> -- --ignored`.
                    They carry `test = false`, so `just test` does not build
                    them; `just check` type-checks them instead. Naming one
                    still builds and runs it, exactly as that command does.
                    meaning_corpus.rs also wants a network and ~220 MB of model,
                    which it keeps in .dev/meaning-model between runs
ui/                 the Svelte app; ui/src/app.css is the design tokens' source
docs/svelte/        Svelte's documentation for models, pinned (read this first)
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

## Changing what a recipe holds

A Version's id is the fingerprint of its content, and that is the whole of how
two people who never spoke are known to hold the same recipe (ADR 0004). **A
field holding nothing is no part of it** (ADR 0038): `null`, the empty list and
the empty object are stripped before the hash, so a field added to a recipe
starts out empty on everything already written and moves no existing id.

That makes the ordinary case free and leaves exactly one trap. **Before adding a
field to `parse_recipe_content`, ask what it defaults to on a recipe that
predates it.**

- **Empty (`null` or `[]`) — the normal answer.** Nothing else to do. No id
  moves, no migration, no Bundle already written is orphaned.
- **Anything else** — `0`, `"metric"`, `false` — **re-fingerprints the entire
  library.** Every Version id changes, and every row naming one has to change
  with it: `branches`, `branch_versions`, `readings`, `attempts`,
  `meaning_vectors`. Migration 29 is the worked example and the only time
  Kamosu has done it. Either give the field an empty default instead, or write
  that migration deliberately and say so in an ADR.

`a_field_added_to_a_recipe_moves_no_existing_id` in `src/core/recipes.rs` fails the build
on the second case. Two behaviour tests guard the rest: one sweeps a database
carried forward from an earlier schema, which fails if a migration rewrote
`versions.content` without re-fingerprinting it, and one drives every path that
writes a Version and checks the same thing on what the Doors produced. `version_fingerprint(content)`
is registered on every SQLite connection, so the same question can be asked of
a live instance:

```sh
sqlite3 /data/kamosu.db 'SELECT COUNT(*) FROM versions WHERE id <> version_fingerprint(content);'
```

It answers zero from a Kamosu binary, and only from one — the function is
Kamosu's, not SQLite's.

## Meaning Search ships no model

**Kamosu ships no weights** (ADR 0029), so nothing about EmbeddingGemma is in
the image, in the repository, or in a Backup. An Operator accepts Google's
terms through an ordinary Operation at either Door, `download_meaning_model`
fetches ~220 MB into `/data/model/` at a **pinned revision**, and
`build_meaning_index` reads the library. An instance that never does any of
that searches by words and is complete — which is the state every instance
starts in and most stay in.

Two consequences worth knowing before touching `src/meaning.rs`:

- **The index is derived, never truth.** Every vector rebuilds from the recipes,
  which is what makes turning Meaning Search off a thing that discards nothing.
  It is brought level with the library on a short tick rather than inside a
  save, because re-embedding a recipe is seconds of model inference and a save
  is an Immediate Operation.
- **The database lock is never held across inference.** `build` reads what is
  owed, releases, embeds, and takes the lock again to write one recipe's rows.
  Holding it across a first build would freeze the instance for minutes.
- **Exactly one build runs at a time**, held by a mutex on `MeaningSearch`. What
  is owed is read before anything is written, so two builds would find the same
  recipes owing and put every one of them in the index twice.

What it costs an instance that turns it on: about **220 MB** on disk under
`/data/model/`, a few hundred megabytes of memory once the model is loaded, and
a first index build measured in minutes on a small machine — 86 recipes took
several on the dev host. An instance that leaves it off pays none of that: the
model is never loaded and the tick that keeps the index level returns
immediately.

## Where the truth lives

**The database is the truth; the Vault is a published copy** (ADR 0003). This
inverts Hatchdoor, this project's usual precedent — if you know Hatchdoor, do
not assume its Markdown-is-truth rule applies here. Deleting Kamosu's database
loses every account, cooking history and shopping list, and no Vault rebuilds
them.

Vocabulary lives in `CONTEXT.md`; decisions live in `docs/adr/`. Where the spec
and an ADR disagree, the ADR wins.
