# Kamosu names what it does not defend

Kamosu ships a plainly written list of the things it does **not** protect you from, in the same voice [ADR 0015](./0015-a-name-is-not-identification-it-is-a-reminder.md) uses to say a **Hand** is never verified and [#11](https://github.com/BattermanZ/Kamosu/issues/11) uses to call the Operator boundary a courtesy. The list is part of the product, not an internal note.

- **Whoever holds the disk holds everything.** The Operator boundary is a courtesy. Nothing is encrypted at rest, and there is no pepper ([ADR 0031](./0031-a-secret-is-spent-or-revoked-never-on-a-clock.md)).
- **A Hand is not checked.** Anyone editing a Bundle can put any name in it.
- **A Share Link, once passed on, cannot be recalled.** Ending it stops new arrivals and reaches no copy already sent ([ADR 0018](./0018-a-share-begins-at-the-beginning-and-there-is-no-unsay.md)).
- **A stranger holding a Share Link can make another stranger wait.** The lane bounds it to latency ([ADR 0032](./0032-a-stranger-may-cause-work-never-work-that-scales-with-them.md)); it does not remove it.
- **A stolen unlocked phone is a logged-in phone.** The answer is ending that session, not a timer that would have fired too late anyway.
- **Someone who knows your name here can keep you from signing in while they keep guessing.** A try at a name still waiting out its wrong passwords is refused, not checked ([ADR 0031](./0031-a-secret-is-spent-or-revoked-never-on-a-clock.md), #138). A crowd of tries at made-up names can hold off everyone's login the same way while it lasts, since two passwords are checked at a time and a full line is refused. Devices already signed in are untouched, and a recovery link from an Operator gets round it.
- **A web page opened on your network before setup can do the setup itself.** Through DNS rebinding it can create the first Person as if it were Kamosu's own page, and nothing checks the Host (#153). It cannot sign in with what it made, since the Session cookie is always `Secure` and a rebound page is plain HTTP, but it chose the password. You meet "Welcome back" where you expected "Set up Kamosu"; empty `/data` on an instance that holds nothing yet, and never keep one you did not set up, since whoever did can sign in once it is reachable over HTTPS.
- **An agent does what it is told, including by text it reads.** An agent holding your **Access Key** reads recipe pages and Bundles written by other people, and Kamosu cannot tell an instruction from an ingredient. A read-only Key limits the blast; nothing prevents it.
- **Kamosu has not been audited by anyone.** It is a recipe app written for one household and shared.

## Why

- **This is where the house rule either holds or turns into marketing.** Kamosu has refused walls it cannot build at every turn — no verified Hand, no Operator who can read your recipes but also no pretence that they couldn't, no unsay on a Share Link. A security section is the single place where the temptation to imply more is strongest, because nobody checks.
- **The agent line is Kamosu's own, and almost nobody states it.** Making an agent a first-class door ([ADR 0001](./0001-both-doors-generated-from-one-operation-catalogue.md)) means removing the human who would otherwise sit between a web page's text and the instruction acted upon. That is a real, unfixed exposure created by a deliberate design choice, and the person minting a Key deserves to know it before they mint one.
- **Naming a gap is what makes the mitigations legible.** "Revoke the session" is obviously the right answer to a stolen phone only once it is written down that no timer will save you. Every item here is paired with the thing Kamosu *does* do instead.
- **The destination invites strangers to self-host.** Someone deciding whether to run this on a machine at home is owed the honest version, and this is cheaper to write now than to be asked for later.

## Considered options

- **Say nothing**, on the grounds that naming weaknesses helps attackers. Rejected: every item here is either public knowledge about how the internet works or a direct consequence of a design decision already documented in these ADRs. Silence would mislead only the people running it.
- **Soften the agent line** to something about "using trusted sources." Rejected: it implies a distinction Kamosu cannot draw.
- **Keep the list internal**, in `docs/` but not in front of users. Rejected: the people who need it are the ones deciding to install it.

## Consequences

- **The list must be maintained.** An entry that stops being true is worse than one that never existed, so a change to any of these ADRs is a change to this list.
- **Some of these are things a future version could fix** — encryption at rest, an audit — and doing so removes a line rather than adding a claim. That direction is deliberate.
