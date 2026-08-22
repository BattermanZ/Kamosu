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

### People and access

**Person**:
Someone with an account on this instance. Cooks in one or more Kitchens, keeps their own Attempts, mints their own Access Keys and reads Kamosu in their Reading Language. An agent is never a Person — it acts as one.
_Avoid_: User, member, profile, actor, account

**Kitchen**:
The circle a recipe is held by, and the only circle that may edit it. Every Branch belongs to exactly one Kitchen; a Person cooks in one or more, and a Person on their own is a Kitchen of one — there is no solo case sitting beside a shared one. Anyone may create one. It has a Name given by its creator, which any member may change, and a Nickname each member may set for themselves alone and which nobody else ever sees. Any member may invite another Person or remove one; the last member cannot be removed.
_Avoid_: Household, group, team, family, workspace, organisation, account

**Home Kitchen**:
The Kitchen a Person was created with. Where anything they write, import, receive or Copy is held unless they say otherwise — a question only ever put to someone who cooks in more than one.
_Avoid_: Default kitchen, primary group, personal space

**Visibility**:
How far a recipe can be seen, set by the Kitchen holding it: its own Kitchen, Chosen (named People and Kitchens), Everyone here, or Anyone with the link. Seeing a recipe means reading it, cooking from it and recording an Attempt against it — never editing it. Changing a recipe you can see means taking a Copy.
_Avoid_: Permission, access control, privacy level, sharing setting

**Copy**:
Taking a recipe you can see into your own Kitchen — a new Branch of the same Lineage, held by you from then on. What one recipe becoming two looks like inside a single instance, identical in every respect to receiving a share bundle from another server.
_Avoid_: Duplicate, fork, clone, save, import

**Share Link**:
The unguessable address of a recipe whose Visibility is *Anyone with the link*. One per recipe, never expiring, freely passed on. Turning sharing off ends it; turning it back on makes a new one, so a withdrawn link stays dead. It shows the recipe and its Translations, and offers both the share bundle and a PDF. It never shows Attempts.
_Avoid_: Public URL, guest access, token link, published recipe

**Invite**:
A one-use link that turns a stranger into a Person, or adds a Person to a Kitchen. Kamosu has no signup: every account begins with an Invite, and the first is the Operator's own, minted when the instance first runs.
_Avoid_: Signup, registration, join code, magic link

**Access Key**:
A long secret a Person mints for an agent to act with — named, revocable on its own, optionally read-only, and never expiring. One of the two sources of a Credential, and not a Credential itself. A Key can never mint another Key.
_Avoid_: API key, token, secret, password, Credential

**Operator**:
A Person who also administers the instance: minting Invites, disabling and deleting accounts, resetting a forgotten password, setting the Vault root, merging Foods, and deleting a Kitchen nobody is left in. Nothing beyond that list — no Operator can read another Person's recipes or Attempts. More than one is allowed and the last cannot be demoted. That this is a courtesy rather than a wall is said plainly, because whoever holds the disk holds everything.
_Avoid_: Admin, root, superuser, owner, host

### Doing and permission

**Credential**:
What a request presents to prove who is acting — always as a Person. It comes from one of two places: a Person logging in, or an Access Key they minted. Every Door resolves one before the Core runs anything, so the same permissions apply whether a person or an agent is asking. Obtaining a Credential is not an Operation.
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
An optional folder of Markdown notes and images in which one Kitchen's recipes are published, kept up to date by Kamosu and optionally backed by Git. Kamosu writes a Vault and never reads it back; changes made to the files return only through Import. A Vault is a publication with a Kitchen that owns it, a destination and a scope — not where recipes are stored. One per Kitchen, set up by any of its members.
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
An optional interpretation of an Ingredient Line into its parts — how much, in what unit, of what. That last part is either a **Food** or a **Recipe**; where it is a Recipe, the Ingredient is a Component. A Reading may be absent, partial or mistaken; anything built on it degrades politely rather than failing.
_Avoid_: Parse, structured ingredient, decomposition, breakdown

**Component**:
The role an Ingredient plays when its Reading names a Recipe rather than a Food — the dough inside the pizza. Not a separate kind of thing and not a list of its own: a Component is an ordinary Ingredient, and if the Recipe it names is deleted, absent or never received, its line still reads as written. It names a Lineage, so it always shows whichever Branch of that recipe the reader holds. How much of it is wanted is worked out by comparing the Reading against that recipe's Yield, and is never stored. It unfolds inside the recipe using it, and travels with it as a passenger — readable through it, without a page, a link or a Visibility of its own.
_Avoid_: Sub-recipe, nested recipe, linked recipe, component list, ingredient group

**Food**:
An edible thing Kamosu knows about, pointed at by the Readings that mention it and shared across every recipe that uses it. Created automatically from whatever word a Reading found. The one place a food's other names and its nutrition are recorded. It holds a name per Language — flour and farine are one Food — so a shopping list merges them and nutrition is matched once. There is one list of Foods for the whole instance, because matching a Food to nutrition data is the expensive part and is worth doing once: anyone may create one or add a name to it, but only the Operator may merge two into one.
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
A word a recipe can be described by, kept once in a single list belonging to a Kitchen and pointed at by every recipe of that Kitchen which uses it — so renaming or merging one reaches all of them at once, and named per Language rather than split by it. A tag arriving with a recipe from elsewhere lands in the receiving Kitchen's list; what one Kitchen means by "quick" is its own business. Flat: tags have no hierarchy.
_Avoid_: Keyword, category, label, folder

