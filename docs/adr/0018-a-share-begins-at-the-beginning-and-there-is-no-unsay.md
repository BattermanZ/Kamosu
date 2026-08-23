# A share begins at the beginning, and there is no unsay

A share of a recipe carries **the complete chain of Versions back to the first one**. A share cannot be made to start part-way along, and a Version cannot be withheld from one.

It carries the writing as well as the recipe: each Version's **name**, its **what changed** line and its **Hand** ([ADR 0015](./0015-a-name-is-not-identification-it-is-a-reminder.md)) travel with the state it names.

A Version is **never removed** from a Branch by anyone holding it — the person who wrote it or a person who received it. Deleting an entire recipe from your Kitchen is an ordinary act and remains one; turning a **Share Link** off ends that link and mints a new one if it is turned back on ([ADR 0007](./0007-a-recipe-is-held-by-a-kitchen-not-a-person.md)). Neither reaches a copy already sent, and Kamosu says so rather than letting it be discovered.

Because of this, **a Version missing from a chain has exactly one meaning: the bundle is damaged.** Nothing legitimate produces a short chain. And **a Branch Point is always computable** between two valid Branches of one Lineage, because both chains reach the same first Version.

Two things follow in the interface:

- The **Share Link page** shows the recipe and, beneath it, the **Thread** of the Branch shared — every Version in order with its name, its *what changed* line and its author — collapsed by default. Attempts still never appear on it ([ADR 0005](./0005-an-attempt-is-a-version-you-cooked-but-did-not-keep.md)).
- The share screen carries a **standing line**, in the same words every time, on both the Share Link and the export-a-bundle path: what travels, and that turning the link off cannot reach a copy already sent. It is not a dialog, there is nothing to dismiss, and there is no first-time special case.

## Why

