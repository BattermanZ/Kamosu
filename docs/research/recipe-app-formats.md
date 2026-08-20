# Recipe data models across Crouton, Mealie, Tandoor, and KitchenOwl

**Ticket:** GitHub issue #2 — "What does a recipe actually look like inside Crouton, Mealie, Tandoor and KitchenOwl — field by field?"
**Purpose:** Kamosu must import from all four apps. Their export/data formats are the closest thing we have to a survey of what a recipe *is* in practice, and this feeds Kamosu's core domain model.
**Method:** Primary-source research — official docs, source code (cloned repos), and community reverse-engineering only where no source code exists (Crouton). Every claim below is cited to a URL or file path. Confidence is marked explicitly wherever a source is anything less than "read it in the actual model/schema code."

---

## Summary — single most decision-relevant finding

**Structured ingredients (quantity / unit / food-item as separate fields, not one free-text line) are the norm, not a differentiator — with one instructive exception.**

- **Mealie**: ingredient is fully decomposed — `quantity`, `unit` (its own referenced entity), `food` (its own referenced entity), `note`, **plus `original_text` always preserved verbatim alongside the structured fields**.
- **Tandoor**: ingredient is fully decomposed — `amount`, `unit` (FK to a `Unit` model), `food` (FK to a hierarchical `Food` model), `note`, and an `original_text` field on the live model — **but Tandoor's own native export explicitly drops `original_text`**, so the raw source line does not round-trip through Tandoor's own backup format.
- **Crouton** (reverse-engineered, medium confidence): ingredient is structured too — a `quantity` object (`amount` + `quantityType` enum, e.g. `GRAMS`) plus a referenced `ingredient` object (`name` + `uuid`) — much closer to Mealie/Tandoor's shape than to a plain string. No separate free-text "original line" field was found in any source, and no per-ingredient note field distinct from the ingredient name was found either (prep notes appear folded into `ingredient.name`, e.g. `"asparagus, stalks trimmed"`).
- **KitchenOwl is the outlier**: an ingredient is `{ name (→ shared Item entity), description (free-text "quantity + unit + prep", not split further), optional (bool) }`. KitchenOwl actually *runs* an NLP/LLM parser at import time that computes a real quantity/unit split internally — but the result is immediately re-flattened into one `description` string before it's ever stored. The structured data is thrown away.

**Implication for Kamosu**: three of four apps (Mealie, Tandoor, Crouton) treat quantity/unit/food-item as separate fields as their canonical, stored representation — this is table stakes, not a differentiator. What *is* a genuine differentiator is **reliably preserving the original free-text ingredient line through import/export round-trips** — Mealie does this, Tandoor's live API does but its own export doesn't, KitchenOwl never captures it as stored data at all, and Crouton's status is unconfirmed pending the real export file. Kamosu keeping `original_text` as a first-class, always-preserved field (not an afterthought some exports drop) is a real, defensible design choice grounded directly in this survey.

---

## Mealie

**Repo:** `mealie-recipes/mealie` (FastAPI backend, Pydantic schemas). Cloned at commit `4ea0770b4f159bc748fdf392a161485631d59373`, branch `mealie-next` (repo default), `pyproject.toml` version `3.23.1`. Permalinks below use `https://github.com/mealie-recipes/mealie/blob/4ea0770b4f159bc748fdf392a161485631d59373/<path>#L<n>`.

### Export file format(s)

Mealie has **two separate export systems**:

