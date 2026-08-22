# Cooking shows the amounts for the step it is on

The cooking screen — one **Step** at a time, filling the phone, used at arm's length with wet hands — carries above that step the amounts for the **Ingredients** that step uses, and nothing else. `800 ml water` on the step that adds water; `4 cups cooked rice` and `4 Chicken Katsu Cutlets` on the step that plates it; *nothing new to add — just the pot* on the steps that add nothing.

The rest of Kamosu's shape follows from the same session: **Home** is shelves rather than a list, **Recipes** is a separate screen holding the whole library and its search, a recipe is one long page led by its photograph, and the tab bar is **Home · Recipes · Shopping · Cooked** with settings behind the Kitchen's name.

## Why

- **The thing that interrupts cooking is a number.** A step says "break the curry roux cake into small blocks" and never says 230g, because the amount lives in the ingredient list and the sentence assumes you remember it. In Aurélien's own 86-recipe export this is not an edge case: 28% of Ingredient Lines carry no quantity at all, and effectively none of the 599 Steps repeat one. A cooking screen showing only the instruction therefore sends the cook back to the list on most steps.
- **It costs a fifth of the screen and the step stays the largest type in the app.** The alternative that also solved it — showing every step at once, so the ingredient panel has somewhere to live — shrank the instruction to the point of squinting. Scoping the panel to *this step* is what keeps both.
- **The Reading already knows which Ingredients a Step uses**, so nothing new is stored and no one types anything. Where a Reading is absent the line is shown whole, per [ADR 0002](./0002-the-written-ingredient-line-is-the-truth.md), and the panel degrades to prose rather than failing.
- **A Component needs no special case.** "4 Chicken Katsu Cutlets" appears in the panel as an amount like any other, because [ADR 0008](./0008-a-composed-recipe-is-an-ingredient.md) made a composed recipe an ordinary Ingredient. The cooking screen learns nothing about composition.

## Considered options

- **One step, nothing else** — the most beautiful, and rejected: it is beautiful in the one place where going to look something up costs the most.
- **The whole method on one rail**, current step highlighted, ingredients ticked in a panel at the top. Solves the same problem and was rejected as too dense — the instruction is no longer the largest thing on screen, and the screen reads as a document rather than as a place you are standing.
- **A permanent split with the full ingredient list pinned.** Rejected as the panel's worst version: nine lines of which one matters, re-read each step.
- **One step, with the amounts for that step above it** (chosen).

## Consequences

- **Durations are offered as timers, read out of the Step's own text.** "Simmer for about 7 minutes" offers a 7-minute timer; nothing is typed beside the text and nothing is stored, honouring [#6](https://github.com/BattermanZ/Kamosu/issues/6)'s rule that a duration is read out of the words. A running timer follows the cook to the next step, because that is why one is set, and says *Time* rather than disappearing when it ends.
- **Home and the library are separate screens.** Home answers *show me something* — shelves computed from what already exists (cooked most, quick tonight, never cooked) — and Recipes answers *find me this*, which is where search belongs. Both are computed on top of stored data under [ADR 0009](./0009-the-v1-cut-line-is-drawn-at-shape-not-at-features.md), so neither constrains anything.
- **A cooking diary exists: Attempts sorted by date rather than by recipe.** [ADR 0005](./0005-an-attempt-is-a-version-you-cooked-but-did-not-keep.md) put Attempts on the person, but until now the only way to reach one was through its recipe — backwards for *what did I cook that week*. The diary stores nothing new and is therefore cuttable from v1 without consequence.
- **Settings hang off the Kitchen's name, not off a tab.** The tab bar is for what you open during a week of cooking; Kitchens, language, Access Keys, Backup and Operator tools are what you open when you get a new phone.
- **The visual identity is not decided here, and deliberately so.** Palette, typefaces, spacing, logo and icon are computed-on-top under ADR 0009: they can change in v2 over the library that already exists, without touching a recipe. Building a design system before the frontend stack ([#14](https://github.com/BattermanZ/Kamosu/issues/14)) is chosen would mean guessing twice, so it is the build's first task rather than the spec's last.
- **The rules that do survive any palette** are recorded as spec: the Ingredient Line is always shown at full size with its Reading subordinate; the Step's text is the largest type in the app; a recipe's photograph leads its page; nothing during a cook costs a tap the cook did not intend; and quantities Kamosu could not read are shown whole rather than guessed at.
