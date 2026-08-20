# Kamosu

A self-hosted cookbook. People and software agents are equal users of it: everything a person can do to a recipe, an agent can do too.

## Language

### The shape of the system

**Operation**:
One thing Kamosu can be asked to do, defined once — its name, what it takes in, what it gives back, and the permission it requires. The unit of everything Kamosu offers.
_Avoid_: Endpoint, tool, command, action, API method

**Catalogue**:
The complete list of Operations. There is exactly one, and it is the sole source of what Kamosu can do.
_Avoid_: Registry, API surface, tool list, schema

**Core**:
Where Operations are carried out and where permission is checked. It is reached only through a Door and knows nothing about how a request arrived.
_Avoid_: Backend, service layer, business logic, engine

**Door**:
A way in to the Catalogue. Kamosu has two — the **web door** and the **MCP door** — and both offer the entire Catalogue. A Door translates between the outside world and the Core; it never decides what may be done.
_Avoid_: Adapter, interface, transport, front end, client

**Parity**:
The guarantee that both Doors offer the same Operations. It holds because the Doors are built from the Catalogue rather than written separately, so an Operation cannot exist at one Door alone.
_Avoid_: Feature parity, equivalence

### Doing and permission

**Credential**:
What a request presents to prove who is acting. Every Door resolves one before the Core runs anything, so the same permissions apply whether a person or an agent is asking. Obtaining a Credential is not an Operation.
_Avoid_: Token, session, auth, API key, login

**Immediate Operation**:
An Operation that answers within a single request.
_Avoid_: Sync operation

**Job**:
An Operation too slow to answer within a single request — importing a library, fetching a recipe from a website, producing a PDF. Asking for one returns a job id at once; its progress is then read through ordinary Operations.
_Avoid_: Task, background job, async operation, queue item

### Getting recipes in and out

**Import**:
Bringing recipes into Kamosu from elsewhere — another app's export, or a web page. An Operation like any other, available at both Doors.
_Avoid_: Migration, sync, ingest

**Export**:
Producing recipes in a form usable outside Kamosu. An Operation like any other.
_Avoid_: Download, backup, share

**Vault**:
An optional folder of Markdown notes and images in which one person's recipes are published, kept up to date by Kamosu and optionally backed by Git. Kamosu writes a Vault and never reads it back; changes made to the files return only through Import. A Vault is a publication with an owner, a destination and a scope — not where recipes are stored.
_Avoid_: Mirror, source of truth, repository, notes folder

**Backup**:
An Operation producing a single archive from which a Kamosu instance can be restored — a consistent copy of the database together with the photographs. A Vault is not a Backup.
_Avoid_: Export, dump, snapshot

### What a recipe is made of

**Recipe**:
A dish as recorded in Kamosu. It requires only a title; a bare name and a link is a real recipe, not an unfinished one. Every Recipe belongs to a Lineage and is held as a Branch of it, at some Version.
_Avoid_: Dish, entry, card

**Ingredient**:
One row of a recipe's ingredient list. Its truth is its Ingredient Line; a Reading may sit over it.
_Avoid_: Item, component

**Ingredient Line**:
The ingredient exactly as written or imported — `"2 poignées de farine, environ"`. Always preserved, always what is displayed, and the thing a person edits.
_Avoid_: Original text, raw text, free text, display string

**Reading**:
An optional interpretation of an Ingredient Line into its parts — how much, in what unit, of what Food. It may be absent, partial or mistaken; anything built on it degrades politely rather than failing.
_Avoid_: Parse, structured ingredient, decomposition, breakdown

**Food**:
An edible thing Kamosu knows about, pointed at by the Readings that mention it and shared across every recipe that uses it. Created automatically from whatever word a Reading found. The one place a food's other names and its nutrition are recorded.
_Avoid_: Item, product, ingredient (an Ingredient is a row in a recipe; a Food is the thing itself)

**Step**:
One instruction in a recipe, in order. Its truth is its text. It may also carry a photo of what the step should look like, and links to the Ingredients it uses. A duration or a temperature is read out of that text, never typed beside it.
_Avoid_: Instruction, direction, method, procedure

**Section**:
A named part of a recipe — "For the sauce", "Assemble" — holding some of its Ingredients or some of its Steps. Optional: most recipes have none and are a single flat list.
_Avoid_: Group, block, heading, part, component

**Yield**:
What a recipe makes — an amount and what it is an amount of: "4 servings", "24 cookies", "1.5 litres". Optional. Scaling a recipe means changing this amount, which multiplies every Reading in step.
_Avoid_: Servings, serves, portions, makes

**Note**:
Free text about the recipe as written — serving suggestions, substitutions, "this doubles well". True whether or not anyone has ever cooked it. A record of one particular cooking is not a Note.
_Avoid_: Comment, remark, tip, annotation

**Tag**:
A word a recipe can be described by, kept once in a single shared list and pointed at by every recipe that uses it — so renaming or merging one reaches every recipe at once. Flat: tags have no hierarchy.
_Avoid_: Keyword, category, label, folder

**Source**:
Where a recipe came from — free text attribution such as "Marmiton" or "Mum's ring binder, p.40", with an optional link. A hint that two recipes are related, never a way of identifying one.
_Avoid_: URL, link, origin, attribution, provenance

**Prep Time**:
How long a recipe needs hands-on before cooking begins, in whole minutes. Optional.
_Avoid_: Active time, preparation

**Cook Time**:
How long a recipe takes from the end of prep until it is ready, in whole minutes — including resting, proving, marinating and chilling. Cooking is not only what happens over heat. Optional.
_Avoid_: Bake time, waiting time, rest time, total time

**Main Photo**:
The single image that stands for a recipe wherever it is listed. Optional. Distinct from a Step's photo, which shows one moment in the cooking.
_Avoid_: Hero image, cover, thumbnail, picture

### Identity and history

**Lineage**:
A recipe's identity in the world. One id, minted once when the recipe is created, carried by every copy of it on every instance anywhere. It never changes and is never joined to another Lineage — two recipes that grew up separately are related, not the same recipe.
_Avoid_: Recipe id, UUID, slug, canonical id, family

**Version**:
One saved state of a recipe, named by a fingerprint of its own content, recording the Version it came from and optionally a name and a line saying what changed and why. Every save makes one; none is ever rewritten or deleted. Two holders of the identical state hold the same Version without having communicated.
_Avoid_: Revision, snapshot, commit, edit, history entry

**Branch**:
One holder's line of Versions within a Lineage. What a person has on screen is a Branch at its latest Version. An instance may hold more than one Branch of the same Lineage — that is divergence, a normal and permanent state, never something to reconcile.
_Avoid_: Copy, fork, variant, clone

**Branch Point**:
The last Version two Branches share — a fact computed by walking both parent chains until they meet, not something anyone declares. What "you diverged here" means.
_Avoid_: Common ancestor, base, merge base, split
