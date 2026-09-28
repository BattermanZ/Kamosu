# A divergence is two recipes, not a difference

When an instance holds two Branches of one Lineage, Kamosu shows **two recipes**, never a difference between them. There is no comparison screen, no side-by-side, no object called "the difference". You stand inside one recipe or the other — whole, in order, cookable — and a switch at the top moves you between them.

_(Amended by [ADR 0041](./0041-a-recipe-is-written-in-a-cookbook-and-seen-in-a-kitchen.md): a recipe now often has more than two Branches seen in one Kitchen, so the switch must hold more than two. How it does is a screen question still open.)_

Each Branch marks the handful of lines that are not the same as the other's, and tapping one unfolds the other side **in place**. A line that only one of the two has is a **Ghost**: shown struck through, in the position it occupies in the recipe that really has it, labelled with whose it is.

History is a separate screen — the **Thread** — running oldest to newest and forking at the Branch Point.

## Why

- **A recipe is a thing you stand in.** Everything else in Kamosu already says this: a Version is a complete state, not a delta ([ADR 0004](./0004-one-lineage-many-branches.md)); an As Cooked is a complete state, not a rival structured record ([ADR 0005](./0005-an-attempt-is-a-version-you-cooked-but-did-not-keep.md)); an Ingredient Line is a written line, not a bundle of fields ([ADR 0002](./0002-the-written-ingredient-line-is-the-truth.md)). A difference view would have been the one place Kamosu shows a cook something that is not a recipe.
- **You can cook what you are looking at.** A difference screen is unusable in a kitchen. Because the reading surface is a recipe, "Marc's sounds better, I'll make his tonight" is one tap, and needs nothing built for it.
- **It is what "never merges" looks like from the outside.** Two recipes, both real, both kept. A screen that summed them into one list of differences would quietly imply there is a single reconciled recipe waiting to be assembled — the exact idea [ADR 0004](./0004-one-lineage-many-branches.md) refuses.
- **The Ghost makes the page symmetric, and it is one mechanism, not two.** A line the other person removed and a line the other person added are the same object seen from opposite sides. So the same treatment serves both, and every difference is visible from either Branch: nothing is knowable only by standing in the right place.

## Considered options

Three directions were built over the real recipe and looked at on a phone.

- **The letter** — a divergence arrives as a message and is read as sentences: *"Marc has ¾ cup potato starch where you have 1 cup."* No columns, no gutter. Rejected, and it is the closest loss: sentences are genuinely how a person would say it, but eight of them is a wall, and a rewritten step collapses to *"Marc rewrote the step that starts…"*, which is the one change where knowing *how* is the entire point.
- **The thread** as the whole answer — divergence is not an event, just what a history screen looks like when two people write in it, with a stacked comparison behind it. Rejected as the primary surface for putting *history* in front of a cook who only wanted to know what changed — but **kept for history itself**, where it won outright.
- **Side-by-side columns.** Never built. At 390px each column gives a recipe line about twenty characters, which is how a diff becomes unreadable exactly where it matters.
- **The switch, with Ghosts** (chosen).

An earlier pass of the chosen direction claimed a removed line was unshowable, since it is not on the page to mark. That was wrong: it is showable by putting it on the page.

## Consequences

- **One card per Lineage on the shelf**, however many Branches it has — consistent with [ADR 0006](./0006-a-translation-is-a-branch.md), which had already settled this for Translations. The card says a second Branch exists; it does not become two recipes in the library.
- **Carrying a change across is a gesture, and it stays honest.** Taking a line writes it into your recipe and leaves it **unsaved**, marked in place with what it replaced, until you save — which produces an ordinary Version with a *what changed* line, exactly as [ADR 0004](./0004-one-lineage-many-branches.md) describes. **There is no "take all".** The absence is the decision: taking every line one at a time is a person making a recipe, whereas one button doing it is a merge with extra steps.
- **A Ghost is offered too.** Where the other person removed a line you still have, the offer is *take it out of mine as well*, so a deletion can be carried across like anything else.
- **An Ingredient Line needs an identity that survives editing and travels.** Without it, "Marc has ¾ cup where you have 1 cup" cannot be told from "Marc removed a line and added another", and neither the marking, the Ghosts nor their positions work at all. This is the load-bearing dependency underneath the whole decision and it is a question about how a recipe is *stored*. Left open here deliberately; see the ticket it generated.
- **A Ghost's position is a best effort.** It is anchored to the line before it in the recipe that really has it. Where that neighbour is not in both recipes, the position is a guess, and Kamosu should be readable when it guesses wrong rather than assert an order it cannot know.
- **Reading an older Version is reading, not editing.** Any Version can be opened and **cooked from** — the Attempt pins to it, so an old cooking note keeps describing the state it was written about. No Version can be edited. "Restoring" an old Version is saving it again as a new Version at the end of the line: history only grows.
- **A Version's name can be changed at any time; nothing else about it can.** The fingerprint is of the recipe's content, so the name is not part of what makes a Version that Version — it is a label a person puts on a moment, and the moment worth naming is reliably the one nobody thought to name at the time. The recipe's content and the *what changed* line stay frozen, because those are an account of something that happened. Two holders of the same Version may label it differently, and that is correct: a label is yours, not the recipe's.
- **Everything here is computed on top of what is already stored** ([ADR 0009](./0009-the-v1-cut-line-is-drawn-at-shape-not-at-features.md)) — it derives entirely from Versions that exist whether or not a screen reads them. It is in v1 anyway, because [#12](https://github.com/BattermanZ/Kamosu/issues/12) kept cross-instance share bundles: a bundle that arrives with nowhere to be read is a feature that does not exist.
- **Two Branches, not five.** Everything assumes exactly two. A third Branch arriving from a second friend is not designed, and the switch is the part that will not survive it unchanged.
