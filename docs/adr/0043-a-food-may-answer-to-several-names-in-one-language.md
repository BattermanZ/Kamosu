# A Food may answer to several names in one Language

A **Food** holds any number of names in a Language, in order. "œufs" and "œuf" can both be the eggs, and an Ingredient Line naming either reads as them. Matching stays exactly what [ADR 0022](./0022-doubt-makes-a-new-food-never-a-merge.md) says it is: same Language, equal after folding case and stray whitespace, accents and plurals respected. What changed is how many words a Food may hold, not how a word is compared.

- **The first name in a Language is the one shown.** A list, a shopping row and a Food's own heading show it. A person puts the names in the order they want.
- **A merge keeps every word.** The survivor answers to all of the absorbed Food's names as well as its own, after its own, in a Language it already has a name in too. Before this, a merge kept one name per Language and dropped the rest.
- **An arriving Food that hits one Food here teaches it every word it lacks**, which is ADR 0022's rule read literally now that a Food has room for them.
- **A Bundle carries every name.** `names` keeps the one name per Language it always held, the one shown, and `other_names` carries the rest. An instance written before this reads `names` alone, exactly as it did, so the sidecar's `format` stays 1 ([ADR 0020](./0020-a-bundle-is-one-recipes-worth-of-vault.md)).
- **One Operation says a Language's names.** `set_food_names` takes the whole list for one Language and replaces what was there, and an empty list takes the Language off. It replaces `set_food_name` and `remove_food_name`.

## Why

- **The same ingredient is written in both numbers.** A recipe library says "1 œuf" and "3 œufs", "1 avocat" and "2 avocats". Whichever form the Food was not named in read as a new Food and split the shopping list. Translating 12 recipes into French on the live instance on 2026-09-27, about a third of the corrections were singular/plural misses ([#179](https://github.com/BattermanZ/Kamosu/issues/179)).
- **The merge that was supposed to repair it did not stick.** Merging the stray "œuf" into "œufs" dropped the word "œuf", so the next "1 œuf" minted the same stray again. ADR 0022 promised that the near-duplicate tail decays once a cluster is merged. With one name per Language, that held across Languages and failed within one.
- **Nothing is guessed.** A person, or a merge the Operator chose, puts both words on the Food. ADR 0022's case against a plural rule stands: a per-Language rule for how words pluralise is usually right and wrong in the welding direction, and a weld cannot be undone.

## Considered options

- **A small plural rule per Language** (French adds *s* or *x*, English *s*, *es* or *ies*). Rejected: it guesses, and it welds the pairs a cook keeps apart. It would also have superseded part of ADR 0022.
- **Several names and the plural rule as a fallback.** Rejected for the same reason: the guessing is the risk, whether or not it runs first.
- **Several names per Language** (chosen on 2026-09-27).

## Consequences

- **The first miss still happens.** A form nobody has given the Food yet still mints a new Food, and doubt still makes a new Food, never a merge. What changes is that one repair, a merge or a name typed onto the Food, fixes it for good.
- **The Food page shows a Language's names on one row**, "œufs · œuf", and edits them together (#179, option B).
- **Migration 40** rebuilt `food_names` with the folded word in its key and a `position`. Every name held before was the only one in its Language, so it became the first and is still the one shown.
