# Issue tracker: GitHub

Issues and specs for this repo live as GitHub issues. Use the `gh` CLI for all operations.

## Conventions

- **Create an issue**: `gh issue create --title "..." --body "..."`. Use a heredoc for multi-line bodies.
- **Read an issue**: two calls, both `--json` (see [Reading an issue needs `--json`](#reading-an-issue-needs---json)):
  `gh issue view <number> --json number,title,state,assignees,labels,body,url`
  then `gh issue view <number> --json comments`. **Read both.** A ticket's brief
  is as often a comment as the body, so the body alone can miss the contract.
- **List issues**: `gh issue list --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'` with appropriate `--label` and `--state` filters.
- **Comment on an issue**: `gh issue comment <number> --body "..."`
- **Apply / remove labels**: `gh issue edit <number> --add-label "..."` / `--remove-label "..."`
- **Close**: `gh issue close <number> --comment "..."`
- **`ready-for-agent` goes on a ticket to build, never on a parent spec.** A spec
  whose work is its child tickets (each child names it under `## Parent`) is read
  when a child needs it and closed when its last child closes. Labelled, it is
  the oldest open issue in the queue, so every session fetches it whole before
  passing it over (#192 was, for eight sessions running).

Infer the repo from `git remote -v` — `gh` does this automatically when run inside a clone.

### Reading an issue needs `--json`

**`gh issue view <number>` fails on this repo in every form but `--json`.** Bare,
and with `--comments`, it prints one line and no issue:

```
GraphQL: Projects (classic) is being deprecated in favor of the new Projects
experience ... (repository.issue.projectCards)
```

The `gh` packaged for this host (2.45.0) asks for `projectCards` when it renders
an issue for a terminal, and GitHub has since removed Projects classic, so the
server rejects the whole query. `--json` takes a different path and works.

Read this as a tooling fault, never as an answer about the issue. It is a
one-line error on stdout, so it passes for "this issue has no comments" if you
are skimming, and Kamosu keeps briefs in comments.

`gh issue list`, `create`, `comment`, `edit` and `close` are all unaffected.
`--json stateReason` is rejected by this version too; ask for `closed` and
`closedAt`. **Upgrading `gh` past 2.45.0 is the actual fix** and would let this
section go.

## Pull requests as a triage surface

**PRs as a request surface: no.** _(Set to `yes` if this repo treats external PRs as feature requests; `/triage` reads this flag.)_

When set to `yes`, PRs run through the same labels and states as issues, using the `gh pr` equivalents:

- **Read a PR**: `gh pr view <number> --comments` and `gh pr diff <number>` for the
  diff. Untested here, since this repo has no pull requests; if it returns the
  same Projects-classic error the issue view does, reach for `--json` the same way.
- **List external PRs for triage**: `gh pr list --state open --json number,title,body,labels,author,authorAssociation,comments` then keep only `authorAssociation` of `CONTRIBUTOR`, `FIRST_TIME_CONTRIBUTOR`, or `NONE` (drop `OWNER`/`MEMBER`/`COLLABORATOR`).
- **Comment / label / close**: `gh pr comment`, `gh pr edit --add-label`/`--remove-label`, `gh pr close`.

GitHub shares one number space across issues and PRs, so a bare `#42` may be
either — resolve with `gh pr view 42` and fall back to
`gh issue view 42 --json number,title,url`.

## When a skill says "publish to the issue tracker"

Create a GitHub issue.

## When a skill says "fetch the relevant ticket"

Run `gh issue view <number> --json number,title,state,assignees,labels,body,url`
and `gh issue view <number> --json comments`, and read both.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a single issue with **child** issues as tickets.

- **Map**: a single issue labelled `wayfinder:map`, holding the Notes / Decisions-so-far / Fog body. `gh issue create --label wayfinder:map`.
- **Child ticket**: an issue linked to the map as a GitHub sub-issue (`gh api` on the sub-issues endpoint). Where sub-issues aren't enabled, add the child to a task list in the map body and put `Part of #<map>` at the top of the child body. Labels: `wayfinder:<type>` (`research`/`prototype`/`grilling`/`task`). Once claimed, the ticket is assigned to the driving dev.
- **Blocking**: GitHub's **native issue dependencies** — the canonical, UI-visible representation. Add an edge with `gh api --method POST repos/<owner>/<repo>/issues/<child>/dependencies/blocked_by -F issue_id=<blocker-db-id>`, where `<blocker-db-id>` is the blocker's numeric **database id** (`gh api repos/<owner>/<repo>/issues/<n> --jq .id`, _not_ the `#number` or `node_id`). GitHub reports `issue_dependencies_summary.blocked_by` (open blockers only — the live gate). Where dependencies aren't available, fall back to a `Blocked by: #<n>, #<n>` line at the top of the child body. A ticket is unblocked when every blocker is closed.
- **Frontier query**: list the map's open children (`gh issue list --state open`, scoped to the map's sub-issues / task list), drop any with an open blocker (`issue_dependencies_summary.blocked_by > 0`, or an open issue in the `Blocked by` line) or an assignee; first in map order wins.
- **Claim**: `gh issue edit <n> --add-assignee @me` — the session's first write.
- **Resolve**: `gh issue comment <n> --body "<answer>"`, then `gh issue close <n>`, then append a context pointer (gist + link) to the map's Decisions-so-far.

## Forgejo mirror

GitHub remains the only editable tracker. After a Wayfinder session, mirror its
tickets to Forgejo with:

```bash
scripts/sync-wayfinder-to-forgejo.sh --dry-run
scripts/sync-wayfinder-to-forgejo.sh --apply
```

The apply command needs `FORGEJO_TOKEN`, which **already exists** at
`~/.config/forgejo/kamosu.env` — source that file rather than hunting for a
token or minting a new one, and never print it. Setup and the deliberate
one-way limitations are in [Wayfinder Forgejo mirror](wayfinder-forgejo-mirror.md).
Never create or edit the Forgejo copy by hand.
