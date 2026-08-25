# A stranger may cause work, never work that scales with them

A **Share Link** is handed to people outside the house and freely passed on ([ADR 0026](./0026-a-recipe-is-seen-by-its-kitchen-or-by-anyone-with-the-link.md)), so the page it opens is served to anonymous strangers in unknown numbers — and that page offers a **Sheet** and a **Bundle**, both of which are **Jobs**. [ADR 0023](./0023-a-sheet-carries-the-recipe-not-the-library.md) saw this and parked it. The answer is not a rate limit:

**Work asked for by someone holding no Credential runs in its own single lane** — one at a time, behind a short waiting line, and when the line is full the page says *busy, try in a moment* rather than the queue growing. Members have their own lane and never wait behind strangers.

**And a Sheet is remembered.** A Version is frozen the moment it is written, so the same Version at the same page size and serving count renders byte-identical output forever. It is rendered once and kept, the same habit [ADR 0017](./0017-a-photograph-is-known-by-its-contents.md) applies to a **Display Copy**.

## Why

- **Concurrency is the thing that kills a server, not request count.** Ten thousand requests into a lane of one is ten thousand strangers told *busy* and a single Typst render. Ten thousand into an unbounded lane is ten thousand renders competing for the same memory on a machine in someone's house, which stops answering — including to the household. Capping the lane converts the entire exposure from **collapse** into **latency**, and latency a stranger can inflict only on other strangers.
- **A rate limit would need a client to limit**, and behind a proxy there isn't one ([ADR 0033](./0033-kamosu-assumes-the-proxy-did-nothing-but-carry-the-bytes.md)). A lane needs no identity at all, which is why it is the right instrument here and a throttle is not.
- **Remembering is a benefit rather than a defence.** The real case is one recipe sent to eight people; the eighth print costs nothing. It does not bound the work — servings is a number a stranger can vary — which is exactly why the lane, not the cache, is what makes the promise.
- **Removing the Sheet would remove the point of the Share Link.** What the person you sent it to wants is to print the recipe. Deleting the feature to avoid a cost a lane of depth one already bounds is a bad trade.
- **It states a property rather than a mechanism**, which is what [ADR 0009](./0009-the-v1-cut-line-is-drawn-at-shape-not-at-features.md) asks of v1: the sentence survives a change of queueing library, and a list of middlewares would not.

## Considered options

- **Unlimited.** Rejected: ADR 0023 flagged it in its own words — "a 5 MB download generated on demand by strangers on a home server."
- **Rate-limit the Share Link page.** Rejected: no trustworthy client identity exists behind a proxy, and the lane bounds the damage without needing one.
- **No Sheet and no Bundle for strangers**, making the Share Link read-only. Rejected: it guts the feature to solve a problem already solved.
- **Expire or throttle the Share Link token itself.** Rejected by [ADR 0031](./0031-a-secret-is-spent-or-revoked-never-on-a-clock.md): 256 bits cannot be enumerated, and a clock is refused.

## Consequences

- **A stranger can make another stranger wait.** If someone points a script at your Share Link, your mother's print spins for a few seconds. Nothing falls over and the household is unaffected — but it is not nothing, and it is named in [ADR 0034](./0034-kamosu-names-what-it-does-not-defend.md) rather than papered over.
- **Kept Sheets are derived files and must be sweepable**, like Display Copies: a Version they belong to can be superseded and they are regenerable, so they may be deleted at any time without loss.
- **The lane applies to every unauthenticated Job**, present and future — it is a property of *having no Credential*, not a property of Sheets. Anything later added to the Share Link page inherits it without a new decision.
