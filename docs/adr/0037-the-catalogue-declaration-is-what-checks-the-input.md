# The Catalogue's declaration is what checks the input

**An Operation's declared `input_schema` is what validates its input, compiled once and checked in the Core at dispatch.** Not in a Door, not per Operation, and not by a JSON Schema crate. `src/schema.rs` holds the compiler and the check; a declaration it cannot compile breaks the build in `tests/parity.rs`.

This closes the one declaration in Kamosu that was never wired to anything. Every Operation already said what it accepts — field names, types, which are required, `additionalProperties: false` — and that statement was load-bearing in three directions: the MCP door publishes it to agents, `just client` generates the interface's types from it, and `just check` fails when the generated copy drifts. It was load-bearing in none of the direction that matters when a call actually arrives.

## Why

- **The same fact was written down twice, with nothing forcing agreement.** Once in the Catalogue as a declaration, once in a handler as code. That is precisely the drift this codebase spends effort preventing everywhere else: both Doors are generated from one declaration list ([ADR 0001](./0001-both-doors-generated-from-one-operation-catalogue.md)), the typed client is generated from it, and `tests/parity.rs` exists only to break the build when the two Doors fall out of step.

- **The failure was destructive, not merely lenient.** Through the MCP door, `set_shopping_yield` called with `yield` misspelled did not ignore the field — it **erased** the stored Yield and reported success. An absent Yield legitimately means *back to the recipe as written*, so a typo and a deliberate reset were the same request. An assistant asked to shop for eight got a cheerful OK and a list for four, which is [ADR 0024](./0024-a-shopping-list-is-a-choice-of-recipes-not-a-list-of-things.md)'s own warning from the other end: *a thing that quietly disappears from a shopping list is a thing that does not get bought.*

  The dangerous shape is narrow and worth naming: **an optional field whose absence carries meaning.** A misspelt *required* field already failed loudly, because the handler's check for the missing field caught it by accident. `revoke_access_key` called with `key_id` was refused; `set_shopping_yield` called with `shopping_yield` was not.

- **The check belongs beneath both Doors for the same reason authorisation does.** A validation check written inside a Door is a bug exactly as a permission check inside a Door is one: it is the second copy that goes stale. Put it in the Core at dispatch and both Doors inherit it, and a new Operation cannot forget it any more than it can forget to appear at both.

- **The interface could never reach this and an agent always could.** The frontend calls through the generated client, so an undeclared field does not compile — verified by introducing one, which `svelte-check` refused with *"Object literal may only specify known properties."* The MCP door had no such protection, and an agent driving Kamosu is a first-class use of it, not a side door.

## Why not a JSON Schema crate

Because **a JSON Schema validator is required by its own specification to ignore a keyword it does not recognise** — so it cannot be the thing that catches a typo in the Catalogue.

Checked against `boon` 0.6.1 rather than argued: a schema written `"requird": ["branch_id"]` compiles without complaint and silently drops the requirement. That is this ADR's own failure mode, moved from the caller's input to Kamosu's declaration, and it would arrive with no error anywhere. `src/schema.rs` instead refuses every keyword it does not implement, which makes a misspelt keyword a failed build.

The size of the surface is the supporting argument, not the deciding one. Measured across all 85 Operations on 2026-09-03, the whole input surface is nine keywords — `type`, `properties`, `required`, `additionalProperties`, `items`, `enum`, `minimum`, `maximum`, `exclusiveMinimum` — plus `description` and `default`, which annotate and never validate. No `$ref`, no `oneOf`, no `pattern`, no `format`; `additionalProperties` is `false` in all 109 places it appears.

## Considered options

- **`jsonschema` 0.53.0.** Complete and well made, and it costs 31 crates Kamosu does not already build. Rejected on the specification argument above, which no implementation quality fixes.
- **`boon` 0.6.1.** Six crates, 6,210 lines, a competent implementation — and the one the ignore-unknown-keywords behaviour was measured on. Rejected for the same reason.
- **Unknown fields only, leaving types and enums to the handlers.** Smaller, and it would have closed the reported bug. Rejected as half a wiring: the declaration would still be a statement nothing fully honoured, and the second copy would still be in the handlers going stale.
- **A compiler that refuses what it does not implement** (chosen).

## Consequences

- **Sixty-five Operations began refusing what they silently accepted, and this was measured before it was turned on.** The enforcement was first run in report-only mode across the behaviour suite, the seven `--ignored` corpus tests against the real 86-recipe library, and the live interface at `https://kamosu-dev.batterlan.cc`. One real caller depended on lenient input, and it was Kamosu itself: 80 calls sent a step with no `photo`, which the shared content properties declared *required*. `parse_step_list` had always read an absent `photo` as "no photograph", so the declaration had simply never been true of input. It was the declaration that was wrong, not the callers — `recipe_content_input_properties` now says so.

- **Semantic checks did not move and never will.** Whether a Branch exists, whether a Person may touch it, whether an amount parses: none of that is a schema's job. What moved is shape alone.

- **What a handler still writes is extraction, not validation.** `input.get("x").and_then(Value::as_str).ok_or_else(…)` has to produce a `&str` either way; its error arm is now ordinarily unreachable, and it stays because the alternative is a panic.

- **`set_shopping_yield` and `add_to_shopping_list` take `shopping_yield`, not `yield`.** The Shopping List answers with `shopping_yield`, and the sibling `advance_attempt` takes `cooking_yield`, so the inputs had agreed with neither the output nor each other — and the plausible mistake was the destructive one. A breaking change to the API and the generated client, made while Kamosu is unreleased and its only caller is its own interface, because it will never be cheaper.

- **The check runs after authorisation.** A caller who may not perform an Operation learns nothing about its input.

- **Adding a keyword to a declaration means implementing it.** `"maximum"` was found this way: `probe_job` uses it behind the `test-jobs` feature, and the build broke until `src/schema.rs` knew it. That is the intended cost, and it is the same bargain `tests/parity.rs` already strikes.
