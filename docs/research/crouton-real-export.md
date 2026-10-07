# Crouton: what a real 86-recipe library actually contains

Source: `Crouton Recipes - 20 Aug 2026.zip`, the first Operator's own library, exported 2026-08-20.
Files lived at `samples/crouton/` (gitignored — 110 MB, mostly photos, and personal);
since #128 this export sits in `samples/crouton-2026-08-20/` and the 23 September 2026
one is the corpus. See *Update, 23 September 2026* at the end.
Resolves issue [#5](https://github.com/BattermanZ/Kamosu/issues/5).

This is the real-data counterpart to `recipe-app-formats.md`, which read the format from
documentation. Where the two disagree, this document wins: it is measured.

## The file format

One `.crumb` file per recipe. Despite the extension it is **plain, unwrapped, single-line
JSON** — no encryption, no envelope, no ZIP-inside-ZIP. `cat` it and read it. An importer
is a JSON parse, not a reverse-engineering job.

Whole-library export produces a flat ZIP of `<Recipe Name>.crumb` files. There is no
manifest, no folder file, no cross-recipe index — so **the filename is the only ordering
you get**, and duplicate names get a `-1` suffix.

### Fields, measured across all 86 recipes

| Field | Present | Non-empty | Notes |
|---|---|---|---|
| `name`, `uuid`, `defaultScale` | 86 | 86 | always there |
| `ingredients` | 86 | 82 | 863 ingredient rows total |
| `steps` | 86 | 80 | 599 step rows total |
| `images` | 86 | 59 | base64 JPEG, inline |
| `webLink` | 66 | 66 | where it came from |
| `sourceName` | 60 | 60 | e.g. `youtube.com` |
| `serves` | 54 | 54 | integer |
| `duration` / `cookingDuration` | 45 / 44 | — | integer minutes, prep vs cook |
| `sourceImage` | 44 | 44 | base64 JPEG, second copy |
| `neutritionalInfo` *(sic)* | 25 | 17 | free text; the typo is in the format |
| `notes` | 21 | 19 | free text |
| `tags`, `folderIDs`, `isPublicRecipe` | 86 | **0** | present but never populated |
| `duplicatedFromRecipeUUID` | 1 | 1 | and its source is not in the export |

**`tags` and `folderIDs` are empty in all 86 recipes.** Whatever organisation the first Operator has
in Crouton, the export does not carry it — worth confirming with him before assuming he
has none.

> **Update, 23 September 2026 (#128):** a new export populates `tags` — 64 of the 86
> recipes, 21 distinct tags. `folderIDs` is still empty. See the note at the end.

## What this tells us about the domain model

### 1. The lost original line is real, and it is worse than expected

`recipe-app-formats.md` predicted Crouton loses the free-text ingredient line. The data
confirms it and shows *how*: the unit noun gets shoved into the food name.

```
"cloves minced garlic"                    qty: {amount: 2, quantityType: ITEM}
"block soft or silken tofu"               qty: {amount: 16, quantityType: OUNCE}
"to taste Freshly ground black pepper"    qty: null
", boneless, skinless chicken breasts, trimmed (8-ounce)"   qty: {amount: 2, ITEM}
"dry red wine (750mL)"                    qty: {amount: 1, quantityType: BOTTLE}
```

That fourth one begins with a comma. These are not edge cases — they are what the field
routinely holds. The structure is a lossy *guess* laid over a line that was never kept, and
the guess degrading leaves debris in the food name with no way to recover the original.

**For Kamosu:** this is the single strongest argument in the corpus for `original_text`
as a first-class, always-preserved field. It also means the Crouton importer must
**reconstruct** an approximate original line (`"2 cloves minced garlic"`) rather than find
one — and should mark it as reconstructed, not authored.

### 2. Roughly a quarter of ingredients have no quantity at all

**239 of 863 rows (28%) have `quantity: null`.** Not spread evenly — whole recipes go
without: *Salmon Cold Shirataki Noodle Asian Salad* (17/17), *Avocado Rice* (12/12),
*Ceviche* (12/13), *Beef Short Ribs* (11/11).

A model that treats quantity as required, or that renders an ingredient list as a
quantity column, breaks on a quarter of this library. Quantity is optional, and
"to taste" is a legitimate amount.

### 3. The unit vocabulary is a closed enum, and it is Anglo

`ITEM`(136) `CUP`(118) `TABLESPOON`(113) `GRAMS`(104) `TEASPOON`(92) `OUNCE`(16)
`POUND`(12) `MILLS`(9) `CAN`(5) `KGS`(5) `SECTION`(5) `BOTTLE`(3) `LITRES`(3)
`PACKET`(2) `PINCH`(1)

Fifteen values, cups and tablespoons dominating a library with nine French recipes in it.
No millilitre spelled properly (`MILLS`), no European volume habits. Fractions are stored
as decimals (`0.25`, `0.5`) plus an occasional `secondaryAmount` for ranges ("2–3 cloves").

**For Kamosu:** a closed enum this small is a ceiling, not a foundation. Kamosu's unit
model should be open, and the importer needs a Crouton-enum → Kamosu-unit mapping table.

### 4. Sections are faked with sentinel rows — in two different ways

Crouton has no section container. It marks sections inline, and inconsistently:

- **Steps**: a step row with `isSection: true` whose `step` text is the heading
  (20 rows — "Make the Pizza Dough", "Assemble the Pizza", "Bake The Pizza").
- **Ingredients**: an ingredient row whose `quantityType` is the sentinel `SECTION`
  (5 rows — "Chicken", "Sauce", "Optional toppings", "For 500g chicken").

Two mechanisms for one concept, both riding in the item list. A Kamosu model with real
ingredient/step *groups* imports both cleanly; a flat model inherits the hack.

### 5. Duplicate-as-version is happening in the wild, by hand

Three name-collision pairs in 86 recipes:

| Pair | What differs |
|---|---|
| Beef Bourguignon / -1 | 11 vs 16 ingredients, 9 vs 7 steps, **same source link**, one has notes |
| Korean Fried Chicken / -1 | 16 vs 19 ingredients, **identical steps**, same source link |
| Dan Dan Noodles / -1 | one is an empty stub (link only), one is fully filled in |

Same link, same servings, drifting ingredients — that is a *version*, saved as a second
recipe because the app has nowhere else to put it. `duplicatedFromRecipeUUID` exists in the
schema and is used exactly once, pointing at a recipe **not present in the export** — so
even Crouton's own lineage field does not survive its own backup.

**This is direct evidence for issue [#9](https://github.com/BattermanZ/Kamosu/issues/9)**
(attempts, adjustments, versions): the need is not hypothetical, it is already being met
badly by copy-paste.

### 6. `notes` is where cooking attempts are hiding

19 recipes carry notes, and they split into two kinds:

- **Serving advice** — "You can serve it with mashed potatoes, pasta or whatever you feel
  like!", "The recipe is best served hot." (usually scraped from the source site)
- **Attempt feedback** — "Pretty spicy with one tbsp of chilli crunch.",
  "Try adding the seaweed crunches next time"

The second kind is a cooking log with no home, flattened into the same free-text box as
the first. Kamosu splitting *recipe notes* from *attempt notes* is not a new feature — it
is giving existing behaviour somewhere to live.

### 7. Nutrition is unparsed scrape debris

`neutritionalInfo` is one free-text blob, and the shape varies per source site:

```
Calories: 738 kcal,\nCarbohydrates: 63 g,\nProtein: 26 g, ...
Sugar: 5 grams,\nProtein: 35 grams,\nCalories: 1139 calories, ...
@context: http://schema.org,\nFiber: 7 grams, ...        ← JSON-LD keys leaked in
\t•\tCalories: 670 kcal\n\t•\tFat: 41.4 g ...              ← bulleted, from a different site
610 Calories\n67g Carbs\n16g Fat\n210 Calories ...        ← two servings, unlabelled
```

Units drift (`g` / `grams`), the schema.org `@context` key leaks straight through, and one
entry is two nutrition panels concatenated. Crouton scrapes it and never parses it.

**For Kamosu:** either parse nutrition into structured fields at import, or keep it as
opaque text and be honest that it is not queryable. The half-measure Crouton took gives
you neither. Relevant to issue [#4](https://github.com/BattermanZ/Kamosu/issues/4).

### 8. Photos are inline base64, and that is the whole 110 MB

Every image is a base64 JPEG **inside the JSON**. 109.4 MB of the 110 MB export is image
payload. One recipe (*Beef Short Ribs*, 20 MB) holds a single 7 MB photo. There are two
image fields — `images` (user photos) and `sourceImage` (the scraped hero shot) — stored
identically, and 44 recipes carry both, i.e. the same kind of data in two places.

Counts: 58 recipes have one image, one has two, 27 have none. **The maximum is two.**

**And there are no per-step photos.** Zero of 599 step rows carry any key beyond
`uuid`/`isSection`/`step`/`order`. Crouton steps are text only. The wish-list feature has
no prior art here and nothing to import from — confirming `recipe-app-formats.md`.

Relevant to the map's *Photos and media* fog: importing this means **extracting base64 to
files**, and Markdown-authoritative storage makes that mandatory rather than optional.

### 9. Nine of 86 recipes are not in English

*Îles Flottantes*, *Purée de Pommes de Terre*, *Gâteau Au Chocolat*, *Curry Japonais sans
eau*, *Meringue : recette facile*, *Bollo limpio* (Spanish), and *Sukiyaki Udon* — whose
title is English but whose ingredient section header is French.

There is no language field anywhere in the format. Language is implicit in the text, mixed
*within* a single recipe, and roughly 10% of a real library. Issue
[#10](https://github.com/BattermanZ/Kamosu/issues/10) is confirmed as a live concern, not a
someday-nice-to-have — and note the unit of language may be smaller than the recipe.

## Importer gotchas, concretely

- **Filenames are lossy identity.** `-1` suffixes for collisions; UUIDs inside are the real
  identity. Import by UUID, not filename.
- **HTML entities survive unescaped** — `Noodles &amp; choi sum` appears in step text.
  Crouton scraped HTML and never decoded it. Kamosu must, or inherit the artefact.
- **Field name `neutritionalInfo` is misspelled** in the format. Match it exactly.
- **`quantityType: SECTION`** is not a unit; treat it as a control row.
- **Empty stubs are normal** — *Dan Dan Noodles* and *Chef Tyler Noodles* are ~280 bytes:
  a name, a link, and nothing else. A "save the link, fill it in later" habit. The model
  must allow a recipe that is nothing but a URL.
- **`defaultScale`** is a per-recipe saved scaling factor (mostly `1`, four are fractional).
  Kamosu needs somewhere for it or the value is dropped.

## What is not in this export

`tags` and `folderIDs` are empty everywhere, so **no organisational data was exported** —
no folders, no tags, no favourites, no cook counts, no dates. There is no `dateAdded`, no
`lastCooked`, no rating field anywhere in the schema. If Kamosu wants recipe history, it
starts from zero on import; nothing carries over.

> **Update, 23 September 2026 (#128):** tags now do carry over. See below.

## Update, 23 September 2026: the export carries tags

Source: `Crouton Recipes - 23 Sep 2026.zip`, the same library exported again. It is now the
corpus at `samples/crouton/`; the 20 August file moved to `samples/crouton-2026-08-20/`.
The measurement above is left as it was taken.

- **Same format, same recipes.** The same top-level keys and the same 86 UUIDs. Beyond tags,
  one recipe changed: *Korean Fried Chicken* has a different `sourceImage` and a
  `defaultScale` of `0.5` where it had `1`.
- **`tags` is populated: 64 of 86 recipes, 21 distinct tags.** Each tag is
  `{ "uuid", "name", "color" }`, e.g. `{"uuid": "A9179A12-…", "name": "Vegan", "color":
  "#FFCC00"}`. A name has exactly one uuid and one colour across the export.
- **By recipe count:** Hearty 36, Vegetarian 33, Asian 22, Meat 18, Dessert 13, Japanese 9,
  French 9, Beef 8, Chicken 8, American 7, Weeknight dinner 7, Vegan 6, Fish 5, Chocolate 5,
  Stew 4, South American 3, Salads 3, Italian 2, Colombian 1, Middle Eastern 1, Bread 1.
- **`folderIDs` is still empty everywhere.**

Kamosu imports each tag by its name into the Kitchen's own tag list, as an English (`en`)
word, and drops the colour. Re-importing adds tags and never removes one (#128).
