# The visual identity — chosen 26 August 2026

**This is a dated record, and it is never updated.**

It says what was chosen on one day and why. The mockups beside it, in
`2026-08-26-the-visual-identity/`, are what those choices looked like applied to real
recipes on that day — nothing more. They are **not** a reference to build against and
must never be treated as one: the screens in them were borrowed from an earlier
prototype in order to have something real to judge a palette against, and the real
screens are decided in [#37](https://github.com/BattermanZ/Kamosu/issues/37) and the
screen tickets. A mockup that no longer matches the app is a confident lie about what
Kamosu looks like; this one stays true precisely because updating it is forbidden.

What *is* always current lives in [#80](https://github.com/BattermanZ/Kamosu/issues/80):
a tokens page rendered from the real stylesheet, which cannot drift because it is the code.

## What was chosen

**Noren · tight** — the indigo-over-unbleached-paper direction Aurélien first chose in
`prototype/13-look-and-feel-attempt-2`, with its Japanese typefaces kept and its type
scale tightened.

## The palette

| Token | Value | Role |
|---|---|---|
| `--ground` | `#f4efe3` | *Kinari*, unbleached paper. The app's ground. |
| `--ground-2` | `#ece5d5` | Recessed ground — empty tiles, hero backing |
| `--card` | `#fffdf8` | Cards and fields, a shade above the ground |
| `--ink` | `#1a1a1c` | *Sumi*. All primary text. |
| `--ink-2` | `#5c5a55` | Secondary text — Readings, meta, blurbs |
| `--rule` | `#ded5c2` | Hairlines |
| `--accent` | `#1d2b4c` | *Ai*, indigo. The one thing you tap. |
| `--on-accent` | `#f4efe3` | Text on indigo |
| `--support` | `#b8474b` | *Beni*, safflower. Language badges, alerts. |
| `--support-2` | `#7d8b5c` | *Matcha*. A Reading that points at a Component. |

The cooking screen is **the one place the ground changes** — it turns indigo, so that
standing at the stove feels like a different room. That is part of the identity, not a
detail of the cooking screen:

| Token | Value |
|---|---|
| `--cook-ground` | `#131c33` |
| `--cook-ink` | `#f4efe3` |
| `--cook-ink-2` | `#9aa8c4` |
| `--cook-rule` | `#2c3a5c` |
| `--cook-panel` | `#1d2b4c` (the amounts panel) |
| `--cook-accent` | `#f4efe3` |
| `--cook-on-accent` | `#131c33` |

## The typefaces

- **Zen Old Mincho**, weight 600 — Steps, titles, Covers, numerals in the amounts panel.
- **Zen Kaku Gothic New**, weights 400 / 500 / 700 — the whole interface, Readings, meta.

Both are published on Google Fonts under the SIL Open Font License. #80 must vendor the
licence file alongside them.

**Ship the Latin subsets only.** These are Japanese families; their full coverage is
2.7 MB and 1.4 MB respectively, and Kamosu's interface is French, English and Spanish.
Latin + Latin-ext across every weight above is **70 KB total** — the exact files are
vendored in `2026-08-26-the-visual-identity/fonts/` and can be lifted straight into #80.

Zen Kaku Gothic New has no 600 weight; anywhere the shape asks for 600 it must resolve
to 700, not synthesise.

## The type scale

| Role | Size | Line height | Tracking |
|---|---|---|---|
| **Step** | 33px | 1.34 | −0.015em |
| **Recipe title** | 27px | 1.16 | −0.012em |
| **Ingredient Line** | 17px | 1.4 | — |
| **Body** | 16px | 1.55 | — |
| **Reading** | 12.5px | — | — |
| **Label** (uppercase) | 10.5px | — | +0.15em |

The ranking is spec, from [ADR 0011](../adr/0011-cooking-shows-the-amounts-for-the-step-it-is-on.md):
the Step is the largest type in the app, the Ingredient Line is set at full size, and its
Reading is subordinate beneath it. The Ingredient Line is deliberately **larger than body
text** — it is what you break off cooking to check, and ADR 0002 makes it the truth of the
ingredient.

This is not a modular scale. The sizes were fitted to real content, and #80 should ratify
that rather than inherit it silently.

## The spacing and shape scale

- Spacing steps: **4 · 8 · 12 · 16 · 24 · 32 · 48**
- Screen gutter: **20px**
- Corner radius: **2px everywhere**, including buttons and fields. Near-square is the direction.
- Rule weight: **1px**
- Shelf tile width: **168px**
- Hero height: **282px**

## The mark

- **Wordmark** — 醸 reversed out of a 2px-cornered indigo square, followed by *Kamosu* set
  in Zen Old Mincho 600.
- **App icon** — the same indigo square with 醸 in kinari, filling it.

醸 (*kamosu* — to ferment, to brew) is one Japanese glyph, and the Latin-only font subsets
above do not contain it. **#80 must convert it to an SVG path** rather than depend on a
font, which is what a logo should be anyway.

## Why this, over the alternatives

Six directions were built on one fixed shape, so that only colour, type and spacing varied.
Four were discarded outright — *Service* (near-black and industrial), *Koji* (ivory and
persimmon, generous radii), *Paper* (cream, bordeaux, Fraunces) and *Slate* (cool mist and
teal). Noren was preferred on colour immediately; the open question was its lettering.

Two probes settled it. Setting Noren's Japanese faces to Paper's numbers, and setting
Noren's colours with Paper's actual typefaces, were built side by side and measured against
all **579 real Steps** in the Crouton export, in the cooking screen's real text area
(320 × 424px):

| | Characters that fit | Steps fitting without a scroll |
|---|---|---|
| Noren, as first drawn (34px / 1.42) | 147 | 72% |
| **Noren · tight** (33px / 1.34) | **178** | **79%** |
| Noren with Fraunces (33px / 1.34) | 178 | 79% |

Mincho at those numbers fits **exactly** as much text as Fraunces does. The legibility gain
was entirely the spacing and never the typeface, so the Japanese lettering was kept at no cost.

Roughly a fifth of Steps will still scroll at any size in this range — the corpus runs to a
median of 92 characters, a 90th percentile of 269 and a longest of 1469. That is a property
of the recipes, not of the type, so the design must make scrolling a Step feel intended
rather than pretend it will not happen.

## What #80 still has to finish

Not every size in the prototype's shape file is a token yet — shelf headings, tile titles,
list titles and the amounts-panel figure are still hard-coded. #80 must finish tokenising
them rather than copy the numbers across.

## Where the working is kept

The full six directions, interactive, are the primary source for this decision and live on
the throwaway branch **`prototype/36-visual-identity`**. They are not on `main` and are not
maintained.
