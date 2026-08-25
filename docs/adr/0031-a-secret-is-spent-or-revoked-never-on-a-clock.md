# A Secret is spent or revoked, never on a clock

Kamosu has one kind of secret and one rule for it. A **Secret** is a long random string where holding it *is* the permission — the **Share Link** token, the **Access Key**, the **Invite**, the recovery link, and the **Session** a browser holds after logging in. Every one is **256 bits from the operating system's random source, stored hashed, revocable on its own, and never on a clock**. A Secret ends one of exactly two ways: it is **spent** (an Invite and a recovery link die on first use) or it is **revoked** (you turn it off). Time never kills one.

The password is the sole exception, and it is not a Secret: it is short, human-chosen, and never stored — only an **Argon2id** hash of it is, tuned to roughly a tenth of a second and ~20 MB per check. Argon2id salts every password separately and stores the salt inside the hash; that is automatic and is recorded here so it is not later mistaken for an oversight.

- **A Session is a Secret.** Logging in mints one, carried in a cookie that is `HttpOnly`, `Secure` and `SameSite=Lax`. It appears in a list beside the Person's Access Keys — *iPhone, last used 2 hours ago* — and is ended from any device.
- **An Access Key acts as the Person**, across every Kitchen they cook in. It cannot mint another Key, change the password, or mint an Invite: an agent cannot turn a leak into an account.
- **A Secret is shown once**, at minting, and is otherwise known only by its name and when it was last used.
- **Kamosu throttles where the secret is human-chosen, and nowhere else.** A wrong password makes the *next* attempt on that account name slower — nothing, nothing, 1s, 2, 4, 8, 16, capped at 30 — and a correct password clears the counter instantly. Nothing else is throttled.

## Why

- **Expiry is a wall Kamosu cannot build.** [ADR 0018](./0018-a-share-begins-at-the-beginning-and-there-is-no-unsay.md) already established there is no unsay: ending a Share Link "reaches no copy already sent." A clock on one would be exactly that pretence. The same runs down the list — a 90-day Access Key does not protect a Key that leaked on day 3; it annoys you on day 90. **Spent-or-revoked is a real defence; a clock is theatre**, and this house does not ship walls it cannot build.
- **The clock would fire on the household, not on an attacker.** [ADR 0013](./0013-offline-you-may-write-your-own-history-never-the-recipes.md) makes the installed app work with no server at all, so a session expiry is not discovered at a laptop — it is discovered by a phone that has been offline for a fortnight, at the stove, when the thing needed is a recipe already on the device.
- **A lockout is a weapon handed to the attacker.** Freezing an account after N failures means anyone who knows an account name can lock the household out of its own cookbook, for free, without ever guessing right. That is a worse outcome than the attack it prevents. Slowing the door has no state a stranger can push you into that the correct password does not clear in one keystroke.
- **Counting per account name rather than per address sidesteps the proxy entirely.** Behind a reverse proxy ([ADR 0028](./0028-kamosu-is-one-file-and-one-directory.md)) every request arrives looking like the proxy, and the visitor's real address survives only in a header the visitor can forge — so a per-address limit is either useless or bypassable. The account is the thing being attacked and is therefore the right unit. The same delay applies to names that do not exist, so the login page does not answer *who has an account here*.
- **256 bits makes enumeration impossible, not merely slow**, which is why nothing but the password is throttled. Rate-limiting Share Link lookups would defend a door with no handle while implying the entropy was not enough.
- **Two things that look alike are kept apart.** A Session and an Access Key share a shape but not their powers: a Key is minted deliberately for an agent, may be read-only, and can mint nothing; a Session belongs to a person who typed a password and has that person's full powers. The glossary's rule that a **Credential** comes from one of two places stays true.
- **A leaked Key is answered by visibility, not by restriction.** Scoping a Key to one Kitchen would be a wall to explain at every Operation, against a model where a Person cooking in several Kitchens is one fact about them. A name and a last-used time make an unused Key visible, which is the failure this actually has.

## Considered options

- **A rule per secret** — Invites expiring in 7 days, recovery links in 15 minutes, Keys renewed at 90 days, Sessions at 30. Rejected: five policies to explain and get wrong, in a project that has repeatedly collapsed scales ([ADR 0026](./0026-a-recipe-is-seen-by-its-kitchen-or-by-anyone-with-the-link.md), [ADR 0022](./0022-doubt-makes-a-new-food-never-a-merge.md)), and each of the five defends less than it appears to.
- **Account lockout after N failed passwords.** Rejected above: a free denial of service against the household.
- **No throttling at all**, relying on Argon2id's tenth of a second. Rejected as thin rather than wrong: it still permits ~850,000 guesses a day against a box that is on the open internet by design.
- **A Session *is* an Access Key**, minted automatically and named for the device. Rejected: it collapses two things with genuinely different powers.
- **Scoping an Access Key to a Kitchen.** Rejected: a wall against the grain of [ADR 0007](./0007-a-recipe-is-held-by-a-kitchen-not-a-person.md).
- **A pepper** — one secret mixed into every password and kept outside the database. Rejected: it defends the case where the database is stolen without the filesystem, which [ADR 0028](./0028-kamosu-is-one-file-and-one-directory.md) makes impossible by putting everything in one directory. In `/data` it is stolen alongside the database; as an environment variable it breaks [#27](https://github.com/BattermanZ/Kamosu/issues/27)'s rule that no environment variable is required at all, and losing it kills every password in the house — a new way to brick an install, in an app whose packaging exists to remove those. A pepper on the same disk is a wall contradicting the glossary's own sentence that whoever holds the disk holds everything.

## Consequences

- **An Invite you sent and forgot stays live until you revoke it.** Accepted, and answered by showing the live ones rather than by a timer.
- **A stolen unlocked phone is a logged-in phone.** The answer is ending that session from another device — immediate, rather than up to thirty days late. Recorded in [ADR 0034](./0034-kamosu-names-what-it-does-not-defend.md).
- **Six wrong passwords in a row means the seventh waits 16 seconds.** That is the whole penalty, and it is gone the moment you get it right.
- **Every Secret must be generated from a cryptographic source, never a general-purpose random number generator.** One lapse is indistinguishable from correct behaviour until someone exploits it.
- **The Sessions-and-Keys list is a v1 screen**, not a nicety: revocation is the only answer this ADR gives to every loss, so it has to exist and be reachable.