- **Truncation is a promise that cannot be kept.** A Share Link is public, permanent and expected to be reshared ([ADR 0007](./0007-a-recipe-is-held-by-a-kitchen-not-a-person.md)). Shortening one copy hides nothing that a copy already sent does not still carry, and there is no way to know which copies exist. It would be a privacy control that works only against people who have not received the recipe yet — which is to say, against nobody.
- **It is the sometimes-fires trap, twice refused already.** [ADR 0015](./0015-a-name-is-not-identification-it-is-a-reminder.md) rejected marking the one class of provably-forged bundle because *a check that fires sometimes teaches people it fires always*; [ADR 0016](./0016-kamosu-converts-to-the-kitchen-not-to-the-arithmetic.md) rejected a conditional "about" for the same reason. A shortening tool teaches people history is retractable. It is not, and the third refusal is the consistent one.
- **Refusing it hands [#17](https://github.com/BattermanZ/Kamosu/issues/17) a rule instead of a marker.** If a share could legitimately begin part-way along, a chain that does not reach Version 1 would mean either *deliberately shortened* or *corrupt*, and the bundle format would have to carry a flag saying which — a flag that anyone can hand-edit, in a file whose whole point is that it works with the sharer's server switched off. Forbidding truncation makes the distinction structural. There is nothing to declare and nothing to trust.
- **It keeps divergence readable.** Two people holding differently-shortened copies could share no Version at all, leaving Kamosu genuinely unable to say where they diverged and [ADR 0014](./0014-a-divergence-is-two-recipes-not-a-difference.md)'s Thread with no point to fork at. Rare, and unrecoverable when it happened. The guarantee is worth more than the escape hatch.
- **Dropping the commentary was legal and still wrong.** [ADR 0015](./0015-a-name-is-not-identification-it-is-a-reminder.md) put the name, the author and the *what changed* line **outside** the fingerprint, so a bundle omitting them would still compute every Branch Point correctly — this was a free choice, not a constraint. It was refused because the *what changed* line is the vehicle for credit: ADR 0015 settled that credit is **written down, never wired up**, with a carried-across change pre-filling *"Took the marinade step from Chez Dupont"*. Commentary that does not travel is credit that does not travel. It is also the only thing that makes reading someone else's history worth doing.
- **The private place already exists, and it is the Attempt note.** *"Cut the sugar, Marie hated it"* is a note about a **cooking**, not about a change to a recipe. [ADR 0005](./0005-an-attempt-is-a-version-you-cooked-but-did-not-keep.md) made an Attempt freely editable, freely deletable, unable to leave the instance and absent from the Share Link page. Nothing needed building; what needed fixing was an interface that invites a diary entry into a field addressed to everyone who will ever hold the recipe.
- **The rule has to bind the receiver or it is a speed bump.** The early stretch of a received Branch is legitimately somebody else's history under your own Branch id ([ADR 0015](./0015-a-name-is-not-identification-it-is-a-reminder.md)). If the receiver could prune it before resharing, truncation would be forbidden at the source and permitted one hop away, and the Branch Point guarantee would be gone one hop later too.
- **A page you can look at beats a warning you have to believe.** The exposure created here is real, and the person least able to see it is the sharer, who would otherwise open their own link, see a clean recipe card, and conclude that is what they published. Putting the Thread on the page makes the promise self-evidencing. [ADR 0017](./0017-a-photograph-is-known-by-its-contents.md) called stripped GPS *the only invisible privacy leak in the design* and designed it out rather than warning about it; a history that travels unseen would have been the second one.

## What no fact could decide

There is no evidence to appeal to here, and the absence is worth writing down rather than papering over.

Crouton exports **no recipe history whatsoever** ([#2](https://github.com/BattermanZ/Kamosu/issues/2), [#5](https://github.com/BattermanZ/Kamosu/issues/5)), so the 86-recipe library contains not one *what changed* line to measure. The nearest reading is that **21 of 86 recipes carry a note at all** — and a note on a Crouton recipe was written in a private app with no sharing, which tells us what people write when nobody is watching, not what they will write in a field the interface says is public. The three hand-made duplicate pairs are versioning that happened without any place to explain itself.

So the decision rests on the shape of the promise rather than on a count, and it is deliberately the conservative direction: a rule that carries everything can be relaxed later over a library already written, whereas a rule that quietly drops writing cannot be tightened without a permanent era of shares that lost it.

## What was rejected

- **Truncation from a chosen Version**, with the receiver told honestly that the history starts here. It is the whole subject of this ADR: it protects nothing, and it costs the one meaning a short chain is allowed to have.
- **Withdrawing a Version from a chain you hold.** Every later Version records its parent, so removing one leaves the rest pointing at nothing — producing, on your own instance, the exact shape reserved for a damaged file.
- **Dropping the *what changed* lines from bundles**, always and categorically. Consistent, and it kills credit and guts the Thread. See above.
- **A per-share toggle for the commentary.** The truncation argument in different clothes: a switch that works sometimes teaches people the notes are private-able, when an earlier share already carried them.
- **A per-note private flag** written at the time. It asks a cook at eleven at night to predict what will embarrass them, adds stored shape, and still cannot reach a copy already sent.
- **Making the *what changed* line editable later, as a Version's name is.** Structurally harmless — [ADR 0015](./0015-a-name-is-not-identification-it-is-a-reminder.md) keeps both outside the fingerprint — and it is the retraction illusion once more, since editing changes only what future shares carry.
- **A one-time confirmation the first time a person shares.** Ruled out by the same principle as the rest: a warning seen eighteen months ago is a warning that is not there, and its absence afterwards reads as reassurance.
- **Saying nothing on the share screen**, on the grounds that the page now evidences the promise. The export-a-bundle path has no page to look at, and a person can mint a link without ever opening it.

## Consequences

- **You cannot hand someone today's recipe without handing them its whole diary.** Accepted, stated plainly at the moment of sharing, and the reason the Attempt note has to be visibly the private one.
- **Once sent, your writing is in someone else's hands and neither of you can remove it.** A receiver who thinks your note is nobody's business can reshare the recipe whole or not at all.
- **A public page can be a bigger thing than a recipe card.** Someone will think twice before sharing a recipe with three years of commentary on it. That is the correct response to a public, permanent, freely resharable link, and it is better prompted by seeing the thing than by reading a caution.
- **[#17](https://github.com/BattermanZ/Kamosu/issues/17) inherits a sharpened rule, not a new field.** A bundle whose chain does not reach the first Version is damaged. What a receiving instance *does* about a damaged bundle stays that ticket's to settle; what it *means* is settled here.
- **The Share Link page gains a second rendering of a recipe's history**, server-side and in plain HTML ([ADR 0012](./0012-the-compiler-is-the-reviewer-the-frontend-does-not-have.md)). It shows the shared Branch and its Translations ([ADR 0006](./0006-a-translation-is-a-branch.md)), never Attempts.
- **[ADR 0004](./0004-one-lineage-many-branches.md)'s open question is closed**, and its "everything written travels forever" is now a promise the format enforces rather than an observation about append-only history.
