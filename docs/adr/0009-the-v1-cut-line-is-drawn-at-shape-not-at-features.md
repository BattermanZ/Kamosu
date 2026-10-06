# The v1 cut line is drawn at shape, not at features

v1 is **the version Aurélien switches to** — done the day Crouton is deleted from the phone, not the day Kamosu does one thing Crouton cannot.

What goes in is decided by a single rule. Every candidate is one of two kinds:

- **It changes what is stored** — versioning, Attempts, Lineage, Kitchens, translations, the Ingredient Line, the share bundle's format. Adding one of these later means rewriting recipes people have already written.
- **It is computed from what is stored** — a search index, a nutrition figure, a unit conversion, a Markdown rendering, a suggestion. Adding one of these later costs exactly what it costs now: nothing is stranded, and it switches on over the library that already exists.

**v1 ships every stored-shape decision in full, and only as much computed-on-top as the switch test demands.**

## Why

- **The switch test is the only measure that produces daily use, and daily use is the point of a first version.** Shipping a beachhead — one thing Crouton cannot do, with both apps open — sounds pragmatic and is a trap: two cookbooks means neither is *the* cookbook, the import never gets finished because nothing forces it, and the feedback that tells you what is wrong never arrives.
- **It makes scope decidable rather than a matter of taste.** "Would its absence send you back to Crouton?" is a question Aurélien can answer about his own hands. "Is this important?" is a question nobody can answer.
- **Retrofit cost is the real asymmetry, and it is wildly uneven.** The eleven decisions already on this map are almost entirely about stored shape, which is why they were worth making before any code exists. Nearly everything else is derived, and derived things are cheap forever.
- **It was already the working rule.** The map had been applying it for eleven tickets without saying so. Naming it is what lets it be applied to the remaining wish-list consistently instead of item by item on instinct.

## Considered options

- **Cut by feature importance** — rank the wish-list, draw a line, ship the top half. Rejected: it is a taste contest with no tie-breaker, and it reliably trades away foundations for visible features, which is the specific mistake that forces a rebuild.
- **Cut by effort** — ship what is cheap, defer what is expensive. Rejected for the same reason in reverse: it defers exactly the expensive-to-retrofit things, because stored-shape work is the least visible and often the most costly up front.
- **Ship a beachhead alongside Crouton** and grow. Rejected, as above.
- **Cut at the boundary between stored and computed, judged against the switch test** (chosen).

## Consequences

- **The rule has teeth in both directions, which is the evidence it is a rule and not a rationalisation.** It *kept* semantic search — a pure add-on by any feature ranking — because [ADR 0006](./0006-a-translation-is-a-branch.md) already promised that searching "chocolate" finds the French Branch of a Lineage you hold, and keyword search cannot do that at any price; the promise is only real if the multilingual index ships with it. It *removed* the Markdown Vault, decided in full detail by [ADR 0003](./0003-the-database-is-the-truth-the-vault-is-a-published-copy.md), because a Vault is written out of the database and never read back, so switching it on later regenerates the whole library, history included, in one pass.
- **Every deferral carries a written foundation requirement, or an explicit statement that it has none.** "Free to add later" is a claim that has to be checked, not assumed. Two such requirements are binding on work still to be done:
  - **The share bundle must be a readable folder** — human Markdown plus a `.kamosu/` sidecar — never an opaque machine blob, so the deferred Vault is later "many of those, written to a directory instead of zipped". A closed format in v1 would force v2 to invent the note rendering from scratch. Constrains [#17](https://github.com/BattermanZ/Kamosu/issues/17).
  - **The unit set must be open, never a closed enum.** Crouton's 15-value Anglo list cannot express what is already in the real 86-recipe library.
- **Deferring is not free by default, and two candidates failed the test.** Cross-instance share bundles were proposed for deferral on the premise that no counterparty exists; the premise was false — one friend will install Kamosu — so [#17](https://github.com/BattermanZ/Kamosu/issues/17), [#18](https://github.com/BattermanZ/Kamosu/issues/18) and [#19](https://github.com/BattermanZ/Kamosu/issues/19) are all required before the spec is finished, and divergence stops being theoretical. Metric↔imperial conversion was proposed for deferral as arithmetic over a Reading; it stays, because much of what gets imported is imperial and a recipe that reads "1 cup" is unusable in the exact moment it is needed.
- **Agents are the primary interface, not a second door.** The MCP door ships fully featured, including the standard long-running-task extension over the Jobs of [ADR 0001](./0001-both-doors-generated-from-one-operation-catalogue.md). The web UI is a peer surface rather than *the* product — which is a constraint on [#13](https://github.com/BattermanZ/Kamosu/issues/13) and [#14](https://github.com/BattermanZ/Kamosu/issues/14), and which makes the RPC-shaped web API that ADR 0001 accepted as a cost considerably less costly.
- **Conversion has a stated limit rather than a silent one.** Within a family it is arithmetic and always correct — ounces↔grams, cups↔millilitres, °F↔°C. Across families it is not, because a cup of flour depends on what is in the cup; [#4](https://github.com/BattermanZ/Kamosu/issues/4) found Tandoor needed a hand-built per-food weight table and still shows "couldn't work this out" flags. So Kamosu converts within a family always, across families only where a Food has a known density, and otherwise shows the line as written — which [ADR 0002](./0002-the-written-ingredient-line-is-the-truth.md) guarantees is always there to fall back on.
- **Nutrition in v1 is a number a person or an importer typed**, per recipe, flagged per-100 g or per-serving. The bundled CIQUAL table and per-Food links land later without touching a recipe, because [#4](https://github.com/BattermanZ/Kamosu/issues/4) established that the matching is the cost and that three of four mature apps skip automatic nutrition entirely.
- **This ADR governs scope, not vocabulary.** `GLOSSARY.md` is unchanged by it, and no term in the domain model means anything different because of it.
