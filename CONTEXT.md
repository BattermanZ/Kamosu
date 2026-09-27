# Kamosu

A self-hosted place to keep and cook recipes. People and software agents are equal users of it: everything a person can do to a recipe, an agent can do too.

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

**Meaning Search**:
The half of searching that matches on meaning rather than on words, so *that thing with aubergines* finds a recipe that never says aubergine. It is **optional and off until an Operator turns it on**, because it needs a model Kamosu does not ship and whose terms only a person can accept. Searching is one Operation either way and the **Catalogue** never changes shape — what changes is how a result can match, which a result already says. Its index is derived from the database and can always be rebuilt, so turning it on is a **Job** and turning it off discards nothing.
_Avoid_: Semantic search, vector search, embeddings, AI search, similarity search

### People and access

**Person**:
Someone with an account on this instance. Writes in exactly one Cookbook, cooks in any number of Kitchens, keeps their own Attempts, mints their own Access Keys, reads Kamosu in their Reading Language and writes under a Hand. An agent is never a Person — it acts as one, under that Person's Hand. Deleting a Person's account frees their name for somebody new, while what they wrote keeps their Hand. Disabling an account does not free the name.
_Avoid_: User, member, profile, actor, account

**Cookbook**:
Where a recipe belongs. Only its Co-authors may change what is in it. Every Person has exactly one, and anything they write, import or receive, and every Branch they start, goes into it. It is named after its Co-authors until one of them gives it a name, and it has a Hand of its own, which travels with every Branch it writes.
_Avoid_: Library, collection, account, personal space, home kitchen

**Co-author**:
A Person who writes in a Cookbook. Every Cookbook starts with one; two or more people may join their Cookbooks into one and later separate again, each leaving with their own Branch of every recipe in it. Co-authors change the same recipes as one history with several names on it.
_Avoid_: Owner, editor, collaborator, partner

**Kitchen**:
A group of People who cook from each other's Cookbooks. Everyone in it sees, cooks and records Attempts against every recipe in the Cookbook of every member; it holds no recipe and changes none. A Person cooks in any number of Kitchens, or in none. Anyone may create one. It has a Name given by its creator, which any member may change, and a Nickname each member may set for themselves alone and which nobody else ever sees. Any member may invite another Person or remove one. A Cookbook leaves a Kitchen once none of its Co-authors cooks there, and each member who stays keeps a Branch, in their own Cookbook, of every recipe of it they cooked.
_Avoid_: Household, group, team, family, workspace, organisation, account

**Hand**:
How Kamosu says who wrote something: a name together with a permanent id, minted with the account and never changed, carried by every Version a Person writes and every Branch a Cookbook writes. It travels in a share bundle and is never verified — anyone editing that file can put any Hand in it, and Kamosu says so plainly rather than implying a check it does not make. Recognisable in the way handwriting on a card is recognisable, and forgeable for the same reason. On the instance that minted it the name is looked up live, so renaming yourself reaches all of your history at once; anywhere else the name that arrived is all there will ever be.
_Avoid_: Author id, account id, signature, public key, handle, attribution, fingerprint (that names a Version)

**Copy**:
A new Branch of a recipe, started in your own Cookbook, forking at a Version and carrying the whole chain behind it. It happens when you change a recipe your Cookbook did not write, when you start a second Branch of one of your own on purpose, and when a Cookbook leaves a Kitchen and you keep one of its recipes you cooked. It starts only from a Branch one of your Kitchens already holds, which is where a received Bundle or a kept Share Link puts it; a change to any other is refused as though it were not here. Merely reading a recipe never starts one: a **Bundle** arriving puts the sender's Branch in your Cookbook under their id, and only changing it makes one of your own. What one recipe becoming two looks like inside a single instance, identical in every respect to receiving a Bundle from another server.
_Avoid_: Duplicate, fork, clone, save, import

