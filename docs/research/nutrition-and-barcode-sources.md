# Nutrition and barcode data — where it comes from, and whether it can be self-hosted

**Ticket:** GitHub issue #4 — "Where would nutrition and barcode data for Kamosu come from, and can it be self-hosted?"
**Purpose:** Two wish-list features need an outside food database: (a) nutrition per 100 g and per serving for a recipe, and (b) barcode-scanner pantry management (deferred past v1). Both are shaky if they depend on a commercial API that could disappear or start charging. This document establishes, from primary sources, what the realistic data sources are, what their licences actually oblige, how well they cover French/European and non-English food, whether each can be bundled or self-hosted offline, and what the ingredient-line→nutrition matching problem really costs. It ends with an opinionated v1 recommendation.
**Method:** Primary sources only — licence texts read in full, live API responses fetched with `curl`, dataset archives downloaded and their records counted directly, and the source code of four self-hosted recipe/pantry apps cloned and read. Several claims are *verified by doing*: real Open Food Facts product JSON fetched for French products; the FoodData Central Foundation Foods and SR Legacy CSV archives downloaded and parsed; the CIQUAL XML downloaded and parsed; the OpenNutrition dataset downloaded and parsed; and prototype SQLite databases actually built to measure on-disk size. Every claim is cited to a URL or file path; source-code claims use commit-hash permalinks. Confidence is marked explicitly wherever a source is anything less than "read directly in the actual code, the actual file, or a live HTTP response."

All measurements taken 2026-08-20 unless stated.

---

## Summary — single most decision-relevant finding

**The data is not the problem. The matching is. And the strongest evidence for that is that none of the four mature self-hosted apps Kamosu is being built against computes nutrition properly — and the one that tries needed a whole per-food unit-conversion subsystem to do it, and still shows the user "missing conversion" errors.**

Read directly in the code:

- **Mealie** stores nutrition as **eleven free-text strings on the recipe** (`calories: str | None`, …). Its food entity (`IngredientFoodModel`) has **no nutrition fields at all**. Mealie cannot compute nutrition from ingredients — it only stores what a scrape or a human typed, and does not even scale it by servings.
- **KitchenOwl** has **no nutrition concept whatsoever** — zero matches for `nutrition`/`calorie`/`kcal` across its backend models.
- **Grocy** has exactly **one** nutrition field, `products.calories INTEGER`, typed by the user, and sums it across a recipe in SQL. No protein, no fat, no carbs. Its Open Food Facts barcode plugin fetches **only `product_name`, `image_url` and the localized name** — it does not pull nutrition even though the API returns it.
- **Tandoor** is the only one that genuinely computes. It needed: a per-food `Property` model, a `PropertyType` carrying an `fdc_id`, a `UnitConversion` model **whose conversions are per-food**, a `UnitConversionHelper` with a hard-coded weight/volume table that *refuses* to convert between the two systems, and a `FoodPropertyHelper` that returns three distinct failure flags to the UI — `missing_value`, `missing_unit`, `missing_conversion`. It then hydrates per-food nutrition from the **USDA FoodData Central API**, one food at a time, with a user-supplied API key defaulting to `DEMO_KEY` (30 requests/hour).

Everything else follows from that:

- **Durable, self-hostable data exists and is cheap.** French raw ingredients: **CIQUAL** (ANSES) — 3,185 foods, 67 nutrients, bilingual FR/EN names, French Open Licence — measured as a **5.3 MB SQLite file** including a full-text index. Barcoded products: **Open Food Facts** — 4,695,023 products, 1,260,089 of them French — a France-only barcode+nutrition subset measured as a **169 MB SQLite file**. Both bundle into a Docker image without drama.
- **What does not exist is a ready-made French ingredient-name→nutrition matcher.** CIQUAL calls crème fraîche "*Crème de lait, 30% MG, épaisse, rayon frais*". There is **no entry beginning "Crème fraîche"** in the whole table. Exact matching fails; fuzzy matching on the wrong string is worse than no answer.
- **No dataset ships household-measure conversions for European cooking.** USDA SR Legacy has 14,449 portion rows with gram weights (all 14,449 populated, covering 7,533 of 7,793 foods) — but the units are US: `cup`, `tbsp`, `oz`, `stick`, `pat (1" sq, 1/3" high)`. Searching all 14,449 modifiers for "handful", "knob" or "pinch" returns **zero hits**. CIQUAL has no portion data at all.
- **Recommendation in one line:** **ship v1 with a manual nutrition field only** (free-text or numeric, per recipe, entered or imported from schema.org — matching what Mealie does and what three of four apps effectively do); **bundle CIQUAL** as a lookup people can attach to an ingredient by hand if they want to; defer automatic recipe-wide nutrition computation, and defer barcode pantry entirely — but when barcode comes, use a **locally-ingested Open Food Facts subset**, not the live API.

---

## 1. The realistic sources, at a glance

| Source | What it is | Records (measured/live) | Licence | Bulk download? | Verdict for Kamosu |
|---|---|---|---|---|---|
| **CIQUAL** (ANSES, FR) | French national food composition table — generic/raw foods | **3,185** foods, **67** nutrients, **211,898** composition values | Licence Ouverte / Etalab (`fr-lo` on data.gouv.fr) | Yes — 3.5 MB ZIP of XML; also XLS | **Best fit for raw ingredients in French recipes** |
| **Open Food Facts** | Crowdsourced packaged/branded products, barcode-keyed | **4,695,023** products; **1,260,089** France; **932,743** French products with nutrition complete | Database ODbL 1.0; contents DbCL 1.0; images CC-BY-SA | Yes — MongoDB 14.4 GiB, JSONL 11.8 GiB, CSV 1.19 GiB, Parquet 7.25 GiB, nightly | **The barcode source. Weak for raw ingredients** |
| **USDA FoodData Central — SR Legacy** | US reference composition, frozen | **7,793** foods, **644,125** nutrient values, **14,449** portions with gram weights | Public domain / CC0 1.0 | Yes — 6.1 MB ZIP CSV | **Only worth it for the gram-weight portion table** |
| **USDA FDC — Foundation Foods** | New analytically-derived US foods | **394** (live API) / 469 rows typed `foundation_food` in the archive | Public domain / CC0 1.0 | Yes — 3.8 MB ZIP CSV | Too small to matter on its own |
| **USDA FDC — FNDDS (Survey)** | US dietary-survey foods and portions | **5,432** | Public domain / CC0 1.0 | Yes — 200 MB ZIP / 1.6 GB CSV | US dishes; not useful here |
| **USDA FDC — Branded** | US/international label data | **433,403** | Public domain / CC0 1.0 | Yes — 428 MB ZIP / 2.9 GB CSV, monthly | US-centric; OFF is better for Europe |
| **OpenNutrition** | AI-enhanced aggregate of national tables + OFF | **326,759** foods (**5,299** generic "everyday") | ODbL + *modified* DbCL with aggressive attribution | Yes — 62.9 MB ZIP / 282 MB TSV | Interesting; English-only; one release, Mar 2025 |
| **CoFID** (McCance & Widdowson, UK) | UK national composition table | not counted (archive not downloaded) | Open Government Licence v3.0 | Yes — 4.42 MB XLSX | English food naming; fine, but CIQUAL is closer |
| **Frida** (DTU, DK) | Danish national table | >1,000 foods, rev 5.5 (2025-12-19) | Credit required on each display | Yes (download offered on site) | Too small/Danish to be primary |
| **EuroFIR** | Federation of national tables via FoodEXplorer | 30+ national DBs | **Membership / pay-per-view** | No open bulk download | **Fails** — paywalled |
| **Nutritionix** | Commercial API | — | Proprietary | No | **API-only.** Public free tier discontinued |
| **Edamam** | Commercial API | — | Proprietary | No | **API-only.** From $14/mo; caching largely prohibited |
| **Spoonacular** | Commercial API | — | Proprietary | No | **API-only.** 1-hour cache limit; delete-all-on-exit |
| **FatSecret** | Commercial API | — | Proprietary | No | **API-only.** Free "Basic" tier for evaluation |

