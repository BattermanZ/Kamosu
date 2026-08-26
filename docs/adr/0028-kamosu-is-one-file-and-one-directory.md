# Kamosu is one file and one directory

Kamosu is delivered as **one executable and one data directory**. The web interface ([#14](https://github.com/BattermanZ/Kamosu/issues/14): SvelteKit `adapter-static`) is compiled **into** the binary rather than copied beside it, and every durable thing Kamosu owns — the database, the Photographs, the Display Copies, the Backups, the Snapshots, and the model if one was ever downloaded — lives under a single `/data`.

- **One container image**: the binary, and nothing else. No Node at runtime ([#14](https://github.com/BattermanZ/Kamosu/issues/14)), no browser ([ADR 0023](./0023-a-sheet-carries-the-recipe-not-the-library.md) set the Sheet in Typst as a library), no `git` (the Vault is deferred past v1 by [#12](https://github.com/BattermanZ/Kamosu/issues/12)), and **no model** ([ADR 0029](./0029-kamosu-ships-no-model.md)).
- **One mount**, `/data`. Users never type a filesystem path ([#7](https://github.com/BattermanZ/Kamosu/issues/7)), so where it lands on the host is the mount's business and never Kamosu's.
- **One published port**, plain HTTP. Kamosu never handles a certificate.
- **A healthcheck the binary runs against itself**, because a distroless image has no shell and no `curl`.

`docker run -v ./kamosu-data:/data -p 5266:5266` is a complete install. Nothing else is required, and **no environment variable is required at all**.

## Why

- **The interface and the program cannot be different versions of each other.** Compiling the web files in removes a whole class of failure — a half-upgraded image, a stale asset directory, a mount someone helpfully pointed at last month's build — by making it unrepresentable. The artefact is the version.
- **A stranger who does not use containers still has something to run.** The destination says Kamosu is built so strangers *could* self-host it. For the stranger who has never written a compose file, "download this file and run it" is what that sentence has to mean, and it only means that if the file is self-sufficient.
- **Hatchdoor's four mounts solve a problem Kamosu does not have.** Its Vault is the *user's own Obsidian folder* and cannot live inside an app-owned directory; its `/models` mount exists so an image upgrade does not re-download 200 MB. Kamosu owns all of its state, and Docker's layer caching gives the second benefit for free. Copying the shape without the reasons would have bought four ways to get an install wrong.
- **Display Copies live inside `/data` even though they are rebuildable.** Splitting them out would be a mount whose only purpose is to avoid regenerating thumbnails, paid for by every person who ever installs Kamosu.
- **The whole point of `/data` is that it is the one thing to carry.** One directory to back up, one to move to a new server, one to get wrong.

## Considered options

- **Web files copied beside the binary**, Hatchdoor's shape. Rejected: it permits the binary and the interface to disagree, and it means the "just run this file" story needs a second file.
- **Separate mounts for photographs, database and models.** Rejected: no mount here is user-owned, so every split is friction without a reason.
- **A configurable data path.** Rejected by [#7](https://github.com/BattermanZ/Kamosu/issues/7): users never type a filesystem path. The container path is fixed; the host path is the mount.

## Consequences

- **The interface of a built binary cannot be changed without rebuilding it.** Accepted, and arguably the point: the binary someone downloaded is the binary that was tested. Development is unaffected — `npm run dev` and `cargo run` remain two processes side by side, and the embedding library reads from disk in debug builds, so a frontend change recompiles no Rust.
- **The binary is larger** — a few megabytes for the web files. Irrelevant next to the model that is *not* in it.
- **A distroless image has no shell**, so every terminal act must be one executable with arguments — `docker exec kamosu /app/kamosu recover <name>`, never a pipeline. This constrains the recovery design in [ADR 0030](./0030-an-upgrade-goes-forward-only.md)'s neighbourhood and costs nothing there.
- **A second process touching the database requires WAL mode.** The recovery command opens the same SQLite file as the running server; WAL makes that safe, and it is therefore a requirement rather than a detail.
