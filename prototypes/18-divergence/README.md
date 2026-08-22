# Prototype — what divergence looks like (issue #18)

**Throwaway. Not production code.** Its only job is to answer one question:

> You hold two versions of the same recipe — yours and a friend's. What do you
> actually see, and what do you do about it?

[#16](https://github.com/BattermanZ/Kamosu/issues/16) settled the machinery and
[ADR 0004](../../docs/adr/0004-one-lineage-many-branches.md) recorded it: one
Lineage, many Branches, **Kamosu never merges**. That decision is only as good as
the screen that expresses it, and there is no prior art — no recipe app has ever
shown a cook a divergence.

## Run it

```bash
cd prototypes/18-divergence
python3 -m http.server 9120 --bind 0.0.0.0
```

Open `http://<LAN-IP>:9120/` on a phone. No build step, no dependencies.

Switch directions with the floating black bar, the `←` / `→` arrow keys, or
`?v=a|b|c&s=shelf|recipe|diverge|history`. The `↺` button throws away any unsaved
edit you have picked up.

## The three directions

They wear **identical clothes** — #13's Noren palette and type — on purpose. The
only thing being judged here is the shape.

| | Direction | Where a divergence lives |
|---|---|---|
| **A** | **The letter** | A **message, written in sentences.** No columns, no gutter, no diff. Kamosu says *"Marc has ¾ cup potato starch where you have 1 cup"* and each sentence carries one offer. |
| **B** | **The switch** — *chosen* | **Nowhere — there is no difference screen.** Marc's Branch is a recipe, so you read it as a recipe, whole and cookable, with the handful of unshared lines marked and the lines only one of you has shown as **ghosts**. Tap any of them to see the other side without leaving the page. |
| **C** | **The thread** — *history chosen* | **The history screen doing its job.** One line runs down the page and forks where you and Marc part. The comparison is a separate, deliberately *stacked* view — yours above, his below, never two columns. |

## What was chosen

**B for reading a divergence, C's thread for history.** B now carries both: its
"Earlier versions" button opens the thread. A and C are kept as they were, for
comparison.

## What each one is claiming

- **A** claims a cook should never have to read a diff — that "Marc has X where
  you have Y" is a sentence, and sentences are what people carry in their heads.
  Its risk is length: eight sentences is a wall, and a rewritten step collapses to
  *"Marc rewrote the step that starts…"*, which tells you nothing about how.
- **B** claims the comparison is a **fiction** — there is no such object as "the
  difference", only two recipes. The first pass called deletions unshowable; that
  was wrong, and the **ghost line** is the fix. A line only one of you has is put
  on the page struck through, in the position it occupies in the recipe that
  really has it, labelled with whose it is. Reading Marc's, your ¼ cup brown sugar
  sits crossed out between the honey and the soy sauce. One mechanism serves both
  directions — a removal seen from his side and an addition seen from yours are
  the same object — so the page is **symmetric**: his gochugaru is a ghost on your
  recipe for exactly the reason your brown sugar is a ghost on his.
- **C** claims divergence is not an event to design at all — the thread was always
  there, and two people writing in it is just what it looks like. Its risk is that
  it puts *history* in front of a cook who only wanted to know what changed.

## The content is real

The recipe, both Crouton saves and every ingredient line come from Aurélien's own
86-recipe export — `Korean Fried Chicken.crumb` and `Korean Fried Chicken-1.crumb`,
one of the **three hand-made duplicate pairs** [#5](https://github.com/BattermanZ/Kamosu/issues/5)
found. The two files really do differ by sections added, `tomato sauce (/ ketchup)`
renamed to `Ketchup`, and `(I used rice bran oil)` dropped — so the first two
Versions on the thread are a divergence that actually happened, before Kamosu
existed to notice it.

**Marc's Branch is invented**, but invented to be plausible: a flat with an air
fryer and less of a sweet tooth. It was built to exercise every kind of change at
once — a quantity altered, a line added, a line **removed**, a step rewritten, a
step inserted, a note appearing.

Decisions already made by the map are visible rather than described:

- **One Lineage, many Branches** ([ADR 0004](../../docs/adr/0004-one-lineage-many-branches.md)) — the Branch Point is computed, and every direction says so in its own words.
- **Kamosu never merges** — taking a line writes it into *your* recipe and leaves it **unsaved**, marked in place with what it replaced, so what you keep is a recipe you read through. There is no "take all".
- **A recipe is held by a Kitchen** ([ADR 0007](../../docs/adr/0007-a-recipe-is-held-by-a-kitchen-not-a-person.md)) — you can read and cook Marc's Branch; editing it would be a **Copy**.
- **An Attempt belongs to the Lineage** ([ADR 0005](../../docs/adr/0005-an-attempt-is-a-version-you-cooked-but-did-not-keep.md)) — the cooking list names the Version cooked, including *cooked before yours and Marc's diverged*.
- **History is append-only** — an older Version can be read and cooked, never edited.

## What it does not answer, and knows it

- **Pairing.** Every direction depends on knowing that Marc's `¾ cup potato starch`
  is *the same line as* your `1 cup potato starch (or corn starch)` — otherwise
  every edit reads as one line removed and another added, and the letter becomes
  gibberish. The prototype cheats: each line carries a hand-written `key`. Whether
  a real Ingredient Line has an identity that survives editing and travels between
  instances is **a data-model question, not a design one**, and it is the first
  thing to settle.
- **Where a ghost goes.** A ghost is anchored to the line before it in the recipe
  that really has it, which works here — gochugaru lands under the gochujang — but
  only because that neighbour exists on both sides. Two adjacent changes, or a
  line added at the top of a section, are untested.
  The same weakness shows in the *unsaved edit*: a line you carry across that you
  had no counterpart for still lands at the bottom of your list.
- **Two Branches, or five.** Everything here assumes exactly two. A third arriving
  from a second friend is not drawn.
- **The shelf.** All three show one card per Lineage, differing only in the mark.
  Whether a Branch ever deserves its own card is untested.

No back end, no persistence, no editing beyond taking a line. State lives in
memory and resets on reload.
