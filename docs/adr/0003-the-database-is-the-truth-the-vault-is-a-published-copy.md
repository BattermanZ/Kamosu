# The database is the truth; the Markdown vault is a published copy

Kamosu stores everything — recipes, photographs, accounts, cooking history — in **SQLite**, and that store is authoritative. Beside it, each user may optionally switch on a **Vault**: a folder of Obsidian-flavoured Markdown notes and images that Kamosu writes out and keeps up to date, optionally git-backed. The Vault is written, never read back. It exists so recipes are portable, readable without Kamosu, and shareable as a folder or a git remote — not so they can be stored there.

This **inverts the precedent this project otherwise follows**. Hatchdoor (same author, same shape, cited throughout Kamosu's planning) holds the opposite as its ADR-01: *"Markdown is the source of truth; SQLite is a disposable read model."* A reader who knows Hatchdoor will see Kamosu's `.md` files and its SQLite file and reasonably assume the same rule applies. It does not, and acting on that assumption destroys data: deleting Kamosu's database loses every account, every cooking attempt and every shopping list, and no Vault can rebuild them.

## Why

- **Kamosu is multi-user; Hatchdoor is not.** Hatchdoor is explicitly single-operator with one token, so its SQLite holds nothing but a rebuildable index — genuinely disposable. Kamosu has accounts, credentials, per-person cooking history, shopping lists and share links. None of that belongs inside a shared recipe file, and all of it must survive.
- **Kamosu is a cookbook, not a notes editor.** Almost nobody using it will care that recipes are files. What files actually buy is portability and shareability, and both are satisfied by a copy Kamosu publishes. Neither requires the file to be authoritative.
- **The expensive half of the precedent buys nothing here.** Files-as-truth costs a filesystem watcher, reindex-on-change, and rules for reconciling a hand-edit against a concurrent app write — Hatchdoor's ADR-10 exists to stop git force-checkouts from destroying manual edits. A write-only publication has none of these problems, because there is only ever one writer.
- **Structure has nowhere to live in a note.** A Reading over an Ingredient Line ([ADR 0002](./0002-the-written-ingredient-line-is-the-truth.md)), a Step's links to the Ingredients it uses, and stable ids are all real data with no natural Markdown spelling. Held in the database, they are simply columns.

## Considered options

- **Markdown authoritative, SQLite disposable** (the Hatchdoor pattern). Rejected: the rule would be false the moment accounts and cooking history exist, and a false rule is worse than the opposite rule — someone eventually deletes the "disposable" database and loses their cooking history.
- **A split truth — recipes in files, everything else in the database.** Rejected: it keeps the whole cost of files-as-truth (watcher, conflict rules, parser bugs as data-loss bugs) for a benefit the product does not need, and leaves two authoritative stores that can disagree about the same recipe.
- **Database authoritative, Vault a published copy** (chosen).

## Consequences

- **The Vault is lossless, or it is not worth having.** A Vault is an interchange format — a git repository that keeps two Kamosu instances in step, or a folder handed to a friend. Anything the format drops is destroyed on every hop. So each recipe is published twice: a clean note for people (frontmatter tags, `[[wikilinks]]`, embedded images — readable by Obsidian and Hatchdoor with nothing taught to them), and a **sidecar** JSON under a hidden `.kamosu/` directory carrying the complete record, including ids, Readings and step-to-ingredient links. Obsidian and Hatchdoor ignore dot-directories; git and `zip` do not.
- **Import reads the sidecar; the note is a rendering.** There is no merging of two sources and no ambiguity about which wins. A vault arriving without `.kamosu/` is imported best-effort from the notes, and says so.
- **A Reading is carried, not recomputed.** A Reading corrected by hand is authored data. Regenerating it on import would silently discard the correction.
- **Backup is a first-class Operation, and it must exist in v1.** Copying a live SQLite file yields a torn copy, and photographs live outside the database as ordinary files on disk. The backup Operation therefore emits **one archive** containing a consistent database snapshot and the images — so the person doing the backing up has one thing to grab. The documentation must say plainly that the Vault is not a backup.
- **A Vault is a publication, not a container.** It has an owner, a destination, and a **scope**. In v1 the only scope is *the owner's recipes*, and the operator sets a single root directory under which per-user vaults are created — a user chooses whether to have one and what remote it pushes to, never a filesystem path. Shared vaults become an additional scope value later, not a redesign.
- **Kamosu publishes and imports Vaults; it does not merge them.** Two instances editing one vault is a conflict problem git cannot solve for a JSON sidecar. A sync protocol is deliberately left to a later decision, which nothing here forecloses.
- **Per-recipe versioning cannot lean on git.** The Vault's git history is history of a derived copy, and only exists if the user switched vaults on at all.
