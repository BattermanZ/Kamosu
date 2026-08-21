# An Attempt is a Version you cooked but did not keep

An **Attempt** is one person's record of one cooking. Where the cook deviated, the Attempt holds an **As Cooked** — a complete recipe state, structurally identical to a Version, differing only in that it never joined a Branch, never becomes part of the recipe's history, and never travels. Deviations are written as ordinary Ingredient Lines and Step text, not as a separate structured record of what changed.

**Promotion** is the deliberate act that moves something provisional out of an Attempt and into the recipe: an As Cooked becoming a Version, or an Attempt photograph becoming the Main Photo or a Step's photo. It is the boundary at which the recipe's rules — append-only history, and the right to edit — take over.

## Why

- **It keeps a promise already made without inventing machinery to keep it.** [ADR 0004](./0004-one-lineage-many-branches.md) committed to promoting an Attempt into an ordinary Version, pre-filled from the Attempt. If a deviation exists only as prose, that promotion is not a mechanism — it is a person re-reading their own note and retyping the recipe. Holding a state makes it mechanical.
- **The duplication is an illusion.** A Version is already defined as one complete saved state named by a fingerprint of its content. An Attempt that changed something *is* such a state. Naming it separately would have modelled the same thing twice; reusing the shape models it once and distinguishes the two by membership alone.
- **It follows [ADR 0002](./0002-the-written-ingredient-line-is-the-truth.md) rather than arguing with it.** "8 g of salt, not 10" is an edited Ingredient Line; "baked five minutes longer" is edited Step text. A structured deviation record would have reintroduced, inside the Attempt, exactly the parse-first model the recipe rejects.
- **Fingerprinting pays for itself again.** An As Cooked is identified before anyone decides to keep it, two cooks who made the identical change hold the identical state without communicating, and promotion mints no new identity.
- **The common case stays free.** Cooked it as written and it was good — the overwhelming majority — stores no As Cooked at all. Structure is paid for only on the attempts that deviated.

## Considered options

- **Attempt as a note**: a date, free text and a rating. Simplest, and rejected because it silently withdraws ADR 0004's promotion commitment and loses the reason for recording an attempt at all.
- **Attempt with a structured deviation record**: this Ingredient, this new amount. Rejected as a second, weaker model of an ingredient sitting beside the real one, contradicting ADR 0002 and unable to express a changed method at all.
- **Attempt as an unnamed branch**: deviations saved as real Versions on a side Branch. Rejected: it makes every one-off experiment part of the recipe's permanent, travelling history, which is precisely what an Attempt exists not to be.
- **Attempt holding a complete un-kept state** (chosen).

## Consequences

- **An Attempt never changes the recipe.** It makes no Version, alters no Branch, and touches no line the author wrote. Its required permission is therefore *may I see this recipe*, not *may I edit it*: anyone who can see a recipe may record their own Attempt on it, which is what makes a shared instance's cooking history complete rather than owner-only.
- **Attempts are freely editable and deletable by their cook**, deliberately *not* inheriting the append-only rule. Append-only is the price of history that is shared and built upon; an Attempt is neither. Once promoted, the resulting Version is permanent and deleting the Attempt does not unmake it.
- **An Attempt inherits its recipe's visibility and always names its cook**, with a per-Attempt private flip available to that cook at any time, including after the fact. Visibility is derived rather than separately managed, so it cannot drift out of step with the recipe. What "can see the recipe" means is left to the accounts decision.
- **Rating is an optional five-star score on the Attempt, and a recipe never averages.** Averaging would merge three incompatible things: different people, different Versions, and different competence on the day — and would let scores from a superseded Version permanently drag down a recipe already fixed, which is the exact lie pinning was introduced to prevent. A recipe shows how many times it has been cooked and when last, each person's most recent rating separately by name, and a quiet mark where that rating came from a different Version.
- **Attempts belong to the Lineage, not the Branch.** An instance holding two Branches shows every Attempt against both, each labelled with the Version cooked — including *cooked before these diverged*, which is evidence about both Branches and which Branch-filtering would have hidden from one of them for no reason. "Cooked ten times" counts cookings of the dish across every Branch held.
- **An Attempt holds any number of photographs, flat, not attached to individual Steps** — zero of 599 steps in the real Crouton library carry a photo, so per-Step attachment doubles the model's photo relationships to serve a case that does not occur. A promoted photograph travels with the recipe under ADR 0004's *photographs currently in use*; the rest of that evening's shots do not.
- **A Vault carries the owner's own non-private Attempts** as a dated log after the recipe, rendering only the lines that differed rather than a second full copy. Never another person's: a Vault may be pushed to a git remote, and visibility inside Kamosu is not consent to republish someone else's candid note.
