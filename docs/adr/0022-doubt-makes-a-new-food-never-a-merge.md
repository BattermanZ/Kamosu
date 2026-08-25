# Doubt makes a new Food, never a merge

A **Food** arriving from another instance is a bag of words and nothing else — its names, in every Language it has one for, and no id ([ADR 0021](./0021-a-reading-is-kamosus-reading-not-the-recipe.md)). Matching those words against the instance's own Food list happens **automatically and silently**: receiving a recipe is never interrupted by a question about Foods.

Two words are the same word when they are in the **same Language** and equal after folding case and stray whitespace. Accents and plurals are respected: *maïs* is not *mais*, and *oeufs* is not *oeuf*.

- **A clean single hit matches**, and the surviving Food **learns** every arriving name that matches nothing at all on the instance.
- **Two hits make a third Food**, carrying all the arriving names, and the collision is **recorded as a suggestion** on the Operator's merge screen with its reason attached. Nothing ever merges itself.
- **One rule at every door.** A word produced by Kamosu's own parser during a Crouton or web Import is an arriving Food that knows one name, tagged with the recipe's Language, and goes through the same matcher. A **Copy** taken from a Kitchen next door matches nothing, because the Reading it carries never leaves this instance's Food list.
- **A lone ambiguous word goes to the busiest Food** — the one the most Readings already point at — never to a new one.

**Merging** is the Operator's, takes all names from both, moves every Reading to the survivor, clears the suggestions naming either, and asks which **Cup Weight** survives when the two disagree. It states its blast radius in plain numbers before it happens and **cannot be undone in v1**. An unreferenced Food **stays**; the Operator may delete one from the same screen.

## Why

