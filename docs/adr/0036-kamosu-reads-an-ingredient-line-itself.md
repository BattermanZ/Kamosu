# Kamosu reads an Ingredient Line itself

**Kamosu reads its own Ingredient Lines, in its own process, off the Unit vocabulary and amount reader it already owns.** There is no `ingredient-parser-nlp` subprocess worker, no Python in the image, and no vendored parsing crate.

Reading a line is one job: deciding where the number ends, where the **Unit** ends and where the **Food** begins. What the number is worth and what the Unit is were already answered by `src/units.rs`, which spells every convertible Unit in all three interface Languages. `src/reading.rs` is the split and nothing else.

This **reverses** [#71](https://github.com/BattermanZ/Kamosu/issues/71)'s first acceptance criterion and the line in the spec ([#32](https://github.com/BattermanZ/Kamosu/issues/32)) deferring ingredient parsing to a subprocess worker. It leaves [ADR 0021](./0021-a-reading-is-kamosus-reading-not-the-recipe.md) standing exactly as written: that ADR's argument is that *a parser is a moving part and must stay outside the fingerprint*, which is true of a parser in this process as much as one in another — more so, since this one is now ours to change.

## Why

- **The libraries lost on the real corpus.** All 863 Ingredient Lines of the 86-recipe Crouton export, scored against Crouton's own amount/unit/name split:

  | parser | amount | unit | name | outright failures |
  |---|---|---|---|---|
  | Python `ingredient-parser-nlp` 2.7.0 | 98.5% | 94.6% | 56.9% | 0.9% |
  | Rust `ingredient` crate 0.3.0 | 99.3% | 95.0% | **62.0%** | 2.4% |
  | Kamosu, on its own tables | **100%** | **96.4%** | 60.1% | 1.6% |

  The *name* column is low for all three because the ground truth is not clean: Crouton files `2 cloves minced garlic` under the name *"cloves minced garlic"*, so a parser answering *"garlic"* is marked wrong for being right. It compares the three rows and is not an absolute score. The unit column understates Kamosu for the same reason — **there is no line in the corpus where the export names a Unit and Kamosu reads none**; every disagreement is Kamosu reading `cloves`, `dash` or `slices` where Crouton, having no word for them, filed a bare count.

- **Both libraries carry an English-only vocabulary of Units, and Kamosu already has a trilingual one.** This is the whole result. `20 cl de crème fraîche` is read correctly here and by neither library: they do not know centilitres, and they take the article into the Food, leaving `de crème fraîche` — a Food that will never match the `crème fraîche` beside it. Adopting either would have meant two vocabularies of Units in one program, the worse one deciding where a Food begins.

- **It removes a limitation rather than accepting one.** The subprocess route would have read 77 of Aurélien's 86 recipes, because `ingredient-parser-nlp` supports `lang="en"` and nothing else. This reads all 86. Kamosu's own corpus is 9-in-86 non-English and its interface is in three Languages; a reader that works in one of them was never the right shape.

- **It dissolves a deployment problem instead of paying for one.** [ADR 0028](./0028-kamosu-is-one-file-and-one-directory.md) promises one distroless image holding the binary and nothing else. A Python worker needed roughly 150 MB of interpreter and packages inside that image — doubling it — or a second container, or a download-on-demand machinery in the shape of [ADR 0029](./0029-kamosu-ships-no-model.md). Every one of those was a real cost to every person who installs Kamosu, paid for a parser that was not better. In-process Rust costs nothing and runs at 0.04 ms a line against Python's 1.9.

- **A reader is a moving part, and the moving part should be the one we can move.** ADR 0021 already established that a Reading must sit outside the fingerprint precisely because parsers change. Given that, the deciding question is which parser we can improve when the corpus shows it something new — and the answer is not a crate whose last release was April 2023, nor a Python package behind a process boundary.

## Considered options

- **A Python `ingredient-parser-nlp` subprocess worker**, as #71 and the spec said. Rejected on the measurement above, and on what it would have done to the image.
- **Vendoring the Rust `ingredient` crate** (MIT, 1,636 lines). Genuinely competitive — it holds the best food-name column of the three — and rejected because its unit tables are English-only and would sit beside `units.rs` doing the same job worse. Eighteen times the code to maintain, for a vocabulary Kamosu would have to keep overriding.
- **Both, the crate as a fallback where Kamosu reads no name.** Rejected: two parsers that disagree about one line is a bug nobody can reason about later, and the fallback would fire on 1.6% of lines.
- **Kamosu's own reader on Kamosu's own tables** (chosen).

## Consequences

- **A Food is one thing to buy, so a sentence is not read at all.** The real corpus keeps whole cooking steps inside ingredient entries; past six words `src/reading.rs` declines rather than minting a Food nothing will ever match. About 3% of the corpus is left unread this way, which is the honest answer — an unread line is a working line ([ADR 0002](./0002-the-written-ingredient-line-is-the-truth.md)).
- **Kamosu reads the lines a write puts there**, at `create_recipe`, at Import and at each save — and only the lines that save actually wrote. A line nobody edited keeps what it has, **including nothing**, so a Reading somebody cleared on purpose is never resurrected ([ADR 0003](./0003-the-database-is-the-truth-the-vault-is-a-published-copy.md)).
- **`read_ingredient_lines` is the Job for a library that predates the reader**, and for the day this module gets better. It is safe to repeat because a Reading is derived from the written line, and it overwrites nothing.
- **The accuracy figures are a test, not a claim.** `tests/reading_corpus.rs` re-measures all of it against the real export and fails on a regression. The floors are set under what was measured, so an improvement never breaks the build.
- **This module will be wrong about some lines forever, and that is survivable by construction.** ADR 0002 put the truth in the written line and ADR 0021 kept the Reading out of the fingerprint precisely so that being wrong here costs a correction and never a divergence.
