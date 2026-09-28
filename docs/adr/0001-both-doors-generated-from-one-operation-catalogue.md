# Both doors are generated from one Operation Catalogue

Kamosu must guarantee that agents using the MCP server can do exactly what web users can do. We define every Operation once — name, input, output, required permission, and the function that performs it — in a single Catalogue, and build **both** the HTTP routes and the MCP tool list by walking that Catalogue at startup. Parity is therefore a property of how the program is assembled rather than a rule someone has to remember: there is no place to hand-write a web-only route, so adding a web feature makes its MCP tool appear in the same commit.

## Considered options

- **A shared core with two hand-written doors.** This is what Hatchdoor does, and it is correct there. It leaves parity resting on discipline, which is a poor bet for a codebase no human reviews closely.
- **A shared core plus a conformance test** asserting both doors cover a declared list. Drift is caught, but only *after* a mismatched door has been written — and a test that annoys someone can be weakened or skipped.
- **Catalogue-generated doors** (chosen). Drift is not detected; it is unrepresentable.

## Consequences

- **The web API is RPC-shaped** (`POST /api/op/create_recipe`), not REST-shaped (`POST /api/recipes`). This is deliberate. A future reader will find it unconventional and may be tempted to "fix" it by hand-writing REST routes — **doing so destroys the parity guarantee** and is the specific mistake this record exists to prevent. For a PWA talking to its own backend the RPC shape costs nothing real, and it lets the frontend generate a typed client from the same Catalogue.
- **Authorisation belongs in the Core, beneath both doors.** Parity is total at the Catalogue level; what scopes an actor is the Credential, not which door it came through. A permission check written inside a door is a bug.
- **Slow work needs a Job, not a longer request.** Anything that cannot answer within a single request returns a job id watched through ordinary Operations, because polling behaves identically for a browser and an agent. A progress mechanism only the web can use — a streaming connection, say — would break parity; such a thing may exist only as decoration over a Job.
- **Non-JSON payloads need a declared path.** Photos and PDF exports travel by out-of-band upload authenticated with the same Credential, with base64 as the fallback, so both doors retain the same ability.
- **Read-only MCP becomes trivial** — a filter applied while walking the Catalogue.
