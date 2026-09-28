# A photograph is known by its contents

A **Photograph** is identified by the picture itself, not by a filename, a place on disk or a number handed out when it arrives. Two identical pictures are one Photograph; a Photograph never changes once it exists; and it is stored once however many Versions, Attempts and recipes point at it.

A Photograph is **part of what a Version is**, and therefore part of what a Version's fingerprint covers ([ADR 0004](./0004-one-lineage-many-branches.md)). Replacing the Main Photo is an edit to the recipe, exactly like rewording a step, and mints a Version.

Whatever a person hands over is **remade at the door** into one agreed form — **WebP, quality 80, at most 2560 pixels on the long edge**, with the camera's attached metadata stripped — and it is that remade file, not the file that was handed over, which the Photograph is and which travels in a share bundle. Remaking happens **only where a Photograph first enters Kamosu**: one arriving inside a bundle is already made, and is stored exactly as it came.

A **Display Copy** is the Photograph reduced for a screen, in a small fixed set of sizes — **1400 pixels** for a recipe page on a phone, **600** for a card on a shelf — with a large screen served the Photograph itself. Display Copies are worked out from the Photograph, belong to no Version, travel in nothing, and may be thrown away and remade.

Photographs nothing points at are removed by a **sweep** that runs daily, works out afresh from the database what is unreferenced, and deletes only what has been unreferenced for a week.

## Why

- **The alternative to content identity is an identity that has to be agreed.** A number handed out on arrival means two instances holding the same picture disagree about what it is, and a Version's fingerprint stops being computable without asking somebody — the precise thing [ADR 0004](./0004-one-lineage-many-branches.md) built fingerprints to avoid. Content identity needs no registry, no server to ask and no clocks to agree on, for pictures for the same reason it needed none for words.
- **Photographs were already inside a Version; this only says how.** ADR 0004 states that "photographs belonging only to superseded Versions are not carried" in a bundle. That sentence presupposes Versions owning photographs. Putting them outside the fingerprint would have been reopening a settled decision, not making a fresh one.
- **A Step's photo has nowhere else to live.** *This is what the dough should look like now* is attached to step 4, and steps exist only inside a Version. Held outside, inserting a step in a later Version would slide the photograph silently onto the wrong instruction, and nothing would report it.
- **The divergence risk is real but self-inflicted, so it is forbidden rather than mitigated.** Two people would be reported as diverged if something re-squeezed the same picture into different bytes. Nothing does: a Photograph is remade once, at the door, before it has an identity, and never again. Display Copies are derived and never travel, so changing their sizes or format cannot cause a phantom divergence.
- **Remaking at the door is what makes a bundle openable.** iPhones do not shoot JPEG; they shoot HEIC, which desktop browsers will not display. A bundle is asked to "survive email, a memory stick and ten years" ([ADR 0004](./0004-one-lineage-many-branches.md)), and a format lottery is what that sentence forbids. The decoder was needed anyway to make Display Copies, so normalising bought size control at no extra machinery.
- **The stripped metadata is the only invisible privacy leak in the design.** A bundle is self-contained and permanent by intent, so a GPS tag inside one is permanent too, and neither the sender nor the receiver would ever see it. A cookbook has no use for where the cook stood. The date a photo was taken goes with it and costs nothing, because an Attempt already carries its own date ([ADR 0010](./0010-a-cook-in-progress-is-an-unfinished-attempt.md)).
- **The store is machinery; the readable rendering is the Vault.** [ADR 0003](./0003-the-database-is-the-truth-the-vault-is-a-published-copy.md) already assigned the job of being human-readable to something else. Filing photographs under recipe names would need renaming when a title changes, would have to pick an owner for a picture belonging to ten Versions and two recipes, and would compete with the Vault at the one thing the Vault exists for. Every way *out* — Backup, Export, PDF, and one day the Vault — writes proper names.
- **Nothing about Promotion needed inventing.** An Attempt photograph promoted to Main Photo is the same Photograph, so the new Version points at what already exists: nothing is moved, nothing is copied, and deleting the Attempt afterwards cannot take the picture with it. This is why an Attempt photograph is stored at full size like any other — storing cooking photos small would produce a blurry Main Photo months later.
- **A sweep that recomputes cannot be wrong; a running tally can drift.** One missed decrement deletes a picture still on screen, unrecoverably. The week of grace exists because deleting a cooking is an ordinary, ceremony-free act ([ADR 0005](./0005-an-attempt-is-a-version-you-cooked-but-did-not-keep.md)) and should not destroy something permanently as a side effect; within that week the nightly **Backup** still holds it.

## The facts that decided it

Measured directly from Aurélien's 86-recipe Crouton export ([#5](https://github.com/BattermanZ/Kamosu/issues/5)), and by encoding his own camera originals on the target server:

