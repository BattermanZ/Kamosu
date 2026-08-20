# Kamosu

A self-hosted cookbook. People and software agents are equal users of it: everything a person can do to a recipe, an agent can do too.

## Language

### The shape of the system

**Operation**:
One thing Kamosu can be asked to do, defined once — its name, what it takes in, what it gives back, and the permission it requires. The unit of everything Kamosu offers.
_Avoid_: Endpoint, tool, command, action, API method

**Catalogue**:
The complete list of Operations. There is exactly one, and it is the sole source of what Kamosu can do.
_Avoid_: Registry, API surface, tool list, schema

**Core**:
Where Operations are carried out and where permission is checked. It is reached only through a Door and knows nothing about how a request arrived.
_Avoid_: Backend, service layer, business logic, engine

**Door**:
A way in to the Catalogue. Kamosu has two — the **web door** and the **MCP door** — and both offer the entire Catalogue. A Door translates between the outside world and the Core; it never decides what may be done.
_Avoid_: Adapter, interface, transport, front end, client

**Parity**:
The guarantee that both Doors offer the same Operations. It holds because the Doors are built from the Catalogue rather than written separately, so an Operation cannot exist at one Door alone.
_Avoid_: Feature parity, equivalence

### Doing and permission

**Credential**:
What a request presents to prove who is acting. Every Door resolves one before the Core runs anything, so the same permissions apply whether a person or an agent is asking. Obtaining a Credential is not an Operation.
_Avoid_: Token, session, auth, API key, login

**Immediate Operation**:
An Operation that answers within a single request.
_Avoid_: Sync operation

**Job**:
An Operation too slow to answer within a single request — importing a library, fetching a recipe from a website, producing a PDF. Asking for one returns a job id at once; its progress is then read through ordinary Operations.
_Avoid_: Task, background job, async operation, queue item

### Getting recipes in and out

**Import**:
Bringing recipes into Kamosu from elsewhere — another app's export, or a web page. An Operation like any other, available at both Doors.
_Avoid_: Migration, sync, ingest

**Export**:
Producing recipes in a form usable outside Kamosu. An Operation like any other.
_Avoid_: Download, backup, share
