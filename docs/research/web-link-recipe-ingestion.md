# Pulling a recipe from a web link — what already exists

**Ticket:** GitHub issue #3 — "Research: pulling a recipe from a web link — what already exists."
**Purpose:** Kamosu's wish-list asks it to "automatically ingest a recipe from a supplied web link." Kamosu's standing preference is to not reinvent the wheel where the cost/benefit of an existing tool is good. This document surveys what already exists for (a) scraping structured recipe data off a web page and (b) parsing a free-text ingredient line into amount/unit/food, assesses what each costs to integrate into a Rust backend, and lands on a concrete recommendation — including whether this feature is cheap enough to belong in v1.
**Method:** Primary-source research only — the actual schema.org/Recipe property table, cloned library source code (counted and read directly, not taken from READMEs), package-registry APIs (crates.io, npm, PyPI), GitHub repo metadata via the GitHub API, and one real recipe page's JSON-LD fetched and parsed directly. Every claim is cited to a URL or file path; source-code claims use commit-hash permalinks. Confidence is marked explicitly wherever a source is anything less than "read directly in the actual code or a live HTTP response."

---

## Summary — single most decision-relevant finding

**Getting the JSON-LD off a page is the easy 80%; turning its ingredient lines into structured data is the hard, unavoidable 20% — and Kamosu already owns the second problem regardless of what scrapes the first.** A large majority of real recipe sites publish `schema.org/Recipe` JSON-LD, and schema.org itself declares `recipeIngredient` as `Text` among its accepted types (confirmed on the live property page) — so even a perfect scrape yields free-text lines like `"120g butter softened"`, never split fields. This means:

- A **thin Rust-native JSON-LD extractor** (`scraper` crate + a `<script type="application/ld+json">` filter + `@type: Recipe`/`@graph` handling) captures the great majority of `recipe-scrapers`' value: of 637 site-specific scraper files inspected, **289 (45%) are ≤10-line stubs that do nothing but register a domain** — every field is filled by the library's built-in schema.org fallback plugin, and even the 247 files (39%) with custom HTML-scraping logic mostly patch one field (usually ingredient grouping), not full-page parsing.
- The genuinely hard problem — ingredient-line parsing — is a **separate, Python-only, NLP problem** (`ingredient-parser-nlp`) that Kamosu doesn't need to solve well, because Kamosu already always preserves `original_text` (per the sibling doc, `docs/research/recipe-app-formats.md`) — a wrong split is recoverable, a missing scrape is not.
- **Recommendation in one line:** build the Rust-native JSON-LD extractor now (cheap, in-process, no runtime dependency), skip per-site scraping entirely, defer ingredient-line parsing to a follow-up ticket (it can ship later without blocking import), and treat manual entry via the source URL as the fallback for the minority of sites with no structured data — don't build an LLM fallback for v1.

---

## 1. The structured-data landscape (schema.org/Recipe)

### What schema.org actually declares

