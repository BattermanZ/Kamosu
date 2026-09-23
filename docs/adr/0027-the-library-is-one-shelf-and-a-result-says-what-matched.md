# The library is one shelf, and a result says what matched

**Recipes** shows one shelf: everything the **Kitchen**s you cook in hold, merged, alphabetical by the title in your **Reading Language**, one card per **Lineage**. No Kitchen switcher, no modes. Where you cook in more than one Kitchen a filter names them; beside it stands ***My recipes*** — the recipes you created, branched or cooked — and neither filter sticks when you leave the screen.

_(Amended by [ADR 0041](./0041-a-recipe-is-written-in-a-cookbook-and-seen-in-a-kitchen.md): the shelf is your own **Cookbook** and every Cookbook in a Kitchen you cook in. "A Kitchen is who may change a recipe" below is no longer true: a Cookbook is. The *My recipes* filter is unchanged, and the rejection of *recipes you personally wrote* argued from the rule ADR 0041 replaces.)_

Search over that shelf matches **words and meaning together**, with an exact title match winning. It reaches your own **Attempt**s and nobody else's. Every result can say **what matched**, quoting the line — for a meaning-match as well as a word-match. Finding nothing shows the closest anyway, says so, and offers to add or import the recipe you were looking for.

A recipe with no **Photograph** is led, on its card and at the top of its page, by a **Cover** generated from its Lineage id.

## Why