1. **Admin/full backup** (`mealie/services/backups_v2/backup_v2.py`, `backup_file.py`) — a single `.zip` named `mealie_{version}_{timestamp}.zip`. Contains `database.json`, a **raw dump of every SQLAlchemy table** keyed by internal table name (not the clean `Recipe` shape), plus a `data/` tree of all uploaded files. Meant only for Mealie-to-Mealie restore.
2. **Group/single-recipe export** (`mealie/services/exporter/recipe_exporter.py`, `mealie/services/recipe/template_service.py`) — this is the useful one. Per recipe: `recipes/<slug>/<slug>.json` (the full Pydantic `Recipe` model, `model_dump_json()`'d directly) plus that recipe's images/assets directory copied alongside. Also reachable live via `GET /api/recipes/{slug}/exports?template_name=raw|zip` (`mealie/routes/recipe/exports.py`).
3. **Direct REST API** — `GET /api/recipes/{slug}` (`response_model=Recipe`, `mealie/routes/recipe/recipe_crud_routes.py:559`) is the fastest way to see the live shape; Mealie auto-serves an OpenAPI schema at `/openapi.json`.

Encoding: UTF-8 JSON throughout. **Field naming is camelCase by default** (`MealieModel` base class sets `alias_generator=camelize`, `mealie/schema/_mealie/mealie_model.py:53`) for the live API and the "raw" export template; the internal backup `database.json` and the zip-export json are snake_case (no `by_alias`).

### Complete field list (`Recipe`, `mealie/schema/recipe/recipe.py:182-393`, extending `RecipeSummary` lines 116-176)

| Field | Type | Notes |
|---|---|---|
| `id` | UUID4 \| null | |
| `user_id`, `household_id`, `group_id` | UUID4 | ownership |
| `name` | str \| null | |
| `slug` | str | auto-generated |
| `image` | str \| null | **not a path** — a 4-char cache-busting key (see Images) |
| `recipe_servings` | float | |
| `recipe_yield_quantity` | float | |
| `recipe_yield` | str \| null | free-text yield unit |
| `total_time`, `prep_time`, `cook_time`, `perform_time` | str \| null | **free-text**, not parseable durations |
| `description` | str \| null | |
| `recipe_category` | list[RecipeCategory] | `{id, group_id, name, slug}` |
| `tags` | list[RecipeTag] | same shape as category |
| `tools` | list[RecipeTool] | adds `households_with_tool` |
| `rating` | float \| null | computed average of all users' ratings |
| `org_url` (alias `orgURL`) | str \| null | source URL |
| `date_added`, `date_updated`, `created_at`, `updated_at` | date/datetime \| null | |
| `last_made` | datetime \| null | last "cooked" event |
| `recipe_ingredient` | list[RecipeIngredient] | see Ingredients |
| `recipe_instructions` | list[RecipeStep] \| null | see Steps |
| `nutrition` | Nutrition \| null | see Nutrition |
| `settings` | RecipeSettings \| null | `public`, `show_nutrition`, `show_assets`, `landscape_view`, `disable_comments`, `locked` |
| `assets` | list[RecipeAsset] \| null | `{name, icon, file_name}` — arbitrary attachments |
| `notes` | list[RecipeNote] \| null | `{title, text}` |
| `extras` | dict \| null | arbitrary user key/value pairs |
| `comments` | list[RecipeCommentOut] \| null | see Comments |

### How ingredients are represented

`RecipeIngredient` (`mealie/schema/recipe/recipe_ingredient.py:330-357`) — **fully decomposed, and the original free text is always preserved alongside it**:

| Field | Type | Meaning |
|---|---|---|
| `quantity` | float \| null | numeric amount |
| `unit` | IngredientUnit \| null | own referenced entity (below) |
| `food` | IngredientFood \| null | own referenced entity (below) |
| `note` | str \| null | free-text annotation |
| `display` | str | computed human-readable rendering, overridable |
| `title` | str \| null | section header |
| `original_text` | str \| null | **the raw source line, verbatim** |
| `reference_id` | UUID | links this ingredient to instruction steps |
| `referenced_recipe` | Recipe \| null | lets an ingredient reference a whole sub-recipe |

`unit` (`IngredientUnit`): `id, name, plural_name, description, extras, fraction, abbreviation, plural_abbreviation, use_abbreviation, aliases, standard_quantity, standard_unit` — a shared, group-scoped, DB-referenced entity (table `ingredient_units`), not embedded text.
`food` (`IngredientFood`): `id, name, plural_name, description, extras, label_id, label, aliases, households_with_ingredient_food, created_at, updated_at` — also its own group-scoped referenced entity (table `ingredient_foods`), including pantry "on hand" tracking.

Mealie is lenient on *input* (a bare string is auto-wrapped into `CreateIngredientUnit`/`CreateIngredientFood`, and a flat list of plain strings for `recipe_ingredient` is accepted and wrapped as `RecipeIngredient(note=x)`), but its canonical stored/output shape is always the structured one with `original_text` retained. DB table `recipes_ingredients` confirms this: `id, position, recipe_id, title, note, unit_id (FK), food_id (FK), quantity, original_text, reference_id, referenced_recipe_id (FK), note_normalized, original_text_normalized`.

### How preparation steps are represented

`RecipeStep` (`mealie/schema/recipe/recipe_step.py`): `id, title, summary, text, ingredient_references: list[{reference_id}]`. Structured (title/summary/text + ordering), and steps **can reference specific ingredients** via `reference_id` matching (join table `recipe_ingredient_ref_link`). **No timer field and no per-step image field exist anywhere in the schema.** The DB model has an unused `type` column not exposed in the API. A step's only media path is the recipe-level `assets` list, which is not scoped to a step.

### Images

Three fixed-name WebP derivatives per recipe: `original.webp`, `min-original.webp`, `tiny-original.webp` (`mealie/schema/recipe/recipe_image_types.py`), one hero image per recipe. `Recipe.image` is not a path — it's a 4-char cache-busting random string; actual bytes live at the fixed filenames under `data/recipes/<recipe_id>/images/`. Additional non-hero files travel via `assets: list[RecipeAsset]` (arbitrary attachments, not step-scoped) at `data/recipes/<recipe_id>/assets/`. In a zip export, the whole per-recipe directory (images + assets) is copied alongside the JSON. Timeline/cooking-log events can independently carry their own image.

### Tags, categories, notes, servings, times, source, nutrition, ratings

- **Tags / categories**: `RecipeTag`/`RecipeCategory`, identical shape `{id, group_id, name, slug}`, group-scoped, many-to-many.
- **Notes**: `RecipeNote` list, `{title, text}` — free-text recipe-level annotations.
- **Servings/yield**: `recipe_servings: float`, `recipe_yield_quantity: float`, `recipe_yield: str|null` (unit label).
- **Times**: `prep_time`, `cook_time`, `perform_time`, `total_time` — all **free-text strings**, not machine-parseable durations.
- **Source URL**: `org_url` (single string, no multi-source history).
- **Nutrition** (`Nutrition`): `calories, carbohydrate_content, cholesterol_content, fat_content, fiber_content, protein_content, saturated_fat_content, sodium_content, sugar_content, trans_fat_content, unsaturated_fat_content` — **all strings**, not numeric (`coerce_numbers_to_str=True`). A `serving_size` field is deliberately commented out in the DB model (scaling interaction judged too confusing).
- **Ratings**: two separate concepts — (1) `Recipe.rating: float|null`, a computed average across all users, recalculated by a SQLAlchemy event listener; (2) per-user `UserRatingOut`: `{recipe_id, user_id, rating: float|null, is_favorite: bool}` — favorite is a distinct boolean, stored separately, not embedded in the recipe JSON.

### Languages, versioning, cooking log, comments

- **Translations**: none stored. Only a one-shot AI-import translate option (`ScrapeRecipeAI.translate_language`) — translates once at import time; no field holds multiple language variants of a recipe.
- **Versioning**: none found — no revision/audit-log table anywhere in the schema or DB models.
- **Cooking log**: yes — `RecipeTimelineEvent`: `{id, recipe_id, user_id, group_id, household_id, subject, event_type (system|info|comment), message, image, timestamp, created_at, updated_at}`. `Recipe.last_made` is a denormalized "most recent cook" timestamp; `made_by: list[Household]` tracks which households cooked it.
- **Comments**: first-class, `RecipeCommentOut {id, recipe_id, text, user_id, user, created_at, updated_at}`, embedded on the recipe, can be disabled per-recipe.

### Existing parser libraries

| Name | Lang | License | Status |
|---|---|---|---|
| `mealie-rs` (crates.io) | Rust | Apache-2.0 | API client (HTTP wrapper), not an export-file parser. v0.1.6, published 2024-10-31, ~2 years stale, no linked repo. |
| `node-mealie` (npm) | TypeScript | MIT | API client, not an export-file parser. v0.1.1 (2025-04-27), 0 GitHub stars, low adoption but more recently touched. |

No dedicated third-party library was found for parsing Mealie's **export/backup file** specifically — both libraries above wrap the live REST API instead.

---

## Tandoor Recipes

**Repo:** `TandoorRecipes/recipes` (Django backend). Cloned at commit `93c9a1763e3509482edb7e5c73511c1839ada617`. Permalinks use `https://github.com/TandoorRecipes/recipes/blob/93c9a1763e3509482edb7e5c73511c1839ada617/<path>#L<n>`.

### Export file format(s)

The "Default" integration (`cookbook/integration/default.py`) is Tandoor's native, documented-as-preferred backup/portability format — a **nested-zip**:

- Outer: `export_<YYYY-MM-DD>.zip`.
- Inside: one inner zip per recipe, `<recipe_pk>.zip`.
- Inside each inner zip: `recipe.json` (UTF-8 JSON, via `RecipeExportSerializer` + DRF `JSONRenderer`) and `image<ext>` (the recipe's single main image, if any).
- If exporting only one recipe, the outer-zip layer is skipped.

Docs (`docs/features/import_export.md`): *"The default integration is the built-in (and preferred) way to import and export recipes... one of the few recipe formats that is actually structured in a way that allows for easy machine readability."* Tandoor also has ~20 other one-way/partial importers (Paprika, Mealie, Nextcloud, Chowdown, etc.) — see that doc for the full compatibility matrix. Separately, `cookbook/management/commands/export.py` is a raw Django `dumpdata` full-DB fixture dump for whole-instance backups, not the per-recipe portable format.

### Complete field list (`RecipeExportSerializer`, `cookbook/serializer.py:2037-2054` — narrower than the full REST API `RecipeSerializer`)

| Field | Type | Notes |
|---|---|---|
| `name` | str (≤128) | |
| `description` | str (≤512) | short blurb, **not** a long-form notes field |
| `keywords` | array of Keyword | `{name, description, created_at, updated_at}` — no id, no hierarchy preserved |
| `steps` | array of Step | see Steps |
| `working_time` | int | minutes |
| `waiting_time` | int | minutes |
| `internal` | bool | full recipe vs. external-link placeholder |
| `nutrition` | object \| null | `{id, carbohydrates, fats, proteins, calories, source}` — note: leaks a DB `id` due to a serializer inconsistency |
| `servings` | int | default 1 |
| `servings_text` | str (≤32) | free-text override, e.g. "4 people" |
| `source_url` | str \| null (≤1024) | |

**Not included in the export** (present on the live `Recipe` model, `models.py:1086-1149`, but dropped): `id`, `image` (travels separately, unreferenced in JSON), `diameter`/`diameter_text`, `private`, `shared`, `properties`, file-import bookkeeping fields, `created_by`, `created_at`/`updated_at`, `show_ingredient_overview`. Ratings and cook-log history are not part of the export at all.

### How ingredients are represented

`Ingredient` (`cookbook/models.py:936-959`) — **fully decomposed on the live model**:

```python
food = ForeignKey(Food, null=True, blank=True)
unit = ForeignKey(Unit, null=True, blank=True)
amount = DecimalField(default=0, decimal_places=16, max_digits=32)
note = CharField(max_length=256, null=True, blank=True)
is_header = BooleanField(default=False)
no_amount = BooleanField(default=False)
order = IntegerField(default=0)
original_text = CharField(max_length=512, null=True, blank=True, default=None)
```

- `amount` — high-precision decimal.
- `unit` / `food` — nullable FKs to separate `Unit`/`Food` models, not embedded text.
- `note` — short qualifier, separate from the parsed food name.
- `is_header` — an ingredient row can act as a section divider (e.g. "For the sauce:").
- `no_amount` — flags ingredients with no meaningful quantity (e.g. "salt to taste").
- `original_text` — the raw unparsed source line, set by every import integration and by Tandoor's own ingredient parser (`cookbook/helper/ingredient_parser.py:296-315`).

**Critical export gotcha**: `IngredientExportSerializer` (`serializer.py:2011-2023`) only exports `('food', 'unit', 'amount', 'note', 'order', 'is_header', 'no_amount')` — **`original_text` is deliberately excluded**. It's present in the live REST API (`IngredientSerializer`) but does not survive Tandoor's own native backup/restore round-trip.

`Food` (`models.py:774-909`) is a genuinely rich, **hierarchical tree entity** (parent/child inheritance for `supermarket_category`/`ignore_shopping`), plus `plural_name`, `url`, `description`, `substitute` (M2M self-reference), `properties` (nutrition/allergen/price/goal data per 100g via a generic `Property`/`PropertyType` system), `fdc_id` (USDA FoodData Central link), `onhand_users`. `Unit` (`models.py:742-773`) has `name`, `plural_name`, `description`, `base_unit`, `open_data_slug`. A separate `UnitConversion` table stores explicit conversion rules (e.g. "1 cup flour = 120g"), optionally scoped to a specific food. **All of this richness is stripped for export**: `FoodExportSerializer` → only `(name, plural_name, ignore_shopping, supermarket_category)`; `UnitExportSerializer` → only `(name, plural_name, description)` — no ids, no hierarchy, no substitutes, no properties, no conversions (import re-matches by name via `get_or_create`).

### How preparation steps are represented

`Step` (`cookbook/models.py:961-988`): `name, instruction (TextField, rendered as Markdown), ingredients (M2M to Ingredient — steps own their ingredient subset), time (int, minutes), order, file (FK to UserFile — a step CAN carry its own image/attachment live), show_as_header, show_ingredients_table, step_recipe (FK to another Recipe — a step can embed a whole sub-recipe as a component)`.

**Export gotcha, same pattern as ingredients**: `StepExportSerializer` (`serializer.py:2025-2035`) only exports `(name, instruction, ingredients, time, order, show_as_header, show_ingredients_table)` — **`file` (step image) and `step_recipe` (sub-recipe link) are dropped from export**.

### Images

Only **one image per recipe** travels in the native export — `Recipe.image` (`ImageField`), written as `image<ext>` inside the recipe's inner zip, sibling to `recipe.json`, not path-referenced from the JSON. Per-step images (`Step.file`) are **not** included in the export at all. Cross-integration convention documented for other formats (e.g. Nextcloud): one image next to one JSON per recipe folder.

### Tags, categories, notes, servings, times, source, nutrition, ratings

- **Keywords/tags**: `Keyword` is a hierarchical tree model (`name` ≤64 chars, `description`), M2M on `Recipe.keywords`. No separate "category" concept — one hierarchical tag system serves both roles.
- **Notes**: no dedicated recipe-level long-form notes field. `Recipe.description` is a short 512-char blurb. Closest things: `Comment` model, or `Ingredient.note` (per-ingredient).
- **Servings/yield**: `servings` (int, default 1) + `servings_text` (free-text override).
- **Times**: `working_time`, `waiting_time` (both int minutes) at recipe level; `Step.time` (int minutes) at step level.
- **Source URL**: `source_url` (≤1024 chars); a legacy `link`/`cors_link` pair also exists from an older URL-import feature.
- **Nutrition**: exactly 4 macro fields (`calories, carbohydrates, fats, proteins`, high-precision decimals) + `source` (free text). **No fiber field.** A generic `Property`/`PropertyType` system (categories: NUTRITION/ALLERGEN/PRICE/GOAL/OTHER) can hold more (fiber, sodium, allergens, cost), but this is **not** included in the native recipe export.
- **Ratings**: not a `Recipe` field at all — computed/annotated from the separate `CookLog.rating` (int, nullable, no DB-level bounds). Excluded from the native export entirely.

### Languages, versioning, cooking log, comments, collections

- **Translations**: no evidence found — no per-recipe language field or translation model for recipe content (only Django's own UI-string `gettext` i18n).
- **Versioning**: none — no `django-reversion`/`simple_history`/`HistoricalRecords` anywhere; only `created_at`/`updated_at` timestamps.
- **Cook log**: `CookLog` — `{recipe (FK), rating (int, nullable), servings (int, nullable — records scaling at cook time), comment, created_by, created_at, updated_at}` — a genuine "I cooked this, here's how it went" record, distinct from meal planning (`MealPlan`) and view tracking (`ViewLog`).
- **Comments**: `Comment` — flat, one level, `{recipe, text, created_by, created_at, updated_at}`, no threading.
- **Books**: `RecipeBook`/`RecipeBookEntry` — named collections of recipes, a space-level organizational feature, not embedded in a recipe's own export.

### Existing parser libraries

No dedicated Rust crate or standalone published npm/TS package for Tandoor's export format was found (searched crates.io and npm). Tandoor generates its own TS API client in-repo (`vue3/src/openapi/`) from its OpenAPI schema, but that's bundled into Tandoor's own frontend, not a standalone external package. Community tooling found instead: two MCP servers wrapping Tandoor's *live REST API* (`starbuck93/tandoor-mcp-server`, `Cliftonz/tandoor-recipe-mcp`) and a loose script collection (`kyleskorner/Tandoor-Recipe-Scripts`) — none parse the native export zip/JSON specifically. **Conclusion: no off-the-shelf parser exists for Tandoor's native export; a custom parser would need to be written from the field shapes above.**

---

## KitchenOwl

**Repo:** `TomBursch/kitchenowl` (personal account, not an org — confirmed via web search). Flask + SQLAlchemy + marshmallow backend (not FastAPI), Flutter frontend, AGPLv3. Cloned/fetched at commit `09aaf5fbd2343fcc10b12e906c63c3764dd38919`. Permalinks use `https://github.com/TomBursch/kitchenowl/blob/09aaf5fbd2343fcc10b12e906c63c3764dd38919/<path>#L<n>`.

### Export file format(s)

**There is no bulk file-export feature at all** — no zip, no downloadable archive bundling images. What exists is a plain JSON HTTP API response, per household:

- `GET /api/household/{id}/export` → whole household backup (recipes + items + expenses + shoppinglists + member usernames), one JSON object, no file envelope. (`backend/app/controller/exportimport/export_controller.py:1-46`, `backend/app/models/household.py:145-156`)
- `GET /api/household/{id}/export/recipes` → `{"recipes": [...]}`.
- `GET /api/household/{id}/export/items` → `{"items": [...]}`.

Import is symmetric: `POST /api/household/{id}/import`, validated by a marshmallow schema, dispatching to `importRecipe`/`importItem`/etc.

**Images are never embedded in the export JSON.** `Recipe.obj_to_export_dict()` emits `photo` as a bare filename string only (`backend/app/models/recipe.py:186-206`) — the binary lives server-side, referenced by a UUID filename. A bare filename from another server's upload folder is meaningless off that server; only a resolvable URL gets re-fetched on import.

### Complete field list (`Recipe`, `backend/app/models/recipe.py:46-457`)

| Field | Type | Notes |
|---|---|---|
| `id` | int | |
| `name` | str (≤128) | |
| `description` | str (unbounded) | **single free-text/Markdown blob** — see Steps |
| `photo` | str \| null, FK → `file.filename` | filename, not image bytes |
| `time` | int | total minutes |
| `cook_time` | int | minutes |
| `prep_time` | int | minutes |
| `yields` | int | servings/yield count |
| `source` | str | source URL, or `kitchenowl:///recipe/{id}` for internal copies |
| `visibility` | enum (PRIVATE/LINK/PUBLIC) | sharing visibility |
| `server_curated` | bool | admin curation flag |
| `server_scrapes` | int | copy count via public suggestion server |
| `suggestion_score`/`suggestion_rank` | int | internal meal-planner scoring, not user data |
| `household_id` | int | |
| `created_at`/`updated_at` | datetime | |
| `planned`, `planned_days`, `planned_cooking_dates` | computed | derived from the `Planner` relationship, not stored |
| `photo_hash` | computed | blurhash, only present with a photo |

No fields exist for notes, ratings, nutrition, per-cook-attempt log, or multi-language variants (confirmed by grep — zero matches for `nutrition|rating|notes?` across the models/controllers).

### How ingredients are represented

**Semi-structured — a shared food-name entity, but quantity/unit are NOT split into separate columns.** The join table `RecipeItems` (`backend/app/models/recipe.py:359-415`):

```
recipe_items:
  recipe_id -> FK recipe.id
  item_id   -> FK item.id
  description: String   # free text, e.g. "300 g" or "2 cups, diced"
  optional: Boolean
```

`item_id` links to the shared `Item` entity (`backend/app/models/item.py:19-64`, also used by shopping lists): `name`, `icon`, `category_id`, plus shopping-related fields — so the **food name is structured/deduplicated and reused across recipes and the shopping list**, but **quantity + unit + prep notes all collapse into one `description` string**.

Notably, KitchenOwl *does* run NLP/LLM ingredient parsing at scrape time (`ingredient-parser-nlp` and `litellm` dependencies, `backend/app/service/ingredient_parsing.py:14-118`) that computes a real `name`/quantity/unit split internally — but the parsed result is **immediately reassembled into one `description` string** (`f"{quantity} {unit}"`) before storage (`ingredient_parsing.py:35`, call site `backend/app/service/recipe_scraping.py:112-121`). The structured intermediate data never reaches the database. Confirmed at the schema level too: `AddRecipe.RecipeItem`/`UpdateRecipe.RecipeItem` (`backend/app/controller/recipe/schemas.py`) and `ImportSchema.Recipe.RecipeItem` (`backend/app/controller/exportimport/schemas.py`) all define exactly the same 3 fields: `name, description, optional`. No `quantity` field, no `unit` field, no UOM table anywhere.

### How preparation steps are represented

**One text blob, not a structured step list.** `Recipe.description` is a single string column. On scrape, it's built by concatenating the page's schema.org `description` and `instructions` with `\n\n` (`backend/app/service/recipe_scraping.py:87-108`). The Flutter frontend renders it as Markdown (`flutter_markdown_plus`, `markdown` packages) — so numbered/bulleted steps are just Markdown syntax inside one string. No per-step image, no per-step timer, no step IDs.

### Images

Upload endpoint renames every file to a random UUID + extension, computes a blurhash, stores a `File` row (`backend/app/controller/upload/upload_controller.py:19-48`). `Recipe.photo` is literally an FK to `file.filename`. Retrieval via `GET /api/upload/{filename}`, access-checked. On import, `photo` can be an existing accessible filename or a full external URL (which gets downloaded server-side) — **a bare filename from a different server is not portable**, so a JSON-only export/import round-trip does not carry image bytes unless the URL path is used.

### Tags, categories, notes, servings, times, source, nutrition, ratings

- **Tags**: yes, `Tag` model, M2M via `RecipeTags`, plain strings, no color/hierarchy.
- **Categories**: only at the `Item`/shopping-list level (bucketing grocery items) — **not a recipe-level category/cuisine field**. Recipes are organized only by tags.
- **Notes**: none, no field anywhere.
- **Servings/yield**: `yields: int`.
- **Prep/cook time**: `prep_time`, `cook_time`, plus overall `time` — all int minutes.
- **Source URL**: `source: str`.
- **Nutrition**: absent — no fields/tables anywhere.
- **Ratings**: absent — no fields anywhere.

### Languages, versioning, cooking log, meal planning

- **Multi-language**: only at the *household* level (`Household.language`), used to filter/search recipes by locale — **not** per-recipe translation variants.
- **Versioning**: none — no revision table, only `created_at`/`updated_at`.
- **Per-cook-attempt log**: `RecipeHistory` records `ADDED`/`DROPPED` planner status events (a meal-planner audit trail for "recent"/"suggestions" features) — **not** a cooking journal; no notes, no photos, no ratings per cooking attempt.
- **Meal planning**: `Planner` model, composite PK `(recipe_id, cooking_date)`, plus planned `yields`.

### Existing parser libraries

**None found.** npm registry search for "kitchenowl" returns zero results; crates.io search returns zero results. The backend generates its own OpenAPI/Swagger spec at runtime, but it's schema-equivalent to the marshmallow schemas already cited, not an independent library.

---

## Crouton — CONFIDENCE: MEDIUM (community reverse-engineering only; no official schema exists)

**Crouton is closed-source** (iOS/macOS app by Ainsley Clark). The official site (crouton.app) documents no export format, and no first-party FAQ/support page addressing the `.crumb` schema was found. **Everything below is reconstructed from independent third-party reverse-engineering** — five separate open-source codebases plus one real recipe pasted verbatim into a public gist. Confidence is graded per claim; **the project owner will supply a real Crouton export separately, and the specific open questions at the end of this section must be checked against it before trusting this section for implementation.**

### HIGH CONFIDENCE (corroborated by 3+ independent sources, including a real pasted export)

**File format**: `.crumb` = single-recipe share/export file, **UTF-8 JSON text**, not a zip or binary blob — stated verbatim in the [LukeChannings gist schema comment](https://gist.github.com/LukeChannings/11ba3649bcb9b9086e3e271c7c3e950d) (2021): *"a .crumb file is a UTF-8 JSON document."* **Bulk/full-library export is a `.zip` of multiple `.crumb` files** — confirmed both by [RecipeSage's Crouton-import documentation](https://recipesage.com/alternatives/crouton/) and by working production code that unzips and recursively collects `*.crumb` files: [`croutonImportJobHandler.ts`](https://github.com/julianpoy/RecipeSage/blob/master/packages/util/server/src/general/queue/import/handlers/croutonImportJobHandler.ts).

**Core recipe fields**, present across the 2021 gist, a real pasted sample export ([matiaskorhonen gist](https://gist.github.com/matiaskorhonen/40d46b122f061420787dde414b238260), an "Asian Cucumber Salad" recipe), RecipeSage's TypeScript handler, [recipya's Go struct](https://github.com/reaper47/recipya/blob/main/internal/models/recipe_apps.go), and [recipya-rs's Rust struct](https://github.com/reaper47/recipya-rs/blob/main/crates/integrations/src/apps/crouton.rs):

| Field | Type | Notes |
|---|---|---|
| `name` | str | title |
| `uuid` | str | |
| `serves` | number | |
| `duration` | number | **prep time, minutes** |
| `cookingDuration` | number | **cook time, minutes** |
| `webLink` | str | source URL |
| `notes` | str | recipe-level free text |
| `folderIDs` | array | |
| `images` | array | base64-encoded image strings |
| `ingredients` | array | see Ingredients |
| `steps` | array | see Steps |

### How ingredients are represented (high confidence for structure, some sub-fields unconfirmed)

Structured, not a flat string:

```json
{ "ingredient": { "name": "...", "uuid": "..." },
  "quantity": { "amount": 2, "quantityType": "GRAMS" },
  "order": 0, "uuid": "..." }
```

- The `quantity` sub-object can be **entirely absent** for quantity-less ingredients (e.g. "Kosher salt") — confirmed against a real test fixture in [`bridges-wood/crouton-sync/tests/test_crumb.py`](https://github.com/bridges-wood/crouton-sync/blob/main/tests/test_crumb.py).
- Section headers within the ingredient list use a **special `quantityType: "SECTION"`**, with the header text held in `ingredient.name` — corroborated independently by RecipeSage's handler and recipya-rs's Rust struct.
- No separate per-ingredient free-text note field distinct from the name was found in any source; in the one real sample seen, prep notes are folded straight into `ingredient.name` (e.g. `"asparagus, stalks trimmed"`). **Unconfirmed whether a dedicated note field exists** — not exhaustively tested by any source.
- No field resembling Mealie/Tandoor's "original unparsed text" was found in any source.

### How preparation steps are represented

`steps`: array of `{step: string, order: number, uuid: string, isSection: boolean}`. `isSection` marks a step that is actually a section header rather than an instruction — corroborated independently by RecipeSage's handler and recipya-rs. No timer or per-step image field found in any source.

### Images

`images` is an array of base64-encoded strings embedded directly in the JSON (not referenced by filename/path) — this is a real structural difference from Mealie/Tandoor/KitchenOwl, all of which reference images out-of-band. **Encoding format is disputed between sources** (see Open Questions): PNG per the 2021 gist, JPEG per the crouton-sync docs.

### Tags, categories, notes, servings, times, source, nutrition, ratings

- **Tags**: exist, but **shape is disputed** — see Open Questions.
- **Notes**: `notes: string`, recipe-level free text — high confidence.
- **Servings**: `serves: number` — high confidence.
- **Prep/cook time**: `duration` / `cookingDuration`, minutes — high confidence.
- **Source URL**: `webLink` — high confidence. A separate `sourceName` (attribution string) is **medium confidence** — found in recipya Go, recipya-rs, and crouton-sync's docs (3 sources), absent from the older 2021 gist (possibly added in a later app version).
- **Nutrition**: a field spelled **`neutritionalInfo`** (sic — typo for "nutritional") appears independently in three unrelated codebases (recipya-rs, RecipeSage, crouton-sync), which is strong evidence it's genuinely reverse-engineered rather than guessed — three developers are unlikely to coincidentally invent the same typo. Value is a **free-text, multi-line string** of `Key: value` lines (e.g. `"Sugar: 2g\nCalories: 457"`) — **not structured nutrition data**, unlike Mealie's (string-typed but at least field-per-nutrient) or Tandoor's (numeric, field-per-macro) approach.
- **Ratings**: **not found in any `.crumb` JSON schema from any source.** A low-confidence single source (see below) claims a `ZRATING` column exists in Crouton's local Core Data database, implying ratings exist in the app but likely do **not** travel in the export — this is a negative finding worth flagging rather than assuming ratings are "somewhere I didn't look."
- **Difficulty**: same situation as ratings — claimed to exist locally (`ZRAWDIFFICULTY`) by the single low-confidence source, not found in any export schema.

### Medium-confidence fields (2 corroborating sources, absent from the oldest 2021 gist)

`sourceName`, `defaultScale` (a scaling factor), `sourceImage` (a separate base64 image distinct from the `images` array), `isPublicRecipe` (bool) — each found in 2 of {recipya Go, recipya-rs, crouton-sync docs}, not in the oldest sources. Plausibly fields added by the app after 2021, or simply omitted from the older gist.

### Low confidence — single unverified source only

Everything below comes from **one repository only**, [bridges-wood/crouton-sync](https://github.com/bridges-wood/crouton-sync) (0 GitHub stars, ~5 months old), whose broader claims are about Crouton's *local Core Data SQLite database* (bundle ID `com.meal.plan.ios`, CloudKit container `iCloud.br.com.dinner.plan`), not the `.crumb` export format itself. Its [`docs/CROUTON_INTERNALS.md`](https://github.com/bridges-wood/crouton-sync/blob/main/docs/CROUTON_INTERNALS.md) is detailed and plausible but has **zero corroboration elsewhere** — treat as an interesting lead, not fact:
- `ZRATING` (star rating) and `ZRAWDIFFICULTY` columns on the local recipe table, with no corresponding `.crumb` export field found anywhere.
- `ZSECONDARYAMOUNT` column, suggesting possible ingredient quantity *ranges* (e.g. "2–3 cups") — again, no corresponding export field found in any source.
- Tags stored locally as `{name, color}` (`ZCDTAG` table) — matches recipya-rs's `Tag {color, name}` struct (partial corroboration for shape), but conflicts with RecipeSage's importer, which treats `tags` as an array of plain strings.

### Explicitly NOT found — do not treat as fact

- No `version`/`schemaVersion` field in any source.
- No per-recipe language/localization field (the app's UI is officially English/French/German per its [App Store listing](https://apps.apple.com/us/app/crouton-recipe-manager/id1461650987), but nothing found about per-recipe language tagging of content).
- No cooking-log/cook-history field found in any source.
- PDF import/export is a confirmed real app feature (official tweet, App Store listing) but is a separate, presumably non-reimportable visual-only path — no parser exists and it's out of scope for schema reconstruction.

### Open questions to confirm against the real Crouton export file (from the project owner)

1. Decode an `images[]` base64 string and check magic bytes — PNG or JPEG?
2. Exact `quantityType` enum values present — `DECILITER`, `CENTILITER`, both, neither, others?
3. `tags` field's actual shape — array of plain strings, or array of `{name, color}` objects?
4. Is there a rating or difficulty field anywhere in the JSON? (Every source suggests no — confirm.)
5. Is there a quantity-range / secondary-amount field on ingredients?
6. Are `sourceImage`, `sourceName`, `defaultScale`, `isPublicRecipe` present (medium-confidence, possibly newer fields)?
7. Is `notes` per-recipe only, or is there any per-ingredient note field separate from the ingredient name?
8. Bulk-export zip's internal structure — flat list of `.crumb` files, or nested by folder?
9. Is there a version/schema field at the top level?

### Existing parser libraries

| Project | Lang | License | Stars | Last push | Notes |
|---|---|---|---|---|---|
| [julianpoy/RecipeSage](https://github.com/julianpoy/RecipeSage) (`croutonImportJobHandler.ts`) | TypeScript | AGPL (widely cited; no explicit `license` field via API) | 940 | 2026-08-19 | Actively maintained; production import path exercised against real users' bulk exports. Written defensively (tolerates missing fields) — likely under-specifies rather than over-specifies the schema. |
| [reaper47/recipya](https://github.com/reaper47/recipya) (`internal/models/recipe_apps.go`) | Go | GPL-3.0 | 410 | 2026-08-15 | Actively maintained mainstream recipe manager. |
| [reaper47/recipya-rs](https://github.com/reaper47/recipya-rs) (`crates/integrations/src/apps/crouton.rs`) | **Rust** | AGPL-3.0 | 36 | 2026-08-04 | Actively maintained Rust rewrite. Its `QuantityType` enum only implements 5 of ~17 known values (Item/Grams/Mills/Teaspoon/Tablespoon) — **incomplete**, would error on `POUND`, `CUP`, etc. |
| [bridges-wood/crouton-sync](https://github.com/bridges-wood/crouton-sync) (`crumb.py`) | Python | none declared | 0 | 2026-04-14 | New, unreviewed, single-source only for its broader DB-internals claims. |
| [jd-santos/cookdown](https://github.com/jd-santos/cookdown) (`parsers/crumb.py`) | Python | Apache-2.0 | 6 | 2026-05-12 | Second independent Python implementation, not deeply read. |
| [chrishutchinson/transform-crouton-data](https://github.com/chrishutchinson/transform-crouton-data) | JS/TS | none declared | 3 | 2026-05-16 | Converts Crouton export → schema.org Recipe JSON; not deeply read. |

**No off-the-shelf library is production-grade/complete enough to depend on as-is** — RecipeSage's TS handler is the most battle-tested (real user uploads), but is defensive/partial by design; the only Rust option (recipya-rs) is confirmed incomplete for the unit enum.

---

## Synthesis 1 — Common core (the floor for Kamosu's recipe model)

Fields present, in some form, across **all four** apps:

| Concept | Mealie | Tandoor | Crouton (reverse-eng.) | KitchenOwl |
|---|---|---|---|---|
| Title/name | ✅ `name` | ✅ `name` | ✅ `name` | ✅ `name` |
| Ingredient list, structured (qty/unit separate from food name) | ✅ | ✅ | ✅ (medium confidence) | ❌ qty+unit fused into free text |
| Steps/instructions, some ordering | ✅ structured list | ✅ structured list | ✅ structured list | ❌ single Markdown blob, order is implicit in text |
| Servings/yield | ✅ | ✅ | ✅ `serves` | ✅ `yields` |
| Prep time | ✅ (free text) | ✅ (int minutes) | ✅ `duration` (minutes) | ✅ `prep_time` (minutes) |
| Cook time | ✅ (free text) | ✅ `waiting_time` (int) | ✅ `cookingDuration` | ✅ `cook_time` |
| Source URL | ✅ `org_url` | ✅ `source_url` | ✅ `webLink` | ✅ `source` |
| Tags | ✅ | ✅ (as hierarchical `keywords`, also serving as categories) | ✅ (shape disputed) | ✅ |
| Images, at least one per recipe | ✅ (out-of-band file) | ✅ (out-of-band file) | ✅ (inline base64) | ✅ (out-of-band file, referenced by filename only) |

**Fields present in 3 of 4** (not universal, but a strong signal): a food/ingredient-name entity distinct from the raw ingredient line (Mealie, Tandoor, Crouton — not KitchenOwl); a recipe-level free-text notes field distinct from description (Mealie `notes`, Crouton `notes`, KitchenOwl none, Tandoor none dedicated — actually only 2/4 have this cleanly, see Gaps); nutrition of some kind (Mealie structured-per-field strings, Tandoor structured-per-field decimals, Crouton one free-text blob — KitchenOwl has none).

**Conclusion for Kamosu's floor**: title, structured-enough ingredient list (name + quantity + unit, even if only 3/4 apps fully commit to this), an ordered step list, servings, prep/cook time, source URL, tags, and at least one image are safe to treat as baseline fields every importer needs to map into. Kamosu should **not** assume ingredient decomposition is universal — KitchenOwl imports need a text-splitting/parsing step Kamosu must own itself, same as KitchenOwl's own (discarded) internal parser does.

---

## Synthesis 2 — Gaps

### Fields some apps carry that Kamosu currently has nowhere to put

- **Per-step ingredient references** (Mealie `ingredient_references`, Tandoor `Step.ingredients` M2M) — linking specific ingredients to specific steps, not just a flat recipe-level list.
- **Sub-recipe composition** — a step or ingredient that references another whole recipe as a component (Mealie `RecipeIngredient.referenced_recipe`, Tandoor `Step.step_recipe`). Notably Tandoor's own export drops this on round-trip, so it's a "live app" feature more than a "portable data" feature today.
- **Structured food entity with pantry/on-hand tracking** — Mealie's `IngredientFood.households_with_ingredient_food`, Tandoor's `Food.onhand_users`. Both apps track which households currently have an ingredient on hand, tied to the shared food entity, not the recipe.
- **Explicit unit-conversion tables** — Tandoor's `UnitConversion` model (explicit "1 cup flour = 120g" rules, optionally food-scoped).
- **Hierarchical/tree tags or foods** — Tandoor's `Keyword` and `Food` are both tree-structured (parent/child with field inheritance); Mealie's tags/categories are flat.
- **Per-user rating + separate favorite boolean** — Mealie stores these as two independent per-(user, recipe) fields, not a single recipe-level number.
- **Cooking-log / "I made this" events with commentary** — Mealie's `RecipeTimelineEvent` (with its own optional image) and Tandoor's `CookLog` (rating + servings-at-time-of-cooking + comment) are both closer to what Kamosu's wish-list wants than a generic view-count.
- **Recipe comments** — Mealie has first-class threaded-adjacent comments (flat list) on a recipe; Tandoor has a flat `Comment` model too.
- **Recipe collections/books** — Tandoor's `RecipeBook`, independent of tags, with sharing.
- **Property/label system** for arbitrary per-food or per-recipe metadata (Tandoor's `Property`/`PropertyType`: nutrition/allergen/price/goal categories; Mealie's food `label` for shopping-list color coding).
- **Inline (not file-referenced) image encoding** — Crouton embeds images as base64 directly in the recipe JSON rather than referencing files out-of-band; this is a real structural choice Kamosu doesn't currently have an equivalent for, and matters for import-time size/streaming handling.

### Kamosu wish-list items with NO prior art in any of the four apps

Checked explicitly against all four sources — **none of the following exist in any of Mealie, Tandoor, KitchenOwl, or (as far as public sources show) Crouton**:

- **Per-recipe versioning / edit history.** Confirmed absent by direct source-code/grep check in Mealie (`schema/`, `db/models/` — no `Revision`/`EditHistory`/`AuditLog` table) and Tandoor (no `django-reversion`/`simple_history`/`HistoricalRecords` dependency or usage). Not found for KitchenOwl or Crouton either. All four apps only track `created_at`/`updated_at` timestamps, which is not the same as retrievable history.
- **Cooking attempts with adjustments, as a rich structured concept** (e.g., "I made this with 20% less sugar and it came out great" as queryable, comparable data). The closest analogues — Tandoor's `CookLog` (rating + servings + free-text comment) and Mealie's `RecipeTimelineEvent` (message + optional image) — are simple logs, not structured adjustment records. There is no field anywhere for capturing *what was changed* in a machine-readable way (e.g., a per-ingredient override).
- **The same recipe in multiple languages, as first-class linked variants.** All four apps either have no language concept at all (KitchenOwl only tags a *household's* locale, not per-recipe content; Mealie's translate feature is a one-shot AI conversion at import time, not stored variants) or no evidence was found (Tandoor, Crouton).
- **Per-step personal photos** (a user's own photo of *their* attempt at a specific step, as opposed to the recipe's canonical hero image). No app has a step-scoped image field at all except via Tandoor's `Step.file` — but that's the recipe author's own attachment, not a separate per-user "my attempt" photo, and it's dropped from Tandoor's own export anyway. Mealie's cooking-log events can carry one image per event, which is the closest analogue found, but it's recipe-scoped, not step-scoped.
- **Structured pantry matching** (matching recipe ingredients against a real, quantified pantry/inventory to know what's missing). Mealie and Tandoor both have a boolean-ish "who has this food on hand" flag tied to the *food entity*, not a quantified pantry with amounts, expiry, or the ability to compute "I have 300g flour, this recipe needs 500g, I'm short 200g." KitchenOwl links recipe ingredients to shopping-list `Item`s (closest structural precedent for "ingredient ↔ inventory identity"), but has no quantity tracking on either side to make matching meaningful.

**Bottom line**: Kamosu's ambitions around versioning, structured cooking-attempt adjustments, multi-language recipe variants, per-step personal photos, and quantified pantry matching are all **genuinely novel relative to this survey** — none of the four apps studied provide usable prior art for any of them. These are real product differentiators, not gaps Kamosu can shortcut by copying an existing schema.

---

## Confidence key

- **High**: read directly from source code (Mealie, Tandoor, KitchenOwl) or corroborated by 3+ independent third-party sources including at least one real captured export sample (Crouton).
- **Medium**: corroborated by 2 independent third-party sources, or a single source that is itself high-trust (e.g., production code exercised against real user data).
- **Low**: a single, unreviewed, unconfirmed source. Marked explicitly throughout the Crouton section; treat as leads requiring confirmation, not facts.

All Crouton findings should be re-validated against the real `.crumb`/export file the project owner will supply, using the specific open-questions list at the end of the Crouton section as a checklist.
