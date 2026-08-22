# Wayfinder Forgejo mirror

GitHub is the only source of truth for Kamosu's Wayfinder map and tickets.
Forgejo is a read-only operational mirror: it must never be edited by hand or
used as the source for a GitHub update.

`scripts/sync-wayfinder-to-forgejo.sh` mirrors every GitHub issue carrying a
`wayfinder:*` label. It copies title, body, state, and labels. GitHub's native
sub-issue edge becomes `Part of #<map>` at the top of the Forgejo body.

Run a preview first:

```bash
scripts/sync-wayfinder-to-forgejo.sh --dry-run
```

To write, the script needs `FORGEJO_TOKEN` — a Forgejo token that can read the
`battermanz/Kamosu` repository and write issues.

**The token already exists. It lives in `~/.config/forgejo/kamosu.env`**, mode
`600`, alongside one env file per project (`hatchdoor.env`, `swolenmate.env`).
That file also sets `FORGEJO_API`. Load it and run:

```bash
set -a; . ~/.config/forgejo/kamosu.env; set +a
scripts/sync-wayfinder-to-forgejo.sh --apply
```

Sourcing it this way keeps the secret out of the terminal, which matters when
an agent is driving: never `cat` the file, `echo $FORGEJO_TOKEN`, or pass the
token on a command line, because all three write it into a transcript that
outlives the session. It is deliberately outside the repository and outside
the shell profile, so it is absent from a fresh shell — a run that fails on a
missing token means the file was not sourced, not that no token exists.

If it is ever genuinely gone, mint a replacement in Forgejo under
*Settings → Applications*, and write it back to the same path rather than
somewhere new.

The script uses the existing `gh` login for GitHub. It stores the immutable
GitHub-node-ID → Forgejo-issue-number mapping outside the repository at
`$XDG_STATE_HOME/kamosu/wayfinder-forgejo.json` (or
`~/.local/state/kamosu/wayfinder-forgejo.json`). Back up that small file along
with the operator configuration; losing it would make the next run create
duplicate Forgejo tickets.

Run the apply command after a Wayfinder session and daily as a reconciliation
job. This first version is deliberately one-way; comments and Forgejo native
blocking edges are not mirrored yet, so Forgejo should remain a status view,
not a second working tracker.
