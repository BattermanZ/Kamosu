# A Backup is three archives at three distances

A **Backup** is one archive holding a consistent copy of the database and every **Photograph** ([#78](https://github.com/BattermanZ/Kamosu/issues/78)). Kamosu keeps exactly **three** of them under `/data/backups`, and what distinguishes the three is not which is newest but **how far back each one reaches**:

| Slot | Replaced | Reaches back |
| --- | --- | --- |
| `daily` | every day | to yesterday |
| `weekly` | every seven days | up to a week |
| `monthly` | every thirty days | up to a month |

Kamosu wakes hourly, asks each slot whether it is due, and takes **one** archive covering every slot that is. Placing that archive is what prunes the one it replaces, so three is a ceiling the directory cannot exceed. `take_backup` asks for the daily slot by name; nothing else about the schedule is configurable.

The archive holds `kamosu.db` and `photographs/`, laid out exactly as they sit under `/data`. Restoring is three steps: stop Kamosu, **delete `kamosu.db`, `kamosu.db-wal` and `kamosu.db-shm`**, then unzip the archive over `/data`. Deleting the two neighbours is not optional — the database inside an archive carries no write-ahead log, and one left over from the old install would replay across the restored file. [ADR 0030](./0030-an-upgrade-goes-forward-only.md) says the same thing about putting a **Snapshot** back, in the message Kamosu prints when a migration fails.

**Kamosu never sends an archive anywhere.** Carrying one off the machine is an Operator's act, through `GET /api/backups/<name>`, under the same **Credential** as any Operation.

## Why

- **One archive per slot, and the slots are about distance.** Keeping "the last three" would give three copies of this week — three answers to a question nobody asks, and no answer at all to *what did this recipe say before I rewrote it last month*. Corruption and a bad edit are both usually noticed late, and lateness is the whole problem a Backup exists to solve.
- **Three is what a home server can afford.** An archive is roughly the size of the library: the first Operator's is 82 MB of photographs against a few megabytes of database, and someone with a decade of cooking will have gigabytes. Three is about as many copies as a self-hoster will tolerate sitting beside the thing they are copies of.
- **`VACUUM INTO` is why "consistent" is a real word here.** It reads the database inside one transaction, so the file that lands holds the database as it stood at one instant — never a page from before a save next to a page from after it. `cp` gives no such promise, and the failure it permits is silent. WAL is what lets that read happen on a second connection without stopping anybody writing ([ADR 0028](./0028-kamosu-is-one-file-and-one-directory.md)).
- **Ages are measured on SQLite's clock**, in SQLite's calendar, because every other timestamp in Kamosu already is. An archive's name carries the fixed-width moment it was taken, so *is this older than seven days* is one string comparison against `strftime('%Y%m%dT%H%M%SZ','now','-7 days')` and no date arithmetic exists to get wrong.
- **What is owed is read off the files themselves**, never a table. The archives are what an Operator can actually restore from; a tally that disagreed with the directory would be a lie about the one thing this feature exists to promise.
- **Hourly wake-ups, daily backups.** A daily timer resets at every boot, so on a server rebooted each evening it never fires. An hourly tick that finds nothing owed twenty-three times out of twenty-four costs three `stat` calls and makes the schedule survive an evening reboot. The first wake-up is one tick away rather than immediate, so restarting an instance never costs it a copy of its whole library before it answers anything — at the price named under Consequences.
- **Display Copies are not in it.** They are rebuildable from the Photographs ([ADR 0017](./0017-a-photograph-is-known-by-its-contents.md)) and would roughly double what an Operator stores. The downloaded model is out for the same reason and a stronger one: it is 220 MB of somebody else's weights that Kamosu never shipped ([ADR 0029](./0029-kamosu-ships-no-model.md)).
- **Fetching is an Operator power.** An archive holds every Person's recipes, Attempts and shopping lists in one file. An ordinary Person may not read another Person's recipes, so an ordinary Person may not fetch the file that contains all of them.

## Considered options

- **A schedule the Operator sets**, as GLOSSARY.md first described it. Rejected for now: it buys a dial in exchange for a setting that can be wrong, a screen to set it on, and a way to end up with no Backups at all. The rotation above is what a dial would mostly be set to, and a later ticket can add the dial with the screen it belongs on.
- **Keep the last N archives, N configurable.** Rejected: with any N a self-hoster will accept, all N cover the same fortnight. Distance is the thing worth spending disk on.
- **Promote yesterday's daily into the weekly slot** rather than taking a fresh archive. Rejected: it looks thriftier and is not. The weekly slot would then hold whatever the daily happened to be seven days ago, which is the same file the monthly would later inherit, and the three slots collapse toward one moment. Taking a fresh archive at each cadence keeps their ages honest.
- **Keep taking a Backup out of the Catalogue**, leaving only the schedule and the fetch. Rejected: GLOSSARY.md's own **Backup** entry opens "An Operation producing a single archive", and the criterion "tested through Operations" cannot be met by a feature no Operation reaches. `take_backup` is a **Job** rather than an Immediate Operation, because an archive is the size of the library and `Kind::Job` is defined as work too slow to answer within one request.
- **`cp` the database file, or `.backup` through a second connection.** `cp` is rejected outright: it can copy a torn database. The online backup API restarts whenever a writer commits mid-copy, which on a busy instance can mean never finishing; `VACUUM INTO` completes once and compacts as it goes.
- **tar.gz rather than zip.** Rejected on the one criterion that matters at four in the morning: an Operator on any machine can open a zip without installing anything. The database is deflated inside it; the Photographs are stored uncompressed, being compressed pictures already.
- **Wrap the archive in a JSON envelope as base64**, so it travels through the Catalogue like any Operation. Rejected: base64 of a gigabyte is a gigabyte and a third, held in memory, to say what a byte stream says. Photographs already travel out of band for the same reason ([ADR 0001](./0001-both-doors-generated-from-one-operation-catalogue.md)).

## Consequences

- **The most a Backup can lose is a day**, and that is the deliberate floor. Somebody who saves a recipe and immediately regrets it is served by the **Version** history, not by this.
- **Right after the monthly slot is refreshed, all three archives are nearly the same moment.** Unavoidable with one file per slot, and it lasts a day. The alternative is keeping more files. The first run of a new instance is the same situation: an empty slot is due, so all three are filled at once and pull apart from there.
- **`/data` is up to four times the size of the library.** Stated plainly because it is the price of the feature, and because [ADR 0028](./0028-kamosu-is-one-file-and-one-directory.md) made `/data` the one thing to carry: whoever mounts it is now mounting the backups too.
- **A Backup is not a **Snapshot** and the two never share a mechanism.** A Snapshot is the database alone, taken automatically before a migration, about the last few minutes ([ADR 0030](./0030-an-upgrade-goes-forward-only.md)). Building one thing to do both jobs would have done neither well.
- **An instance restarted more often than once an hour is never backed up**, because the first wake-up is always an hour after startup. Accepted, and stated rather than hidden: a machine rebooting that often has a worse problem than a stale Backup. The alternative — checking at startup — copies the whole library on every restart of a busy day, which is how an Operator learns to dread restarting.
- **Nothing carries an archive off the machine.** If the disk dies, so do the Backups. That is the honest boundary of a program that never uploads anything: whatever already backs the machine up is what makes these useful, and `/data` is the one directory it needs.
