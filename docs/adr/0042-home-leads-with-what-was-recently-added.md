# Home leads with what was recently added

Home gains a fifth shelf, ***recently added***, and it comes first. It holds every recipe you can see, newest first, cut to the same twelve cards as the others. *Never cooked* stops sorting newest first and is **shuffled** instead, each time Home is asked for. This amends the list in [ADR 0011](./0011-cooking-shows-the-amounts-for-the-step-it-is-on.md), the same way [ADR 0027](./0027-the-library-is-one-shelf-and-a-result-says-what-matched.md) added *recently opened*.

- **An arrival is a new Lineage, whatever route it came by.** The importer, a web link, a paste, an agent's `create_recipe` and a recipe typed by hand all count. The order is the Lineage's age, read off its oldest Branch, which is the order the library's shelf already keeps. A Translation or a variation of a recipe you already had is a new Branch, not a new recipe, so it does not bring the recipe forward.
- **It leads Home.** Nobody has to have cooked anything for it to have something to say, and a recipe that just came in is the likeliest reason to open Home in a week you added some.
- ***Never cooked* is shuffled before it is cut to length**, so the twelve shown are drawn from every recipe you have not made, not the newest twelve in a new order. The seed is the Core's to hand in, which is how a test pins the order instead of asserting on chance. A real instance draws a fresh seed every time Home is asked for.

## Why

- **The first Operator asked for it** on 2026-09-25 ([#151](https://github.com/BattermanZ/Kamosu/issues/151)): *"On the homepage, I want a last imported carousel."* Nothing on Home was about arrival.
- **"Imported" was read as "added" on purpose.** The import ledger is where [ADR 0025](./0025-an-imported-recipe-is-an-ordinary-recipe.md) keeps import facts, and it was the other reading. On the live instance it would have shown 86 Crouton recipes tied for first, from one timestamp, and never the recipes an agent added the same evening. The need behind the request is *show me what just came in*, and only the Lineage's age answers that. No import fact is read, so ADR 0025 is untouched.
- ***Never cooked* had to change or it repeated the new shelf.** Newest first was its reason for being useful, *the week you added it*. On a library nobody has cooked from yet, it and *recently added* would have been the same twelve cards in the same order. The new shelf now does the newest-first job, and a shuffle gives *never cooked* one of its own, a different handful of recipes you have not got round to.

## Considered options

- **Import ledger only, headed *Last imported*.** Rejected for the reasons above. It fits a shelf meant to review a migration batch, and the Import Report already does that.
- **The new shelf after *quick tonight*,** leaving the spec's order intact up to there. Rejected in favour of first: *cooked most* is absent on an instance nobody has cooked on, which is every instance at the start.
- **Leaving *never cooked* newest first** and letting the two shelves drift apart as recipes get cooked. Rejected: the overlap is total exactly when Home is newest and most looked at.

## Consequences

- **Home is no longer the same answer twice in a row.** `home_shelves` is a read the phone caches, so the order seen offline can differ from the order seen online a moment later. That is harmless and expected, not a bug.
- **Still computed on top of stored data.** Nothing new is stored. The arrival order is a column that already existed, and the shuffle keeps nothing, so under [ADR 0009](./0009-the-v1-cut-line-is-drawn-at-shape-not-at-features.md) the screen stays free to be redesigned.