---

## 2. Licences — what each one actually obliges

### USDA FoodData Central — the cleanest licence of the lot

Read directly from the [FDC API Guide](https://fdc.nal.usda.gov/api-guide), "Licensing" section (confidence: **High** — quoted verbatim from the live page):

> "USDA FoodData Central data are in the public domain and they are not copyrighted. They are published under CC0 1.0 Universal (CC0 1.0) No permission is needed for their use, but we request that users list FoodData Central as the source of the data…"

Suggested citation given on the same page: *U.S. Department of Agriculture, Agricultural Research Service. FoodData Central, 2019. fdc.nal.usda.gov.*

**Bulk-download terms vs. API terms are not different here** — the licensing statement sits on the API guide and covers "FoodData Central data" without distinguishing. What *is* different is the operational constraint: the **API** is rate-limited (below), the **bulk download** is not. Confidence: **High** for the licence; **Medium** for "there is no separate download-only terms document" (an absence of evidence — the [Download Datasets](https://fdc.nal.usda.gov/download-datasets) page carries no additional terms text, read directly).

**API rate limits**, quoted from the same page: **1,000 requests/hour per IP address**; exceeding it blocks the key for 1 hour (HTTP 429). The special `DEMO_KEY` is limited to **30 requests/hour and 50/day per IP**. This matters because Tandoor ships `FDC_API_KEY = os.getenv('FDC_API_KEY', 'DEMO_KEY')` ([`recipes/settings.py#L149`](https://github.com/TandoorRecipes/recipes/blob/93c9a1763e3509482edb7e5c73511c1839ada617/recipes/settings.py#L149)) — a default Tandoor install gets 30 nutrition lookups an hour.

### Open Food Facts — a three-way split, and the share-alike question

Read directly from [world.openfoodfacts.org/terms-of-use](https://world.openfoodfacts.org/terms-of-use) (confidence: **High**):

- The **database** is under the **Open Database License (ODbL) 1.0**.
- The **individual contents** are under the **Database Contents License (DbCL) 1.0**.
- **Product images** are under **CC BY-SA**, and OFF warns explicitly that images "may contain graphical elements subject to copyright or other rights" (packaging design, trademarks, image rights of people on packaging) that the CC licence does not cover.

The terms also state, in OFF's own words: attribution must name Open Food Facts with a link, "Such attribution is also necessary for derivative works," and "Derivative works must be shared under the same conditions."

**What ODbL share-alike actually requires of a self-hosted app that ships a derived subset.** Read in full from the [ODbL 1.0 legal text](https://opendatacommons.org/licenses/odbl/1-0/) (confidence: **High** — clauses quoted from the licence itself):

1. **Extracting a subset creates a Derivative Database.** §4.4(b): "Extraction or Re-utilisation of the whole or a Substantial part of the Contents into a new database is a Derivative Database and must comply with Section 4.4." An OFF-France-only nutrition table is unambiguously that.
2. **The trigger is *Publicly Use*, not *use*.** §4.5(c): "Use of a Derivative Database internally within an organisation is not to the public and therefore does not fall under the requirements of Section 4.4." "Publicly" is defined as "to Persons other than You or under Your control." **A person running Kamosu on their own NAS for their own household is squarely inside this exception.** The share-alike obligations do not bite on the end user at all.
3. **But shipping the subset does trigger it — for the Kamosu project, not the user.** If Kamosu bakes an OFF-derived table into a public Docker image or a downloadable data bundle, that is Publicly Conveying a Derivative Database, and §4.2 + §4.4 + §4.6 apply: the derived database must itself be offered under ODbL (or a compatible licence), the licence text/URI must ship with it, and — §4.6 — recipients must be offered "a copy in a machine readable form of: a. The entire Derivative Database; or b. A file containing all of the alterations… (such as an algorithm)", free of charge over the internet.
4. **§4.6(b) is the cheap escape hatch.** Kamosu does not have to host a 169 MB database dump. Publishing the *ingest script* — "we take the nightly OFF CSV, keep these 15 columns, filter `countries_en` to France, load it into SQLite" — satisfies 4.6(b) as "the method of making the alterations." Combined with a **download-on-first-run** design (the user's own server pulls from OFF; Kamosu never redistributes anything), share-alike stops being a practical constraint at all.
5. **Displaying nutrition in the UI is a "Produced Work", which is *lighter*.** §4.5(b): using the database to create a Produced Work "does not create a Derivative Database for purposes of Section 4.4". §4.3 asks only for a notice; the licence even supplies the wording: *"Contains information from DATABASE NAME, which is made available here under the Open Database License (ODbL)."*

**Verdict on OFF licensing:** attribution is a real, easy obligation (a line in the UI and the docs). Share-alike is **not a real problem** for Kamosu provided it either (a) ships the ingest script rather than a pre-baked dump, or (b) publishes the derived table under ODbL too, which costs nothing. The genuinely risky part is **images** — CC-BY-SA plus unresolved third-party packaging/trademark rights that OFF explicitly disclaims. Kamosu should not redistribute OFF product photos.

**DbCL 1.0** ([full text](https://opendatacommons.org/licenses/dbcl/1-0/), confidence: **High**) is deliberately permissive on the *contents*: §2.1 grants a "worldwide, royalty-free, non-exclusive, perpetual, irrevocable copyright license… These rights explicitly include commercial use", and §2.4 records the position that "factual information is not covered by copyright." §2.2 is the sting: "You must comply with the ODbL." So DbCL adds nothing restrictive — the ODbL analysis above is the whole story.

### CIQUAL — French Open Licence, and the easiest of all

[data.gouv.fr dataset record](https://www.data.gouv.fr/datasets/table-de-composition-nutritionnelle-des-aliments-ciqual-2020), queried via the data.gouv.fr API (confidence: **High** — read from the live API response): licence field is **`fr-lo`** = *Licence Ouverte / Open Licence* (Etalab), publisher **ANSES**. Etalab's Open Licence permits reuse including commercial reuse, requiring only attribution of source and date of the last update. Confidence: **High** for the licence identifier read from the API; **Medium** for the summary of its obligations (standard Etalab terms, not re-read in full for this document).

Dataset record `last_modified` is 2025-11-20, but the files themselves are the **2020-07-07** release — CIQUAL is updated infrequently. Confidence: **High** (file dates inside the ZIP are `2020-07-03`).

### CoFID (UK) — Open Government Licence v3.0

Read directly from the [GOV.UK publication page](https://www.gov.uk/government/publications/composition-of-foods-integrated-dataset-cofid): "All content is available under the Open Government Licence v3.0". Last updated **19 March 2021**; primary file is an MS Excel spreadsheet, **4.42 MB**. Confidence: **High**.

### OpenNutrition — ODbL plus a *modified* DbCL that is unusually demanding

Read directly from the downloaded `LICENSE-DbCL.txt` and `README.md` inside `opennutrition-dataset-2025.1.zip` (confidence: **High** — read from the actual files):

> "a. Attribution Requirement: If You display, publish, or otherwise make available any of the Contents to users, You must provide clear and visible attribution to 'OpenNutrition' with a link to https://www.opennutrition.app **at every location where the Contents are displayed or made available**."
> "b. No Consolidated Attribution: Providing attribution in a single location (such as only on an 'About' page or in 'Terms of Use') is insufficient…"

The published download page adds that attribution is required in "Every interface where data is displayed", app-store listings, the website and legal/about sections. It also notes the dataset incorporates Open Food Facts data, so OFF attribution must be maintained too. This is a materially heavier obligation than plain ODbL and would put a permanent OpenNutrition credit on every nutrition panel in Kamosu's UI.

### The commercial APIs — all fail the durability test, explicitly

- **Spoonacular** ([pricing page](https://spoonacular.com/food-api/pricing), confidence: **High** — fetched and quoted): Free $0/mo at 50 points/day; Cook $29/mo; Culinarian $79/mo; Chef $149/mo; Enterprise from $300. The terms are the disqualifier: *"You may cache user-requested data to improve performance (for a maximum of 1 hour). After 1 hour, you must delete your cache and refresh the data via the spoonacular API"* and, on ceasing use, *"you must delete all data you ever obtained from the spoonacular API."* A self-hosted app cannot build a durable local pantry on those terms.
- **Edamam** ([Food Database API pricing](https://developer.edamam.com/food-database-api), confidence: **High** — read from the fetched page): cheapest plan **$14/month** (Enterprise Basic, 100,000 calls/mo), then $69 and $299. No free tier. Terms: *"All plans allow only human, end user driven requests. The terms of the API prohibit any automated programatic requests with the goal to collect, scrape or save data… and can not be stored unless explicitely permetted"*; caching, where allowed, is limited to *"only the four basic macro nutrient datapoints — protein, total fat, net carbs and calories"* plus id/label/image, *"behind a password"*, and explicitly "does not allow API subscribers to build a copy of the Edamam data." Attribution with Edamam's own image is mandatory and *"Not providing attribution is considered breach of contract and is followed by immediate service suspension."*
- **Nutritionix** (confidence: **Medium** — from Nutritionix's own site via search, page itself returned HTTP 402 to direct fetch): the public free tier has been discontinued in favour of application-gated trials for commercial/research/enterprise evaluation. No public pricing.
- **FatSecret** ([Platform API page](https://platform.fatsecret.com/platform-api), confidence: **Medium** — fetched, but the page discloses no numbers): a free "Basic" tier exists for testing/evaluation; Premier pricing is sales-gated, month-to-month.

**All four are API-only with no bulk export.** For an app whose whole premise is that it keeps working when a vendor doesn't, they are out — not because of price, but because three of them contractually forbid keeping the data.

---

## 3. Coverage — and specifically, is any of this any good in French?

### Open Food Facts: enormous, French-heavy, but it is a *product* database

Live counts via the OFF v2 search API (confidence: **High** — real HTTP responses):

| Query | Count |
|---|---|
| All products | **4,695,023** |
| `countries_tags_en=france` | **1,260,089** (26.8%) |
| `countries_tags_en=france` + `states_tags_en=nutrition-facts-completed` | **932,743** (74.0% of French products) |
| `states_tags_en=nutrition-facts-completed` (all) | **3,543,071** (75.5%) |
| `countries_tags_en=germany` | 421,927 |
| `countries_tags_en=united-kingdom` | 192,715 |
| `countries_tags_en=netherlands` | 106,284 |

The site's own homepage banner read **"4,695,052 products"** at the same moment — consistent.

*(Caveat, confidence **Medium**: an independent sample of the first 219,990 rows of the CSV export showed only 18.1% with `energy-kcal_100g`. That sample is heavily biased — the CSV is barcode-ordered and those rows are almost all `0`-prefixed US/Canada GS1 barcodes, many from bulk imports with no nutrition. Within that same sample, France-tagged rows were 60.0% populated, consistent in direction with the API's 74.0%. Treat the API figures as authoritative and the CSV sample as an illustration of regional skew.)*

**The multilingual field structure is real and it works.** A live fetch of barcode `3228857000852` (Harrys *Pain de mie 100% mie nature*, a French supermarket bread) returned a product object with **365 keys**, including `product_name_fr`, `generic_name_fr`, `ingredients_text_fr` and per-language variants for `ab, ar, ch, de, en, es, hi, id, it, nl, or, pt, ro` — plus OCR-derived variants like `ingredients_text_fr_ocr_1642445989`. `lang: "fr"`, `countries_tags: [en:belgium, en:france, en:luxembourg, en:martinique, en:reunion, en:switzerland]`, `serving_size: "25g"`, `serving_quantity: 25`, `nutrition_data_per: "100g"`. Nutriments came as a triplet per nutrient — `carbohydrates_100g: 48`, `carbohydrates_serving: 12`, `carbohydrates_unit: "g"` — i.e. **per-100 g and per-serving are both pre-computed by OFF**, which is exactly the two numbers the wish-list asks for. Confidence: **High** — this is a captured live response.

Two honest caveats from that same real record:
- `product_name_en` was **"Harys"** — the brand, not a translation. Localised name fields exist but are populated by volunteers and are frequently wrong or absent.
- `categories_tags` contained `en:cheeses`, `en:breaded-cheeses`, `en:frozen-foods` and `en:fermented-milk-products` **for a packet of sliced bread**. OFF's category data is crowdsourced and visibly noisy. Do not build logic that trusts it.

**But OFF is the wrong shape for recipe ingredients.** It is keyed on barcodes and populated with packaged goods. "Farine de blé" as a recipe ingredient is a *generic* food; OFF will offer you two hundred branded flour packets, each with slightly different label values, and no canonical "wheat flour". That is a product catalogue, not a composition table.

### CIQUAL: small, canonical, French-native — and the right shape for ingredients

Downloaded `XML_2020_07_07.zip` (3,554,394 bytes) and parsed directly (confidence: **High** — counted from the actual files):

- `alim_2020_07_07.xml`: **3,185** foods, each with `alim_code`, `alim_nom_fr`, `ALIM_NOM_INDEX_FR`, `alim_nom_eng`, and three levels of group codes.
- `alim_grp_2020_07_07.xml`: **136** food groups.
- `const_2020_07_07.xml`: **67** constituents, **bilingual** (`const_nom_fr` / `const_nom_eng`), including `Energie, Règlement UE N° 1169/2011 (kcal/100 g)` — i.e. **the EU-regulation energy figure**, which is the number a European user expects, not the US Atwater figure.
- `compo_2020_07_07.xml`: **211,898** composition values, each with `alim_code`, `const_code`, `teneur`, a **`code_confiance`** (confidence grade) and a `source_code`.

Real lookups performed against it:

| Query (French) | CIQUAL hits |
|---|---|
| names starting "Farine de blé" | **7** — e.g. `9435 Farine de blé tendre ou froment T65` / *Wheat flour, type 65*; `9436 … T55 (pour pains)`; also T110, T150 |
| names starting "Beurre" | **11** — e.g. `16400 Beurre à 82% MG, doux`, `16402 Beurre à 80% MG, demi-sel` |
| names starting "Sucre" | **4** — `31016 Sucre blanc`, `31017 Sucre roux` |
| names starting "Oeuf" | **23** — `22000 Oeuf, cru`, `22001 Oeuf, blanc… cru`, `22002 Oeuf, jaune… cru` |
| **names starting "Crème fraîche"** | **0** |

That last row is the single most useful thing in this section. French flour types (T45/T55/T65/T110/T150) are modelled properly — something no US database does at all. But **crème fraîche is filed as `19410 Crème de lait, 30% MG, épaisse, rayon frais`**, and searching the whole table for a name starting with "Crème" returns 26 rows dominated by *crème dessert*, *crème brûlée* and *crème de marrons*. A user typing "crème fraîche" gets either nothing or a dessert. See §5.

Also worth noting, as a real-world ingest hazard: the CIQUAL XML is encoded **windows-1252**, uses **CRLF**, and contains at least one **unescaped `<`** (`Panaché préemballé (<1° alc.)`) that makes it *not well-formed XML* — a strict parser (Python's `ElementTree`) fails on line 1274. A tolerant/regex ingest is required. Confidence: **High** — reproduced directly.

### USDA FoodData Central: US-centric, and mostly the wrong foods

Live counts from the FDC search API on 2026-08-20 (confidence: **High** — real API responses, `DEMO_KEY`, `query:"*"`):

| Data type | Records |
|---|---|
| Foundation Foods | **394** |
| SR Legacy | **7,793** |
| Survey (FNDDS) | **5,432** |
| Branded | **433,403** |
| **Total** | **447,137** |

Cross-checked against the archives: the SR Legacy CSV ships its own `all_downloaded_table_record_counts.csv` stating `food = 7793`, `food_nutrient = 644125`, `food_portion = 14449` — an exact match. The Foundation Foods April-2026 archive's `food.csv` has 87,990 rows, but only **469** are `data_type = foundation_food`; the other 87,521 are `sub_sample_food` (75,055), `market_acquisition` (7,577), `sample_food` (4,079) and `agricultural_acquisition` (810) — laboratory sampling metadata, not usable foods. **Anyone quoting "Foundation Foods" as a big database is counting sample rows.**

Update cadence, read from the [Data Type Documentation](https://fdc.nal.usda.gov/data-documentation) comparison table: Foundation Foods and Experimental Foods "April and October of each year"; FNDDS "Every two years"; Branded "Monthly"; **SR Legacy "Final release April 2018"** — i.e. the 7,793-food reference table that everyone actually uses has been **frozen for eight years**.

For a French cook, FDC's naming is the problem: "Wheat flour, white, all-purpose, enriched, bleached" has no T-number, and there is no *crème fraîche*, no *fromage blanc*, no *crème d'Isigny*. **Its one genuinely valuable asset for Kamosu is the portion table — see §5.**

### OpenNutrition: the most *matchable* dataset found, but English-only and possibly abandoned

Downloaded `opennutrition-dataset-2025.1.zip` (**62,927,029 bytes**), which unpacks to `opennutrition_foods.tsv` at **282,413,682 bytes**, **326,759 rows** (confidence: **High** — downloaded and parsed directly). Composition by `type`:

| type | rows |
|---|---|
| grocery | 313,442 |
| everyday | **5,299** |
| restaurant | 4,182 |
| prepared | 3,836 |

**313,442 rows (95.9%) carry an `ean_13` barcode**, and 100% carry a `serving` field. The **`everyday`** subset is the interesting part: 5,299 *generic* foods — exactly the shape a recipe ingredient needs — and it is only **13.8 MB** of TSV (9.4 MB if trimmed to id/name/alternate_names/serving/nutrition_100g).

A real row, read from the file:

```
name:            All-Purpose Flour
alternate_names: ["AP Flour","plain flour","white all-purpose flour","general purpose flour","wheat flour"]
serving:         {"common":{"unit":"cup","quantity":1},"metric":{"unit":"g","quantity":125}}
nutrition_100g:  90 nutrients, energy 364 kcal
```

That single row solves three of the four sub-problems in §5 at once: **synonyms for matching**, a **household measure**, and its **gram weight** — and its 1 cup = 125 g agrees exactly with USDA SR Legacy's independently-derived value for all-purpose flour. That is a genuine cross-validation.

The catches, all verified directly: names are **English-only** (a scan for French names returns Starbucks *Crème Frappuccino®* products, not French foods); the data is self-described as "AI-enhanced", which for a nutrition claim is a real accuracy caveat with no published methodology found; the licence demands per-display attribution (§2); and there has been **exactly one release, `2025.1`, with all files dated 2025-03-28** — seventeen months stale at the time of writing, from a single small company. Durability rating: **worse than OFF or CIQUAL**, better than a commercial API.

---

## 4. Bundling and self-hosting — real sizes, measured

### Open Food Facts bulk exports

All figures from live HTTP `HEAD` requests against `static.openfoodfacts.org` (confidence: **High** — `Content-Length` and `Last-Modified` read off the wire). OFF's [data page](https://world.openfoodfacts.org/data) states exports are "generated nightly", which the timestamps confirm.

| Export | URL | Size | `Last-Modified` |
|---|---|---|---|
| MongoDB dump | `…/data/openfoodfacts-mongodbdump.gz` | **15,475,791,780 B = 14.4 GiB** | 2026-08-20 06:36 |
| JSONL (NDJSON) | `…/data/openfoodfacts-products.jsonl.gz` | **12,708,597,139 B = 11.8 GiB** | 2026-08-20 06:36 |
| CSV (tab-separated) | `…/data/en.openfoodfacts.org.products.csv.gz` | **1,275,171,186 B = 1.19 GiB** (~9 GB uncompressed per OFF's own note) | 2026-08-19 11:48 |
| **Parquet** (Hugging Face) | `huggingface.co/datasets/openfoodfacts/product-database/resolve/main/food.parquet` | **7,781,363,645 B = 7.25 GiB** | — |

**Parquet is confirmed to exist**, hosted on Hugging Face, described by OFF as "a simplified version of the JSONL dump" with debug/duplicate columns filtered out. **Daily delta exports** for the previous 14 days are published at `static.openfoodfacts.org/data/delta/index.txt`, importable in alphabetical order with `mongoimport` — but OFF warns they cannot express deletions, so a periodic full re-import is still needed.

### Is a filtered subset dramatically smaller? Yes — by a factor of ~75.

Verified by doing. I downloaded the first 60 MB of the gzipped CSV, decompressed it (613 MB, **219,990 rows**), and confirmed the export has **211 columns**, of which **123 are `*_100g` nutrient columns**.

**A finding that matters for language:** the CSV export has **zero `_fr`-suffixed columns**. It flattens multilingual names to a single `product_name` in the product's own language. The per-language `product_name_fr` / `ingredients_text_fr` fields visible in the API and the MongoDB/JSONL dumps **do not survive the CSV export**. If Kamosu wants localised names it must use JSONL/Parquet/Mongo, not the CSV.

Keeping 15 useful columns (`code, product_name, brands, quantity, serving_size, serving_quantity, countries_en` + 8 core nutrients) costs **92.4 bytes/row** on average, measured over the 219,990-row sample.

Then I **actually built the SQLite database** — schema plus an FTS5 full-text index using `tokenize="unicode61 remove_diacritics 2"` — from the France-tagged rows of that sample, and VACUUMed it:

| | Rows | Measured / extrapolated size |
|---|---|---|
| Measured (sample, France rows) | 14,962 | **2,007,040 B — 134.1 bytes/row including the FTS index** |
| Extrapolated: France-only OFF | 1,260,089 | **≈ 169 MB** |
| Extrapolated: whole world | 4,695,023 | **≈ 630 MB** |

So: **a France-only barcode-lookup + nutrition database is a ~169 MB SQLite file.** That is smaller than most Docker base images and entirely reasonable to build on first run. The whole world at 630 MB is still bundleable if a bit rude. Confidence: **High** for the measurement, **Medium** for the extrapolation (French rows may be slightly longer on average than the mostly-US sample rows).

**Ingest work**: download 1.19 GiB (or the 7.25 GiB Parquet if localised names are wanted), stream-parse, filter, insert. On a NAS this is a background job of minutes-to-tens-of-minutes and needs ~10 GB of scratch space for the CSV path. That is a real but one-off cost, repeated at whatever refresh cadence the user chooses.

### CIQUAL — measured at 5.3 MB, i.e. free

I built the equivalent SQLite from CIQUAL: all 3,185 foods (FR + EN names), all 211,898 composition values, plus an FTS5 index with diacritic folding. VACUUMed size: **5,345,280 bytes — 5.3 MB.** Confidence: **High** — measured.

**Restricting to the ~10 EU-label nutrients would cut the 211,898 composition rows to roughly 32,000 and bring the file under 1 MB.** At 5.3 MB *unfiltered*, there is no reason to bother filtering. **CIQUAL is small enough to commit into the Docker image with no first-run download at all** — no network dependency, no ingest job, nothing to break.

### USDA FDC

Sizes read directly from the [Download Datasets](https://fdc.nal.usda.gov/download-datasets) page (April 2026 release), cross-checked against the archives I actually downloaded:

| Data type | Zipped | Unzipped |
|---|---|---|
| Foundation Foods (04/2026) | 3.7 MB CSV *(measured: 3,825,741 B)* | 32 MB *(measured: 32,744,127 B)* |
| SR Legacy (04/2018) | 6.7 MB CSV *(measured: 6,074,592 B)* | 54 MB *(measured: 39,791,883 B)* |
| FNDDS 2021-2023 (10/2024) | 200 MB CSV | 1.6 GB |
| Branded (04/2026) | 428 MB CSV | 2.9 GB |
| **Full download, all types** | **460 MB CSV** | **3.1 GB** |

The one piece Kamosu would plausibly want — SR Legacy's `food_portion.csv` — is **918,790 bytes**. Trivial.

### Summary of the bundling axis

| Source | Bundle in image? | Download on first run? | API-only? |
|---|---|---|---|
| CIQUAL | **Yes — 5.3 MB** | unnecessary | no |
| USDA SR Legacy portions | **Yes — <1 MB** | unnecessary | no |
| OpenNutrition `everyday` subset | **Yes — 13.8 MB** | unnecessary | no |
| Open Food Facts (France) | possible at 169 MB, but ODbL redistribution obligations attach | **Yes — best option** | no |
| Open Food Facts (world) | 630 MB — rude but possible | yes | no |
| Nutritionix / Edamam / Spoonacular / FatSecret | **impossible** | **impossible** | **yes — disqualifying** |

---

## 5. The matching problem — this is the actual feature

Getting from `"120g butter, softened"` or `"2 poignées de farine"` to a nutrition row decomposes into four sub-problems, of which only the first is solved by existing tooling.

### 5.1 Quantity/unit parsing — already scoped elsewhere, and solved-ish

Splitting `"120g butter, softened"` into `{120, g, butter, softened}` is the subject of the sibling document `docs/research/web-link-recipe-ingestion.md` §3, which identified `ingredient-parser-nlp` (MIT, CRF model, 288 KB compressed, ~96% self-reported sentence accuracy) as the credible option, and confirmed **no maintained Rust crate exists**. Nothing in this research changes that. Note the dependency: **automatic nutrition cannot ship before ingredient parsing ships.**

### 5.2 Food-name normalisation and matching — no tool exists, and the French case is hard

This is where it breaks. Concretely, from the data actually inspected:

- `"crème fraîche"` → CIQUAL has **no** entry starting with those words. The correct answer is `19410 Crème de lait, 30% MG, épaisse, rayon frais`. No string-similarity metric gets you there from "crème fraîche"; it requires a **synonym table**.
- `"farine"` (unqualified, as most French recipes write it) → CIQUAL offers T45/T55/T65/T110/T150 as separate foods with materially different fibre and mineral values. Something has to *choose*, and the honest choice is T55, which is a guess.
- `"beurre"` → 11 CIQUAL entries differing by fat percentage and salt. Again a guess.
- OFF, by contrast, returns hundreds of *branded* flours for "farine", none of them canonical.

**OpenNutrition's `alternate_names` field is the only dataset-shipped synonym mechanism found** in any of these sources (`"AP Flour", "plain flour", "white all-purpose flour", "general purpose flour", "wheat flour"` for All-Purpose Flour) — and it is English-only. CIQUAL's `ALIM_NOM_INDEX_FR` is an inverted-word-order index name, not a synonym list. OFF's taxonomy files do contain multilingual synonyms, but exploiting them is a separate ingest project.

**Conclusion: any credible French ingredient→nutrition matcher needs a hand-curated alias table that does not exist and would have to be built.** This is the true cost of the feature, and it is not a coding cost — it is a data-curation cost that recurs forever as users add ingredients.

### 5.3 Household measures → grams: partially solved, and only for American cooking

**USDA FoodData Central's `food_portion` table is the only real gram-weight resource among these datasets.** Measured directly from SR Legacy's `food_portion.csv` (confidence: **High**):

- **14,449 portion rows**, of which **14,449 have a populated `gram_weight`** — 100%.
- They cover **7,533 of 7,793 foods (96.7%)**.
- Schema: `id, fdc_id, seq_num, amount, measure_unit_id, portion_description, modifier, gram_weight, data_points, footnote, min_year_acquired`.

Real rows pulled for the foods a recipe actually uses:

```
Wheat flour, white, all-purpose, enriched, bleached  →  1 cup = 125 g
Butter, salted                                       →  1 tbsp = 14.2 g; 1 cup = 227 g;
                                                        1 stick = 113 g; 1 pat (1" sq, 1/3" high) = 5 g
Cream, fluid, heavy whipping                         →  1 tbsp = 15 g; 1 fl oz = 29.8 g;
                                                        1 cup, fluid = 238 g; 1 cup, whipped = 120 g
```

That is genuinely good data. Four caveats, all verified:

1. **The unit lives in a free-text field.** All 14,449 SR Legacy rows have `measure_unit_id` pointing at the literal value **`undetermined`**; the real unit is in the free-text `modifier` column. Top modifiers by frequency: `oz` (3,166), `cup` (1,691), `tbsp` (548), `fl oz` (492), `lb` (281), `steak` (280), `serving` (265), `slice` (186), `tsp` (171), `fillet` (162), and things like `piece, cooked, excluding refuse (yield from 1 lb raw meat with refuse)` (212). **Using this table means parsing English portion prose.**
2. **The units are American.** cup / oz / lb / fl oz / stick / pat. A French recipe says *cuillère à soupe*, *pincée*, *noix de beurre*, *verre*. Searching all 14,449 modifiers for "handful", "knob" or "pinch" returns **zero rows**.
3. **It is per-food, not universal.** 1 cup of butter is 227 g; 1 cup of flour is 125 g; 1 cup of whipped cream is 120 g while 1 cup of the same cream unwhipped is 238 g. There is no such thing as a global cup→gram factor, which is exactly why Tandoor's `UnitConversion` model carries a nullable **`food` foreign key** ([`cookbook/models.py#L909-L933`](https://github.com/TandoorRecipes/recipes/blob/93c9a1763e3509482edb7e5c73511c1839ada617/cookbook/models.py#L909)).
4. **Foundation Foods' portion table is not a substitute.** Its 10,951 rows are dominated by `egg` (6,129) and `Banana` (916) — per-sample laboratory weights from specific studies, not a general household-measure table.

**"2 handfuls of flour" is not solvable by any dataset here.** It requires an arbitrary house convention. The honest engineering answer is to store the ingredient as unquantified and exclude it from the total — which is precisely what Tandoor's `FoodPropertyHelper` does when `i.amount == 0 or i.no_amount`.

### 5.4 What the mature self-hosted apps actually do — read in their source

This is the decision-relevant section the brief asked for, and the answer is unambiguous.

**Mealie** (`mealie-recipes/mealie` @ `ed2f87907b52642c8a15d8a37ac2cc1d07bcd864`):
- [`mealie/db/models/recipe/nutrition.py`](https://github.com/mealie-recipes/mealie/blob/ed2f87907b52642c8a15d8a37ac2cc1d07bcd864/mealie/db/models/recipe/nutrition.py): a `recipe_nutrition` table with a `recipe_id` FK and **eleven `sa.String` columns** — `calories`, `carbohydrate_content`, `fat_content`, … Every one is a *string*, matching schema.org's "308 calories" style.
- [`mealie/schema/recipe/recipe_nutrition.py`](https://github.com/mealie-recipes/mealie/blob/ed2f87907b52642c8a15d8a37ac2cc1d07bcd864/mealie/schema/recipe/recipe_nutrition.py): same eleven fields, all `str | None`, with `coerce_numbers_to_str=True`.
- [`mealie/db/models/recipe/ingredient.py#L153`](https://github.com/mealie-recipes/mealie/blob/ed2f87907b52642c8a15d8a37ac2cc1d07bcd864/mealie/db/models/recipe/ingredient.py#L153): `IngredientFoodModel` has `name`, `plural_name`, `description`, `label`, `aliases`, `extras` — and **no nutrition fields whatsoever**.
- [`frontend/app/composables/recipes/use-recipe-nutrition.ts`](https://github.com/mealie-recipes/mealie/blob/ed2f87907b52642c8a15d8a37ac2cc1d07bcd864/frontend/app/composables/recipes/use-recipe-nutrition.ts) is 57 lines of **display labels and unit suffixes only** — no arithmetic, no per-serving scaling.

  **Verdict: Mealie does not compute nutrition. It is a manual/scraped text field.** It also does not scale it when you change servings.

**KitchenOwl** (`TomBursch/kitchenowl` @ `09aaf5fbd2343fcc10b12e906c63c3764dd38919`): a case-insensitive grep for `nutrition|calorie|kcal` across the entire Python backend returns **no model, route or service files**. The `backend/app/models/` directory contains no nutrition model. **Verdict: no nutrition feature at all.**

**Grocy** (`grocy/grocy` @ `acd061e2cde09fc5bc4e34673021039b25ab7067`):
- `products.calories INTEGER` — a **single integer**, user-entered ([`migrations/0103.sql#L69`](https://github.com/grocy/grocy/blob/acd061e2cde09fc5bc4e34673021039b25ab7067/migrations/0103.sql#L69)). No protein, fat or carbohydrate columns exist anywhere in the migrations.
- Recipe totals are pure SQL: `rp.amount * … * IFNULL(p_effective.calories, 0) AS calories` ([`migrations/0249.sql#L34`](https://github.com/grocy/grocy/blob/acd061e2cde09fc5bc4e34673021039b25ab7067/migrations/0249.sql#L34)).
- Its [`plugins/OpenFoodFactsBarcodeLookupPlugin.php`](https://github.com/grocy/grocy/blob/acd061e2cde09fc5bc4e34673021039b25ab7067/plugins/OpenFoodFactsBarcodeLookupPlugin.php) calls `GET https://world.openfoodfacts.org/api/v2/product/{barcode}?fields=product_name,image_url,product_name_{locale}` and returns **name, location, quantity unit and image only**. It even builds the localised field name as `'product_name_' . substr(GROCY_LOCALE, 0, 2)` — independent confirmation that OFF's `product_name_fr` pattern is real and used in production. **It fetches no nutrition.**

  **Verdict: Grocy tracks calories only, typed by hand, and deliberately does not import nutrition from the barcode lookup it already has.**

**Tandoor** (`TandoorRecipes/recipes` @ `93c9a1763e3509482edb7e5c73511c1839ada617`) — the only one that really does this:
- [`cookbook/models.py#L990`](https://github.com/TandoorRecipes/recipes/blob/93c9a1763e3509482edb7e5c73511c1839ada617/cookbook/models.py#L990): `PropertyType` with categories `NUTRITION | ALLERGEN | PRICE | GOAL | OTHER`, an `fdc_id` integer, and an `open_data_slug`.
- `Property` (`property_amount` + FK to `PropertyType`) and `FoodProperty` (M2M `Food` ↔ `Property`) — per-food nutrition, per-100-of-something.
- [`cookbook/helper/property_helper.py`](https://github.com/TandoorRecipes/recipes/blob/93c9a1763e3509482edb7e5c73511c1839ada617/cookbook/helper/property_helper.py) `FoodPropertyHelper.calculate_recipe_properties()` walks the recipe's ingredients, converts each to the food's declared `properties_food_unit`, and accumulates `(c.amount / i.food.properties_food_amount) * p.property_amount`. **It exports three failure flags to the UI**: `missing_value`, `missing_unit`, and `missing_conversion` (which even names the two units it couldn't bridge). This is what honest partial nutrition looks like in production.
- [`cookbook/helper/unit_conversion_helper.py`](https://github.com/TandoorRecipes/recipes/blob/93c9a1763e3509482edb7e5c73511c1839ada617/cookbook/helper/unit_conversion_helper.py) hard-codes a `CONVERSION_TABLE` with separate `weight` and `volume` sub-tables (g/kg/ounce/pound; ml/l/tbsp/tsp/us_cup/imperial_*) and raises `ConversionException` when asked to cross between them. **Volume→mass is only possible via a per-food `UnitConversion` row.**
- Nutrition data is hydrated **from the live USDA API**: [`cookbook/views/api.py#L1142`](https://github.com/TandoorRecipes/recipes/blob/93c9a1763e3509482edb7e5c73511c1839ada617/cookbook/views/api.py#L1142) calls `https://api.nal.usda.gov/fdc/v1/food/{fdc_id}?api_key={FDC_API_KEY}`, and [`#L3052`](https://github.com/TandoorRecipes/recipes/blob/93c9a1763e3509482edb7e5c73511c1839ada617/cookbook/views/api.py#L3052) proxies FDC search. **The user must paste an `fdc_id` per food** — the UI links straight to `fdc.nal.usda.gov/food-search` and asks them to go find it ([`vue3/src/pages/PropertyEditorPage.vue#L20`](https://github.com/TandoorRecipes/recipes/blob/93c9a1763e3509482edb7e5c73511c1839ada617/vue3/src/pages/PropertyEditorPage.vue#L20)). **Tandoor did not solve automatic matching either. It made the human do it.**

**Tandoor's escape hatch is worth stealing: Open Tandoor Data.** [`TandoorRecipes/open-tandoor-data`](https://github.com/TandoorRecipes/open-tandoor-data) @ `3ba9dc4fb4a1cc438ebd006744795b2e41303dea` — a community-curated seed dataset, licensed **ODbL (schema) + DbCL (contents)** per its README, built into per-language JSON. Counted directly from `build/meta.json` and `build/fr.json`:

- **391 foods** in the base/English build; **390 in French**, and translations in **25 languages** including `de, nl, es, it, pt, pl, sv, da, uk, ru`.
- **163 per-food unit conversions**, **4 nutrition property types only** — `property-calories [kcal] fdc=1008`, `property-proteins [g] fdc=1003`, `property-fats [g] fdc=1004`, `property-carbohydrates [g] fdc=1005`.
- Each food carries `properties.food_amount: 100`, `food_unit: "unit-g"`, a `source` URL (usually an `fdc.nal.usda.gov` food-details link) and an `fdc_id`.
- Total repo size **6.4 MB**; the French build file alone is **275 KB**.

Data-quality caveat, read directly from `build/fr.json`: conversion sources include `"My pantry"` and `coolconversion.com` alongside proper citations. This is community best-effort, not measured science. And note the French *build* is a **translation of English food names** — `food-butter-unsalted` → "Beurre doux" — so it inherits an Anglo-American view of what foods exist (no T55 flour, no crème fraîche distinctions).

**Interpretation.** Four mature apps, four different answers, and the aggregate signal is loud: **automatic nutrition is expensive enough that three of four skipped it, and the fourth shipped a version that requires manual per-food linking plus a per-food unit-conversion table, and still tells the user when it can't compute.** That is a strong prior against putting automatic nutrition in Kamosu v1.

### 5.5 How wrong would fuzzy matching be, and is "approximate nutrition" acceptable?

No published benchmark for ingredient-name→food-database matching accuracy was found (confidence: **High** that none was located; **Medium** that none exists). What the data *does* let us say concretely:

- **Systematic, not random, error.** Choosing T55 for an unqualified "farine" or 30%-fat cream for "crème" biases every recipe the same direction. Errors do not cancel across a recipe.
- **The spread within a single ingredient name is large.** CIQUAL's four wheat-flour types and eleven butters differ materially in fat, fibre and mineral content; USDA's own data has 1 cup of the *same* heavy cream weighing 238 g fluid or 120 g whipped — a 2× error from picking the wrong portion row.
- **Failures are silent unless you make them loud.** A fuzzy matcher that returns *Crème dessert au chocolat* for "crème fraîche" produces a confident, plausible, badly wrong number.
- **The genuinely dangerous case is medical.** Someone counting carbohydrates for diabetes, or sodium for hypertension, cannot use a number derived from an unverified fuzzy match.

**Assessment: approximate nutrition is acceptable if and only if it is visibly approximate and per-ingredient auditable** — the user must be able to see which database row each ingredient matched to, change it, and see which ingredients were excluded. That is Tandoor's `missing_value` / `missing_unit` / `missing_conversion` design, and it is the right one. A single opaque "412 kcal" badge on a recipe card, computed by fuzzy match, is actively misleading and Kamosu should not ship it.

---

## 6. Barcode pantry (post-v1)

### Is Open Food Facts the source? Yes, unambiguously.

Nothing else combines a European product catalogue with barcodes and an open licence: 4,695,023 products, 1,260,089 French, 932,743 of those with nutrition complete, keyed on EAN/UPC, ODbL. OpenNutrition's 313,442 `ean_13` rows are a distant second and English-only. USDA Branded is US-centric.

Confirmed by the market too: Grocy's only barcode-lookup plugin targets Open Food Facts.

### Does barcode lookup need the dump or an API call?

**Either works, and the offline path is the better one.** A live lookup is exactly one HTTP GET — `GET https://world.openfoodfacts.org/api/v2/product/{barcode}.json` — which I exercised directly. Two real results:

- `3017620422003` (Nutella) → HTTP 200, `status: 1`, **149,568 bytes** of JSON with **365 fields**.
- `3033491602054` → HTTP 200, `{"code":"3033491602054","status":0,"status_verbose":"product not found"}` — a real miss, 72 bytes. Coverage is broad but not total.

Use `?fields=` to trim the response, as Grocy does — the full product object is 150 KB.

**Documented OFF rate limits** ([OFF API docs](https://openfoodfacts.github.io/openfoodfacts-server/api/), confidence: **Medium** — read via fetch-and-summarise, not the raw source file): **15 requests/min/IP for product reads**, **10 requests/min/IP for search**, plus a required custom `User-Agent` in the form `AppName/Version (ContactEmail)`. I hit the limiter repeatedly during this research — OFF served an HTML "Page temporarily unavailable" page instead of JSON, which is a JSON-parse failure, not an HTTP error code. **Any live-API integration must handle that failure mode explicitly.**

**The offline path exists and is cheap: the 169 MB France-only SQLite measured in §4.** For a pantry app, offline is strictly better — no rate limits, no network round-trip at the till, works when OFF is down, works when OFF is gone. The obvious hybrid: **local dump first, live API as a fallback for misses**, with a periodic refresh. Given §2's analysis, download-on-first-run also sidesteps ODbL redistribution obligations entirely.

### Scanning a barcode in a PWA

**The native Barcode Detection API is not usable on iPhone.** From [MDN browser-compat-data](https://github.com/mdn/browser-compat-data/blob/main/api/BarcodeDetector.json) (confidence: **High** — read from the raw JSON):

| Browser | Support |
|---|---|
| Chrome (desktop) | 88+, **partial — "Supported on ChromeOS and macOS only"** |
| Chrome Android | **83+ — full** |
| Edge | 83+, macOS only |
| Firefox / Firefox Android | **No** ([bug 1553738](https://bugzil.la/1553738)) |
| **Safari** | **17, behind a preference flag: "Shape Detection API" = true** |
| **Safari iOS** | **mirrors Safari — i.e. flag-gated, effectively unavailable** |

The API is marked `experimental: true` against a **WICG** draft spec, not a W3C standard. **Practical conclusion: a JavaScript/WASM scanner is not a fallback, it is the primary implementation.** Feature-detect `BarcodeDetector` and use it where present (Android Chrome), but ship the library path for everyone else — including the owner's iPhone.

Library options, from the npm registry (confidence: **High** — registry API + downloads API):

| Package | Licence | Latest | Last published | Weekly downloads |
|---|---|---|---|---|
| **`zxing-wasm`** | MIT | 3.1.3 | **2026-08-14** | **1,467,782** |
| `@zxing/library` | Apache-2.0 | 0.23.0 | 2026-04-29 | 1,305,283 |
| `html5-qrcode` | Apache-2.0 | 2.3.8 | **2023-04-15** | 1,186,847 |
| `@ericblade/quagga2` | MIT | 1.12.1 | 2025-12-20 | 42,697 |

**`zxing-wasm` is the clear pick**: MIT, actively published (six days before this research), 1.47M weekly downloads, and a WASM build of the canonical ZXing decoder — meaningfully faster than the pure-JS `@zxing/library`. `html5-qrcode` has comparable downloads but **no release in over three years**; it should be treated as unmaintained.

One iOS-specific caveat to plan for (confidence: **Medium** — not tested on a device for this research): camera access via `getUserMedia` on iOS Safari requires HTTPS and a user gesture, and historically did not work inside third-party in-app browsers. A self-hosted Kamosu must therefore be served over TLS for scanning to work at all — which it should be anyway.

---

## 7. Rust ecosystem — honest answer: almost nothing exists, but you don't need much

Searched via the crates.io API directly (not the web UI). Confidence: **High**.

**Food-database clients: nothing.** `q=openfoodfacts` returns **zero** crates. `q=fooddata` returns zero. `q=usda` returns only unrelated homonyms (USD/USDZ 3-D formats, an Altius USD token SDK). `q=open food facts` surfaces exactly one on-topic crate: **`tree-sitter-off-taxonomy`** (v0.2.5, 2026-08-06) — a Tree-sitter grammar for OFF *taxonomy* files, **76 total downloads**. `q=nutrition` returns only toys: `nutrition_rs` (116 dl), `cooklang-reports-nutrition` (51 dl), `cookmd-nutrition-client` (65 dl), `nutrition-ai` (a Gemini wrapper, 824 dl).

**This mirrors the sibling doc's finding for recipe scraping and ingredient parsing exactly: there is no Rust food ecosystem.** But unlike ingredient parsing (which needs a real NLP model), this doesn't matter — the work here is "download a CSV, filter it, insert into SQLite", which needs no domain crate at all. `reqwest` + `flate2` + `csv` + `rusqlite` covers the entire ingest path.

**String matching: excellent options exist.**

| Crate | Downloads | Latest | Assessment |
|---|---|---|---|
| [`strsim`](https://crates.io/crates/strsim) | **1,001,819,913** | 0.11.1 (2024-04-02) | The standard. Levenshtein, Jaro-Winkler, Damerau, Sørensen-Dice. Stable, not stale |
| [`unicode-normalization`](https://crates.io/crates/unicode-normalization) | 535,806,734 | 0.1.25 (2025-10-30) | NFC/NFD — needed to fold `é`/`e` before comparing |
| [`rusqlite`](https://crates.io/crates/rusqlite) | 94,710,309 | 0.40.2 (2026-08-08) | Already the obvious SQLite binding |
| [`fuzzy-matcher`](https://crates.io/crates/fuzzy-matcher) | 28,809,846 | 0.3.7 (**2020-10-04**) | Popular but ~6 years stale |
| [`frizbee`](https://crates.io/crates/frizbee) | 2,773,567 | 0.13.0 (2026-08-13) | SIMD Smith-Waterman; designed for editor-style fuzzy *filtering*, not record linkage |

### FTS5 vs trigram in SQLite — tested locally

**Verified by doing** against SQLite 3.45.1 (confidence: **High** — real queries run):

```sql
-- Diacritic folding: "creme" finds "Crème"
CREATE VIRTUAL TABLE u USING fts5(name, tokenize="unicode61 remove_diacritics 2");
INSERT INTO u VALUES('Crème fraîche épaisse');
SELECT name FROM u WHERE u MATCH 'creme';   -->  Crème fraîche épaisse   ✓

-- Trigram: substring matching, no word-boundary requirement
CREATE VIRTUAL TABLE t USING fts5(name, tokenize='trigram');
INSERT INTO t VALUES('Farine de blé tendre ou froment T55 (pour pains)');
SELECT name FROM t WHERE t MATCH 'farine';  -->  matched   ✓
```

**Recommendation for Kamosu's matcher:** `unicode61 remove_diacritics 2` as the primary tokenizer (word-level, index-efficient, and it makes "creme"/"crème" and "ble"/"blé" equivalent for free — essential for French where users often type without accents), with `strsim`'s Jaro-Winkler as a re-ranker over the FTS5 candidate set. A `trigram` index is the fallback for substring/typo cases but is larger and slower. The 169 MB and 5.3 MB figures in §4 were both measured **with the FTS5 index included**, so the index cost is already in those numbers.

---

## Confidence key

- **High** — read directly from a downloaded file, a cloned repository's source code, a live HTTP/API response captured for this research, or a measurement I performed (file sizes, record counts, SQLite builds).
- **Medium** — a first-party page read via fetch-and-summarise rather than raw source; a figure extrapolated from a measured sample; a licence summarised rather than re-read in full; a claim about absence of evidence.
- Anything weaker is flagged inline as "not found", "not tested" or "unquantified" rather than asserted.

---

## Recommendation

### For v1: ship a manual nutrition field. Do not build automatic nutrition.

1. **Nutrition in v1 is a per-recipe, user-editable field.** Model it as a small set of numeric fields with explicit units (`energy_kcal`, `protein_g`, `fat_g`, `saturated_fat_g`, `carbohydrates_g`, `sugars_g`, `fibre_g`, `salt_g`) **plus a basis flag** (`per_100g` / `per_serving`) so the wish-list's "per 100 g and per serving" can be derived. Prefer numeric-with-basis over Mealie's all-strings design — Mealie's strings exist because it inherits schema.org's "308 calories" text, and the sibling doc already established Kamosu should parse that at import rather than store it raw. Populate it automatically from `schema.org/NutritionInformation` on web import (the sibling doc confirmed BBC Good Food emits it), and let the user type or correct it otherwise. **This is what Mealie does, what Grocy does in miniature, and what KitchenOwl skips entirely — it is not a weak position, it is the market norm.**
2. **Bundle CIQUAL in the image as a read-only lookup table.** 5.3 MB with a full-text index, French Open Licence, 3,185 French foods with proper T-number flours and EU-regulation kcal, bilingual FR/EN names, and a per-value confidence code. Cost: one ingest script run at build time and a `<1 MB` git-LFS-free binary asset, or ship the XML (3.5 MB zipped) and build the SQLite on first boot. Give the user a **manual "attach a CIQUAL food to this ingredient"** action and a "fill nutrition from ingredients" button that computes only from ingredients the user has explicitly linked, showing plainly which ones were skipped. That is Tandoor's honest-partial design at a tenth of the cost, and it makes the curated alias table (§5.2) grow organically from real use instead of needing to exist up front.
   - Attribution: a line naming ANSES/CIQUAL and the 2020-07-07 release date. Trivial.
   - Watch out for the encoding trap: windows-1252, CRLF, and at least one unescaped `<` that breaks strict XML parsers.
3. **Do not attempt automatic ingredient→nutrition matching in v1.** It is blocked on ingredient-line parsing (a separate deferred ticket per the sibling doc), it needs a French synonym table that no dataset provides, it needs per-food volume→mass conversions that no European dataset provides, and the four apps surveyed prove the cost is real. If it ships badly it produces confident wrong numbers, which is worse than no numbers.
4. **Do not integrate any commercial API. Ever, on current terms.** Spoonacular contractually requires deleting cached data after one hour and deleting everything on cancellation; Edamam prohibits building a local copy and mandates an attribution image on pain of immediate suspension; Nutritionix has withdrawn its public free tier; FatSecret's pricing is sales-gated. None survives a "what if this vendor disappears" test, which is the whole point of self-hosting.
5. **Do not use the USDA FDC API the way Tandoor does.** A default install would be on `DEMO_KEY` at 30 requests/hour, and asking the user to paste an `fdc_id` per food is a bad experience for a French cook whose ingredients aren't in it. If FDC data is wanted later, take the **bulk download** (CC0, no key, no rate limit) — specifically SR Legacy's `food_portion.csv` at 919 KB for the gram-weight table, which is the one genuinely irreplaceable thing USDA has.

### For later — in this order

6. **Ingredient parsing ships first** (existing ticket). Nothing downstream works without it.
7. **Then per-food nutrition linking and recipe totals**, built on the manual CIQUAL links from v1 plus USDA SR Legacy portions for cup/tbsp gram weights. Copy Tandoor's three failure flags — `missing_value`, `missing_unit`, `missing_conversion` — verbatim as a design pattern. Never show a total without showing what was excluded from it.
8. **Consider seeding from Open Tandoor Data** (ODbL+DbCL, 275 KB for French, 390 foods, 163 per-food conversions, `fdc_id` links) as a starter alias/conversion table. It is community best-effort, not measured science — some conversion sources are literally `"My pantry"` — but it is free, correctly licensed, already translated into French and 24 other languages, and it is the only ready-made *multilingual* food-with-conversions dataset found anywhere in this research.
9. **Barcode pantry, when it comes: ingest an Open Food Facts subset locally.** France-only measures at **169 MB** as SQLite with a full-text index; the whole world at ~630 MB. Download-on-first-run from OFF's nightly export rather than baking it into the image — this sidesteps ODbL §4.4/§4.6 redistribution obligations entirely (§2), keeps the image small, and lets the user refresh on their own schedule. Fall back to the live API for misses, with the required custom `User-Agent`, the documented 15 req/min budget, and explicit handling for OFF serving an HTML rate-limit page instead of JSON.
10. **Client-side scanning: `zxing-wasm`, not the Barcode Detection API.** Native support is flag-gated on Safari and therefore unavailable on the owner's iPhone; feature-detect and use it on Android Chrome as an optimisation only. Serve Kamosu over HTTPS or `getUserMedia` won't grant camera access at all.

### The cheapest credible version, in one paragraph

**Bundle a 5.3 MB CIQUAL SQLite file. Add one manual nutrition block per recipe, filled from schema.org on web import. Let the user optionally link an ingredient to a CIQUAL food, and offer a "compute from linked ingredients" button that is honest about what it skipped.** Zero external API dependencies, zero API keys, zero rate limits, zero recurring cost, ~5 MB of disk, French-native food names, and an EU-regulation kcal figure. Everything above that line — automatic matching, barcode scanning, a 169 MB Open Food Facts mirror — is a later ticket that this foundation does not block.

### Where confidence is weakest

- The **169 MB France-only OFF figure** is extrapolated from a 14,962-row measured sample (134.1 bytes/row) to 1,260,089 products. The sample skews toward US-registered barcodes; French rows may average slightly longer. Treat 169 MB as ±30%.
- **OFF's 75.5% "nutrition-facts-completed"** rate is taken from a state tag whose exact semantics were not verified against the OFF codebase — it may be set when a contributor records that a package *has no* nutrition panel. The direction (French products are well covered) is corroborated by an independent CSV sample, but the precise percentage is Medium confidence.
- **FDC Branded's 433,403** is the API's `totalHits` for `query:"*"`, which may not be a true table count; the archive itself is 2.9 GB unzipped and was not downloaded.
- **Frida (Denmark)** and **EuroFIR** were assessed from summarised fetches rather than downloaded data; neither is a candidate for v1, so this was not pursued further.
- **No published accuracy benchmark for ingredient→food matching was found.** §5.5's assessment is reasoned from the dataset structures actually inspected, not measured. If automatic matching is ever seriously scoped, measuring it against a sample of the owner's own 86 Crouton recipes (`docs/research/crouton-real-export.md`) would be the cheapest way to get a real number.