**Share Link**:
The unguessable address at which a recipe can be read by anyone holding it — the only way a recipe reaches anyone outside the Kitchens its Cookbook is in, and the same way whether the reader is on this instance or another. One per recipe, never expiring, freely passed on. There is no scale of audiences and no naming of individuals: a recipe is seen by the Kitchens its Cookbook is in, or by anyone with its link. Turning sharing off ends it; turning it back on makes a new one, so a withdrawn link stays dead. Unlike every other **Secret**, its address stays readable: the share screen shows it for as long as the link lives, because a link only reads one recipe and whoever holds the database already holds that recipe (ADR 0031, as amended by #171). It shows the recipe and its Translations, the recipe's **Thread** beneath it, and offers both the share bundle and a **Sheet**. It never shows Attempts. Ending a link stops new people arriving and reaches no copy already sent, which the share screen says in the same words every time.
_Avoid_: Public URL, guest access, token link, published recipe

**Invite**:
A one-use link that turns a stranger into a Person, adds a Person to a Kitchen, or joins another Person's Cookbook to yours — a **Cookbook Invite**, which only somebody who already has an account can open, and which says how many recipes on each side become one before anything happens. Where either Cookbook has other Co-authors, accepting one joins nothing until each of them has said yes too (#135). Kamosu has no signup: every account begins with an Invite, and the first is the Operator's own, minted when the instance first runs.
_Avoid_: Signup, registration, join code, magic link

**Access Key**:
A **Secret** a Person mints for an agent to act with — named, revocable on its own, optionally read-only, and never expiring. One of the two sources of a Credential, and not a Credential itself. It acts as that Person in their Cookbook and every Kitchen they cook in, and is never scoped to one. It can mint no Key, change no password and mint no Invite that makes an account, so an agent cannot turn a leak into an account.
_Avoid_: API key, token, secret, password, Credential

**Operator**:
A Person who also administers the instance: seeing who holds an account, minting Invites, raising a Person to Operator and standing one down, disabling and deleting accounts, resetting a forgotten password, setting the Vault root, changing the instance's public address, merging Foods and deleting one nothing points at, sweeping Photographs nothing points at, deleting a Kitchen nobody is left in, and taking and fetching a **Backup**. Nothing beyond that list — no Operator can read another Person's recipes or Attempts, and seeing who holds an account is a list of names and nothing about what they cook. More than one is allowed and the last cannot be demoted, disabled or deleted, because an instance with nobody to administer it can never appoint anybody. That this is a courtesy rather than a wall is said plainly, because whoever holds the disk holds everything.
_Avoid_: Admin, root, superuser, owner, host

### Doing and permission

**Credential**:
What a request presents to prove who is acting — always as a Person. It comes from one of two places: a Person logging in, or an Access Key they minted. Every Door resolves one before the Core runs anything, so the same permissions apply whether a person or an agent is asking. Obtaining a Credential is not an Operation.
_Avoid_: Token, session, auth, API key, login

**Secret**:
A long random string where holding it *is* the permission — a **Share Link** token, an **Access Key**, an **Invite**, a recovery link, a **Session**. All are 256 bits of randomness, revocable one at a time, and **never on a clock**. All but the Share Link are stored hashed, shown once at minting, and known afterwards only by a name and when they were last used. A Share Link keeps its Secret readable, so its address can be shown again for as long as it lives: it grants reading one recipe, which whoever holds the database already has (ADR 0031, as amended by #171). A Secret ends in one of exactly two ways — it is *spent*, as an Invite and a recovery link are on first use, or it is *revoked*. A password is not a Secret: it is short, chosen by a person, never stored, and the only thing in Kamosu that is throttled.
_Avoid_: Token, key, credential, nonce, bearer token

**Session**:
The Secret a browser holds after a Person logs in, so they stay logged in. Ends when it is revoked and at no other time. The browser keeps it for 400 days, counted again each time it is used, so a device opened at least that often never forgets it (#137). A Person sees their own beside their Access Keys, each named for the device it signed in on (and renamable), marking the one in hand and showing when it was last used, and ends any of them from any device. It carries a Person's full powers, which is what keeps it distinct from an Access Key despite the shared shape.
_Avoid_: Cookie, login, token, sign-in

**Immediate Operation**:
An Operation that answers within a single request.
_Avoid_: Sync operation

**Job**:
An Operation too slow to answer within a single request — importing a library, fetching a recipe from a website, producing a **Sheet**. Asking for one returns a job id at once; its progress is then read through ordinary Operations.
_Avoid_: Task, background job, async operation, queue item

### Getting recipes in and out

**Import**:
Bringing recipes into Kamosu from elsewhere — another app's export, a web page, or a **Bundle**. An Operation like any other, available at both Doors, and held in the **Cookbook** of the Person who asked for it. What it produces is an ordinary Recipe: nothing about where it came from is kept on it, no line is marked as one Kamosu had to write rather than read, and the **Hand** on its first **Version** is that Person's, where it came from being a matter for its **Source**. The Import itself remembers what it made — the id a recipe had in the place it came from, against the **Lineage** it became — so importing the same file again matches what is already there instead of shelving it twice. That memory belongs to the Import and never to the recipe, so it is in no fingerprint and no Bundle, and it can be thrown away whole once the place it names is gone.
_Avoid_: Migration, sync, ingest

**Import Report**:
What an **Import** made, as a person reads it — and it does not go away when the screen does. It says what arrived, including which recipes came as a bare name and a link; what is waiting for a tap, which is where a recipe the Import has seen before and found changed is offered rather than written over, and where **Related Recipe** candidates sharing a **Source** are ticked; and what could not be read at all, named with the reason, a damaged **Bundle** among them. It is read through ordinary Operations, so an agent asking how an import went is answered from the same place.
_Avoid_: Log, error list, summary, receipt, job output

**Export**:
Producing recipes in a form usable outside Kamosu — a **Bundle**, or a **Sheet**. An Operation like any other.
_Avoid_: Download, backup, share

**Bundle**:
The one form in which recipes leave Kamosu: a folder holding a readable Markdown note per recipe — the recipe as it reads today, with its **Thread** beneath it — and a hidden `.kamosu/` sidecar carrying the complete past Versions, the Readings and the ids. Handed over as a plain zip; a **Vault** is the same folder holding a whole library, written to a directory and kept up to date. It is self-contained and works with the sender's server switched off, carries the complete chain back to the first Version, and names which Lineages it is *about* — everything else in it is a **Passenger**. It carries no Attempts.
_Avoid_: Export file, archive, package, payload, blob

**Passenger**:
A recipe present in a **Bundle** only because something else in it needed it — the dough travelling with the pizza. It arrives whole, since a **Component**'s line has nothing to unfold without it, and it is not what the Bundle is about. The word says how it travelled and not what it is: once **Import** has opened the Bundle it is a Recipe like any other, on the shelf and readable on its own, exactly as the dough your own pizza points at already is.
_Avoid_: Dependency, attachment, extra, sub-recipe

**Sheet**:
One recipe rendered for paper — the printable form a **Share Link** offers beside the **Bundle**, made by anyone who can see the recipe and a stranger holding a link alike. It carries the recipe, not the library: what is a fact about the dish is on it, what is a fact about how Kamosu files the dish is not — so Tags, Attempts, the **Thread** and every past Version stay behind. It is one **Branch**, the one on screen, printed as it stands there with any scaling already applied and no dialog of its own; **Components** unfold after it, parent first, each already scaled. The written **Ingredient Line** is what is printed and a **Reading** is not, save for a scaled amount beneath a line and a Component's page number. It ends with a small block saying which **Version** it came from and whose **Hand** wrote it, so a page found in a drawer can still say what it is. Dead in both directions: nothing leads back into Kamosu from it, and nothing is ever read out of it.
_Avoid_: PDF, printout, export, print view, hard copy

**Vault**:
An optional folder of Markdown notes and images in which one Cookbook's recipes are published, kept up to date by Kamosu and optionally backed by Git. Kamosu writes a Vault and never reads it back; changes made to the files return only through Import. A Vault is a publication with a Cookbook that owns it, a destination and a scope — not where recipes are stored. One per Cookbook, set up by any of its Co-authors.
_Avoid_: Mirror, source of truth, repository, notes folder

**Backup**:
An Operation producing a single archive from which a Kamosu instance can be restored — a consistent copy of the database together with the photographs. Kamosu writes it beside the database under `/data/backups` and keeps three, distinguished by how far back each reaches rather than by which is newest: one taken daily, one weekly, one monthly (ADR 0039). Taking a fresh one is what prunes the one it replaces. Any archive it holds can be listed and fetched at either Door, and taking, listing and fetching are all Operator powers — an archive holds everybody's recipes. Restoring is stopping Kamosu, deleting `kamosu.db` and its `-wal` and `-shm` neighbours, and unzipping the archive over `/data`. **Kamosu never sends one anywhere** — carrying it off the machine belongs to whatever already backs the machine up. Display Copies are not in one, being rebuildable. A Vault is not a Backup, and neither is a **Snapshot**.
_Avoid_: Export, dump, cloud backup

**Snapshot**:
The copy of the database Kamosu takes of its own accord before it migrates, and the only way back from an upgrade. It is the database alone and never the photographs, which is not a shortcut: a **Photograph** cannot change once it exists, so a migration cannot touch one, and the database by itself is a complete way back. Restoring is putting the file back. It is **not a Backup** — it is small, automatic, unscheduled, and about the last few minutes rather than the last few weeks.
_Avoid_: Backup, dump, restore point, checkpoint

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
An optional interpretation of an Ingredient Line into its parts — how much, in what **Unit**, of what. That last part is either a **Food** or a **Recipe**; where it is a Recipe, the Ingredient is a Component. A Reading may be absent, partial or mistaken; anything built on it degrades politely rather than failing. It is Kamosu's reading of a line rather than part of the line, so it is no part of a Version's identity: correcting one makes no Version and appears in no Thread. It travels beside the Version it belongs to and is carried, never recomputed — where it points at a Food it carries that Food's names in every Language it has one for and no id, and where it points at a Recipe it names a Lineage. A Reading is either the reader's or set by hand: the Operator may have the whole library read again with the current reader, which replaces only the reader's own Readings and reports every one it changed.
_Avoid_: Parse, structured ingredient, decomposition, breakdown

**Unit**:
What a quantity is counted in — grams, cups, spoons, *poignées*. Whatever the cook wrote is a Unit, and Kamosu keeps it as written. It knows a closed list of them well enough to do arithmetic on, each with its own spellings in every interface Language, so `g`, `gr`, `gramme` and `grams` are one Unit; anything outside that list is no less real and simply never converts. The set of Units is open; the set Kamosu can convert is closed.
_Avoid_: UOM, measure, measurement, quantity type, dimension

**Component**:
The role an Ingredient plays when its Reading names a Recipe rather than a Food — the dough inside the pizza. Not a separate kind of thing and not a list of its own: a Component is an ordinary Ingredient, and if the Recipe it names is deleted, absent or never received, its line still reads as written. It names a Lineage, so it always shows whichever Branch of that recipe the reader holds. How much of it is wanted is worked out by comparing the Reading against that recipe's Yield, and is never stored. It unfolds inside the recipe using it, and travels with it as a passenger — readable through it, without a page or a Share Link of its own.
_Avoid_: Sub-recipe, nested recipe, linked recipe, component list, ingredient group

**Food**:
An edible thing Kamosu knows about, pointed at by the Readings that mention it and shared across every recipe that uses it. Created automatically from whatever word a Reading found. The one place a food's other names and its nutrition are recorded. It holds names per Language — flour and farine are one Food — so a shopping list merges them and nutrition is matched once. A Language may hold several, such as œufs and œuf, and a line naming any of them reads as the Food; the first is the one shown (ADR 0043). There is one list of Foods for the whole instance, because matching a Food to nutrition data is the expensive part and is worth doing once: anyone may create one, or add a name to it, or remove one, but only the Operator may merge two into one or delete one nothing points at. It is known by its words alone: nothing about a Food's identity, its nutrition or what a cupful of it weighs ever arrives from another instance. A Food nothing points at is kept, because what it knows was expensive to learn. Only a Reading on a recipe somebody can still open points at a Food: one a deleted recipe left behind does not, and goes with the Food when it is deleted.
_Avoid_: Item, product, ingredient (an Ingredient is a row in a recipe; a Food is the thing itself)

**Food Match**:
How a word looking for a Food finds one — the same act whether the word arrived in a Bundle, was read out of a Crouton file or a web page, or was typed. Two words are the same word when they are in the same Language and equal once capitals and stray spaces are folded away; accents and plurals are meaning, so *maïs* is not *mais* and English *raisin* is not French *raisin*. It never asks a question and never interrupts a recipe arriving. One hit matches, and the Food learns any arriving name that nothing on the instance answers to. **Doubt makes a new Food, never a merge**: where an arriving Food's words hit two, it lands as a third carrying all of them, because welding two Foods together cannot be undone while keeping two apart is one tap to repair. A lone word that is ambiguous goes to the Food the most Readings already use. Getting it wrong is survivable in every case, since the Ingredient Line reads as written whatever the Reading over it points at.
_Avoid_: Matching, resolution, deduplication, fuzzy match, lookup

**Merge Suggestion**:
A note that two Foods on this instance are probably one thing, kept with the reason it was made and shown to the Operator where merging happens. Made when an arriving Food's words hit two — which is somebody on another server saying plainly that they are the same — or when a name is typed onto a Food that another already answers to. It is evidence, never an instruction: nothing merges itself, and a merge clears every suggestion naming either Food.
_Avoid_: Duplicate warning, conflict, candidate, proposal

**Merge**:
Joining two Foods into one, which only the Operator may do. The survivor takes every name from both, every Reading that pointed at the other points at it instead, and where the two disagree about **Cup Weight** the Operator says which figure survives. It says how many Ingredient Lines it is about to move before it moves them, and that saying is the whole safety net: a merge cannot be undone.
_Avoid_: Combine, deduplicate, link, alias

**Cup Weight**:
What one cup of a Food weighs, about — the one figure that lets a volume become a weight, since a cup of flour is 125 g and a cup of butter is 227 g. Kamosu ships a figure for the staples and anyone may correct it; a Food that has none is not a gap to be filled, only a line that offers millilitres instead of grams. Every other volume measure of that Food follows from it by arithmetic. It is a fact about a cupful rather than about the food, which is why it is not a density: the same cream is 238 g poured and 120 g whipped. It belongs to this instance and never travels in a share bundle.
_Avoid_: Density, specific gravity, conversion factor, weight, gram weight

**Step**:
One instruction in a recipe, in order. Its truth is its text. It may also carry a photo of what the step should look like. Which Ingredients it uses is worked out from their Readings rather than carried — nothing points at anything and nobody types a link. A duration, a temperature or an amount is read out of that text, never typed beside it — a duration becoming a timer that can be started, and a temperature in the other system or an amount with a convertible Unit ("425°", "1 lb.") becoming its conversion, drawn straight after what it converts; an amount's is scaled to the Yield being cooked. None of them is ever written into the text.
_Avoid_: Instruction, direction, method, procedure

**Section**:
A named part of a recipe — "For the sauce", "Assemble" — holding some of its Ingredients or some of its Steps. Optional: most recipes have none and are a single flat list.
_Avoid_: Group, block, heading, part, component

**Yield**:
What a recipe makes — an amount and what it is an amount of: "4 servings", "24 cookies", "1.5 litres". Optional. Scaling a recipe means changing this amount, which multiplies every Reading in step. A recipe that states no Yield is scaled by a multiplier instead — "×2", held as a Yield whose noun is empty — since there is nothing to count from; a Yield a recipe itself states always has a noun.
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

**Nutrition**:
One figure a recipe carries, and what that figure counts: calories per serving of the Yield as written, or calories per 100 g. Optional, and commonly absent. It is typed by the cook, or taken from a source page's own structured data where the page stated a number — never worked out from the Ingredient Lines or the Foods they name, because a plausible-but-wrong calorie figure is worse than an empty field. The basis is part of the figure rather than a setting: 308 says nothing until it says what it counts, and the two bases do not convert into one another without a weight a recipe does not carry. It is one of the recipe's own words, so it rides in the fingerprint and travels with the recipe. The nutrition a **Food** records is a different thing under the same word — per-ingredient, instance-wide, and never travelling — and neither is computed from the other in v1.
_Avoid_: Calories, macros, nutritional information, nutrients, energy

**Main Photo**:
The single Photograph that stands for a recipe wherever it is listed. Optional — and commonly absent. Distinct from a Step's photo, which shows one moment in the cooking.
_Avoid_: Hero image, thumbnail, picture, cover (that names the generated stand-in)

**Photograph**:
A picture held by Kamosu, known by its own contents rather than by a name or a place: two identical pictures are one Photograph, and a Photograph never changes once it exists. It is stored once however many Versions, Attempts and recipes point at it, and it is part of what a Version is — so replacing one is an edit to the recipe, like rewording a step. Whatever arrives is remade at the door into a single agreed form and it is that form, not the file that was handed over, which the Photograph is; a Photograph arriving from another instance is already made and is left alone. What is stripped at the door is everything a camera staples on and nobody asked for, the place a picture was taken above all.
_Avoid_: Image, file, upload, original, master, asset

**Display Copy**:
What Kamosu shows on a screen when a Photograph is asked for — the Photograph reduced to one of a small fixed set of sizes, so that a shelf of cards is not built out of full-size pictures. Never sent anywhere but a screen: Export and a Backup carry the Photograph itself, and a **Sheet** carries a print-sized rendering of it — one more size in the same small set. A Display Copy is worked out from the Photograph and kept only for convenience, so it belongs to no Version, travels in nothing, and can be thrown away and remade — which is what makes its sizes and its format free to change later. A whole library of Display Copies is small enough to live on a phone, which is what lets recipes be read and cooked with no network.
_Avoid_: Thumbnail, resized image, derivative, optimised image, web version

**Cover**:
What leads a recipe that has no **Main Photo** — a coloured ground carrying the recipe's title, generated from the recipe's **Lineage** id and therefore the same after a rename, the same on every instance the recipe reaches, and the same across its **Translation**s. Used on a card and at the top of a page, in the space a Photograph would have filled. Not a placeholder for something missing: a recipe needs only a title, and roughly a third of a real library has no photograph at all.
_Avoid_: Placeholder, fallback image, default image, thumbnail, avatar, identicon

### Identity and history

**Lineage**:
A recipe's identity in the world. One id, minted once when the recipe is created, carried by every copy of it on every instance anywhere. It never changes and is never joined to another Lineage — two recipes that grew up separately are related, not the same recipe.
_Avoid_: Recipe id, UUID, slug, canonical id, family

**Version**:
One saved state of a recipe, named by a fingerprint of what a person wrote and chose **alone** — the words and the Photographs, never the **Reading** over a line, and never who wrote it, when, what it is called, or what they said about it — which is what lets two holders of the identical state hold the same Version without having communicated. **A field holding nothing is no part of it** — an empty slot and one that did not exist yet are the same thing, so a field added to a recipe in a later release moves the id of nothing already written. It records the Version it came from, the **Hand** of the Person who wrote it, and optionally a name and a line saying what changed and why. Every save makes one; none is ever rewritten, deleted, or withheld from a share — a share always carries the complete chain back to the first Version, names and *what changed* lines included, and nobody holding a Version may remove it, its author and its recipients alike. Its **name** is the one thing about it that can be changed later — a label a person puts on a moment. Its *what changed* line is frozen once written but is no more part of its identity than its name is, so one Version may reach two people carrying two different notes. On the instance where it was written it also records which Access Key wrote it, if any; that is for its author to read and never travels.
_Avoid_: Revision, snapshot, commit, edit, history entry

**Branch**:
One holder's line of Versions within a Lineage, carrying a Language. What a person has on screen is a Branch at its latest Version. It has an id of its own, minted when it starts and carried wherever it travels, and it records the **Hand** of the Cookbook writing it — so a second bundle from the same friend is that Branch continuing rather than a third one to line up beside the others. That id is really two, told apart below: a **Travelling id** and a **Local id**, the same on the instance the Branch started on and different on one that received it. A Cookbook may hold several Branches of one Lineage, a Translation being the ordinary case, so a Branch is never identified by its Cookbook. One started beside your own on purpose carries a name you give it, since nothing else tells the two apart. An instance may hold more than one Branch of the same Lineage — that is divergence, a normal and permanent state, never something to reconcile. It may also carry the address of the instance it lives on — a hint written into a Bundle, never fetched and never required, since a Bundle must work with that server switched off. A Branch in a different Language from the one it grew out of is a Translation.
_Avoid_: Copy, fork, variant, clone

**Variation**:
A second **Branch** of one of your own recipes, started beside it on purpose, named, and kept in your own **Cookbook**; the first stays exactly as it was. The one kind of **Copy** you make of a recipe you already write, and only its Cookbook's Co-authors may start one. It may start with no change at all.
_Avoid_: Variant, fork, alternate version

**Travelling id**:
The id a **Branch** travels under: minted when the Branch starts, written into every **Bundle** it appears in, and never changed by anything that receives it. It is what makes the sender's next Bundle that Branch continuing rather than a second one lining up beside it. It is unique to a **Cookbook**, not to an instance — two Cookbooks on one Kamosu may each receive the same friend's recipe, and each holds its own copy of it under his travelling id.
_Avoid_: Global id, canonical id, origin id, foreign id (that is what an importer's source called a recipe)

**Local id**:
The id one instance files its own copy of a **Branch** under — what every Operation asks for, what every URL carries, and the only one of the two a person ever sees. It is this instance's to mint, so a Branch that arrived from elsewhere has a local id that is not its **Travelling id**. On a Branch that started here the two are the same.
_Avoid_: Row id, primary key, internal id, database id

**Branch Point**:
The last Version two Branches share — a fact computed by walking both parent chains until they meet, not something anyone declares. What "you diverged here" means. It always exists between two valid Branches of one Lineage, because every chain reaches the same first Version; a chain that does not is a damaged bundle.
_Avoid_: Common ancestor, base, merge base, split

**Thread**:
The whole of a Lineage on one screen: its Versions oldest to newest, forking at the Branch Point, with Attempts hanging off it. Where a person reads back, opens any Version, and cooks from one. Reading, never editing. The interface labels the button and the screen *History* (*Historique*, *Historial*), because "the thread" told a cook nothing about what was behind it (#133). The words below are to be avoided in code and docs, which keep **Thread**; the label is the interface's alone.
_Avoid_: History, log, timeline, changelog, graph

**Ghost**:
A line one Branch has and the other has not, shown on the recipe that has not got it — struck through, in the position it occupies in the recipe that really has it, labelled with whose it is. A line the other person removed and a line the other person added are the same thing seen from opposite sides, so one Ghost serves both and every difference is visible from either Branch.
_Avoid_: Diff, deletion, phantom, missing line

**Pairing**:
Kamosu deciding that a line of one Branch and a line of the other are the same line, changed — worked out by reading them against the **Branch Point**, never by any name a line carries, because a line carries none. Only lines somebody actually edited need one; anything neither side touched is identical on both sides already. Where the reading is uncertain Kamosu declines to pair and shows both lines, one of them as a **Ghost**, and it never labels a Pairing as a guess: both texts are on screen in full, which is better evidence than a badge about them. It reaches the whole recipe, not only the two lists — Sections and Notes pair as lines do, single values such as the title or the Yield are simply the same or not, and a Photograph is exact because it is known by its own contents. Tags are left out, being how a Kitchen files a recipe rather than anything about the dish.
_Avoid_: Diff, match, alignment, correlation, line id, key

### Language and measures

**Language**:
The language a Branch is written in. Detected from the recipe's own text when it is saved — set when there is none, offered when it disagrees with one already there, never changed silently — and always correctable. It may be **Unknown**, which is a permanent and unremarkable state for a recipe that is honestly more than one language: it shows to every reader and can neither be a Translation nor have one. Changing it makes a Version.
_Avoid_: Locale, i18n, language code

**Translation**:
A Branch of a Lineage in a different Language from the one it grew out of. Each of its Versions records which Version of the source it translates, so how far behind it has fallen is computed rather than marked. Kamosu never brings a Translation up to date by itself — it says how far behind it is and a person decides.
_Avoid_: Localisation, alternate version, language variant

**Reading Language**:
The Language a person reads Kamosu in, held on their account. Chooses which Branch of a Lineage is shown, and which of a Food's or Tag's names is used. Never hides a recipe: where no Branch is in it, the recipe is shown in whichever Language exists, marked.
_Avoid_: Locale, UI language, preference

**Reading Measures**:
How a person measures, held on their account beside their Reading Language — metric, US measures, or as written. Where an Ingredient Line is in the other system, or the recipe has been scaled, one line beneath it says how much that is for this reader, right now. It never replaces the written line, always says *about*, and is absent whenever it would only repeat what is already there. Setting it changes nothing that is stored and makes no Version. **The default is American, a stated convention rather than a guess about anybody**: a cup is 240 ml, a tablespoon 15 ml, a teaspoon 5 ml. It reaches a Step too, each conversion drawn straight after what it converts: a temperature in the other system on the conventional oven ladder — 350°F is 180°C, the number on the dial, never the 176.67 the arithmetic gives — and each amount the Step writes, on the same rules as an Ingredient Line's.
_Avoid_: Units preference, locale, metric toggle, unit system

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

### Shopping

**Shopping List**:
One **Person**'s standing choice of what they are about to cook — the recipes they picked, how much of each they are shopping for, and any **Loose Item** they typed. Everyone has exactly one; it has no name, it is never archived, and sending it offers to empty it but never does so by itself. It holds a **Branch** rather than a **Lineage**, because on the day you are cooking one particular text and a friend's version of the dish may want anchovies where yours does not — and always that Branch's latest **Version**, never a pinned one, so a recipe edited on Monday is right on Tuesday. What is stored is the choosing; what is shown is worked out from it every time and kept nowhere. A recipe that is deleted or whose sharing is withdrawn stays on the list, keeps the name it was known by and contributes nothing, saying so.
_Avoid_: Basket, cart, meal plan, groceries, list

**Shopping Row**:
One thing to buy, worked out from a **Shopping List** and stored nowhere. It names a **Food** rather than any **Ingredient Line**, in the reader's **Reading Language** and falling back to whatever name the Food has, marked — so every mention of flour across every recipe on the list becomes one row. It carries as many amounts as the arithmetic honestly supports: added together where the **Unit**s convert, in the reader's **Reading Measures** and saying *about*; side by side where they do not, `500 g + 2 poignées`, because two true amounts beat one wrong one. An amount nobody stated is one of those, shown as *some* and never dropped. The lines it was made from are always one tap away.
_Avoid_: Item, entry, line, aggregate, shopping item

**Loose Item**:
A line typed straight onto a **Shopping List** that belongs to no recipe — bin bags, coffee. Kept exactly as typed and never interpreted, so it has no **Food**, no quantity and merges with nothing: typing *flour* beside a recipe that wants flour gives two lines. Reading it would mean guessing at a number about to be shopped by, where a **Shopping Row** is built on **Reading**s that already exist. An **Ingredient Line** nobody ever read appears on a list the same way.
_Avoid_: Extra, manual item, custom item, note
