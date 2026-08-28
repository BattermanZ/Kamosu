# Prototype — the switch, and how a Ghost reads (issue #55)

**Throwaway. Not production code.**

> Three treatments of the **switch** between two whole recipes and of the
> **Ghost**, rendered against a real divergence — including an uncertain
> Pairing. Switchable with `?v=a|b|c&side=mine|theirs`.

[ADR 0014](../../docs/adr/0014-a-divergence-is-two-recipes-not-a-difference.md)
already chose the *shape*: two whole recipes, a switch between them, Ghosts for
the lines only one side has, and no difference screen anywhere. That decision is
not being reopened. [ADR 0019](../../docs/adr/0019-an-ingredient-line-has-no-name-of-its-own.md)
then settled that no id is stapled to a line — which line is which is read
against the Branch Point.

What #55 is actually deciding, and all this exists to answer:

- **How the switch reads.** A control you operate, or a place you are in?
- **How a Ghost reads.** How loudly does a struck-through line have to announce
  that it belongs to the other recipe?

## Run it

```bash
python3 -m http.server 9120 --bind 0.0.0.0 --directory prototypes/55-the-divergence
```

Open `http://<LAN-IP>:9120/` on a phone. No build step, no dependencies.

Switch directions with the floating black bar or the `←`/`→` arrow keys. The
**Pairing** button in that bar opens scaffolding — see below.

## The three directions

All three render the *same* two recipes from the *same* computed Pairing, and
wear Kamosu's real design tokens. The only things that differ are the switch and
the Ghost.

| | Direction | The switch | The Ghost | The risk, on purpose |
|---|---|---|---|---|
| **A** | **Two tabs** | A segmented `Yours / Marc's` control, stuck to the top of the screen so it never scrolls away. | Struck through **in the flow of the list**, with a caption under it in words: *"Marc's — you haven't got it"*. | Eight captions is eight captions. Judge whether the page stops being a recipe and becomes a report about a recipe. |
| **B** | **Two rooms** | Not a control — a **threshold**. A band names the kitchen you are standing in and one wide target says where it goes: `Chez Marc →`. The paper, the rules and the spine all change colour with the room. | **Quiet.** Struck through and dimmed, marked only by a dot in the margin in the other person's colour. Not one word until you tap it. | A dot is not a sentence. Judge whether a first-time reader knows a struck line is a real line of a real recipe, rather than something crossed off. |
| **C** | **A lever and a slab** | Pinned at the **bottom**, under your thumb, where the flicking between two recipes actually happens. Never in the scroll, so it can never scroll away. | Pulled **out of the list** onto an indented, recessed slab, unnumbered, named with the recipe it really belongs to. | A slab breaks the column. ADR 0014 wanted a Ghost *in the position it holds over there*; indenting keeps the position but loosens the sense that it is part of the sequence. |

The switch and the Ghost are two separate axes, and the treatments can be mixed —
B's threshold with A's captions is a real option, and cheap to build.

## The Pairing is computed, not faked

[#18](https://github.com/BattermanZ/Kamosu/issues/18)'s prototype cheated: every
line carried a hand-written `key`, so "Marc's ¾ cup potato starch is your 1 cup
potato starch, changed" was asserted rather than worked out. ADR 0019 then
refused to staple an id to a line at all.

So `pair.js` here does the real thing. **No line in `data.js` carries an id.**
The reader takes the two Branches and their Branch Point, matches untouched lines
exactly, reads the handful somebody edited by similarity, and — where two lines
both arrived after the Branch Point and do not resemble each other — **declines
to pair them**.

The **Pairing** button in the floating bar opens a panel listing every verdict
and why, so this can be checked rather than believed. It is scaffolding, not part
of any design being judged.

### The cases the fixture was built to produce

| What happened | What the reader concludes |
|---|---|
| Your `1 cup potato starch (or corn starch)` → Marc's `¾ cup potato starch` | **Paired.** Both descend from the same Branch Point line. |
| `2 Tbsp minced garlic`, select-all-deleted and **retyped character for character** | **Nothing.** No divergence, which is ADR 0019's whole argument. |
| `¼ cup brown sugar`, which Marc took out | **A Ghost** when you stand in Marc's recipe — and the offer is *take it out of mine as well*. |
| Your `1 Tbsp rice vinegar` and Marc's `1 tsp gochugaru`, both written after you parted, landing in the same place in the sauce | **Left unpaired.** They score 0.375 against each other, under the bar. Both lines stand, unjoined, with no badge and no hedge — exactly ADR 0019's refusal to claim a connection. |
| Marc's `neutral oil, in a spray bottle` where you have `Some cooking oil (for deep frying — rice bran is best)` | **Left unpaired**, at 0.161. A person would call this one changed line; Kamosu reports a removal and an addition. This is ADR 0019's *accepted loss*, and it is on the page rather than described. |
| Marc's deep-fry step rewritten wholesale as an air-fryer step | **Left unpaired**, for the same reason and by design: the old text struck through beside the new is more use than a label reading *Marc rewrote this*. |

## The content is real

The recipe, the Branch Point and every line of it come from Aurélien's own
86-recipe Crouton export — `Korean Fried Chicken.crumb` — by way of the #18
prototype. **Marc's Branch is invented**, but invented to be plausible: a flat
with an air fryer and less of a sweet tooth. It was built to exercise a quantity
altered, a line added, a line removed, a step rewritten past recognition, a step
inserted, a note appearing, and the two unpairable cases above.

## What it does not answer, and knows it

- **Where the offer lives.** All three put *take this* only on Marc's page, on the
  argument that you shop while reading his. Your own page marks the divergence but
  offers nothing. That asymmetry is assumed here, not tested.
- **Two Branches, not three.** ADR 0014 says the switch is the part that will not
  survive a third Branch unchanged. No direction here tries.
- **A Ghost's position when its anchor is missing.** The anchoring is a best
  effort and the fixture does not stress the case where the line above a Ghost is
  itself absent on the other side.
- **The thread, the shelf, cooking, scaling.** Buttons exist and do nothing.

No back end, no persistence. State lives in memory and resets on reload.

## What was chosen

**B's switch, A's Ghost, and a new third thing — served as direction D**, the
default when the page opens. Aurélien's call, 28 August 2026.

- **The threshold wins as the switch.** Crossing into a named kitchen, with the
  paper and the spine changing colour, beats a pair of tabs — two tabs read as
  two views of one object, which is the merge idea [ADR 0004](../../docs/adr/0004-one-lineage-many-branches.md)
  refuses.
- **The captions stay.** B's silent Ghost was rejected outright: *"it is a bit
  unclear when some things are struck through."* A struck-through line with no
  words next to it reads like something crossed off a shopping list rather than a
  real line of a real recipe. So every marked line and every Ghost is named.
- **The marking gets an off switch**, which no direction had. One control under
  the threshold puts the whole divergence away and leaves the recipe you are
  standing in, plain — no marks, no captions, and **no Ghosts at all**, because a
  Ghost is a line of the *other* recipe and has no business on the page once you
  have stopped comparing. The steps renumber to a clean run. This is the page you
  cook from and the list you shop from.

The off switch is an addition to [ADR 0014](../../docs/adr/0014-a-divergence-is-two-recipes-not-a-difference.md)
rather than a departure from it: the ADR's own argument is that the reading
surface is a recipe you can stand in and cook, and hiding the marking makes it
more of one, not less. Nothing about it implies a reconciled recipe — both
Branches are still whole, still separate, still one tap apart.

A, B and C are kept as they were, for comparison.