**Related Recipe**:
A plain "see also" link between two Lineages — my mum's ratatouille and Marmiton's, or the curry and the naan. It has no type and no direction, both recipes show it, and either may remove it. It is a note about one shelf rather than part of a recipe, so it never travels in a share bundle. Kamosu makes one only when a person says so; after an Import it may offer recipes sharing a Source as candidates to accept.
_Avoid_: Relation, association, see also, cross-reference, link

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

**Display Copy**:
What Kamosu shows on a screen when a photograph is asked for — the photograph reduced once to the largest size a screen will ever use. The original is kept and is what Export, the PDF and a Backup carry; it is never sent to a device. A whole library of Display Copies is small enough to live on a phone, which is what lets recipes be read and cooked with no network.
_Avoid_: Thumbnail, resized image, derivative, optimised image, web version

### Identity and history

**Lineage**:
A recipe's identity in the world. One id, minted once when the recipe is created, carried by every copy of it on every instance anywhere. It never changes and is never joined to another Lineage — two recipes that grew up separately are related, not the same recipe.
_Avoid_: Recipe id, UUID, slug, canonical id, family

**Version**:
One saved state of a recipe, named by a fingerprint of its own content, recording the Version it came from and optionally a name and a line saying what changed and why. Every save makes one; none is ever rewritten or deleted. Two holders of the identical state hold the same Version without having communicated.
_Avoid_: Revision, snapshot, commit, edit, history entry

**Branch**:
One holder's line of Versions within a Lineage, carrying a Language. What a person has on screen is a Branch at its latest Version. An instance may hold more than one Branch of the same Lineage — that is divergence, a normal and permanent state, never something to reconcile. A Branch in a different Language from the one it grew out of is a Translation.
_Avoid_: Copy, fork, variant, clone

**Branch Point**:
The last Version two Branches share — a fact computed by walking both parent chains until they meet, not something anyone declares. What "you diverged here" means.
_Avoid_: Common ancestor, base, merge base, split

### Language

**Language**:
The language a Branch is written in. Detected from the recipe's own text when it is saved — set when there is none, offered when it disagrees with one already there, never changed silently — and always correctable. It may be **Unknown**, which is a permanent and unremarkable state for a recipe that is honestly more than one language: it shows to every reader and can neither be a Translation nor have one. Changing it makes a Version.
_Avoid_: Locale, i18n, language code

**Translation**:
A Branch of a Lineage in a different Language from the one it grew out of. Each of its Versions records which Version of the source it translates, so how far behind it has fallen is computed rather than marked. Kamosu never brings a Translation up to date by itself — it says how far behind it is and a person decides.
_Avoid_: Localisation, alternate version, language variant

**Reading Language**:
The Language a person reads Kamosu in, held on their account. Chooses which Branch of a Lineage is shown, and which of a Food's or Tag's names is used. Never hides a recipe: where no Branch is in it, the recipe is shown in whichever Language exists, marked.
_Avoid_: Locale, UI language, preference

### Cooking it

**Attempt**:
One person's record of one cooking — dated, pinned by fingerprint to the Version that was on screen at the time, and never travelling off the instance. Belongs to a Lineage rather than to a Branch, so cooking the dish is remembered however the recipe later diverges. It holds free text, an optional rating, photographs, and an optional As Cooked. Recording one never changes the recipe. It begins when the cooking begins, not when it is written up: cooking mode is an Attempt In Progress rather than a thing of its own, and one that is never finished still counts as a cooking that happened. Deleting is how a false start is undone.
_Avoid_: Cook, log entry, session, make, bake, journal entry

**In Progress**:
An Attempt between the start of cooking and its end. Held on the server, so one cooking follows its cook from phone to iPad, and it additionally holds where they have got to — which Step, which Ingredients are ticked, and the Yield being cooked to, which is a fact about that cooking and never a deviation. Visible to its cook alone, inheriting the recipe's visibility only once it ends. A Person may have one In Progress per Lineage; two devices are one Attempt, and the last one moved on is where the cook is. Ending is either finishing deliberately — where a rating or a note is added — or simply stopping, after which Kamosu offers to resume for three days from the last action and then stops asking.
_Avoid_: Cooking session, active cook, live session, draft attempt

**As Cooked**:
The complete recipe state an Attempt actually cooked, held only when it differed from the Version it started from — the same shape as a Version, but never joined to a Branch and never shared. Differences are written as ordinary Ingredient Lines and Step text, not as a separate record of what changed.
_Avoid_: Deviation, adjustment, diff, draft, variant, modification

**Promotion**:
The deliberate act of taking something provisional out of an Attempt and making it part of the recipe — an As Cooked becoming a Version, or an Attempt photograph becoming the Main Photo or a Step's photo. The point at which an Attempt's freedoms end and the recipe's rules begin: what is promoted is append-only, and promoting requires the right to edit the recipe.
_Avoid_: Apply, merge, commit, save back, accept
