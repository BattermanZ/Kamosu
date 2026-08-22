# Prototype — what Kamosu looks and feels like (issue #13)

**Throwaway. Not production code.** Its only job is to answer one question:

> What does Kamosu look and feel like, especially while you're cooking?

## Run it

```bash
cd prototypes/13-look-and-feel
python3 -m http.server 9999 --bind 0.0.0.0
```

Then open `http://<LAN-IP>:9999/` on a phone. No build step, no dependencies.

## What's in it

Three **radically different directions**, each covering the same three screens —
library, recipe, and cooking. Switch with the floating black bar at the bottom,
the `←` / `→` arrow keys, or `?v=a|b|c&s=home|recipes|shopping|cooked|recipe|cook` in the URL.

**A is the candidate.** It now has a four-tab bar — **Home · Recipes · Shopping ·
Cooked** — with settings behind the kitchen name in Home's top-right corner, and
**timers read out of the step's own text**. Earlier passes: A's look won, C's home-screen
shelves were folded into it, and its cooking screen now carries the amounts for the
current step. B and C are kept as they were, for comparison.

| | Direction | Cooking answer |
|---|---|---|
| **A** | **Noren** — indigo cloth over unbleached paper, mincho type. Quiet, tactile, Japanese. Home screen is shelves; recipes lead with big photography. | **One step, full screen, with the amounts for that step pinned above it.** Huge type, two enormous buttons, a hanging-cloth progress strip. |
| **B** | **Ticket** — the kitchen order ticket. Printed, dense, monospace data, no hero photography, one highlighter. | **The rail.** Every step stays on screen; the current one is highlighted, past ones collapse to a struck line. You always see what's coming. |
| **C** | **Counter** — warm lamplight over a dark counter. Soft, layered, thumb-first. | **The split.** What this step needs is pinned at the top the whole time; the step fills the middle; a filmstrip in thumb reach moves you along. You never scroll away from a quantity. |

They differ in structure, not just colour — that's the point. The useful reaction
is usually *"the header from B with the cooking screen from C"*.

## The content is real

Every recipe, photograph, ingredient line and cooking note comes from Aurélien's
own 86-recipe Crouton export (`samples/crouton/`, gitignored). Katsu Curry is the
worked example because it exercises most of the model at once.

Decisions already made by the map are all visible on screen, so the design is
judged against the real thing and not a mock-up:

- **The Ingredient Line is the truth** ([ADR 0002](../../docs/adr/0002-the-written-ingredient-line-is-the-truth.md)) — `400g / 0.9lb onion (sliced into 1cm / ⅜" wide pieces)` is displayed verbatim, with the Reading (`400 g onion`) sitting under it, quieter. `fukujinzuke, to serve` has no Reading at all and reads fine.
- **A composed recipe is an ingredient** ([ADR 0008](../../docs/adr/0008-a-composed-recipe-is-an-ingredient.md)) — "4 Chicken Cutlets" is an ordinary ingredient row that unfolds in place.
- **A cook in progress is an unfinished Attempt** ([ADR 0010](../../docs/adr/0010-a-cook-in-progress-is-an-unfinished-attempt.md)) — the cooking screen *is* the Attempt. The library offers to resume it. Finishing is where a rating or note is attached, and "delete — false start" is the counterweight.
- **A recipe never averages its ratings** ([ADR 0005](../../docs/adr/0005-an-attempt-is-a-version-you-cooked-but-did-not-keep.md)) — the history shows each person's rating by name and date.
- **Scaling changes the Yield, never the written line** — the stepper adds a computed quantity beside the line and leaves the line alone.
- **Durations are read out of the step text, never typed beside it** ([#6](https://github.com/BattermanZ/Kamosu/issues/6)) — "simmer for about 7 minutes" offers a 7-minute timer. Nothing about the timer is stored, and it follows you as you move between steps.
- Versions, Branches, Visibility, the Français translation and how far behind it is all appear as ordinary text on the recipe.

## Not in it

No back end, no persistence, no search, no editing, no import, no MCP door. The
state lives in memory and resets on reload.