- **A Kitchen is who may change a recipe, not where it is filed.** A switcher — "you are currently in the Dupont kitchen" — reintroduces filing through the back door, and taxes every search with a permissions question. On a Tuesday you want the katsu curry; which circle holds it is not a thing you should have to remember to find it. [ADR 0026](./0026-a-recipe-is-seen-by-its-kitchen-or-by-anyone-with-the-link.md) then made the shelf uniform anyway: everything on it is held by a Kitchen you belong to, so there is no mixed-provenance problem left for a switcher to solve.
- ***My recipes* is a history, not an ownership.** Created, branched or cooked — all three already stored, none new: *created* and *branched* both mean your **Hand** is on a Version of that Lineage, and *cooked* means you hold an **Attempt** on it. It answers *what do I actually cook*, which is a question you ask; the alternative reading — *what may I edit without a Copy* — is a question you never ask while browsing, because you find out by tapping edit. It also needs no curating ever: cook a Kitchen-mate's recipe once and it joins your shelf permanently, at exactly the moment you would want it to.
- **A filter that persists is a mode, and a mode you forgot you set is the switcher wearing a hat.** A filter you can see yourself holding is fine; one that greets you tomorrow is the thing this decision rejected.
- **Alphabetical lets you scan instead of read.** After a few months the thumb half-remembers where a recipe sits. Recently-added and recently-cooked both reorder themselves behind your back — cook something on Tuesday and the shelf shifts — so every visit starts from scratch. They are also Home's job: *show me something* is shelves computed from what exists, and duplicating them on Recipes blurs the two screens [ADR 0011](./0011-cooking-shows-the-amounts-for-the-step-it-is-on.md) deliberately separated.
- **Most searches are navigation, not search.** You know the recipe's name and you are typing it to get there. That must be exact or it is broken — and pure meaning-matching does not guarantee it, because to a meaning-matcher *Katsu Curry* and a similar Japanese curry are the same kind of thing, with no special respect for letters you typed. Meaning earns its keep on the other search, the one where you do not know what you want. Blending serves both; meaning alone makes the common case worse to keep the rare case pure.
- **A result that cannot explain itself is noise.** Search "chocolate", get *Chilli con carne*, and without the 50 g of dark chocolate on the card the search looks broken. This is the case where trust is won or lost, and it is answerable: [ADR 0006](./0006-a-translation-is-a-branch.md)'s multilingual model makes the match, and the quoted line makes it legible.
- **A recipe with no photograph is not a recipe missing a photograph.** [#6](https://github.com/BattermanZ/Kamosu/issues/6) settled that a recipe needs only a title, and 27 of Aurélien's 86 real recipes — 31% — carry no photograph at all. A grey tile with a broken-image icon would describe a third of a normal library as damaged.

## Considered options

**Which library**

- **One Kitchen at a time, with a switcher.** Rejected: filing by another name, and it makes every search ask a permissions question first.
- **One shelf of everything you may see, including an instance-wide level.** Rejected upstream — see [ADR 0026](./0026-a-recipe-is-seen-by-its-kitchen-or-by-anyone-with-the-link.md), which removed the levels that made "everything you may see" wider than "what your Kitchens hold".
- **One merged shelf of what your Kitchens hold, filtered** (chosen).

***My recipes***

- **Recipes held by a Kitchen you cook in.** Rejected: with [ADR 0026](./0026-a-recipe-is-seen-by-its-kitchen-or-by-anyone-with-the-link.md) in place this is the entire shelf, so the filter filters nothing.
- **Recipes you personally wrote.** Rejected as too narrow in the other direction, and it splits a shared Kitchen into his and hers — the exact thing a Kitchen exists to prevent.
- **Created, branched or cooked** (chosen).

**Search**

- **Meaning only.** Rejected: an exact title someone typed is not guaranteed to win, and that is most searches.
- **Words and meaning together, exact title first** (chosen). A toggle between the two is a plausible v2 refinement once there is real use to judge by, and costs nothing to add.

**Explaining a match**

- **Nothing on the card.** Rejected: the surprising result is the one that decides whether search is trusted.
- **A category label — "matched an ingredient".** Rejected: costs the same as quoting the line and says less than the line would.
- **Quote the matching line** (chosen).

**How a recipe is cut for meaning-matching**

- **Per Ingredient Line and per Step.** Rejected on the evidence. Cutting too fine scatters the signal across many weak vectors — the failure the literature calls context fragmentation — and a bare `salt` gives a meaning-matcher almost nothing to work with.
- **Hatchdoor's fixed-size blocks** (~800 tokens, 50 overlap, per `src/chunk/chunker.rs`). Rejected as Hatchdoor solving a problem Kamosu does not have: its notes are long unstructured prose, while the average recipe is well under one block, so pinpointing dies.
- **The recipe's own blocks — ingredients, method, Note, or per Section where the recipe has them — each embedded with the recipe's title in front of it, plus the recipe as a whole** (chosen as the starting point, not as spec; see Consequences).

**A recipe with no photograph**

- **A neutral tile with a food icon.** Rejected: that is the visual language of a thing that failed to load.
- **The title set large, filling the space.** Rejected — not wrong, but strictly less than the chosen option, which includes it.
- **A generated Cover** (chosen).

## Consequences

- **Cards carry no Kitchen name.** A recipe you imported is yours to read, cook and change; that the Branch was written by Marc's Kitchen is machinery, kept so that his next bundle continues the same Branch rather than colliding with it ([ADR 0020](./0020-a-bundle-is-one-recipes-worth-of-vault.md)). It blocks nothing, so a label would read as a fence that is not there. The **Hand** appears where it means something — in the **Thread**, as credit ([ADR 0015](./0015-a-name-is-not-identification-it-is-a-reminder.md)) — and where two Branches sit side by side ([ADR 0014](./0014-a-divergence-is-two-recipes-not-a-difference.md)).
- **One card per Lineage holds across Kitchens.** Cook in two Kitchens that each hold a Branch of one Lineage and you get one card; tapping it is [ADR 0014](./0014-a-divergence-is-two-recipes-not-a-difference.md)'s two recipes with a switch, which needed no extension. Filtering to one Kitchen shows the card with that Kitchen's Branch alone.
- **The index reaches your own Attempts and no one else's.** Not only because a diary is a diary — a Kitchen-mate's "burnt it again, honestly" turning up when you search *burnt* is a different act from her showing you — but because the alternative puts a flippable private flag inside a prepared index. Own-only means no permission check at query time and no partial rebuild when someone changes their mind. The accepted loss: *the one Camille said was too thick* is unfindable if Camille wrote it, and you ask Camille.
- **The promise is spec; the chunking is a dial.** What is fixed is that a result can say what matched. How finely a recipe is cut to keep that promise is an index — rebuildable from the recipes at any time, therefore computed-on-top under [ADR 0009](./0009-the-v1-cut-line-is-drawn-at-shape-not-at-features.md) — and should be measured against the real 86-recipe library, including its 9 non-English recipes, rather than argued in advance.
- **Pinpointing survives the coarser cut.** Once a block is matched, choosing the best line *within* it is nearly free: the query is already embedded and it is a handful of short strings. Those line comparisons never enter the ranking — they only choose what to display among lines already known to be in a relevant recipe — so display is precise without retrieval being fragmented.
- **Nothing found is not an empty screen.** Meaning-matching always has a nearest neighbour, so "nothing found" means "nothing close enough". Kamosu says exactly that, shows the closest anyway under that label, and offers *add a recipe called X* and *import from a link* — because with a library this size a search that finds nothing usually means you do not have it yet, and adding it was the next thing you were going to do. Silently showing weak matches was refused for the reason four marks have been refused on this map: it lets a bad answer pass as a good one.
- **A Cover is generated from the Lineage id, not the title.** A rename must not change how a recipe looks, or the shelf you learned by colour turns strange. And because the Lineage id is the one thing that travels ([ADR 0004](./0004-one-lineage-many-branches.md)), the same recipe wears the same Cover on Marc's instance as on yours, and a **Translation** — a Branch of the same Lineage ([ADR 0006](./0006-a-translation-is-a-branch.md)) — wears it too. One dish, one face, everywhere. Nobody designed that; it falls out.
- **The Cover still carries the title**, so it is the large-title treatment with a coloured ground behind it rather than an alternative to it. Card height is the same with or without a Photograph, so a third of the shelf does not read as ragged.
- **The palette the Cover draws from is not decided here.** It is a design-system input, and the visual identity remains out of scope on this map ([ADR 0011](./0011-cooking-shows-the-amounts-for-the-step-it-is-on.md)). What is decided is that absence of a photograph is filled rather than shown as a gap — a rule that survives any palette.
- **Home gains a *recently opened* shelf**, amending [ADR 0011](./0011-cooking-shows-the-amounts-for-the-step-it-is-on.md)'s list of *cooked most, quick tonight, never cooked*. It stores one fact per Person per Lineage — when you last opened it — which never travels, appears in no fingerprint and no **Bundle**, and costs nothing if lost.
- **All of it is computed on top of stored data** except that one timestamp, so under [ADR 0009](./0009-the-v1-cut-line-is-drawn-at-shape-not-at-features.md) the whole screen is free to be redesigned in v2 over the library that already exists.
