# A shopping list is a choice of recipes, not a list of things

A **Shopping List** is one **Person**'s standing choice of what they are about to cook. What Kamosu stores is the choosing — the recipes picked, the **Yield** each is being shopped for, and any **Loose Item** typed by hand. What it shows is worked out from that choice every time it is looked at and kept nowhere:

> **The choosing is stored; the rows are computed.**

- **One per Person, unnamed, always there.** Not a Kitchen's, not several, not archived.
- **It points at a Branch, always at its latest Version.** Never a **Lineage**, never pinned.
- **Two kinds of row.** A **Shopping Row** names a **Food** and merges every mention of it; a verbatim line — an **Ingredient Line** with no **Reading**, or a **Loose Item** — merges with nothing.
- **Amounts add where the arithmetic holds**, in the reader's **Reading Measures**, saying *about*; where it does not, the row carries both amounts rather than one wrong one.
- **Nothing is ticked off.** The list leaves as text and Kamosu lets go of it.
- **It works offline**, on the **Attempt**'s side of [ADR 0013](./0013-offline-you-may-write-your-own-history-never-the-recipes.md)'s line.

## Why

- **A stored row is a second copy of a fact the recipe already holds.** Every other derived quantity in Kamosu is computed and kept nowhere — a **Component**'s amount from the inner **Yield** ([ADR 0008](./0008-a-composed-recipe-is-an-ingredient.md)), a **Branch Point** by walking both chains, a **Translation**'s staleness, a **Pairing** by reading two Branches. A saved shopping row would be the first exception, and it would go stale the first time anyone corrected a **Reading**. Computing it means the list is simply right the next time it is opened.
- **But the choosing genuinely has to survive, and that is what [ADR 0009](./0009-the-v1-cut-line-is-drawn-at-shape-not-at-features.md) was pointing at.** Planning happens on the iPad on Sunday and shopping happens on the phone on Tuesday. A purely ephemeral list — tick some recipes, render, forget — cannot span those two days or those two devices. Splitting the feature in half puts the storage exactly where it is load-bearing and nowhere else.
- **A Person's, not a Kitchen's — Aurélien's call, against the recommendation, and it holds up.** A **Kitchen** was the tempting answer because shopping looks like a household act. But the way out is an **iOS Shortcut** appending to *your* Apple Notes, so a Kitchen-owned list would have been shared state with a private door — worse than either pure shape — and it would have forced a *which Kitchen?* prompt onto a **Person** who cooks in more than one. It also retired the hardest thing [ADR 0013](./0013-offline-you-may-write-your-own-history-never-the-recipes.md) foresaw, which was two people ticking one shared list in two shops with bad signal.
- **A merged row cannot be a written line, so the list is the one screen where [ADR 0002](./0002-the-written-ingredient-line-is-the-truth.md) does not hold.** `500 g de farine T55` and `2 cups plain flour` become one row, and neither sentence is true of the other recipe. Naming the **Food** is the only honest option — and it is exactly what [#10](https://github.com/BattermanZ/Kamosu/issues/10) built a per-Language Food name *for*, so that *farine* and *flour* merge. The written lines stay one tap away, which is [ADR 0019](./0019-an-ingredient-line-has-no-name-of-its-own.md)'s instinct again: where a machine judgement might mislead, show the originals rather than a badge about them.
- **The same shape whether or not it merged.** A list that sometimes quotes a line and sometimes names a Food teaches nobody which they are reading — [ADR 0015](./0015-a-name-is-not-identification-it-is-a-reminder.md)'s trap, where a mark that appears sometimes makes its absence look like a promise.
- **Adding across units is the job being handed over.** [ADR 0016](./0016-kamosu-converts-to-the-kitchen-not-to-the-arithmetic.md) made it possible — a closed convertible set, a **Cup Weight** carrying volume to weight. Someone standing in a shop deciding whether one bag is enough is answered by *about 750 g* and not by *500 g + 2 cups*, which hands the arithmetic straight back.
- **An unstated amount is an amount, and 28% of real rows have one.** [#5](https://github.com/BattermanZ/Kamosu/issues/5) found 28% of Aurélien's 599 Ingredient Lines carry no quantity. Dropping those contributions means buying 750 g and coming up short; folding them into the number invents a figure nobody wrote. They ride on the row as one more thing that will not add — the same treatment `poignée` gets, and no new mechanism.
- **Three sources, two shapes.** A Food row that counts, and a verbatim line that does not. A line nobody ever read, and a **Loose Item** typed on the way out of the door, are both the second kind — so *bin bags* needed no machinery of its own.
- **A Loose Item is never interpreted, deliberately.** Reading `500g flour` out of something typed in a hurry means guessing, and a wrong guess silently changes a number about to be shopped by. A Shopping Row is built on Readings that already exist and have been seen; a line typed at the door has no such backing. The visible cost is that typing *flour* alongside a recipe wanting flour gives two lines.
- **A row has no name to staple a tick to.** A row is derived from a Food and however many recipes mention it, and it changes shape the moment a Yield moves. This is [ADR 0019](./0019-an-ingredient-line-has-no-name-of-its-own.md) surfacing somewhere new. Storing rows so they could hold ticks is the rejected option all over again.
- **And the list leaves, so a tick here is a second truth about one trip.** Apple Notes has checkboxes, on the phone already in the hand. Kamosu decides what to buy and something else carries it round the shop — the boundary [ADR 0023](./0023-a-sheet-carries-the-recipe-not-the-library.md) drew for paper, drawn again.
- **A Branch and not a Lineage, because a shopping trip is about a text and not a dish.** A **Component** names a Lineage on purpose — the pizza means *whichever dough you keep*. But two Branches of one Lineage are a permanent, ordinary divergence, and a friend's ratatouille may want anchovies where yours does not. Resolving to a Lineage would send someone home without anchovies, silently, and it would be Kamosu's fault.
- **Latest and never pinned, for the reason an Attempt is the opposite.** An **Attempt** pins by fingerprint because it records what was cooked and must never move. A list describes what is about to be bought, so it must move: a recipe edited between Sunday and Tuesday is right on Tuesday.
- **Offline needs nothing new.** What is written offline is adding a recipe, moving a Yield, typing a line, removing something — facts about one person on one afternoon, with no reconciling to do. Across two devices *the last one moved on is where you are*, already written for an **In Progress** by [ADR 0010](./0010-a-cook-in-progress-is-an-unfinished-attempt.md).

## Considered options

- **A one-shot rendering, storing nothing.** Rejected: cannot span two days or two devices, which is what planning actually is.
- **A stored list of rows, editable and tickable.** Rejected: a stale second copy of the recipe, and a second truth about one shopping trip beside the note it was sent to.
- **Several named lists, archived.** Rejected as filing — the same instinct that dropped folders in [#12](https://github.com/BattermanZ/Kamosu/issues/12), and it implies a shopping history nobody asked for.
- **Emptying the list automatically when it is sent.** Rejected: silent and unrecoverable. Kamosu offers and does not act — language detection is offered when it disagrees, a **Merge Suggestion** is evidence and never an instruction.
- **A Kitchen's list.** Recommended and vetoed. See above; the veto was right.
- **Two amounts as two rows.** Rejected: one thing to buy is one line.
- **Grouping the outgoing text by aisle.** Rejected: a classification to build, maintain and translate, wrong for every shop but one.
- **Grouping it by recipe.** Rejected: defeats merging, a merged row belonging to two recipes at once.
- **Storing the choice and computing the rows** (chosen).

## Consequences

- **The list is in a Backup and in no Bundle.** It is a Person's own data and no part of any recipe.
- **A Shopping Row is the only place a Food's name is read instead of an Ingredient Line.** Qualifiers fall off — `à température ambiante`, `sifted`, `environ` are for cooking rather than buying — and so does `farine T55` becoming `farine`, which is a real loss and lands on the shopper.
- **Merging is only as good as the Foods are.** [ADR 0022](./0022-doubt-makes-a-new-food-never-a-merge.md) accepts a visible tail of near-duplicate Foods, and the shopping list is where that tail is felt: two Foods for flour mean two rows. The repair is the Operator's **Merge**, already granted, and the list is a good place to notice it is needed.
- **The rows compute offline for cached recipes only.** A Kitchen's own recipes are cached in full ([ADR 0013](./0013-offline-you-may-write-your-own-history-never-the-recipes.md)); a recipe seen through someone else's **Visibility** and never opened is not. That entry says so and contributes nothing, rather than shopping short in silence.
- **A recipe that goes away stays on the list.** Deleted, or a sharing withdrawn: the entry keeps the name it remembers, contributes no rows, and says it can no longer be read. A thing that quietly disappears from a shopping list is a thing that does not get bought.
- **Parity costs nothing.** *Add the ratatouille to my list*, *what is on my list*, *give me the list as text* are ordinary Operations under [ADR 0001](./0001-both-doors-generated-from-one-operation-catalogue.md), so the agent door arrives finished.
- **The iOS Shortcut ships as documentation, not code** ([#12](https://github.com/BattermanZ/Kamosu/issues/12)), and works with no network, since handing text to a Shortcut never leaves the phone. Google Keep is the same shape for someone not on Apple and stays deferred.
- **The outgoing text carries a header line** — date and recipe names — because the note accumulates, and three trips appended with no divider are a wall.
