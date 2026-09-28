# A translation is a Branch

A recipe in another language is not another recipe and not a second copy of the text inside one recipe. It is a **Branch** of the same **Lineage**, carrying a **Language**.

Each Version on a translation Branch records, alongside its parent, the Version of the source Branch it **translates**. That pointer moves forward every time the translation is brought up to date, so how far behind a translation has fallen is arithmetic over current facts rather than a flag anyone maintains.

**Language** is a property of a Branch, may be **Unknown**, and is detected from the recipe's full text on save: set when absent, offered when it disagrees with an existing label, never changed silently.

**Food** and **Tag** — the two shared named things of [ADR 0002](./0002-the-written-ingredient-line-is-the-truth.md)'s model — hold a name per Language rather than being split per Language.

## Why

- **The machinery already existed.** [ADR 0004](./0004-one-lineage-many-branches.md) established that one Lineage holds many Branches, permanently, and that Kamosu never merges them. A French and an English rendering of one dish are the same recipe written differently, and nobody would ever want an edit in one automatically applied to the other. Translation is the never-merge model's most natural case, not an exception to it.
- **Cooking the dish counts as cooking the dish.** An Attempt belongs to the Lineage, not a Branch ([ADR 0005](./0005-an-attempt-is-a-version-you-cooked-but-did-not-keep.md)). Cook from the French text and the cook count, the ratings and the history are the same recipe's. Modelling translations as separate recipes would have scattered exactly what the Lineage exists to hold together.
- **The written line stays singular.** [ADR 0002](./0002-the-written-ingredient-line-is-the-truth.md) rests on an Ingredient Line being *the* line as written, and the thing a person edits. Carrying translations inside one recipe would have made every line a bundle of lines-per-language — weakening the rule the whole model rests on, and charging that complexity to the monolingual majority.
- **Staleness becomes honest and free.** The translation's newest Version says it translates E10; the source Branch has reached E11; the translation is one behind. Nothing to remember, nothing to mark, and it travels with the recipe rather than living in one instance's head.
- **Which language a recipe was originally in never has to be declared.** The original is the Branch that translates nothing. Computed, like the Branch Point.

## Considered options

- **One recipe carrying its translations**: a single Recipe holding a French and an English rendering of every field. Rejected because it adds a language dimension to every recipe including the ~90% that will only ever have one, and because it contradicts ADR 0002 by making an Ingredient Line plural.
- **Separate recipes, linked**: two Lineages with a "related" pointer. Rejected because it splits Attempts, ratings, cook counts and history across two identities, and leaves "which one is the real one" with no answer.
- **A translation is a Branch** (chosen).

## Consequences

- **A share carries your Branch plus its translations**, with a per-share toggle to send only the language you are reading. A translation is work done on *this* recipe, so withholding it by default makes the recipient's instance worse for no gain. Branches belonging to other people — a diverged copy you were sent — never ride along.
- **Stale translations travel, labelled.** A bundle carries the fact that its newest French Version translates E10 while the English is at E11, and the recipient's instance shows the same badge the sharer sees. A translation one Version behind is still a translation; hiding the gap would make the bundle less truthful than the app.
- **The recipe list shows one card per Lineage**, rendered in the reader's Language, falling back to whichever Language exists and marking that it did. A Language preference must never hide a recipe from its owner's own cookbook.
- **Unknown is a permanent, unremarkable state.** A recipe that is honestly bilingual — a French section header under an English title occurs in the real corpus — carries no Language, shows to every reader regardless of preference, and can neither be a translation nor have translations. No detector can be right about a recipe that is genuinely both, so Kamosu does not pretend.
- **Changing a Language makes a Version.** Once recipes travel between instances, a label alterable without a trace would be a hole in an otherwise append-only history.
- **Foods and Tags are named per Language, not split by it.** Shopping lists merge "2 cups flour" and "200 g de farine" into one line in the reader's Language; nutrition is matched once, which matters because the matching is the expensive part. CIQUAL ships French and English names for all 3,185 foods, so two of v1's three Languages are seeded free; Spanish names start empty and fill in through use.
- **Kamosu ships no translation engine.** Agents are first-class peers under one Catalogue ([ADR 0001](./0001-both-doors-generated-from-one-operation-catalogue.md)), and creating a Branch is an ordinary Operation, so an agent asked to translate a recipe writes the Branch — no API key, no bundled model, no network call, nothing to keep current. The agent acts under the person's own Credential and is a scribe, not an author.
- **Kamosu's own vocabulary is not translated; its interface is.** Operation names and descriptions stay in one language so agent behaviour never depends on who is signed in — Parity concerns which Operations exist, not what they are called. Interface strings are externalised from the first screen, because retrofitting means touching every screen.
- **Reading Language is held on the account**, seeded from the browser on first sign-in, because the things that need it — shopping-list Food names, Vault publication — run where no browser exists.
- **A Vault publishes one note per Branch, cross-linked.** [ADR 0003](./0003-the-database-is-the-truth-the-vault-is-a-published-copy.md) requires a Vault to be lossless, so publishing only the reader's Language is not available. Two linked notes is also what a person would have written by hand, which is the Obsidian and Hatchdoor interop the format was chosen for. The sidecar's Lineage id reassembles them into one recipe on Import; a note whose sidecar is missing imports as a new Lineage, because guessing would be worse.
- **v1 ships French, English and Spanish** interface translations; a self-hoster adds a fourth by dropping in a file.