- **The two ways of being wrong do not cost the same.** Welding two Foods together is a permanent untruth inside shared, instance-wide data with no un-merge. Keeping two apart is a duplicate line in a list, repaired by the merge [#11](https://github.com/BattermanZ/Kamosu/issues/11) already granted the Operator. Every choice here sorts errors onto the repairable side.
- **The escape hatch makes all of it survivable.** [ADR 0002](./0002-the-written-ingredient-line-is-the-truth.md) makes the Ingredient Line the truth and a Reading optional, absent or mistaken. A Food matched wrongly still leaves `"2 poignées de farine, environ"` reading exactly as written. Bad matching costs a clumsy shopping list and a missing cup-to-gram conversion; it never costs the recipe. Getting the fingerprint wrong would not have been survivable — this is.
- **Asking was rejected because receiving is meant to be a text message and a tap.** A bundle carries ten to twenty Ingredient Lines; a confirmation step per Food puts a data-cleaning chore on the commonest path in the feature, to make a judgement whose downside is asymmetric anyway and whose repair already exists.
- **Never matching was rejected because it defeats what a Food is for.** [#4](https://github.com/BattermanZ/Kamosu/issues/4) found the data is free and small and **the matching is the entire cost**. A list that never matches does that expensive work once per recipe instead of once per food, and merges nothing on a shopping list.
- **Stripping accents merges words that genuinely differ.** French **maïs** is corn and **mais** is *but*. Folding plurals means a rule per Language about how words pluralise — a moving part of exactly the kind ADR 0021 warned about with parsers, usually right and wrong in the welding direction.
- **Language must agree, because five letters are not a meaning.** French *raisin* is a grape and English *raisin* is a dried one. A matcher blind to Language merges them the first time a French recipe meets an English one.
- **A collision is testimony, not a resemblance.** A person on another server put *farine* and *flour* in one Food. That is a far stronger signal than two names looking alike, and it is the only trustworthy signal in the whole design — which is why it is preserved as a Food and surfaced as a suggestion rather than acted on or discarded.
- **Learning names is the only way carrying them pays off.** ADR 0021 carries every name in every Language so that there are more chances to match. If the names are dropped at the moment they match, the instance receives the same lesson repeatedly and forgets it every time, and the next English recipe from anywhere makes another duplicate. Accretion makes each clean match raise the odds the next one is clean too. It costs nothing in history: a Food's names are no part of a Version, so gaining one mints no Version and appears in no Thread.
- **A wrong name is cheap; a wrong merge is not.** Accretion is an automatic write to shared data on a stranger's say-so, and a sloppy sender's error does spread. It is accepted because deleting a word restores the Food exactly, moving nothing else — which is why removing a name must be as ordinary an act as adding one.
- **The lone-word tie-break exists because Q3 makes duplicate names possible.** After a collision the list holds `{fr: farine}` and `{fr: farine, en: flour}`, and a single French word now hits two. Applying the collision rule literally would make a third, then a fourth — a tax on writing your own recipes, not on receiving. A multi-name collision asserts a connection; a lone word asserts nothing, so making a Food from it preserves no information and only multiplies. The only way a lone word can be ambiguous is **inside a cluster already flagged for merging**, so choosing the busiest member is choosing among Foods Kamosu has already said are probably one thing.
- **Undo is deferred because it is not stored shape.** [ADR 0009](./0009-the-v1-cut-line-is-drawn-at-shape-not-at-features.md) says v1 must get right what is expensive to retrofit. An undo log sits beside the Foods and touches no recipe, so v2 may add it, paying only that merges done before that day stay irreversible. The dangerous merge — two things believed identical that are not — is one the Operator had to go looking for; the merges actually performed are the ones Kamosu suggested, on clusters something already identified.
- **An unreferenced Food is knowledge that was paid for.** Its Cup Weight, its accumulated names and one day its CIQUAL binding are the expensive part. Deleting a recipe must not quietly discard the fact that a cup of this flour is 125 g, and re-importing must not buy it again.

## Considered options

- **Offer each arriving Food for confirmation.** Rejected: friction on the commonest path, for a judgement whose repair already exists.
- **Never match; every arriving Food is new.** Rejected: pays [#4](https://github.com/BattermanZ/Kamosu/issues/4)'s matching cost per recipe rather than per food, and breaks shopping-list merging.
- **On a collision, pick a winner.** Rejected: nothing available breaks the tie, and picking is a silent assertion that two of the instance's own Foods are one thing — the permanent error.
- **On a collision, pick a winner and drop the other name.** Rejected: deletes the very information ADR 0021 went to trouble to carry, at the moment it is most valuable.
- **Compute merge suggestions generally, by scanning the list for Foods that look alike.** Rejected: the same guessing refused at import time, moved to a screen, with the same failure modes and none of the evidence.
- **Delete a Food automatically when the last Reading stops pointing at it.** Rejected: discards paid-for knowledge on an unrelated act.
- **Silent conservative matching by name, doubt making a new Food** (chosen).

## Consequences

- **Receiving bundles grows a visible tail of near-duplicate Foods**, and clearing it is the Operator's job. This is untidy on purpose: it is the only option that never states something untrue. It also decays — once a cluster is merged, the resulting Food knows both words and the next bundle carrying either matches cleanly and creates nothing.
- **The merge screen is a worklist rather than a hunt.** Suggestions carry their reason, so the Operator reads *"a bundle said these two are the same"* instead of comparing names by eye.
- **Typing a name onto a Food that another Food already answers to records a suggestion too.** It is the one remaining way to make a duplicate name, and it leaves the same trail.
- **Nothing about nutrition ever arrives from another instance.** CIQUAL bindings are deferred past v1 by [#12](https://github.com/BattermanZ/Kamosu/issues/12) and a **Cup Weight** belongs to its instance and never travels ([ADR 0016](./0016-kamosu-converts-to-the-kitchen-not-to-the-arithmetic.md)), so there is nothing arriving to reconcile and no slot to reserve.
- **A Reading whose Food matched wrongly is corrected without minting a Version**, because a Reading is no part of a Version's identity ([ADR 0021](./0021-a-reading-is-kamosus-reading-not-the-recipe.md)). Repairing a bad match is invisible in history, exactly as correcting any other Reading is.
- **A merge performed in v1 is permanent.** The blast-radius confirmation is the whole safety net until an undo log is added.
