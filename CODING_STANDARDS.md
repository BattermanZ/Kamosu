# Coding standards

Read at review, by whoever reviews a change. An implementer reads `AGENTS.md`.

Only judgement calls live here: what a reviewer has to look at and decide.
Whatever a machine can decide belongs in `just check` or `just test`, and a rule
that gains a check there leaves this file.

## The chosen prototype is the contract

When a ticket's comments record the choice of a prototype option, that option's
page is part of the spec, on a par with the ticket's own words. On the machine
that built it, the page is under `.dev/prototypes/<issue>-<slug>/`, with the
rules that draw it in `proto.css`. The comment recording the choice names the
page.

Compare the built screen with that page at the same size, control by control:
its shape, its size, where it sits, and what it says. Each difference is a
finding, unless a later comment on the ticket records that the person who chose
agreed to it.

Where the chosen page breaks a house rule (a radius token, the 48px control),
the finding is the clash itself, to be put to the person who chose. Building the
house rule's version in the page's place is the fault this rule exists for. On
#202 the chosen page drew − and + round and 40 across. They were built square
and 48 to match the recipe page, and both had to be redone.

A review that cannot reach the prototype folder says so in its report, and
leaves the point open.
