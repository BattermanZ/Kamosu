# Kamosu ships no model

**Meaning Search is optional, and Kamosu ships no weights.** An Operator who wants it accepts the model's terms, and Kamosu downloads it. An Operator who does not gets an instance that installs with no network, holds no third party's licence, and searches by words.

- The model is **EmbeddingGemma-300M**, downloaded on demand into `/data/model/`, where it survives upgrades without a mount of its own.
- **Accepting the terms is an Operation like any other**, at both Doors, and the acceptance record keeps the **Hand** that accepted and whether it arrived by login or by **Access Key**.
- **Searching is one Operation whether or not the model is there.** The **Catalogue** cannot change shape per instance, or an agent listing what Kamosu can do would get a different answer on every server.
- With the model absent, [#29](https://github.com/BattermanZ/Kamosu/issues/29)'s promise that *nothing found is not an empty screen* is met differently: there is no nearest neighbour, so the screen says nothing matched, offers *add* and *import*, and **offers to turn Meaning Search on — once, and never again to an Operator who declined**.

## Why

- **The licence was never a modelling problem.** EmbeddingGemma is the best model in its class and the only one with a licence that forbids shipping it. Chasing a permissive substitute meant accepting a worse model to dodge a screen; shipping Gemma meant redistributing weights under Google's terms. Not shipping any model dissolves both: **the person who accepts the terms is the person the terms are about.**
- **The evidence said the substitutes were not close enough to buy the simplicity.** On Hatchdoor's own 125-query evaluation, EmbeddingGemma at **4-bit** reached **0.915** Recall@5 in **547 MB** of memory, rising to **0.958** once tuned. Snowflake's Arctic Embed M v2.0 — Apache-2.0 and the strongest redistributable candidate — reached **0.907** at **fp32**, needing **3119 MB** and a 1.2 GB weight file to do it. It had every advantage of precision and did not win, so quantising it to something shippable could only make it worse. `multilingual-e5-small` was weaker still and three years old (June 2023, on a 2020 backbone), scoring **55.5** on MMTEB against EmbeddingGemma's **61.15**.
- **Optional costs nothing in quality and buys a great deal in shape.** The image stays small for everyone, including the many people who will never turn this on. Installing on a machine with no internet works completely. First run is not interrupted by a legal agreement — which matters, because [#11](https://github.com/BattermanZ/Kamosu/issues/11) made first run the delicate moment where the **Operator** is minted with no email and no signup.
- **The words half of search is a real product on its own.** [#29](https://github.com/BattermanZ/Kamosu/issues/29) put **exact title first** because most searching is navigation. Crouton, the app v1 has to beat ([#12](https://github.com/BattermanZ/Kamosu/issues/12)), has no meaning search at all. An instance without the model is behind Kamosu, not behind the thing Aurélien uses today.
- **An agent may accept the terms, because the alternative is a wall Kamosu cannot build.** A **Credential** always names a **Person**; an agent acts *as* that Person, and minting an **Access Key** for it was already the act of authorising that. A web-door-only carve-out would mean "a browser sent this", which an agent driving a browser satisfies — it would stop nothing and would put the first hole in [ADR 0001](./0001-both-doors-generated-from-one-operation-catalogue.md)'s guarantee that Parity is a fact of assembly. The house rule holds instead: [ADR 0015](./0015-a-name-is-not-identification-it-is-a-reminder.md) never verifies a **Hand** and says so; [#11](https://github.com/BattermanZ/Kamosu/issues/11) calls the Operator boundary a courtesy and says so. So Kamosu records who accepted and does not pretend to check who they were.
- **A read-only Access Key cannot do it**, by construction, because accepting is a write. No new guard was needed.
- **The offer belongs on the empty result.** A settings screen is where features go to be undiscovered. The moment a search returns nothing is the moment the person can see exactly what they are missing — the same reasoning that puts the public address question at the first **Share Link** rather than in a configuration file.

## Considered options

- **Ship a permissive model in the image** (Arctic M v2.0, or `multilingual-e5-small`). Rejected on the numbers above: it forces every installation to carry 118–311 MB for a model that loses to the one we can have for free.
- **Ship EmbeddingGemma in the image.** Rejected: redistributing the weights puts Google's terms on everyone who pulls the image, which is precisely the obligation Hatchdoor's terms-acceptance machinery exists to discharge.
- **Ship a permissive model, and allow an Operator to point at a downloaded better one.** Rejected: two model paths, two loading routes and two support stories, to serve a hypothetical Operator.
- **Make Meaning Search a different Operation from word search.** Rejected: the Catalogue is the sole source of what Kamosu can do, and one that varies per instance breaks what the MCP door is for.
- **Reserve accepting the terms to the web door.** Rejected: unenforceable, and the first exception to Parity.

## Consequences

- **v1 ships an instance whose flagship search is off until someone turns it on.** Deliberate. Optional is not secondary: Kamosu asks clearly, once, where it is earned.
- **The model choice remains a dial.** The index is derived from the database ([ADR 0003](./0003-the-database-is-the-truth-the-vault-is-a-published-copy.md)), so changing model is a re-index at startup, not a migration. Following [#29](https://github.com/BattermanZ/Kamosu/issues/29)'s precedent exactly — *the promise is spec, the chunking is a dial* — **which model, at which precision, is measured against the real 86 recipes and not argued now**. Every figure above is English or benchmark-wide; nobody has measured any of them on French recipe text, and Aurélien's library is 9-in-86 non-English with the interface in French, English and Spanish.
- **A model that arrived is excluded from a Backup.** It is neither database nor Photograph, it is re-downloadable, and including it would put 200 MB of somebody else's weights in every archive.
- **An instance can lose its model and be fine.** Deleting `/data/model/` returns it to word search; nothing else notices.
