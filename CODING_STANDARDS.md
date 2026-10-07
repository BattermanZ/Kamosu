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

## Committed prose names a role, never a person

The files in this repository are for anyone who runs Kamosu, so none of them
names the person who made a decision, owns a device or supplied a library. Say
what was chosen and where that is recorded: "the rail chosen on #194". Where a
sentence needs somebody, write a role: "the person who chose", or "the
Operator" where the Operator of an instance is meant (`GLOSSARY.md`).

A test needs a cook with a name. That cook is invented, and where one stands
for "you" it is Stéphane: `Stéphane Dupont`, `p_stephane`, `stephane.example`.
A finding is any first name in a comment, a record or a message that belongs
to somebody real.

This covers what is committed. `AGENTS.local.md`, `.env` and `.dev/` stay on
one machine and may say who works there.
