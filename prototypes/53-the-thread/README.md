# Prototype — how a forking Thread reads on a phone (issue #53)

**Throwaway. Not production code.**

> Three variants of the Thread, switchable via `?v=a|b|c&s=small|large`, on a
> new throwaway page — this project has no Thread screen yet to adjust.

[#18](https://github.com/BattermanZ/Kamosu/issues/18) already answered "what does
a divergence look like" and chose **the thread**: one line, forking where two
Branches part, Attempts hanging off it. That decision is not being reopened here.

What #18 never tested, and what #53 is actually deciding:

- **More than two Branches.**
- **A Branch forking off another Branch**, not the trunk — Camille's French
  translation forks off *Marc's* Branch here, not the original.
- **Real scale.** The ticket's own bar: *"legible with three Versions and
  illegible with thirty is not a solution."*

## Run it

```bash
cd prototypes/53-the-thread
python3 -m http.server <port> --bind 0.0.0.0
```

Open `http://<LAN-IP>:<port>/` on a phone. No build step, no dependencies.

Switch directions with the floating black bar or the `←`/`→` arrow keys.
Switch dataset size with the **Small** / **Large** buttons in the same bar.

## The three directions

All three render the *same* fixture — Korean Fried Chicken, real content
carried over from #18 (Aurélien's actual Crouton export), extended with 25
more Versions across two more Branches, invented but plausible.

| | Direction | The bet | The risk, on purpose |
|---|---|---|---|
| **A** | **Lanes** | A fork is a place: two columns, side by side, so you see both sides of the divergence at once. | A phone has room for two columns, not three — a *second* fork inside a rail (Camille off Marc) drops out of the grid and re-forks full-width below it. Judge whether that reads as continuity or as a seam. |
| **B** | **One column, pick a Branch** | A phone is one column; the thread should always be one column too, at any depth. At a fork, chips ask which Branch to follow forward. | You can never see two Branches at once — comparing Marc's and yours costs a tap you might not think to make. |
| **C** | **One list, no graph** | Every Version and Attempt from every Branch, merged by real date, tagged with a coloured letter instead of a lane. Nothing ever gets spatially wider, so it should hold up the same at 3 or at 30. | Once two Branches are active in the same week, their rows interleave — judge whether the letter tag alone keeps "whose save is this" straight without a lane to lean on. |

## The content is real

Versions v1, v2, m1 and m2 are lifted verbatim from #18, which lifted them from
Aurélien's own Crouton export (`Korean Fried Chicken.crumb` /
`Korean Fried Chicken-1.crumb`). Everything after — Aurélien's 14 more saves,
Marc's 4 more, and Camille's French translation forking off Marc's — is
invented, built to give each direction a genuine run of quiet, unremarked
saves to collapse or not, and a fork that isn't off the trunk.

Attempts are scattered across both the trunk and both later Branches,
including one Version (`v2`) carrying two Attempts by two different people —
the fixture that exposed the need to key an Attempt by its own index rather
than by the Version it is pinned to, since a Version can be cooked more than
once.

## What was chosen

*(Filled in once Aurélien picks — see [issue #53](https://github.com/BattermanZ/Kamosu/issues/53).)*

## What it does not answer, and knows it

- **The recipe itself.** Tapping a Version opens a small sheet with its name
  and *what changed* line, not the full ingredients/steps read-through —
  that page already exists in spirit from #18's `shared.js` and isn't what's
  being judged here.
- **More than three Branches at once**, or a fork three levels deep. The
  fixture goes two levels (trunk → Marc → Camille) on purpose, because that
  is the deepest case the real data actually produces today, but the
  algorithms in each direction are written generically (`walkSegment` /
  `renderForkAt` recurse on however many children a Version actually has) —
  they were not special-cased to "exactly two levels."
- **Editing, saving, or any mutation.** Read-only, exactly like #18.

No back end, no persistence. State lives in memory and resets on reload.
