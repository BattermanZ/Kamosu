# An empty field is no part of a Version's fingerprint

A Version's id is the SHA-256 of its content, canonically serialised — **with every field holding nothing removed first**. `null`, the empty list and the empty object go, at every depth, before the hash is taken. Everything else stays, the empty string included.

An absent field and a field set to `null` already mean the same thing to Kamosu: `parse_recipe_content` normalises one into the other, and [ADR 0020](./0020-a-bundle-is-one-recipes-worth-of-vault.md) already says an unrecognised field is ignored rather than rejected. So this loses nothing, and it buys the property the whole scheme stands on: **a field added to a recipe later starts out empty on every recipe already written, drops back out here, and moves no existing id.**

This **sharpens** [ADR 0004](./0004-one-lineage-many-branches.md) and [ADR 0021](./0021-a-reading-is-kamosus-reading-not-the-recipe.md) rather than reversing either. ADR 0004 says a Version is "named by a fingerprint of its own content" and ADR 0021 lists which fields that content is. Neither says what happens to that name the day the list grows. This does.

Adopting it re-fingerprinted the 142 Versions on the dev instance once, in migration 29. That is the only time Kamosu rewrites a Version id, and the rule exists so there is no second.

## Why

- **A field addition was quietly breaking convergence.** [#72](https://github.com/BattermanZ/Kamosu/issues/72) added `nutrition` to a recipe. Nothing rewrote what was already stored — every row still hashed to the id it sat under — but every recipe saved afterwards carried a field the earlier ones did not. The same recipe, written either side of that day, held two different ids. Two people who never spoke holding the same state *is* the mechanism ([ADR 0004](./0004-one-lineage-many-branches.md), [ADR 0005](./0005-an-attempt-is-a-version-you-cooked-but-did-not-keep.md)), and one afternoon's feature was enough to break it.
- **It had already produced a wrong answer in the product.** [#58](https://github.com/BattermanZ/Kamosu/issues/58)'s live acceptance cooked a corpus recipe exactly as written and got an As Cooked stored for it, when cooking as written is supposed to store nothing. The dev database still holds both rows: `Braised Chicken in Red Wine`, identical across all 19 Ingredient Lines and all 29 Steps, under two ids that differ only by whether `nutrition: null` was in the text that was hashed.
- **The alternative left a permanent asterisk.** The other way out was to write down the qualified rule — an id is the fingerprint of its content *as of the day it was saved* — and forbid comparing ids for content equality anywhere. It needs no migration, and it is worse forever: every future field addition adds another cohort of recipes that cannot converge with their own twins, and the one sentence the map is built on stops being simply true.
- **It closes the future case by construction rather than by vigilance.** [#89](https://github.com/BattermanZ/Kamosu/issues/89) asked that a migration changing a recipe's stored shape "cannot be added without answering this". A checklist line answers it by being remembered. This answers it by there being nothing to remember: a new field defaults to empty, so it is invisible to every id already written.
- **The empty string is deliberately not swept up.** `note: ""` is somebody who opened the note box and left it blank; never having opened it is `null`. The parser keeps those apart, so the fingerprint does too. The rule is about a slot that was never filled, not about text that happens to be short.
- **An array entry keeps its place.** Only object keys are removed. A list is positional — an entry that happens to be empty is a line in a position, and dropping it would silently renumber every line after it, which is precisely the manufactured divergence [ADR 0019](./0019-an-ingredient-line-has-no-name-of-its-own.md) refused ids on Ingredient Lines to avoid.

## Considered options

- **Ids frozen at save time**, with the qualified rule written down and id comparison forbidden. No migration, smallest change. Rejected: it makes the asterisk permanent and pays it again on every future field.
- **Store the filled-in shape and re-fingerprint** — write `nutrition: null` into the 111 older rows and rehash. Costs the same migration and fixes only this field; the next one costs another.
- **Never add a field to a recipe again.** Not a decision, an ending. Nutrition was wanted, and v2's is not the last.
- **An empty field is no part of the fingerprint** (chosen).

## Consequences

- **Comparing two Version ids is comparing two recipes.** That is what content addressing means, and it is true again — so `save_recipe_version` and the importer may go on asking "did anything change?" by comparing ids, which is the cheapest form of the question. A behaviour test asserts the invariant against a real database after every path that writes a Version, so it is a checked fact rather than an assumption.
- **Two Versions can merge into one.** The same recipe written before and after a field appeared is, correctly, one Version. Migration 29 found exactly one such pair among 142 and kept the row carrying the Readings; a Reading is authored data and would otherwise be lost ([ADR 0021](./0021-a-reading-is-kamosus-reading-not-the-recipe.md)).
- **`content_as_declared` becomes provably harmless.** It fills a recipe out to the shape the Catalogue declares on the way out, and every field it fills holds nothing — so what a Door serves fingerprints to the id the stored row sits under. That was false before this, and it is what [#89](https://github.com/BattermanZ/Kamosu/issues/89) measured.
- **A field with a non-empty default is now the thing to watch.** Add `unit_system: "metric"` to a recipe and every existing id moves again. `parse_recipe_content` is tested against this directly: a recipe built from the title alone must fingerprint as though nothing else were there, and adding a field that defaults to anything else fails that test with an explanation.
- **SQLite can compute a fingerprint.** `version_fingerprint(content)` is registered on every connection, which is what let migration 29 be written in ordinary SQL and what lets the invariant be asked of a live database in one line. It is not an invitation to re-fingerprint again: doing that rewrites append-only history and every reference to it.
- **Nothing about a Bundle changes.** A Bundle carries ids as written and a receiving instance stores them; under this rule what it receives and what it would compute now agree, where before this they did not.
