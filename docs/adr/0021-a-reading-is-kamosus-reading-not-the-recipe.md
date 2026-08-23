# A Reading is Kamosu's reading, not the recipe

A Version's fingerprint covers **what a person wrote and chose** — the titles, the Ingredient Lines, the Step text, the Sections, the Yield, the Notes, and the **Photographs** ([ADR 0017](./0017-a-photograph-is-known-by-its-contents.md)). It does **not** cover the **Reading**, and it never covered the Hand, the Version's name, its *what changed* line or its date.

A Reading travels in a Bundle beside the Version it belongs to, and is carried never recomputed ([ADR 0003](./0003-the-database-is-the-truth-the-vault-is-a-published-copy.md)). Correcting a Reading therefore mints no Version and does not appear in the Thread.

Where a Reading points at a **Food**, the Bundle carries the Food's **names — all of them, in every Language it has one for** — and no id. Where it points at a **Recipe** it names a **Lineage**, which is global by construction ([ADR 0008](./0008-a-composed-recipe-is-an-ingredient.md)) and needs nothing added.

This **sharpens** [ADR 0004](./0004-one-lineage-many-branches.md) rather than reversing it. That ADR says a Version is named by a fingerprint "of the recipe state alone" and never says whether a Reading is part of the state. It is not.

## Why

- **A Reading is largely produced by a parser, and a parser is a moving part.** [#3](https://github.com/BattermanZ/Kamosu/issues/3) put ingredient parsing in a separate worker process. Inside the fingerprint, two people who fix the same typo in the same Step independently — words now identical, so they must converge — are reported as **diverged** if their two parsers read `1 c. crème` even slightly differently. Neither can see why, because what is on screen is the same on both sides.
- **[ADR 0019](./0019-an-ingredient-line-has-no-name-of-its-own.md) already decided this argument in a neighbouring case.** It refused ids on Ingredient Lines because machine-side detail inside a fingerprint "manufactures a divergence out of nothing, in a system whose whole point is that identical states converge without communicating". A parser's output is machine-side detail of exactly that kind.
- **The company outside the fingerprint is already respectable.** [ADR 0015](./0015-a-name-is-not-identification-it-is-a-reminder.md) deliberately put the Hand, the Version name and the *what changed* line outside it, and accepted the odd-reading consequence that one Version may carry two different authorship claims. A Reading joins them; a date joins them for the same reason, since two people making the identical edit on different days must still converge.
- **What the alternative bought was small.** Inside the fingerprint, a corrected Reading would appear in the Thread as a change. That is a nice line in a history screen, weighed against the one mechanism the whole map stands on.
- **A portable Food id is worse than no id at all.** Your *farine* and a friend's *farine* were minted separately, so their ids differ — meaning the id's only confident statement about two obviously identical Foods is *"not the same"*. It would push [#20](https://github.com/BattermanZ/Kamosu/issues/20) toward trusting a signal that is wrong in the commonest case in the feature, at the cost of an id on every Food forever, paying off only in long re-sharing chains.
- **Carrying every name is the cheap half of a genuinely expensive problem.** [#4](https://github.com/BattermanZ/Kamosu/issues/4) established that matching, not data, is the cost of anything built on Foods, and no dataset resolves it — CIQUAL files crème fraîche as *"Crème de lait, 30% MG, épaisse, rayon frais"*. Because [ADR 0006](./0006-a-translation-is-a-branch.md) names a Food **per Language rather than splitting it by Language**, a Food arriving with *farine* / *flour* / *harina* offers three chances to match instead of one — which matters most for the 9 of 86 real recipes that are not in English ([#5](https://github.com/BattermanZ/Kamosu/issues/5)).
- **The escape hatch was built in ADR 0002 and costs nothing here.** A Reading "may be absent, partial or mistaken; anything built on it degrades politely rather than failing". A Reading that arrives and matches nothing leaves a line that still reads correctly, which is why getting this wrong is survivable and getting the fingerprint wrong is not.

## Considered options

- **The Reading inside the fingerprint.** The intuitive answer — a Reading is part of the recipe, so changing it changes the recipe. Rejected on the parser argument above.
- **A portable Food id, with or without names.** Rejected: confidently wrong in the ordinary case, and a permanent cost.
- **Stripping Readings on the way out and re-parsing on arrival.** Already refused by [ADR 0003](./0003-the-database-is-the-truth-the-vault-is-a-published-copy.md): "A Reading corrected by hand is authored data. Regenerating it on import would silently discard the correction."
- **The Reading outside the fingerprint, Foods by name** (chosen).

## Consequences

- **When a Version you already hold arrives carrying different Readings, yours stay.** One rule, and the same shape [ADR 0015](./0015-a-name-is-not-identification-it-is-a-reminder.md) accepted for two authorship claims on one Version.
- **A parser upgrade cannot re-fingerprint the library**, because Readings are stored when written and never silently recomputed. This is the practical reason the decision matters, not a side effect.
- **[#20](https://github.com/BattermanZ/Kamosu/issues/20) receives exactly what it needs and nothing misleading**: a set of names per Food, in up to three Languages, with no identity claim attached. What to do with them — match automatically, offer for confirmation, or always land as new — is that ticket's to decide.
- **A Food's CIQUAL nutrition binding needs no slot now.** It is deferred past v1 by [#12](https://github.com/BattermanZ/Kamosu/issues/12), and because unrecognised fields are ignored rather than rejected ([ADR 0020](./0020-a-bundle-is-one-recipes-worth-of-vault.md)), v2 may add it without orphaning a Bundle written before that day.
- **Correcting a Reading is invisible in history, on purpose.** It is a correction to Kamosu's understanding, not to the recipe, and the person who made it sees the corrected Reading itself.
