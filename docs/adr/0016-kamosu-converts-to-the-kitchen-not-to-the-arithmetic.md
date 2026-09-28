# Kamosu converts to the kitchen, not to the arithmetic

A **unit** is whatever the cook wrote. Kamosu ships a closed list of units it can do arithmetic on — gram, kilogram, millilitre, litre, teaspoon, tablespoon, cup, fluid ounce, ounce, pound, and a few explicitly-named regional variants — each knowing its own spellings in the three interface languages, so `g`, `gr`, `gramme`, `grammes` and `grams` are one unit. Anything else — `poignée`, `noix`, `sachet`, `bottle` — is kept exactly as written, is a perfectly legitimate unit, and simply never offers a conversion. **The unit set is open; the convertible set is closed.**

A conversion is never a rewrite. The **Ingredient Line** stays verbatim ([ADR 0002](./0002-the-written-ingredient-line-is-the-truth.md)), and beneath it sits **one** number: how much, for you, right now — scaled to the Yield being cooked and expressed in your **Reading Measures**, a per-person setting alongside **Reading Language**. It is absent when it would only repeat the line unchanged, so a recipe already in your measures at its written Yield is untouched.

Every converted number is rounded to something a scale or a jug can show, and **always says "about"**.

Cross-family conversion — a volume of a dry good into grams — runs off a **Cup Weight** on the **Food**: one figure, grams per cup, from a small table Kamosu ships for the staples, overridable by anyone. Where there is none, the line falls back to millilitres and says nothing about grams.

Temperatures in **Step** text convert too, as an addition beside the sentence and only where the step does not already carry both — on the conventional **oven ladder** (350°F → 180°C), not by arithmetic.