- **The library is 60 photographs, not 86.** 27 recipes carry none, 58 carry one, one carries two. Decoded, they are **82 MB** — the export's oft-quoted 110 MB is the archive, not the pictures.
- **`sourceImage` is not a second copy of the photo.** On all 44 recipes that have one it is the **source website's favicon**: median **32 pixels wide, 2.2 KB**, 0.17 MB for all 44 together. Seven are the identical Instagram icon. Not one matches a real photograph. The importer's supposed deduplication question did not exist.
- **There are no duplicate photographs at all.** Every duplicate-byte group in the whole library was a favicon. Storing once and pointing at it saves nothing today; it earns its place on identity, not on disk.
- **All 60 carry an EXIF block, holding orientation and nothing else** — no camera model, no date, no GPS, Crouton having already scrubbed them. **Eight are flagged "rotate 90°"**, which is why the flag survives stripping. Eleven carry a colour profile, which is why that survives too. Photographs taken in Kamosu, straight off a phone, will carry what Crouton removed.
- **Encoded on the target server (4 cores), ten real camera originals at 2560px:** JPEG q80 **721 KB** in 0.17 s; **WebP q80 380 KB** in 0.50 s; AVIF fast **209 KB** in 1.9 s; AVIF slow **186 KB** in 6.5 s.
- **Display Copy sizes, same photographs:** 600px **42 KB**, 1200px **125 KB**, 1320px **146 KB**, **1400px 160 KB**.
- **Browser support decides nothing.** WebP is at **96.2%** globally and in iOS since 14; AVIF is at **94.7%** and in iOS since 16. Kamosu already requires iOS 18.4 for the cooking wake lock ([ADR 0013](./0013-offline-you-may-write-your-own-history-never-the-recipes.md)), so both are simply present on every device it targets.
- **1200 was wrong.** The iPhone 16 Pro Max is **1320 physical pixels** wide (440 points at 3×), and [ADR 0011](./0011-cooking-shows-the-amounts-for-the-step-it-is-on.md) puts a photograph full-bleed at the top of every recipe page — the one place softness shows most.

## What was rejected

- **AVIF**, which is roughly half the size of WebP again. Its win buys nothing that is needed: the only place photograph size is load-bearing is [ADR 0013](./0013-offline-you-may-write-your-own-history-never-the-recipes.md)'s promise that the whole library fits on a phone, and WebP already meets it at about **12 MB** against a 13 MB budget. There is no second prize for winning by more, and AVIF would have cost four times the encoding on every photograph forever. Should that change, re-encoding is a background Job over files Kamosu owns.
- **Storing exactly the bytes that arrived.** Faithful, and it leaks the coordinates of the cook's kitchen into every bundle they ever send, invisibly and permanently.
- **Keeping the untouched upload alongside the remade one.** It gives back the size problem the remaking solves, and the second copy never travels and is never shown, so nobody would ever see it.
- **A second, human-named folder tree** mirroring the library. See above: it cannot name the case that matters.
- **One Display Copy size.** A shelf of 60 cards built from 1400px pictures is roughly a quarter of a gigabyte of decoding to draw pictures the size of a stamp — on iOS, the platform [ADR 0012](./0012-the-compiler-is-the-reviewer-the-frontend-does-not-have.md) chose to test against because it is tightest. The card size costs 42 KB and a thirtieth of a second, made once.
- **Deleting an orphan the moment it is orphaned**, which would make an ordinary act destroy something unrecoverably as a side effect, and **never sweeping automatically**, which puts housekeeping on a person who does not know the feature exists.
- **Keeping `sourceImage` as a site icon.** It would be a fourth kind of image in a model that has three, needing its own answer to every question settled here — fingerprint, bundle, Display Copies, sweep — for a 32-pixel decoration. [#6](https://github.com/BattermanZ/Kamosu/issues/6) already ruled that Source is attribution, never identity.

## Consequences

- **Replacing a photograph mints a Version**, and the history says so. Accepted as honest: the picture is part of what you would hand someone.
- **A Backup taken from Kamosu does not return the bytes that left the phone.** It returns the remade Photograph. Stated here so it is not discovered later.
- **The data directory is not browsable.** Photographs sit under names that are their own contents. Anyone wanting readable files runs a **Backup**, a v1 Operation at both doors ([#7](https://github.com/BattermanZ/Kamosu/issues/7)).
- **Display Copy sizes and format are free to change in v2**, over the library that already exists, without touching a recipe — they are computed-on-top under [ADR 0009](./0009-the-v1-cut-line-is-drawn-at-shape-not-at-features.md). The **Photograph's** own form is stored shape and is not.
- **31% of real recipes have no photograph**, and ADR 0011 does not say what leads a recipe page when there is none. Handed to [#29](https://github.com/BattermanZ/Kamosu/issues/29) as a requirement.
