# Kamosu assumes the proxy did nothing but carry the bytes

[ADR 0028](./0028-kamosu-is-one-file-and-one-directory.md) publishes one port of plain HTTP and puts TLS on a reverse proxy. That is the whole of the proxy's job in v1. Everything about *who may do what* is Kamosu's, and Kamosu never assumes the proxy did any of it.

The test of the rule: **Kamosu exposed directly on port 5266 with no proxy at all is exactly as safe, minus encryption in transit.**

- **Kamosu never trusts `X-Forwarded-For`** — a header the visitor writes. [ADR 0031](./0031-a-secret-is-spent-or-revoked-never-on-a-clock.md) removed the only reason to want it by counting failed passwords per account name.
- **Kamosu never relies on the proxy for authentication or rate limiting.**
- **The public address is the one thing Kamosu cannot infer** and therefore asks for, as an Operator setting ([#27](https://github.com/BattermanZ/Kamosu/issues/27)).

**And the rule turns outward as well as inward.** When Kamosu fetches a URL — a recipe page, and the picture it names — it connects **only to public internet addresses**, refusing private, loopback, link-local, CGNAT and IPv4-embedded-in-IPv6 targets, checked on the **address actually being dialled** and re-checked on **every redirect**. It reads at most 25 MB of an image and 5 MB of HTML whatever size the far end declares, gives up after 10 seconds and 5 redirects, and identifies itself as `Kamosu/1.0`.

This is done with **`reqwest-ssrf-guard`** (MIT, one file, 1,132 lines, dependencies `ipnet` and `reqwest`, 48 tests), pinned.

## Why

- **A proxy guards the way in; nothing guards the way out.** Kamosu sits on a home LAN, so a URL it can be talked into fetching is a request made *as a trusted machine on that network*. A recipe page's `image` field is an ordinary URL, and a redirect can point inward after a benign first hop.
- **This bites Kamosu specifically.** [ADR 0001](./0001-both-doors-generated-from-one-operation-catalogue.md) makes an agent a first-class door, and an agent decides what to import by **reading text** — including text on the page it is importing. *Import this recipe* is a sentence a stranger can put on a web page. Every other self-hosted recipe app has a human between the page and the fetch; Kamosu, by design, does not.
- **The check must bind on the address, not the name.** A hostname is a lookup, and whoever owns the name controls what it resolves to — including a public address the first time it is asked and `192.168.1.1` the second. `reqwest-ssrf-guard` implements `reqwest`'s resolver so filtering happens on the addresses actually returned, and `reqwest` then dials that filtered list; a separate policy re-checks each redirect hop.
- **Taking the crate was decided by reading it, not by counting its downloads.** It has 860 downloads, and rejecting it on that basis was wrong: it handles NAT64 and 6to4 IPv4 embedding, IPv4-mapped IPv6, CGNAT `100.64.0.0/10` (where Alibaba's metadata endpoint lives) and `0.0.0.0/8` — four published bypasses that a hand-rolled check would have missed **while looking complete**, which is the worst failure mode security code has. Its small size is an argument *for* it: one file is auditable in an evening and pinnable.
- **A size a server declares is a claim, not a fact**, so what is capped is what is read.
- **Kamosu says who it is.** Sending a browser's identity to get past blocks is the same species of pretence [ADR 0015](./0015-a-name-is-not-identification-it-is-a-reminder.md) refuses when it declines to imply a check it does not make.

## Considered options

- **Trust `X-Forwarded-For` when a proxy is configured.** Rejected: "configured" is itself a claim, and nothing needed the address once throttling moved to the account name.
- **Ship a "put an auth proxy in front" story.** Rejected: it makes Kamosu unsafe for everyone who does not, and Kamosu cannot tell which they are.
- **Fetch any address a user gives.** Rejected: only account holders can ask, but an agent holding a Key is an account holder taking instructions from text.
- **An Operator setting to allow the LAN.** Rejected: [#27](https://github.com/BattermanZ/Kamosu/issues/27) held the settings list to five deliberate items and called the rest "decided, not knobs." A switch labelled *let Kamosu reach my home network* is a wall shipped with a documented hole, flipped by the person least able to weigh it.
- **Write the address check by hand** (~30 lines). Rejected on reading the alternative: see above.
- **`url_jail`.** Rejected on code: 6,000 lines across nine modules, including a Python binding Kamosu would compile and never use and its own HTTP wrapper — a larger surface doing more than the one job, against a competitor doing exactly it in one file.

## Consequences

- **A recipe page hosted on your own network cannot be imported.** Opening it in a browser and pasting still works; Bundle and file imports are untouched.
- **Some sites will refuse `Kamosu/1.0`.** Accepted as the price of not pretending to be a browser.
- **`reqwest-ssrf-guard` is a security dependency and is pinned**, with its resolver *and* its redirect policy installed together — its own documentation warns that installing one without the other leaves a gap.
- **Every outbound fetch goes through the one guarded client.** A second HTTP client built anywhere in the codebase is a hole, which makes this a rule the build must enforce rather than remember.