_(Amended by [#150](https://github.com/BattermanZ/Kamosu/issues/150), 25 September 2026: a Step's **amounts** convert and scale too, and every conversion, the oven's included, is shown straight after what it converts rather than beside the sentence; a bare `425°` counts as an oven where only one dial fits. See "Amounts in a Step" at the end.)_

## Why

- **The kitchen answer and the arithmetic answer are not the same answer.** A cup is 236.588 ml; no jug has that. 350°F is 176.67°C; no oven has that. A cup of flour is not a density, it is 125 g by convention and 20% either way by packing. Every one of these decisions went the same way for the same reason: the number exists to be acted on while standing at a worktop, and a number the equipment cannot express sends the cook back to make the judgement Kamosu was there to make.
- **"About" is a fact about the number, not an apology for the food.** Kamosu rounded, every time, so "about" is true every time. Saying it only where the estimate feels shaky is the trap [ADR 0015](./0015-a-name-is-not-identification-it-is-a-reminder.md) caught in a different guise: a mark that appears *sometimes* teaches people that its absence is a promise — and a bare `450 g` would then be a lie in the quiet cases, which are most of them.
- **Scaling and conversion are one act, so they get one slot.** Both answer *the written line says one thing; what does that mean for me today*. Given one subordinate line they never have to negotiate: scaled-not-converted, converted-not-scaled and both look identical, and the cooking panel — a fifth of a phone, read at arm's length — carries one number instead of two. A metric cook has no use for `4½ cups`; she has no cup. The cups are on screen regardless, in the line above.
- **A blank Cup Weight is not a gap to close.** It is a line that offers millilitres instead. This is the whole distance between this decision and Tandoor's, which [#4](https://github.com/BattermanZ/Kamosu/issues/4) costed: a per-food conversion model, a USDA id pasted in per ingredient, and three distinct "couldn't work this out" flags still showing in the interface. Nobody here is ever asked to fill anything in, and the failure mode is silence.
- **The open/closed split is the only stored-shape commitment here.** A unit lives inside a Reading and so is expensive to change later ([ADR 0009](./0009-the-v1-cut-line-is-drawn-at-shape-not-at-features.md)); everything else — the shipped weights, the ladder, the rounding, the preference — is computed on top and could be revised in v2 over the library that already exists.

## The facts that decided it

Measured directly from Aurélien's 86-recipe Crouton export ([#5](https://github.com/BattermanZ/Kamosu/issues/5)), 863 ingredient rows and 579 steps:

- **41% of ingredient rows are imperial** — `CUP` 118, `TABLESPOON` 113, `TEASPOON` 92, `OUNCE` 16, `POUND` 12 — against 121 metric rows. [#12](https://github.com/BattermanZ/Kamosu/issues/12)'s promise was well aimed.
- **The cup is the problem; the spoon is not.** Of the 118 cup rows, **42 are liquids** (pure arithmetic to millilitres), **58 are baking staples** (flour, the sugars, butter, cheese, chocolate, oats, rice, nuts) and **18 are the rest** — sliced mushrooms, spinach, blueberries, where a cup is a loose handful and a gram figure would be false precision. A shipped table of roughly twenty entries plus plain arithmetic therefore covers **100 of 118**. Meanwhile the teaspoon and tablespoon rows are dominated by salt, vanilla, baking powder, garlic powder and spice mixes, which nobody wants in grams.
- **Recipe writers already dual-print temperatures.** Of 39 steps carrying one, **20 give both units** (`180C/350F`, `350°F (175°C)`), 11 are Celsius only, and **3 are Fahrenheit alone**. The step text also contains `turn 90 degrees` and `gas 5`, which are not temperatures.
- **The corpus contradicts itself on the ladder.** `Preheat oven to 180C/350F` and `350°F (180°C)` sit alongside `350°F (175°C)` and `350 F (175 C)` — four authors, two answers, one temperature. The disagreement is itself the evidence that no arithmetic result is expected here.

Prior art, read in source rather than in documentation:

| | how big is a cup? | converts? |
|---|---|---|
| **Mealie** | never says. 24 named units seeded per locale, and `en-US.json` and `en-GB.json` are byte-identical for cup/tbsp/tsp — a unit is `name`, `plural_name`, `abbreviation` and nothing else | no, unless you personally fill in an optional "this equals N of that" |
| **KitchenOwl** | no unit table anywhere; quantity and unit fused into one free-text string | no |
| **Crouton** | closed 15-value enum | no |
| **Tandoor** | one `us_cup` (236.6 ml). No British cup, no metric cup. Bare `tbsp`/`tsp` are American; `imperial_tbsp`/`imperial_tsp` are separate named units. **No Australian tablespoon at all** | within a family, yes; across families only via a per-food rule you write |

Three of four do not convert. The one that does chose a single default cup and shipped with exactly the residual gap this decision accepts.

## Considered options

- **Units as free text** (`g`, `grams`, `gr` are three units). Rejected: nothing can ever convert, which abandons [#12](https://github.com/BattermanZ/Kamosu/issues/12)'s promise outright.
- **Units as shared records like Foods**, created on the fly, renameable, mergeable by the Operator. Rejected: a Food is a shared record because matching it to nutrition data is expensive and worth doing once ([#4](https://github.com/BattermanZ/Kamosu/issues/4)). Nothing about `poignée` is expensive. It buys a list to tidy and returns nothing.
- **No cross-family conversion; within-family arithmetic only.** Rejected: turns `3 cups flour` into `≈ 710 ml`, which is not a thing anyone can do with a scale — and 58 of 118 cup rows are exactly that case.
- **A hand-filled weight on every Food** (Tandoor's shape). Rejected on Tandoor's own measured cost; the work recurs forever, once per ingredient anyone ever adds.
- **Conversion as a per-recipe toggle, or a tap on the line.** Rejected against ADR 0011's rule that nothing during a cook costs a tap the cook did not intend — the moment you need to know what a cup of flour weighs is the moment your hands are in the flour. A per-recipe switch also makes a fact about *you* into a setting on 86 separate recipes.
- **"About" only where the estimate is shaky.** Rejected as the tempting middle; see Why.
- **Arithmetic temperature conversion** (350°F → about 175°C). Rejected: 400°F → 205°C sends the cook back to the dial to make the same judgement again.
- **No temperature conversion in v1**, on the grounds that it stores nothing and only 3 of 579 real steps need it. **Recommended and overruled, correctly.** The 86 recipes are the ones Aurélien *kept*, which means the ones he could already cook — a pure-Fahrenheit recipe is one he would have bounced off before it reached the library. The measurement was survivorship-biased, and no oven in France has an F on the dial.
- **Asking which country's cup a recipe uses**, per recipe or per import. Rejected: it hands the cook a question nobody can answer — no one looking at a recipe knows whether its author's tablespoon was Australian. Guessing from Language or Source is the same guess with the user's name taken off it, and would be confidently wrong on the American recipes written in English, which are most of them.

## Consequences

- **The default measures are American, stated plainly in the documentation**: a cup is 240 ml, a tablespoon 15 ml, a teaspoon 5 ml. The ambiguity only reaches a cook through the *volume* path, and the volume path is liquids, where 240 versus 250 ml of stock sits inside the "about" already printed. The case that matters — a cup of flour — never passes through millilitres at all: it goes straight to grams through the Cup Weight, and every published figure in that table is a US-cup figure already.
- **The accepted cost is the Australian tablespoon.** Three tablespoons of soy sauce read as 45 ml where the author meant 60. Mitigated, not solved, by naming regional units explicitly.
- **`imperial tablespoon` and `Australian tablespoon` are ordinary members of the convertible set.** This falls out of the open/closed rule at the cost of one line each, and a recipe that names one gets it right without anyone being asked anything.
- **Round last, once.** Scale, then convert, then round the number actually shown. Rounding twice turns three cups into 720 ml when it is 710.
- **Fractions where the measure is fractional.** A US reader sees `4½ cups`, not `4.5 cups`, because that is how the cups in the drawer are marked. Metric stays whole numbers.
- **A tablespoon is a sixteenth of a cup of the same food**, so one Cup Weight yields every volume measure of that Food by arithmetic: `2 tbsp butter` is about 28 g from the single figure 227.
- **Cup Weight is instance-local and never travels.** Like nutrition, it is a property of this instance's Food ([#11](https://github.com/BattermanZ/Kamosu/issues/11) made Foods instance-wide); a share bundle carries Ingredient Lines and Readings, not the receiving instance's table. Anyone may set or correct one, and an override always beats the shipped figure.
- **Parity needs nothing.** The conversion is computed in the Core from the Reading and the acting Person's Reading Measures, beneath both Doors ([ADR 0001](./0001-both-doors-generated-from-one-operation-catalogue.md)), so an agent asked *how much flour in grams* answers correctly for free.
- **Changing your Reading Measures stores nothing and makes no Version.** It is a preference, like Reading Language.
- **The Share Link page converts nothing.** A stranger has no account and therefore no Reading Measures, so the page shows the recipe as written ([#14](https://github.com/BattermanZ/Kamosu/issues/14) made it plain server-rendered HTML). What the PDF does is [#24](https://github.com/BattermanZ/Kamosu/issues/24)'s to decide.
- **An unrecognised unit still scales.** `2 poignées` at 1.5× is 3 poignées; the quantity multiplies and the unit is untouched.
- **The Crouton importer needs an enum→unit mapping table**, which the closed convertible set now defines — input to [#26](https://github.com/BattermanZ/Kamosu/issues/26). `SECTION` is a control row, not a unit; `ITEM`, `CAN`, `BOTTLE`, `PACKET` and `PINCH` map to unconvertible units.
- **Shopping lists can now merge across units** — `1 cup milk` and `200 ml milk` are the same Food in one family. Whether they should is [#25](https://github.com/BattermanZ/Kamosu/issues/25)'s.
- **The shipped table is a curated list, not an ingest.** [#4](https://github.com/BattermanZ/Kamosu/issues/4) established USDA SR Legacy's `food_portion.csv` (CC0, 919 KB, 14,449 rows, gram weights 100% populated) as the only real source, and also that using it wholesale means parsing English portion prose from a free-text `modifier` column. Roughly twenty hand-checked staples covers the measured need; the file is a reference to check figures against, not a dependency.

## Amounts in a Step

Added by [#150](https://github.com/BattermanZ/Kamosu/issues/150), 25 September 2026, after a Bon Appétit recipe reached a metric reader with every Ingredient Line in grams and every Step still in pounds.

- **An amount written in a Step converts and scales exactly as an Ingredient Line does.** Same rounding, same *about*, same rule that nothing is shown where it would only repeat the text. Scaling and conversion stay one act: a cook making 8 servings who reads "about 905 g" in the list and "1 lb." in the step with nothing beside it has been handed a contradiction.
- **Every conversion is shown straight after what it converts**: "1 lb. (about 455 g) ground chicken", and the oven too, "preheat to 425° (about 220 °C)". Aurélien chose this over one line of figures beneath the Step, because a step with five amounts in it left the cook counting along a row of numbers to match them back, and then asked for the oven to follow the same rule rather than keep the line beneath it had before. The Step's text is still never rewritten; the screen draws each addition between its words, and the Core answers them as one list in the order written.
- **Only an amount with a Unit from the closed set counts.** A sentence is full of numbers that are not amounts — *28–35 minutes*, *step 3*, *turn 90 degrees* — and a convertible Unit right after the number is the one test that tells them apart. A range, `1–1½ cups`, reads as nothing in a Step. On an Ingredient Line it is the written amount, as below. Lengths are out: Kamosu converts no length anywhere.
- **The amount is joined to its Ingredient Line by the words after its Unit**: the same Reading-target join a Step's `uses` makes, taking the Food named soonest, so `1 cup panko` in a step borrows that line's Cup Weight and answers what the line answers, and `1 cup freshly sifted flour` is still flour. With no match it converts by volume, as an unread Food does.
- **A bare degree sign is an oven where only one dial fits.** American magazines print every oven as `425°`. Above 300 only Fahrenheit is possible; from 150 to 300 both dials have the number, and the recipe's own Readings decide (all customary is an American oven, all metric a metric one, spoons casting no vote); anything else is left alone. Below 150 is never an oven: the one bare degree sign in the Crouton export is a Thermomix's `100°`.
- **The Sheet is unchanged.** [ADR 0023](./0023-a-sheet-carries-the-recipe-not-the-library.md) prints written lines; whether it carries step conversions is a separate question.

## A range on an Ingredient Line

Added by [#167](https://github.com/BattermanZ/Kamosu/issues/167), 28 September 2026, after `2-3 basil leaves` reached production as a Food called *2-3 basil leaves* and `2 to 3 cloves garlic` as two of a Food called *to 3 cloves garlic*.

- **A range is the line's amount, kept as written, and worth nothing.** `2-3`, `1–1½`, `2 to 3`, `1 or 2`, `2 ou 3` and `2 a 3` are read as the amount, and the Unit and Food after it are read as they are after any other amount. Two amounts joined by a hyphen, a dash or one of those words, with nothing between them, is the whole test.
- **Kamosu does no arithmetic on a range.** It is never scaled, never converted and never added up: the line gets no subordinate line, a Component line standing for a range of an inner recipe gets no factor, and the shopping row carries the Food with no number. Aurélien chose this over scaling `2-3` to `4-6` and summing ranges on the shopping list, because what went wrong was the Food, and range arithmetic would need rules for adding and converting a range that nobody had asked for.
