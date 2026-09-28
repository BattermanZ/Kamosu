# An upgrade goes forward only, and its undo is a file

The database is the cookbook ([ADR 0003](./0003-the-database-is-the-truth-the-vault-is-a-published-copy.md)), so a schema change carries real recipes rather than rebuilding from files. **Migrations only go forward.** Before any of them run, Kamosu copies the database to a **Snapshot**; going back is putting that file back.

On an ordinary `docker compose pull && up -d`:

1. Kamosu compares the schema version in the database against the one the binary expects.
2. Behind — it writes a **Snapshot** first, then runs each migration **inside a transaction**.
3. A migration that fails leaves the database exactly as it was. Kamosu **refuses to serve**, and the log names the migration that failed and where the Snapshot is. It does not retry and it does not start half-migrated.
4. An **older binary against a newer database refuses to start**, loudly.

There are no down-migrations, and no migration tool: rolling back is copying one file and running the old image.

## Why

- **SQLite rolls schema and data back together, and most databases do not.** Verified rather than assumed: a transaction adding a column, creating a table and rewriting every row, then failing, left the column gone, the table gone and the rows untouched. In MySQL an `ALTER TABLE` commits itself, which is why frameworks there need inverse migrations to crawl back out of a half-applied change. SQLite makes the failure case *nothing happened*, and the whole design rests on that.
- **A down-migration is the code path nobody runs until the worst possible moment.** It is written once, tested never, and by then the person running it is already having a bad day.
- **Half of them cannot be honest anyway.** Undoing *split this column in two* cannot invent back what was merged. A reversible migration that quietly loses data is worse than no reversible migration, because it looks like it worked.
- **The Snapshot is the database alone, and that is not a corner cut.** A **Photograph** cannot change once it exists ([ADR 0017](./0017-a-photograph-is-known-by-its-contents.md)), so a migration cannot touch one — the database by itself is a *complete* way back. It matters: Aurélien's database is a few megabytes where his photographs are 82 MB, and someone else's will be gigabytes. Copying those before every upgrade is how you teach people to avoid upgrading.
- **Refusing to start is the kind thing.** Running old code against a new schema is how a cookbook gets quietly mangled — no error, no crash, just wrong answers found months later. There is no data whose loss is more annoying and less recoverable than a recipe somebody's grandmother dictated.

## Considered options

- **Reversible migrations, up and down.** Rejected: untested by construction, and frequently lossy.
- **Take a full Backup before migrating.** Rejected: it copies gigabytes of photographs a migration provably cannot touch, on every upgrade.
- **Migrate on an explicit command rather than at startup.** Rejected: an upgrade is `pull` and `up -d`, and a second step people forget is a second way to run old code against a new schema.
- **Let an older binary open a newer database read-only.** Rejected: a cookbook you can read but not cook from is not a recovery, and it invites someone to write anyway.

## Consequences

- **You cannot undo an upgrade you did not notice for a week.** The Snapshot precedes the upgrade; everything cooked since lives only in the new database. That is what the scheduled **Backup** is for, and it is why the two are separate things with separate names rather than one mechanism serving both badly.
- **Photographs added after a Snapshot become orphans on rollback.** Harmless: [ADR 0017](./0017-a-photograph-is-known-by-its-contents.md) already sweeps orphans daily, a week late, recomputed each run.
- **The database must run in WAL mode**, which [ADR 0028](./0028-kamosu-is-one-file-and-one-directory.md) needs anyway so the recovery command can open the same file as the running server.
- **A migration is now a thing that must be written carefully once**, since there is no way back except a file. Accepted: that is the honest cost of the database being the truth.
