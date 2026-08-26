# PROTOTYPE — the visual identity (issue #36)

**Throwaway.** Its only job is to let Aurélien choose between four visual
directions. Nothing here is a live reference, nothing here enters the app's
stylesheet (that is #80), and every direction not chosen is discarded.

## Run it

```
./serve.sh          # http://192.168.31.233:8765 — port 8765 is UFW-allowed
```

## What is being decided, and what is not

**Decided here:** palette · typefaces · type scale · spacing scale · logo and app icon.

**Not decided here, and deliberately identical across all four directions:** the
shape of every screen. That was settled by ADR 0011 and by
`prototype/13-look-and-feel-attempt-2`, where Aurélien chose direction A ("Noren").
Home is shelves that scroll sideways, search lives on Recipes and never on Home, a
recipe is one long page led by its photograph, and cooking is one Step with the
amounts for that step above it.

That separation is enforced by the code, not by discipline: `shape.css` and
`shape.js` name no colour, typeface or size — only tokens. A direction is
`themes.js` plus a mark, and nothing else. What you see in the spec panel is read
back off the same tokens the screens render through, so the panel cannot disagree
with the screens.

## The four directions

| | | |
|---|---|---|
| **Noren** | Indigo over kinari paper, Zen Old Mincho | The #13 pick, carried over unchanged |
| **Noren · Latin type** | Noren's colour, Fraunces + Inter | Is it the colour you like, or the lettering? |
| **Noren · tight** | Noren's typefaces, Paper's numbers | Same faces, less air |
| **Koji** | Ivory, persimmon, lacquer, Bricolage Grotesque | Warm, round, contemporary |
| **Paper** | Cream, bordeaux, Fraunces | Classic western cookbook |
| **Slate** | Cool mist, teal, Archivo | The one with no warmth in it |

`?d=noren|norenLatin|norenTight|koji|paper|slate` · `?lang=fr|en|es` · floating bar at the bottom, or
← / → for direction and 1 / 2 / 3 for language.

## Seven screens, chosen to be the ones the choice turns on

Home, Recipes, a recipe led by its photograph, a recipe wearing a generated Cover,
and the three cooking states ADR 0011 names — a step that adds an amount, a step
that adds nothing, and an amount that is a Component.

## The content is real

Every recipe, Ingredient Line, Step and photograph comes from the 86-recipe Crouton
export in `samples/`. Ingredient Lines are written as a person writes them and some
deliberately carry no Reading. The interface translates; recipe text does not,
because a recipe's Language belongs to the recipe (ADR 0006).

## Rules every direction obeys (ADR 0011, spec — not one direction's ideas)

- The Step's text is the largest type on screen.
- An Ingredient Line is set at full size, its Reading typographically subordinate.
- A recipe is led by its photograph, or by the Cover generated in its place.
- Quantities Kamosu could not read are shown whole rather than guessed at.
