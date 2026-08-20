# The written ingredient line is the truth; structure is a reading over it

Kamosu stores an Ingredient as the free-text line a person wrote or a website supplied — `"2 poignées de farine, environ"` — and always displays that line. Beside it may sit a **Reading**: an interpretation into how much, in what unit, of what Food. The Reading is optional, may be partial or wrong, and is what scaling, shopping lists, unit conversion and nutrition run off. Where it is missing, those features go quiet; the recipe itself is unaffected.

This inverts what three of the four apps surveyed in `docs/research/recipe-app-formats.md` do. Mealie, Tandoor and Crouton all treat the structured parts as canonical and render the display line from them. A future reader will assume Kamosu simply hasn't got round to normalising its ingredients yet. It has; this is the design.

## Why

Measured against Aurélien's real 86-recipe Crouton library (`docs/research/crouton-real-export.md`):

- **28% of ingredients (239 of 863) have no quantity at all**, and whole recipes go without — 17 of 17 rows in one, 12 of 12 in another. A model that treats the parts as canonical is guessing for a quarter of a real library, and "to taste" is a legitimate amount.
- **Crouton kept the parts and threw the line away, and the damage is unrecoverable.** `"2 cloves minced garlic"` survives as `quantity 2 ITEM` plus a food literally named `"cloves minced garlic"`; one row begins with a comma. Nothing can reconstruct what was written.
- **No importer will ever supply structure.** schema.org declares `recipeIngredient` as free *text*, so no web scraper anywhere returns split quantity/unit/food (`docs/research/web-link-recipe-ingestion.md`). Every import arrives as a line first and a guess second, always.
- **Nine of 86 recipes are not in English** and one mixes languages internally. A canonical structure has to commit to a parse; a line does not.

## Considered options

- **Structure canonical, original line archived** (Mealie, Tandoor). Simplest for scaling and shopping lists, because the parts are always present and always trusted. Rejected: on this corpus they are frequently *not* present, and trusting a degraded guess is what produces food names like `"cloves minced garlic"`.
- **Line only, no structure** (KitchenOwl, which parses at import and then discards the result). Rejected: it forecloses scaling, shopping lists, unit conversion and nutrition — five wish-list features.
- **Line canonical, structure as an optional Reading** (chosen). Keeps every feature available where the data supports it, and never loses what the cook actually wrote.

## Consequences

- **Editing happens on the line.** Re-reading the edited line refreshes the Reading. A Reading edited on its own would silently disagree with the line above it, so the line is what a person changes.
- **Every feature built on a Reading must degrade, not fail.** An unannotated ingredient displays perfectly and simply does not scale. Any code path that assumes a quantity exists is a bug.
- **A Reading must record that it is a guess.** The Crouton importer *reconstructs* an approximate line (`"2 cloves minced garlic"`) rather than finding one; reconstructed lines are marked as such, not presented as authored.
- **Ingredient parsing is work Kamosu owns**, deferred rather than avoided — no usable parser library exists for any surveyed format. Because the Reading is optional, shipping without a parser is a degraded experience rather than a blocked one.