Fetched directly from [schema.org/Recipe](https://schema.org/Recipe) and [schema.org/recipeIngredient](https://schema.org/recipeIngredient) (confidence: High — read directly from the live property pages).

| Property | Declared type(s) | Description (as published) |
|---|---|---|
| `recipeIngredient` | **`ItemList` or `PropertyValue` or `Text`** | "An ingredient or ordered list of ingredients and potentially quantities used in the recipe, e.g. 1 cup of sugar, flour or garlic." Supersedes the older `ingredients` property. |
| `recipeInstructions` | `CreativeWork` or `ItemList` or `Text` | A step in making the recipe, as a single item or an ordered list of `HowToStep`/`HowToSection` items. |
| `recipeYield` | `QuantitativeValue` or `Text` | The quantity produced by the recipe (servings, items, etc.). |
| `cookTime` | `Duration` | Time to actually cook the dish, in ISO 8601 duration format. |
| `prepTime` | `Duration` | Time to prepare the items used in the recipe (inherited from `HowTo`), ISO 8601. |
| `totalTime` | `Duration` | Total time including preparation (inherited from `HowTo`), ISO 8601. |
| `nutrition` | `NutritionInformation` | Nutrition information about the recipe. |
| `recipeCategory` / `recipeCuisine` / `cookingMethod` | `Text` | Free-text classification fields. |
| `suitableForDiet` | `Diet` or `RestrictedDiet` | Dietary restriction/guideline enum. |

Two things worth flagging precisely:

1. `recipeIngredient`'s declared type set is **wider than "just `Text`"** — it formally permits `ItemList` and `PropertyValue` too (i.e., a schema-compliant publisher *could* emit a structured amount+unit object per ingredient). This is worth stating precisely rather than the common shorthand "it's just a string," because it means the spec doesn't forbid structure — publishers simply don't use it (next section).
2. `cookTime`/`prepTime`/`totalTime` are typed `Duration`, i.e. ISO 8601 (`PT12M`), not free text — this is a real, useful, machine-parseable field, confirmed against a live example below.

### What real pages actually emit

Two independent sources, both first-party:

- **Google's own structured-data documentation** ([developers.google.com/search/docs/appearance/structured-data/recipe](https://developers.google.com/search/docs/appearance/structured-data/recipe)) — a first-party, authoritative reference for what real markup looks like — shows `recipeIngredient` as a plain **array of strings**: `["400ml of pineapple juice", "100ml cream of coconut", "ice"]`. Confidence: High (read directly).
- **A real, live fetch of bbcgoodfood.com** performed for this research (2026-08-20, `curl` with a browser user-agent, no JS execution) — the actual `<script data-testid="page-schema" type="application/ld+json">` block on `https://www.bbcgoodfood.com/recipes/chocolate-chunk-cookies` contains:

```json
"recipeIngredient": ["120g butter softened", "75g light brown sugar", "75g golden caster sugar",
  "1 medium egg", "1 tsp vanilla extract", "180g plain flour", "½ tsp bicarbonate of soda",
  "150g dark chocolate cut into chunks"],
"cookTime": "PT12M", "prepTime": "PT20M", "totalTime": "PT32M",
"nutrition": {"@type": "NutritionInformation", "calories": "308 calories",
  "fatContent": "16 grams fat", "sodiumContent": "0.5 milligram of sodium", ...}
```

Confidence: High — this is a real HTTP response captured and parsed directly, not a secondary description. It confirms:
- `recipeIngredient` lines are indeed flat free-text strings ("120g butter softened") with quantity, unit, food name, and prep note all fused together — never split fields, exactly as the sibling doc's KitchenOwl finding would predict if a site tried to store this directly.
- `cookTime`/`prepTime`/`totalTime` really are ISO 8601 durations in practice, not just in spec — a genuinely reliable machine-readable field.
- `nutrition` values are **strings with embedded units** ("308 calories", not `308`), matching Mealie's design choice (all-string nutrition fields) documented in the sibling doc, and undermining any assumption that nutrition import can be purely numeric.
- `recipeInstructions` is a real structured step list (`HowToStep` objects), and — notably, contradicting nothing in the four-app survey but adding a data point — **each step here carries its own `image` field**, something the sibling doc found no *app* supports at the step level (Tandoor's `Step.file` was the closest, and it's dropped on export). Real-world schema.org data is sometimes richer than any of the four surveyed apps' native models.
- **A methodological finding worth keeping in the extractor design**: the matching `<script>` tag's attributes were ordered `data-testid="page-schema" type="application/ld+json"`, not `type=` first. A naive text search anchored to attribute *position* misses it; a proper CSS attribute selector (`script[type="application/ld+json"]`, which `scraper`/`select.rs` both support) does not. There were also two *other* `<script type="application/json">` blocks on the same page carrying overlapping-looking data (a Next.js `__POST_CONTENT__`/`__NEXT_DATA__` payload) that are not valid JSON-LD documents at all — a real extractor must filter by `type="application/ld+json"` specifically and not just grep for `"@type":"Recipe"` anywhere in the HTML.

**Central finding for this section**: structured qty/unit/food extraction is **always** a downstream parsing problem, never something a good scrape avoids. Even schema.org's own permissive typing (`ItemList`/`PropertyValue` allowed) doesn't change real-world practice — every real example found, and Google's own documented example, uses flat text. This directly validates Kamosu's `original_text`-preserving ingredient model (sibling doc, `docs/research/recipe-app-formats.md`): the scrape can only ever hand Kamosu a raw line: something has to parse it, and something must also keep the raw line, because sites will keep publishing it that way regardless of how good the scraper is.

### How widely is this published?

Confidence: **Medium** — no hard, citable adoption percentage was found for "what fraction of recipe sites publish schema.org/Recipe data." The strongest available signal is indirect: Google's structured-data documentation states plainly that `image` and `name` are the only *required* fields for a page to be eligible for recipe rich-result treatment, and lists a long set of *recommended* fields including `recipeIngredient`/`recipeInstructions`/`nutrition` ([Google recipe structured data docs](https://developers.google.com/search/docs/appearance/structured-data/recipe)) — a strong commercial incentive (rich snippets, recipe carousels) has existed since Google introduced this feature, and `recipe-scrapers`' own scale (637 site-specific classes, see §2) with the finding that 45% of those classes are trivial schema-only stubs is itself indirect evidence that JSON-LD/schema.org coverage across mainstream recipe-publishing sites is high. No independent, dated adoption survey was located; treat "most mainstream recipe sites publish this" as a reasonable inference from these two signals, not a measured fact.

---

## 2. Existing scraping tools

### `recipe-scrapers` (Python) — the reference implementation

**Repo:** `hhursev/recipe-scrapers` — confirmed as the current canonical location via the GitHub API (not renamed/moved) and cross-checked against a companion TypeScript port that names it explicitly as the project it ports (§ below). Cloned at commit `8a7c646342614ccf49d8b938d90dde4dfc27bef4` (`HEAD` as of 2026-08-19). Permalinks below use `https://github.com/hhursev/recipe-scrapers/blob/8a7c646342614ccf49d8b938d90dde4dfc27bef4/<path>#L<n>`.

- **Licence**: MIT, read directly from [`LICENSE`](https://github.com/hhursev/recipe-scrapers/blob/8a7c646342614ccf49d8b938d90dde4dfc27bef4/LICENSE). Confidence: High.
- **Coverage, counted directly (not the README's number)**: `find recipe_scrapers -maxdepth 1 -name "*.py"` returns **648 files**; subtracting the 11 underscore-prefixed support/internal modules (`_abstract.py`, `_schemaorg.py`, `_factory.py`, `_opengraph.py`, `_wprm.py`, `_grouping_utils.py`, `_utils.py`, `_exceptions.py`, `_warnings.py`, `__version__.py`, `__init__.py`) leaves **637 per-site scraper files**. The registry dict `SCRAPERS` in [`recipe_scrapers/__init__.py#L675`](https://github.com/hhursev/recipe-scrapers/blob/8a7c646342614ccf49d8b938d90dde4dfc27bef4/recipe_scrapers/__init__.py#L675) has **738 host-domain entries** mapping to **636 distinct scraper classes** (some classes register multiple country-domain aliases, e.g. `AlbertHeijn.host()` plus `AlbertHeijn.host(domain="ah.be")`). Confidence: High — counted directly from the cloned tree.
- **Last release / activity**: latest tag `15.12.0`, 2026-08-08; most recent commits are from 2026-08-19 (the day of cloning), e.g. `8a7c646` "Adds support for marieclaire (#2048)"; **48 commits in the last 90 days**, 35 in the last 30. Confidence: High — read from `git log`/`git tag` directly.
- **Site-specific vs. generic fallback — the key architectural finding**: `recipe-scrapers` is **not** "every supported site needs its own hand-written class." Every `AbstractScraper` subclass's un-overridden methods (`title`, `ingredients`, `instructions`, `nutrients`, etc.) automatically fall through to a `SchemaOrgFillPlugin`, attached to *every* scraper (`run_on_hosts = ("*",)`) via the default plugin chain in [`recipe_scrapers/settings/default.py`](https://github.com/hhursev/recipe-scrapers/blob/8a7c646342614ccf49d8b938d90dde4dfc27bef4/recipe_scrapers/settings/default.py) and implemented in [`recipe_scrapers/plugins/schemaorg_fill.py`](https://github.com/hhursev/recipe-scrapers/blob/8a7c646342614ccf49d8b938d90dde4dfc27bef4/recipe_scrapers/plugins/schemaorg_fill.py): on `NotImplementedError`, it calls the matching method on `self.schema` (a `SchemaOrg` object built from JSON-LD/microdata via the `extruct` library, [`recipe_scrapers/_schemaorg.py`](https://github.com/hhursev/recipe-scrapers/blob/8a7c646342614ccf49d8b938d90dde4dfc27bef4/recipe_scrapers/_schemaorg.py)). Concretely: `recipe_scrapers/allrecipes.py` and `recipe_scrapers/seriouseats.py` are each **7 lines** — a class with only a `host()` classmethod — and every field they return comes from schema.org data, not custom parsing. Counted across all 637 site files: **289 (45%) are ≤10 lines** (pure registration stubs), and only **247 (39%) call `self.soup.find`/`self.soup.select`** anywhere (i.e. touch the raw HTML at all) — meaning **61% of the library's "site support" is really just a domain→schema.org-fallback registration**, not bespoke scraping logic. Confidence: High — counted and read directly.
- **Explicit generic/wild-mode fallback for *unsupported* domains, too**: `scrape_html(..., supported_only=False)` (deprecated alias `wild_mode=True`) routes unknown hosts to `SchemaScraperFactory.SchemaScraperFactory.generate()` ([`recipe_scrapers/_factory.py`](https://github.com/hhursev/recipe-scrapers/blob/8a7c646342614ccf49d8b938d90dde4dfc27bef4/recipe_scrapers/_factory.py)), a small class whose every method (`title`, `ingredients`, `instructions`, `image`, `yields`, …) is a one-line `return self.schema.<field>()`. If no schema.org data is found it raises `NoSchemaFoundInWildMode` rather than attempting HTML heuristics. Confidence: High — read directly.

**Interpretation for §4 below**: the ratio (45% trivial stubs, only 39% doing any real HTML scraping) is direct evidence for how much of `recipe-scrapers`' value a bare JSON-LD extractor captures on its own — see §4.

### Rust — no credible dedicated crate exists

crates.io search (`https://crates.io/api/v1/crates?q=recipe`, `q=ingredient`; API, not the JS search UI) turned up nothing production-credible for *scraping*:

| Crate | Description (as published) | Last updated | Assessment |
|---|---|---|---|
| [`rust-recipe`](https://github.com/BreD1810/rust-recipe) | "A Rust crate that scrapes recipes from websites." | 2024-01-02 | 4,033 downloads total, single-author side project, ~2.5 years stale as of this research. Not credible as a dependency. |

No other recipe-*scraping* crate was found. (Several unrelated crates literally named `recipe`/`recipes`/`*recipe*` exist for build-system "recipes," package-manager "recipes," etc. — excluded as irrelevant homonyms, per the "be honest, don't pad" instruction.) Confidence: High — searched via the crates.io API directly, not a secondary blog list.

### JS/TS — a real but small ecosystem, split between full ports and light JSON-LD wrappers

npm registry search (`registry.npmjs.org/-/v1/search`) plus direct package/registry lookups:

| Package | Repo | Licence | Approach | Last publish | Maintenance signal |
|---|---|---|---|---|---|
| [`recipe-scrapers`](https://www.npmjs.com/package/recipe-scrapers) (org: `recipe-scrapers/recipe-scrapers`) | `github.com/recipe-scrapers/recipe-scrapers` | MIT | Explicit TypeScript port of the Python project — README states "Supported hosts are registered in `src/scrapers/_index.ts`, split between custom scrapers and Schema.org-only hosts" (WebFetch of the README, confidence: Medium — read via fetch/summary, not the raw file). | v1.10.0, 2026-07-23 | Actively published, but **6 GitHub stars** — very low adoption for a young project (confirmed via GitHub API: `archived: false`, `pushed_at: 2026-07-23`). |
| [`@jitl/recipe-data-scraper`](https://www.npmjs.com/package/@jitl/recipe-data-scraper) | `github.com/thelifenadine/recipe-data-scraper` | Apache-2.0 | Uses `cheerio` (HTML parsing) + `microdata-node`, i.e. a generic microdata/JSON-LD-style extractor, not per-site classes. | v2.0.0, **2022-09-23** | Stale — no publish in ~4 years. |
| `@rethora/url-recipe-scraper`, `@dimfu/recipe-scraper` | — | MIT | Small, low-adoption personal projects (`@dimfu` also microdata/cheerio-based, last published 2023-06). | 2023–2024 | Low-effort/low-adoption; not evaluated further. |

Confidence: Medium overall (npm registry metadata is High confidence; the `recipe-scrapers` TS repo's architecture claim is Medium, sourced from a fetched README rather than the raw file). **None of these are a stronger option than doing the equivalent directly in Rust** (§4) — the most credible one (`recipe-scrapers` TS) has 6 stars and exists mainly as a faithful mirror of the Python project's architecture, which is itself dominated by the schema.org fallback as shown above.

### Generic JSON-LD / microdata / HTML-parsing crates — the real Rust building blocks

These aren't recipe-specific, but they're exactly what a thin Rust-native extractor would be built from. All figures from the crates.io API directly.

| Crate | Purpose | Downloads | Latest version | Last publish |
|---|---|---|---|---|
| [`scraper`](https://crates.io/crates/scraper) | HTML parsing + CSS selectors (the Rust equivalent of BeautifulSoup/cheerio) | 26,014,259 | 0.27.0 | 2026-05-11 |
| [`select`](https://crates.io/crates/select) | Alternative HTML/CSS-selector scraping library | 1,764,867 | 0.6.1 | 2025-03-19 |
| [`json-ld`](https://crates.io/crates/json-ld) | Full JSON-LD 1.1 processor (expansion/compaction/RDF) | 559,252 | 0.21.4 | 2026-02-19 |
| [`oxjsonld`](https://crates.io/crates/oxjsonld) | Lighter-weight JSON-LD parser/serializer | 682,974 | 0.2.5 | 2026-04-19 |
| [`microdata`](https://crates.io/crates/microdata) | Parses HTML microdata annotations | 1,417 | 0.1.1 | 2025-05-31 |

Confidence: High — all counts/dates read directly from the crates.io API. **For Kamosu's purpose, full JSON-LD 1.1 semantics (`json-ld`/`oxjsonld`) are overkill** — recipe JSON-LD in the wild is a flat, non-expanded document; `serde_json::from_str` plus a small amount of manual `@graph`/array-of-`@type` handling is sufficient (this is in fact exactly what `extruct`, the Python library `recipe-scrapers` itself depends on for JSON-LD, effectively does — [PyPI: `extruct`](https://pypi.org/project/extruct/), confidence Medium, summary only). `scraper` is the right building block for finding the `<script type="application/ld+json">` tags themselves (via a CSS attribute selector, robustly handling attribute order — see the bbcgoodfood example in §1).

---

## 3. Ingredient-line parsing

This is the second, separate, harder problem: `recipe-scrapers`/JSON-LD gets you `"120g butter softened"` as a string. Turning that into `{quantity: 120, unit: "g", food: "butter", note: "softened"}` is a distinct NLP problem that scraping quality does not touch.

### `ingredient-parser-nlp` (Python) — the tool KitchenOwl actually uses

**Repo:** confirmed as `strangetom/ingredient-parser` (PyPI package name `ingredient-parser-nlp`; this is the same tool the sibling doc found KitchenOwl calling in `backend/app/service/ingredient_parsing.py`). Cloned at commit `ffd6ae3c6efb9925c40fc9b4454d77b40469ef91` (`HEAD` as of 2026-05-25 — note this is the last commit on the default branch as fetched by `git clone`, i.e. the tip at clone time; the GitHub API separately reports the repo's most recent push as 2026-08-19, meaning further commits landed after this clone — see maintenance signal below). Permalinks use `https://github.com/strangetom/ingredient-parser/blob/ffd6ae3c6efb9925c40fc9b4454d77b40469ef91/<path>#L<n>`.

- **Licence**: MIT, read directly from [`LICENSE`](https://github.com/strangetom/ingredient-parser/blob/ffd6ae3c6efb9925c40fc9b4454d77b40469ef91/LICENSE). Confidence: High.
- **Accuracy claim**: the README states, on a held-out 20% test split of the project's own ~81,000-sentence training dataset: **95.62% sentence-level accuracy, 98.26% word-level accuracy** (`README.md`, lines quoted verbatim above the "Development" heading). Confidence: **Medium — this is a self-reported number on the project's own dataset/split, not an independently benchmarked or third-party-validated figure.** No independent benchmark dataset or third-party evaluation was found; treat the number as "plausible and specific" rather than "verified."
- **Model type and runtime cost — read directly, not assumed**: it is a **CRF (conditional random field)** model, not a neural network — `train/export.py` imports `pycrfsuite` and defines `CRFModelParameters`/`export_crfsuite_to_json` ([`train/export.py`](https://github.com/strangetom/ingredient-parser/blob/ffd6ae3c6efb9925c40fc9b4454d77b40469ef91/train/export.py)); `requirements.txt` lists `python-crfsuite` and `scikit-learn`. The trained model ships as a **gzipped JSON file, 288 KB** (`ingredient_parser/en/data/model.en.json.gz`), inside a `data/` directory totalling **3.8 MB** (the bulk of which, 3.4 MB, is a GloVe-style word-embedding lookup table used as a model feature, `ingredient_embeddings.35d.glove.txt.gz`). This is an extremely light model — CRF inference is a fast linear-chain sequence-tagging pass, not a forward pass through a neural network, so per-line latency and memory footprint are both trivial by ML standards (no GPU, no PyTorch runtime needed — the whole dependency chain is `python-crfsuite` + `scikit-learn` + `nltk`/`pint` for pre/post-processing, per `pyproject.toml`). Confidence: High — read directly from source and file sizes measured directly.
- **Maintenance**: actively developed — GitHub API reports `pushed_at: 2026-08-19` (the day of this research), 163 stars, only 2 open issues; PyPI shows regular releases through 2026 (`2.3.0` 2025-09, `2.4.0` 2025-10, `2.5.0` 2026-01, `2.6.0` 2026-03, `2.7.0` 2026-05). Confidence: High — read from the GitHub and PyPI APIs directly.

### The NYT `ingredient-phrase-tagger` lineage — confirmed superseded

- [`nytimes/ingredient-phrase-tagger`](https://github.com/nytimes/ingredient-phrase-tagger): GitHub API confirms **`archived: true`**, with the last real code push on **2018-07-19** (the `updated_at` timestamp of 2026-08-14 is GitHub metadata churn, not a code change — `pushed_at` is the reliable signal). 805 stars — historically influential, but dead. Confidence: High — read from the GitHub API directly.
- [`mtlynch/ingredient-phrase-tagger`](https://github.com/mtlynch/ingredient-phrase-tagger) (a maintained-sounding fork, adds Docker/CI/tests per its own description): **not archived**, but `pushed_at: 2020-12-11` — no commits in ~5.5 years as of this research. 177 stars. Confidence: High.
- The ecosystem has visibly moved on: `ingredient-parser-nlp` is the tool an actively-developed sibling project (KitchenOwl) chose to depend on in 2024+ rather than the NYT tagger or its forks (per the sibling doc's direct source read of `backend/app/service/ingredient_parsing.py`), and it is the only one of the three still receiving releases. Confidence: High that the NYT lineage is dead; Medium (inference, not a documented migration announcement) that "the ecosystem moved to `ingredient-parser-nlp` specifically" as opposed to simply "NYT's tagger is unmaintained and better options exist."

### Rust — nothing production-credible

crates.io search (`q=ingredient`) surfaced three candidates, none viable as-is:

| Crate | Repo | Last updated | Assessment |
|---|---|---|---|
| [`ingredient`](https://crates.io/crates/ingredient) | `github.com/nickysemenza/ingredient-parser` | 2023-04-18 | 22,150 downloads (respectable historically) but no publish in over 3 years. |
| [`ingreedy-rs`](https://crates.io/crates/ingreedy-rs) | `github.com/Ninjani/ingreedy-rs` | 2021-08-03 | Rust port of an older JS parser ("ingreedy"); 3,042 downloads, unmaintained since 2021 (~5 years). |
| [`recipe_parser_andrewromanyk`](https://crates.io/crates/recipe_parser_andrewromanyk) | — | 2024-11-10 | Splits a whole recipe into name/description/ingredients/instructions sections, not per-ingredient amount/unit/food parsing — wrong problem. |

Confidence: High (crates.io API + repo metadata) that **no actively-maintained, credible Rust ingredient-line parser exists.** Writing one from scratch as a CRF/rule-based model, or wrapping the Python one, are the only real options — see §4.

### What lowers the stakes here for Kamosu specifically

Per the sibling doc, Kamosu's differentiator is that `original_text` is **always** preserved regardless of how the structured fields turn out. This means:
- A parser that's "good enough" (say, `ingredient-parser-nlp`'s ~96% sentence accuracy) is genuinely fine for v1 — the ~4% of lines it gets wrong are not data-loss events, they're mildly-wrong structured fields sitting next to a correct raw line the user (or a later re-parse) can always fall back to.
- This is fundamentally different from Tandoor's own export bug (dropping `original_text` on export, per the sibling doc) — Kamosu's design explicitly avoids that failure mode, which is exactly what makes "imperfect parser, always-reversible" an acceptable trade rather than a risk.

---

## 4. Integration cost for Kamosu (Rust backend)

Kamosu's backend is Rust; the best ingredient-parsing option (`ingredient-parser-nlp`) is Python-only, and (per §2) even the strongest full-scraping option (`recipe-scrapers`) is Python-only. Four integration shapes, compared honestly:

| Option | What it costs | Verdict for Kamosu |
|---|---|---|
| **Sidecar Python container** (separate service in `docker-compose`) | A second image to build/pin/update/patch-for-CVEs, a second runtime to keep alive, inter-process HTTP/RPC plumbing, a second set of logs/health checks. For a *self-hosted* app this is a real tax on every install — more moving parts than a single-binary Rust app promises. | Justifiable **only** if the Python dependency is doing something substantial and hard to replace (see below) — not justified for JSON-LD extraction alone. |
| **Ad-hoc subprocess** (Python script bundled in the same image, invoked via `std::process::Command`) | Avoids a second container, but the image now needs a Python interpreter + `pip` deps baked in (`python-crfsuite`, `scikit-learn`, `nltk`, `pint` for `ingredient-parser-nlp` — all pure-Python/C-extension, no GPU/torch needed, so this is a modest image-size add, not a multi-GB one) — image size grows, plus subprocess-per-call latency/startup overhead (CRF inference itself is fast; interpreter startup is the real cost, mitigated by keeping a long-lived worker process rather than spawning per-request). | The pragmatic middle ground **if** ingredient parsing is wanted — light dependency chain, no second container, no FFI complexity. |
| **PyO3 embedding** (Python interpreter embedded in the Rust binary) | `pyo3` is a mature, extremely widely used crate (232M+ downloads, actively maintained, v0.29.2 as of 2026-08 — confirmed via the crates.io API) — technically solid. But it couples the Rust binary's build/release process to a Python runtime and its C-extension deps (`crfsuite`, `scikit-learn` binary wheels) at compile *and* runtime, which is a heavier, more fragile build than a subprocess call, for no clear latency win over a long-lived subprocess worker. | Not worth the complexity for this use case — the subprocess approach gets the same result more simply. |
| **Rust-native JSON-LD extractor** (`scraper` + a `<script type="application/ld+json">` filter + manual `@type`/`@graph` handling, mapped onto Kamosu's schema) | Pure Rust, in-process, no new runtime, no new image, no FFI. Only needs to handle: locate `<script type="application/ld+json">` tags (CSS attribute selector, order-independent — see §1's bbcgoodfood finding), parse each as JSON, find the node whose `@type` is/contains `"Recipe"` (handling both a bare object and an `@graph`-wrapped array — both observed directly: bbcgoodfood.com used a bare `@type: Recipe` object; `recipe-scrapers`' own `_schemaorg.py` explicitly handles the `@graph` variant too, per `_find_entity()`), then map fields straight onto Kamosu's model (`recipeIngredient` → `original_text` list, `cookTime`/`prepTime`/`totalTime` ISO-8601-parsed, etc). | **This is the recommended v1 approach** — see below. |

### How much of `recipe-scrapers`' value does this actually capture?

Directly answerable from §2's counts: **45% of `recipe-scrapers`' site coverage (289/637 files) is *purely* the schema.org fallback with zero custom code** — a Rust-native JSON-LD extractor reproduces those sites' results exactly, at zero incremental cost per site (no per-site class to write or maintain, ever). Of the remaining 55%, **247 files (39% of the total) add custom HTML scraping** — but reading a sample of these (`bbcgoodfood.py`, 16 lines) shows the custom code is typically a small patch to *one* method (e.g. `ingredient_groups()` for section-header grouping), not a full reimplementation — the rest of that class's fields (title, times, nutrition, instructions) still come from the same schema.org fallback plugin. The remaining ~16% of files (637 − 289 − 247, i.e. classes with no `soup.find`/`soup.select` calls but more than 10 lines — likely custom `to_json`/formatting logic rather than raw-HTML scraping) were not individually sampled; treat that slice as unquantified. Confidence: High for the 45%/39% split (counted directly); Medium for the qualitative claim that most of the 39% custom code is narrow single-field patches rather than full re-scrapes (based on sampling 3 files, not all 247).

**Conclusion**: a Rust-native JSON-LD-only extractor captures the majority of `recipe-scrapers`' effective coverage without reimplementing any of its 636 hand-written classes, at the cost of failing gracefully (not scraping at all) on the minority of sites with genuinely no structured data or with structured data so broken it needs bespoke HTML heuristics to recover.

---

## 5. LLM fallback for non-structured sites

**Verdict: not worth it for v1; potentially worth it much later, and only with a local model, never a required paid API key.**

Reasoning:

- **The self-hosted constraint is real and binding.** Requiring a paid LLM API key to use a "supply a URL" feature is a genuine imposition on a self-hosted app's users — some won't have one, some won't want to pay per-import, and it silently breaks for anyone without connectivity to that provider. If an LLM fallback exists at all, it has to be local-only or fully optional.
- **Local generative models capable of decent structured extraction from raw HTML/text are a different weight class from embedding models**, even within the same household of projects. Hatchdoor already bundles local embedding models via `fastembed` (noted per the task brief, not independently re-verified here) — and this research *did* pull real numbers for that comparison: `fastembed`'s own supported-models table (confidence: Medium — summarized via fetch of the published docs page, not the raw file) lists its smallest/most common models at **67–90 MB** (`BAAI/bge-small-en-v1.5` at 0.067 GB, `sentence-transformers/all-MiniLM-L6-v2` at 0.090 GB), with even its *largest* supported embedding model at 2.24 GB. Against that, the smallest Ollama-distributed models with plausible enough instruction-following to do structured extraction reliably start around **1.3 GB** (Llama 3.2, 1B parameters) to **2.0 GB** (Llama 3.2, 3B — the size Ollama defaults to as "latest," implying 1B alone isn't considered good enough by default) and reach **4.7 GB** for a model most people would trust for a genuinely fiddly extraction task (Qwen2.5 7B) — figures read directly from Ollama's library pages for those models. That's roughly **20–70× the size of the embedding models Hatchdoor already ships**, for a fundamentally heavier kind of inference (autoregressive generation over hundreds of tokens of HTML/text, vs. one forward pass to a fixed-size vector). "Hatchdoor already does local inference" does not transfer to "a local LLM fallback is cheap" — it's a different weight class, not an incremental step.
- **The problem it would solve is already small and already has a graceful answer.** Per §1/§2, the sites that need this fallback are specifically those with *no* schema.org/JSON-LD data at all — plausibly a minority given the apparent breadth of adoption (§1's adoption discussion, Medium confidence) — and for exactly those sites, Kamosu's existing design already has a non-broken answer: **fail gracefully and hand the user the source URL for manual entry.** That's not a degraded experience relative to every recipe app studied in the sibling doc — none of Mealie/Tandoor/Crouton/KitchenOwl claim to solve "scrape a site with zero structured data" either; they all rely on the same JSON-LD/microdata substrate `recipe-scrapers` does.
- **Cost/benefit for v1 specifically**: shipping a multi-gigabyte local model download as a prerequisite for a "paste a link" feature, to cover a minority of sites, when the fallback is "ask the user to type it in" (which they'd have had to do anyway before this feature existed), is a poor trade for a v1 feature. This is exactly the kind of scope creep issue #12 ("The v1 cut line") is meant to catch — see Recommendation.
- **Later, maybe.** If Kamosu ever wants "works on literally any recipe page," a local LLM fallback becomes more defensible once (a) local model sizes for capable structured extraction shrink further, or (b) usage data shows a meaningful fraction of real import attempts hitting sites with no structured data at all. Neither condition is established today; this is a deferred, not rejected, idea.

---

## Confidence key

- **High**: read directly from source code in a cloned repo, a live HTTP response fetched and parsed for this research, or a package-registry/GitHub API response queried directly.
- **Medium**: a claim taken from a first-party but AI-summarized fetch (e.g. a README or docs page read via a fetch-and-summarize tool rather than the raw file), a self-reported benchmark/accuracy number not independently verified, or a reasonable inference drawn from two or more High-confidence facts rather than stated outright by any one source.
- **Low**: not used in this document as a standalone rating — every claim below Medium is explicitly flagged inline as "unquantified," "not found," or similar rather than silently asserted.

---

## Recommendation

**Build the Rust-native JSON-LD extractor now. Do not build or wrap `recipe-scrapers`' per-site scraper classes. Do not require a paid LLM API key, and do not build an LLM fallback for v1. Defer ingredient-line parsing to a follow-up ticket, not this one.**

Concretely:

1. **Scraping**: implement a small Rust module using the `scraper` crate to locate `<script type="application/ld+json">` tags (CSS attribute selector, not position-dependent text search — see §1), parse each as JSON with `serde_json`, find the `Recipe`-typed node (handling a bare object, an array of types, and the `@graph`-wrapped variant — all three are real, observed patterns), and map schema.org fields directly onto Kamosu's model: `name`→title, `recipeIngredient` (array of strings) → one `original_text`-only ingredient row per line (leave `quantity`/`unit`/`food` null at this stage — see point 3), `recipeInstructions` → ordered steps, `cookTime`/`prepTime`/`totalTime` (ISO 8601) → parsed durations, `recipeYield` → servings, `nutrition` → Kamosu's string-typed nutrition fields (matching the string-with-units reality confirmed in §1, and matching Mealie's precedent per the sibling doc), `image`, `author`, source URL. This is genuinely cheap: no per-site logic, no Python dependency, no new container, no FFI — just an HTML parser and a JSON mapper, both already necessary in a Rust web-facing app.
2. **Do not reimplement `recipe-scrapers`' 636 hand-written site classes.** §4 shows the Rust-native extractor already reproduces 45% of that library's coverage exactly (the schema-only stub sites) and most of the value in the other 55% too (custom code there is typically a narrow single-field patch on top of the same schema.org fallback, not a full reimplementation). The genuine gap — sites with real HTML-scraping needs because they have no structured data at all — is a minority, and chasing it site-by-site is precisely the maintenance burden (636 classes and counting, 48 commits in 90 days just to keep up) that "don't reinvent the wheel" is meant to avoid taking on.
3. **Ingredient-line parsing is a separate ticket, not a blocker for this one.** Shipping web-link import with `original_text`-only ingredient rows (no `quantity`/`unit`/`food` populated yet) is a legitimate, honest v1 state — it's strictly better than what a manual-entry-only Kamosu would have, and it doesn't lie about data it doesn't have. When ingredient parsing is tackled, `ingredient-parser-nlp` (§3) is the right tool to depend on — actively maintained, MIT-licensed, genuinely lightweight (CRF model, 288 KB compressed), and it's what an already-studied sibling app (KitchenOwl) has already chosen for the same job. Because Kamosu always preserves `original_text` regardless (the sibling doc's core differentiator), a CRF model with ~96% self-reported sentence accuracy is a perfectly acceptable "good enough, always reversible" choice — Kamosu should explicitly not hold out for a more accurate parser before shipping this. Integrate it as a **long-lived Python subprocess worker** (not PyO3, not a sidecar container) when that ticket is picked up: lightest dependency footprint of the four integration shapes in §4, given the model's own runtime needs are modest (no GPU, no PyTorch).
4. **Fallback for sites with no structured data: manual entry via the source URL, explicitly — not an LLM step.** §5's cost/benefit is clear: local models capable of decent extraction are 20–70× the size of the local embedding models Kamosu's household of projects already ships (Hatchdoor/`fastembed`), for a minority use case that already degrades gracefully to "the user pastes the recipe in themselves," which is no worse than every existing recipe app studied in the sibling doc. Revisit only if real usage data later shows this minority is larger than expected, or local-model size/quality tradeoffs shift meaningfully.
5. **For issue #12 ("The v1 cut line"), not issue #11 as originally framed to this research** (issue #11 is "Accounts, ownership and sharing" — a check of the actual GitHub issues via `gh api` shows the v1-scope-decision ticket is **#12**; this document uses the correct number): **web-link import, in the scoped-down form above, is cheap enough to belong in v1.** The Rust-native extractor is a self-contained, dependency-free module with no new deployment surface — it is not the kind of feature that needs the foundation to "already accommodate" something bigger, unlike (say) versioning or multi-language variants from the sibling doc's Synthesis 2. The one genuine judgment call for issue #12 is whether to ship it *with* or *without* structured ingredient parsing on day one — this document's recommendation is **ship without it** (original-text-only ingredients from web import, filled in by a later ticket), since that keeps the v1 dependency footprint at zero non-Rust runtime pieces while still delivering the actual wish-list ask ("automatically ingest a recipe from a supplied web link").

**Where confidence is weakest**: the adoption-rate claim in §1 ("most mainstream recipe sites publish schema.org data") is Medium/inferred, not a measured statistic — no authoritative percentage was found. The qualitative claim in §4 that most of `recipe-scrapers`' custom-HTML-scraper files (the 39%) are narrow single-field patches rather than full re-scrapes rests on sampling 3 of 247 files, not all of them. Neither gap changes the recommendation's direction, but both are worth a cheap sanity check (e.g. sampling another 10–15 scraper files) before this becomes a firm engineering estimate rather than a scoping recommendation.
