# A Sheet carries the recipe, not the library

A **Sheet** is one recipe rendered for paper — the dead, printable form a **Share Link** offers beside the **Bundle** ([#11](https://github.com/BattermanZ/Kamosu/issues/11)), produced by anyone who can see the recipe, stranger included, as a **Job**. It is the only thing Kamosu makes that somebody holds in their hands, and the only one with no way back in.

One rule decides its contents:

> **A Sheet carries the recipe, not the library.** A fact about the *dish* goes on the page. A fact about *how Kamosu files it* does not.

- **On the page**: Title, Main Photo, Yield, Prep Time, Cook Time, Source with its link written out as text, the Ingredient Lines exactly as written, Sections, the Steps, Notes, and the **Components** unfolded.
- **Off the page**: Tags, Attempts, the **Thread** and every past Version, Related Recipes, **Ghosts** and everything else about divergence.
- **Step photos are off**, though the **Main Photo** stays. A photograph per step triples a one-page recipe for something most steps have not got; the Main Photo answers *is this the dish I meant* from across the counter.
- **Readings are not printed.** The written line is the line. Two narrow exceptions, both earning their ink: a **scaled** Sheet prints the scaled amount beneath the written line marked *about*, and a **Component**'s line carries the page number of its block.

**A Sheet is one Branch** — the one on screen. The Share Link page already shows a recipe's **Translations** and picks one for a visitor the way Kamosu picks one for a reader: their Language if a Branch is in it, otherwise whichever exists, marked. The button prints that choice and follows it if they switch. An unfolded **Component** follows the same rule and prints in whatever Language it exists in, marked, because it names a **Lineage** and not a Branch.

**A Sheet prints what is on screen, scaling included** — there is no print dialog and no serving box. A line Kamosu could not read gains nothing, no guess and no mark; a header says *Scaled to 8 servings — as written, makes 4*.

**Components are appended, parent first**, in order of first mention, each under its own heading and already scaled by comparing the **Reading** against that recipe's **Yield**. Where the repeat-guard of [ADR 0008](./0008-a-composed-recipe-is-an-ingredient.md) stops, the page says so in plain words where the block would have been.

**Every page carries a thin footer** — title, *page 2 of 3*, the date printed — and the recipe ends with a small provenance block: the **Version**'s name and the date it was written, the **Hand** that wrote it as plain text with no claim attached, the Source, the Share Link where there was one, and the fingerprint in the smallest type on the page, shown and never explained.

**A Sheet is set in Typst**, embedded as a library with its template and fonts baked into the binary. **Page size follows the reader** — *US measures* in **Reading Measures** prints Letter, anything else prints A4; a stranger's browser locale decides the same way.

**A Sheet carries a print-sized rendering of a Photograph, not the Photograph itself** — one more size in the small fixed set that **Display Copy** already describes. **Export** and **Backup** are unchanged and still carry the Photograph.

## Why

- **The rule is the deliverable; the list is a consequence of it.** Without it every field added to a recipe reopens a taste argument about whether it prints. With it the answer is read off, and the next person does not have to ask.
- **Tags fail the rule by their own definition.** The glossary calls a Tag how a **Kitchen** files a recipe, kept in that Kitchen's own list, and says outright that what one Kitchen means by *quick* is its own business. A word meaningful only inside one library is the purest case of what a Sheet leaves behind.
- **Paper is the worst place to rewrite what a cook wrote.** [ADR 0002](./0002-the-written-ingredient-line-is-the-truth.md) keeps the Ingredient Line verbatim because a **Reading** may be absent, partial or mistaken. On screen a bad Reading is corrected in a tap; on a Sheet it is permanent, unreportable and unfixable, and the reader cannot tell it happened. So the Sheet prints the line and adds beneath it, never instead of it.
- **The 28% is why silence beats a guess.** [#5](https://github.com/BattermanZ/Kamosu/issues/5) found 28% of Aurélien's 599 real Ingredient Lines carry no quantity at all. On a scaled Sheet those lines can gain nothing true, so they gain nothing — and the page is honest by staying quiet rather than marking them as failures.
- **Printing what is on screen invents no second control.** A servings box in a print dialog is a second place where scaling lives, free to disagree with the first, in an artefact where disagreement cannot be noticed. The same reasoning settles Language: the page already made that choice, so the button inherits it.
- **Stacked or columned Languages make a worse sheet for both readers.** Two columns halve the type on the thing being squinted at with wet hands; stacked pages charge the French cook for English ones. The Share Link page already solves the picking.
- **Inlining a Component destroys the reading order the Sheet exists for.** A method is a sequence followed with busy hands. Splicing eight steps of dough into the middle of a pizza puts the pizza's step 3 on page 2 and loses the cook's place. Appending keeps both recipes whole, in order and cookable — the instinct [ADR 0014](./0014-a-divergence-is-two-recipes-not-a-difference.md) followed when it refused a difference screen and showed two recipes.
- **The parent goes first because the Sheet is about the parent.** It owns the title and the photograph. That the dough is made first is a fact about cooking, not about which recipe the page is for.
- **The provenance block is the one deliberate exception, and it is cheap.** A Sheet found in a drawer six months later is otherwise anonymous — no way to tell whether it predates the fix to the roux, and nobody to ask. A grey line costs nothing and makes the sheet comparable to what the recipe says today.
- **The Hand is printed bare because [ADR 0015](./0015-a-name-is-not-identification-it-is-a-reminder.md) says it is a reminder.** A name, not a check. So it appears as handwriting on a recipe card appears — no badge, no checkmark, no *by*, none of which Kamosu could stand behind.
- **A browser was rejected for the shape it breaks.** Chromium is roughly 400 MB in the container, spikes RAM under load, and is a full web browser inside an app strangers are meant to self-host at home. [ADR 0001](./0001-both-doors-generated-from-one-operation-catalogue.md) chose one binary and one core, and [#14](https://github.com/BattermanZ/Kamosu/issues/14) chose `adapter-static` — there is no server-rendered page to point a browser at, so one would be stood up purely to be screenshotted.
- **A low-level PDF crate is building a typesetting engine to avoid learning one.** Page breaks that do not strand a heading, an ingredient list kept off a boundary, *page 2 of 3* — that is the hard part, and it is precisely what Typst already does. It is also where the standing *do not reinvent the wheel* preference points.
- **The wrong page size still prints.** The template reflows rather than pinning to a fixed page, so a bad guess costs odd margins and no content. Always-A4 would have been a small unkindness to the self-hosters the destination invites, for no saving: in Typst the page size is a parameter, not a second template.
- **Twelve megapixels cannot reach paper.** A Main Photo occupies about 8 cm of a Sheet, roughly one megapixel at print resolution. Embedding the full Photograph makes a 5 MB download generated on demand by strangers on a home server. The distinction that matters is archival versus rendering, and a Sheet is the deadest rendering Kamosu makes — nothing will ever be re-derived from it. Too much resolution wastes bandwidth forever and invisibly; too little is visible at once and repaired by regenerating a page that took a second to make.

## Considered options

- **Print the screen layout as-is.** Rejected: the two are different documents — the Sheet drops Tags, Attempts, the Thread, step photos and Readings, and gains a provenance block. There was no shared layout to reuse.
- **Stack every Language in one Sheet, or set them in parallel columns.** Rejected: worse for both readers, and the picking is already solved upstream.
- **Offer a serving count in a print dialog.** Rejected: a second scaling control that can drift from the page's, in an artefact where drift is invisible.
- **Rewrite scaled Ingredient Lines in place.** Rejected: [ADR 0002](./0002-the-written-ingredient-line-is-the-truth.md), and permanently so on paper.
- **Unfold Components inline.** Rejected: destroys the parent's reading order.
- **Put Components before the parent, since they are made first.** Rejected: buries the recipe the Sheet is about behind its ingredient.
- **Omit the provenance block, since a Sheet is a dead end.** Rejected: a dead end that cannot say where it came from is worse than one that can.
- **Headless Chromium rendering print CSS.** Rejected: 400 MB, a browser's attack surface, and it breaks the one-binary shape.
- **`printpdf` / `pdf-writer` and hand-rolled layout.** Rejected: hand-writing pagination.
- **Always A4.** Rejected: gratuitously unkind to non-metric self-hosters, and saves nothing.
- **Carry the full Photograph, per the Display Copy rule as written.** Rejected, and the rule amended: right for Export and Backup, wrong for a rendering.
- **Typst as a library, one Branch, printed as it stands on screen** (chosen).

## Consequences

- **The glossary's Display Copy entry is amended.** *Export, the PDF and a Backup carry the Photograph itself* becomes Export and Backup only; a Sheet carries a print-sized rendering. Export and Backup are untouched.
- **Two layouts exist and may drift** — Svelte for the screen, Typst for the page. Accepted, because they are genuinely different documents. The gain is that the template is a plain text file, so the printed page is redesigned without touching Rust.
- **Print design is a real design surface, and it is deferred like the rest.** [#13](https://github.com/BattermanZ/Kamosu/issues/13) parked the visual identity until the build; the Sheet's typography parks with it. What is spec here is what the page carries and in what order, never its palette or its type.
- **A stranger on a public Share Link can trigger a Job on somebody's home server, unauthenticated.** Typst compiles a recipe in tens of milliseconds and the print-sized photograph keeps the output small, so the exposure is modest — but it is real, and it is about Share Links triggering Jobs at all rather than about Sheets. Left to [#27](https://github.com/BattermanZ/Kamosu/issues/27).
- **A Sheet is still a Job even though it need not be.** Nothing here made it slow; keeping it a Job costs nothing and keeps a Component-heavy recipe with a dozen unfolded blocks from timing out a request.
- **Naming it opened a door v2 may use.** *Sheet* is the printed recipe, not the file format, so a plain-text or ePub rendering of the same page inherits the rule instead of reopening it.
