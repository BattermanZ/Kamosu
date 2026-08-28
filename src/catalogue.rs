//! The Catalogue: the complete list of Operations, and the sole source of what
//! Kamosu can do (ADR 0001). Both Doors are built by walking this list at startup.
//!
//! There is nowhere to hand-write a web-only route. If a thing Kamosu does is not
//! declared here, neither Door offers it; if it is declared here, both do.

use serde_json::{Value, json};

use std::sync::LazyLock;

use crate::core::{Core, Invocation, OpError};

/// The permission an Operation requires. Checked in the Core — never in a Door.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    /// Any caller, including a stranger presenting no Credential at all.
    Public,
    /// A resolved Person: the Credential must name one, whether it came from
    /// an Access Key or a login Session.
    Person,
    /// A resolved Person who administers this instance.
    Operator,
}

/// Whether an Operation answers within a request or is too slow to.
///
/// Declared once, here — which is what keeps watching slow work a property of
/// every Door rather than a feature of one. A Job's declared output schema
/// describes its eventual result; the ask itself always answers the same
/// envelope, `{ "job_id": ... }`, read back through `get_job` and `list_jobs`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// An Operation that answers within a single request (CONTEXT.md).
    Immediate,
    /// An Operation too slow to answer within a single request: asking returns
    /// a job id at once; state, progress and result are ordinary Operations.
    Job,
}

/// One thing Kamosu can be asked to do, defined exactly once.
pub struct Operation {
    /// The Operation's name, used verbatim by both Doors (`/api/op/<name>`,
    /// and the MCP tool name).
    pub name: &'static str,
    /// A one-line description, carried into the MCP tool list.
    pub summary: &'static str,
    /// The permission required. Enforced by `Core::execute`, beneath both Doors.
    pub permission: Permission,
    /// Whether it answers at once or as a Job. Enforced by the Core as well:
    /// no handler decides for itself whether it is allowed to be slow.
    pub kind: Kind,
    /// Whether performing this Operation changes state. Enforced by the Core:
    /// a read-only Access Key's Credential is refused every Operation that
    /// carries this, and the MCP door does not list it as one of that Key's
    /// tools (ADR 0031) — the filter walks the Catalogue, never a Door.
    pub write: bool,
    /// Whether this Operation may be carried out only by a Person who logged
    /// in directly — never by an Access Key, however unrestricted. An Access
    /// Key can mint no Key, change no password and mint no Invite, so a leaked
    /// Key cannot become an account (ADR 0031).
    pub session_only: bool,
    /// JSON Schema describing the input envelope.
    pub input_schema: Value,
    /// JSON Schema describing the output envelope. For a Job, this describes
    /// the eventual result carried by `get_job`, not the immediate job id.
    pub output_schema: Value,
    /// The function that performs it.
    pub handler: fn(&Core, &Invocation, Value) -> Result<Value, OpError>,
}

/// The Catalogue itself. Exactly one exists.
///
/// Adding an Operation means adding one entry here. Both Doors materialise it on
/// the next build; `tests/parity.rs` makes any drift unbuildable rather than
/// merely visible.
pub static OPERATIONS: LazyLock<Vec<Operation>> = LazyLock::new(|| {
    // The demonstration Job exists only where tests can drive it end-to-end
    // through real Doors. Production instances ship no synthetic Operations;
    // their first Jobs arrive with Import, Sheet rendering and Meaning Search.
    // Without it, nothing is appended — hence the conditional mutability.
    #[cfg_attr(not(feature = "test-jobs"), allow(unused_mut))]
    let mut operations = vec![
        Operation {
            name: "instance_status",
            summary: "The version of this Kamosu and whether setup has happened.",
            permission: Permission::Public,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            input_schema: empty_input(),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "version": { "type": "string" },
                    "setup_complete": { "type": "boolean" },
                },
                "required": ["version", "setup_complete"],
                "additionalProperties": false,
            }),
            handler: crate::operations::instance_status,
        },
        Operation {
            name: "set_reading_preferences",
            summary: "Set the Language and measures this Person reads in.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({ "type": "object", "properties": { "reading_language": { "enum": ["en", "fr", "es"] }, "reading_measures": { "enum": ["us", "metric", "as_written"] } }, "required": ["reading_language", "reading_measures"], "additionalProperties": false }),
            output_schema: json!({ "type": "object", "properties": { "reading_language": { "type": "string" }, "reading_measures": { "type": "string" } }, "required": ["reading_language", "reading_measures"], "additionalProperties": false }),
            handler: crate::operations::set_reading_preferences,
        },
        Operation {
            name: "rename_person",
            summary: "Change this Person's current reminder name.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({ "type": "object", "properties": { "name": { "type": "string" } }, "required": ["name"], "additionalProperties": false }),
            output_schema: json!({ "type": "object", "properties": { "name": { "type": "string" } }, "required": ["name"], "additionalProperties": false }),
            handler: crate::operations::rename_person,
        },
        Operation {
            name: "mint_invite",
            summary: "Mint a one-use Invite link for a new Person.",
            permission: Permission::Operator,
            kind: Kind::Immediate,
            write: true,
            session_only: true,
            input_schema: json!({ "type": "object", "properties": { "is_operator": { "type": "boolean", "default": false } }, "additionalProperties": false }),
            output_schema: json!({ "type": "object", "properties": { "link": { "type": "string" } }, "required": ["link"], "additionalProperties": false }),
            handler: crate::operations::mint_invite,
        },
        Operation {
            name: "disable_account",
            summary: "Disable an account so it can no longer obtain a Credential.",
            permission: Permission::Operator,
            kind: Kind::Immediate,
            write: true,
            session_only: true,
            input_schema: json!({ "type": "object", "properties": { "name": { "type": "string" } }, "required": ["name"], "additionalProperties": false }),
            output_schema: json!({ "type": "object", "properties": { "disabled": { "type": "boolean" } }, "required": ["disabled"], "additionalProperties": false }),
            handler: crate::operations::disable_account,
        },
        Operation {
            name: "delete_account",
            summary: "Delete an account while preserving its Hand in history.",
            permission: Permission::Operator,
            kind: Kind::Immediate,
            write: true,
            session_only: true,
            input_schema: json!({ "type": "object", "properties": { "name": { "type": "string" } }, "required": ["name"], "additionalProperties": false }),
            output_schema: json!({ "type": "object", "properties": { "deleted": { "type": "boolean" } }, "required": ["deleted"], "additionalProperties": false }),
            handler: crate::operations::delete_account,
        },
        Operation {
            name: "mint_recovery_link",
            summary: "Mint a one-use recovery link for a Person who forgot their password.",
            permission: Permission::Operator,
            kind: Kind::Immediate,
            write: true,
            session_only: true,
            input_schema: json!({ "type": "object", "properties": { "name": { "type": "string" } }, "required": ["name"], "additionalProperties": false }),
            output_schema: json!({ "type": "object", "properties": { "link": { "type": "string"} }, "required": ["link"], "additionalProperties": false }),
            handler: crate::operations::mint_recovery_link,
        },
        Operation {
            name: "list_sessions",
            summary: "List this Person's browser Sessions by device and last use.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            input_schema: empty_input(),
            output_schema: json!({ "type": "object", "properties": { "sessions": { "type": "array", "items": { "type": "object", "properties": { "id": {"type":"string"}, "name": {"type":"string"}, "created_at": {"type":"string"}, "last_used_at": {"type":["string","null"]}, "revoked": {"type":"boolean"} }, "required":["id","name","created_at","last_used_at","revoked"], "additionalProperties": false } } }, "required":["sessions"], "additionalProperties": false }),
            handler: crate::operations::list_sessions,
        },
        Operation {
            name: "revoke_session",
            summary: "End one of your browser Sessions.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({ "type": "object", "properties": { "session_id": { "type": "string" } }, "required": ["session_id"], "additionalProperties": false }),
            output_schema: json!({ "type": "object", "properties": { "revoked": { "type": "boolean" } }, "required": ["revoked"], "additionalProperties": false }),
            handler: crate::operations::revoke_session,
        },
        Operation {
            name: "mint_access_key",
            summary: "Mint an Access Key for an agent to act as you, optionally read-only.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: true,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "read_only": { "type": "boolean", "default": false },
                },
                "required": ["name"],
                "additionalProperties": false,
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "name": { "type": "string" },
                    "read_only": { "type": "boolean" },
                    "secret": { "type": "string" },
                },
                "required": ["id", "name", "read_only", "secret"],
                "additionalProperties": false,
            }),
            handler: crate::operations::mint_access_key,
        },
        Operation {
            name: "list_access_keys",
            summary: "List this Person's Access Keys by name and last use.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            input_schema: empty_input(),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "access_keys": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": { "type": "string" },
                                "name": { "type": "string" },
                                "read_only": { "type": "boolean" },
                                "created_at": { "type": "string" },
                                "last_used_at": { "type": ["string", "null"] },
                                "revoked": { "type": "boolean" },
                            },
                            "required": ["id", "name", "read_only", "created_at", "last_used_at", "revoked"],
                            "additionalProperties": false,
                        },
                    },
                },
                "required": ["access_keys"],
                "additionalProperties": false,
            }),
            handler: crate::operations::list_access_keys,
        },
        Operation {
            name: "revoke_access_key",
            summary: "End one of your Access Keys.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({ "type": "object", "properties": { "access_key_id": { "type": "string" } }, "required": ["access_key_id"], "additionalProperties": false }),
            output_schema: json!({ "type": "object", "properties": { "revoked": { "type": "boolean" } }, "required": ["revoked"], "additionalProperties": false }),
            handler: crate::operations::revoke_access_key,
        },
        Operation {
            name: "create_kitchen",
            summary: "Create a Kitchen: a new circle, held by its creator until \
                      they invite someone else in.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({ "type": "object", "properties": { "name": { "type": "string" } }, "required": ["name"], "additionalProperties": false }),
            output_schema: kitchen_schema(),
            handler: crate::operations::create_kitchen,
        },
        Operation {
            name: "list_kitchens",
            summary: "List every Kitchen this Person cooks in.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            input_schema: empty_input(),
            output_schema: json!({
                "type": "object",
                "properties": { "kitchens": { "type": "array", "items": kitchen_schema() } },
                "required": ["kitchens"],
                "additionalProperties": false,
            }),
            handler: crate::operations::list_kitchens,
        },
        Operation {
            name: "rename_kitchen",
            summary: "Change a Kitchen's shared Name. Any member may.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({ "type": "object", "properties": { "kitchen_id": { "type": "string" }, "name": { "type": "string" } }, "required": ["kitchen_id", "name"], "additionalProperties": false }),
            output_schema: json!({ "type": "object", "properties": { "name": { "type": "string" } }, "required": ["name"], "additionalProperties": false }),
            handler: crate::operations::rename_kitchen,
        },
        Operation {
            name: "set_kitchen_nickname",
            summary: "Set this member's own private Nickname for a Kitchen, \
                      seen by nobody else. An absent or empty Nickname clears it.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({ "type": "object", "properties": { "kitchen_id": { "type": "string" }, "nickname": { "type": ["string", "null"] } }, "required": ["kitchen_id", "nickname"], "additionalProperties": false }),
            output_schema: json!({ "type": "object", "properties": { "nickname": { "type": ["string", "null"] } }, "required": ["nickname"], "additionalProperties": false }),
            handler: crate::operations::set_kitchen_nickname,
        },
        Operation {
            name: "invite_to_kitchen",
            summary: "Mint a one-use Invite for another Person to join this \
                      Kitchen. Any member may.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({ "type": "object", "properties": { "kitchen_id": { "type": "string" } }, "required": ["kitchen_id"], "additionalProperties": false }),
            output_schema: json!({ "type": "object", "properties": { "invite_id": { "type": "string" }, "secret": { "type": "string" } }, "required": ["invite_id", "secret"], "additionalProperties": false }),
            handler: crate::operations::invite_to_kitchen,
        },
        Operation {
            name: "accept_kitchen_invite",
            summary: "Open a Kitchen Invite: join the Kitchen it names. Spent \
                      on use.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({ "type": "object", "properties": { "secret": { "type": "string" } }, "required": ["secret"], "additionalProperties": false }),
            output_schema: kitchen_schema(),
            handler: crate::operations::accept_kitchen_invite,
        },
        Operation {
            name: "remove_kitchen_member",
            summary: "Remove a Person from a Kitchen — including yourself, to \
                      leave. The last member cannot be removed.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({ "type": "object", "properties": { "kitchen_id": { "type": "string" }, "person_id": { "type": "string" } }, "required": ["kitchen_id", "person_id"], "additionalProperties": false }),
            output_schema: json!({ "type": "object", "properties": { "removed": { "type": "boolean" } }, "required": ["removed"], "additionalProperties": false }),
            handler: crate::operations::remove_kitchen_member,
        },
        Operation {
            name: "delete_kitchen",
            summary: "An Operator's power over a Kitchen: delete one nobody is \
                      left in. Nothing else about a Kitchen.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({ "type": "object", "properties": { "kitchen_id": { "type": "string" } }, "required": ["kitchen_id"], "additionalProperties": false }),
            output_schema: json!({ "type": "object", "properties": { "deleted": { "type": "boolean" } }, "required": ["deleted"], "additionalProperties": false }),
            handler: crate::operations::delete_kitchen,
        },
        Operation {
            name: "create_tag",
            summary: "Create a Tag in a Kitchen, named in one Language. A word \
                      the Kitchen already files under returns the Tag it \
                      already has rather than making a second.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({ "type": "object", "properties": { "kitchen_id": { "type": "string" }, "language": { "enum": ["en", "fr", "es"] }, "name": { "type": "string" } }, "required": ["kitchen_id", "language", "name"], "additionalProperties": false }),
            output_schema: tag_schema(),
            handler: crate::operations::create_tag,
        },
        Operation {
            name: "list_tags",
            summary: "List every Tag a Kitchen files by, each shown in the \
                      reader's Reading Language where it has a name there.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            input_schema: json!({ "type": "object", "properties": { "kitchen_id": { "type": "string" } }, "required": ["kitchen_id"], "additionalProperties": false }),
            output_schema: json!({
                "type": "object",
                "properties": { "tags": { "type": "array", "items": tag_schema() } },
                "required": ["tags"],
                "additionalProperties": false,
            }),
            handler: crate::operations::list_tags,
        },
        Operation {
            name: "rename_tag",
            summary: "Name a Tag in one Language, or change the name it has \
                      there. Reaches every recipe carrying it at once, and \
                      mints no Version.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({ "type": "object", "properties": { "tag_id": { "type": "string" }, "language": { "enum": ["en", "fr", "es"] }, "name": { "type": "string" } }, "required": ["tag_id", "language", "name"], "additionalProperties": false }),
            output_schema: tag_schema(),
            handler: crate::operations::rename_tag,
        },
        Operation {
            name: "merge_tags",
            summary: "Merge two of a Kitchen's Tags into one: every recipe \
                      filed under the merged Tag is filed under the kept one \
                      instead. Mints no Version.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({ "type": "object", "properties": { "keep_tag_id": { "type": "string" }, "merge_tag_id": { "type": "string" } }, "required": ["keep_tag_id", "merge_tag_id"], "additionalProperties": false }),
            output_schema: tag_schema(),
            handler: crate::operations::merge_tags,
        },
        Operation {
            name: "delete_tag",
            summary: "Take a Tag out of a Kitchen's list and off every recipe \
                      carrying it. No recipe changes.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({ "type": "object", "properties": { "tag_id": { "type": "string" } }, "required": ["tag_id"], "additionalProperties": false }),
            output_schema: json!({ "type": "object", "properties": { "deleted": { "type": "boolean" } }, "required": ["deleted"], "additionalProperties": false }),
            handler: crate::operations::delete_tag,
        },
        Operation {
            name: "set_recipe_tag",
            summary: "File a recipe under one of its Kitchen's Tags, or take \
                      it back out. Mints no Version: filing is not what a \
                      recipe is.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({ "type": "object", "properties": { "branch_id": { "type": "string" }, "tag_id": { "type": "string" }, "carried": { "type": "boolean" } }, "required": ["branch_id", "tag_id", "carried"], "additionalProperties": false }),
            output_schema: json!({
                "type": "object",
                "properties": { "tags": { "type": "array", "items": tag_schema() } },
                "required": ["tags"],
                "additionalProperties": false,
            }),
            handler: crate::operations::set_recipe_tag,
        },
        Operation {
            name: "set_related_recipe",
            summary: "Relate two Recipes on the same Kitchen shelf, or take \
                      that single two-way, untyped link back off. It never \
                      changes either Recipe or travels in a Bundle or Share.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "branch_id": { "type": "string" },
                    "related_branch_id": { "type": "string" },
                    "related": { "type": "boolean" },
                },
                "required": ["branch_id", "related_branch_id", "related"],
                "additionalProperties": false,
            }),
            output_schema: json!({
                "type": "object",
                "properties": { "related_recipes": { "type": "array", "items": related_recipe_schema() } },
                "required": ["related_recipes"],
                "additionalProperties": false,
            }),
            handler: crate::operations::set_related_recipe,
        },
        Operation {
            name: "create_recipe",
            summary: "Create a Recipe: a Lineage, a Branch in this Kitchen, \
                      and a first Version. A title is all it needs.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: create_recipe_input_schema(),
            output_schema: recipe_schema(),
            handler: crate::operations::create_recipe,
        },
        Operation {
            name: "save_recipe_version",
            summary: "Save a new state of a Recipe onto a Branch — the whole \
                      recipe as written, replacing what was there. A rapid \
                      re-save by the same Hand collapses into the Version \
                      already being shaped rather than starting a new one. \
                      Changing a recipe your Kitchen did not write is a \
                      Copy: it starts a new Branch of the same Lineage, held \
                      by your Kitchen, starting at the Version you changed and \
                      carrying the whole chain behind it — the Branch you \
                      changed is left untouched.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: save_recipe_version_input_schema(),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "branch_id": { "type": "string" },
                    "version_id": { "type": "string" },
                    "parent_version_id": { "type": ["string", "null"] },
                    "sequence": { "type": "integer" },
                    "collapsed": { "type": "boolean" },
                    "copied": {
                        "type": "boolean",
                        "description": "True when this save was a Copy: branch_id names the new Branch it started, never the one asked for.",
                    },
                },
                "required": ["branch_id", "version_id", "parent_version_id", "sequence", "collapsed", "copied"],
                "additionalProperties": false,
            }),
            handler: crate::operations::save_recipe_version,
        },
        Operation {
            name: "import",
            summary: "Bring a batch of already-read recipes into your Home \
                      Kitchen, as a Job. Matched by foreign id against this \
                      Kitchen's ledger for the source kind, so re-running \
                      finds what it already made instead of doubling it; a \
                      recipe found changed is offered for review, never \
                      written over. Reading the outside source itself — a \
                      file, a page, a Bundle — is each importer's own job.",
            permission: Permission::Person,
            kind: Kind::Job,
            write: true,
            session_only: false,
            input_schema: import_input_schema(),
            output_schema: import_report_schema(),
            handler: crate::operations::import,
        },
        Operation {
            name: "rename_version",
            summary: "Rename a Version — the one thing about it that can \
                      change later. An absent or empty name clears it. \
                      Targeted by the Branch's own sequence number, since \
                      the same content can recur more than once on one \
                      Branch, each occurrence named on its own.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "branch_id": { "type": "string" },
                    "sequence": { "type": "integer" },
                    "name": { "type": ["string", "null"] },
                },
                "required": ["branch_id", "sequence", "name"],
                "additionalProperties": false,
            }),
            output_schema: json!({
                "type": "object",
                "properties": { "name": { "type": ["string", "null"] } },
                "required": ["name"],
                "additionalProperties": false,
            }),
            handler: crate::operations::rename_version,
        },
        Operation {
            name: "upload_photograph",
            summary: "Upload a Photograph, base64-encoded — the fallback for a \
                      Door that cannot carry raw bytes (ADR 0001). A browser \
                      uses the out-of-band `POST /api/photographs` instead. \
                      Two uploads of the same picture answer the same id.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "data": {
                        "type": "string",
                        "description": "The picture, base64-encoded.",
                    },
                },
                "required": ["data"],
                "additionalProperties": false,
            }),
            output_schema: json!({
                "type": "object",
                "properties": { "photograph_id": { "type": "string" } },
                "required": ["photograph_id"],
                "additionalProperties": false,
            }),
            handler: crate::operations::upload_photograph,
        },
        Operation {
            name: "get_recipe",
            summary: "Read a Recipe: the Branch as it stands and its whole \
                      chain of Versions, oldest first.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            input_schema: json!({
                "type": "object",
                "properties": { "branch_id": { "type": "string" } },
                "required": ["branch_id"],
                "additionalProperties": false,
            }),
            output_schema: recipe_schema(),
            handler: crate::operations::get_recipe,
        },
        Operation {
            name: "set_reading",
            summary: "Correct the Reading on one Ingredient Line of a Recipe's \
                      current state — an amount, a Unit and a target, sent \
                      together as the whole new Reading (never a per-field \
                      patch, the same convention save_recipe_version uses \
                      for the whole recipe). Mints no Version and appears in \
                      no history (ADR 0021). Amount, Unit and target left \
                      out together clears the Reading, taking the line back \
                      to fully unread.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "branch_id": { "type": "string" },
                    "line_index": { "type": "integer", "minimum": 0 },
                    "amount": { "type": ["string", "null"] },
                    "unit": { "type": ["string", "null"] },
                    "target": { "type": ["string", "null"] },
                },
                "required": ["branch_id", "line_index"],
                "additionalProperties": false,
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "line_index": { "type": "integer" },
                    "reading": reading_schema(),
                },
                "required": ["line_index", "reading"],
                "additionalProperties": false,
            }),
            handler: crate::operations::set_reading,
        },
        Operation {
            name: "start_attempt",
            summary: "Start cooking a Recipe: creates the Attempt, or hands \
                      back the one already In Progress for this Lineage — \
                      the cooking screen is that Attempt, never a second \
                      thing beside it. Pinned by fingerprint to the \
                      Branch's head Version at this moment. Anyone who can \
                      see the recipe may.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({
                "type": "object",
                "properties": { "branch_id": { "type": "string" } },
                "required": ["branch_id"],
                "additionalProperties": false,
            }),
            output_schema: attempt_schema(),
            handler: crate::operations::start_attempt,
        },
        Operation {
            name: "advance_attempt",
            summary: "Move an In Progress Attempt forward: which Step, \
                      which Ingredients are ticked, and the Yield being \
                      cooked to — a fact about this cooking, never a \
                      deviation. Any of the three, each sent whole rather \
                      than patched.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "attempt_id": { "type": "string" },
                    "current_step_index": { "type": "integer", "minimum": 0 },
                    "ticked_ingredients": {
                        "type": "array",
                        "items": { "type": "integer", "minimum": 0 },
                    },
                    "cooking_yield": attempt_yield_schema(),
                },
                "required": ["attempt_id"],
                "additionalProperties": false,
            }),
            output_schema: attempt_schema(),
            handler: crate::operations::advance_attempt,
        },
        Operation {
            name: "finish_attempt",
            summary: "End an In Progress Attempt. Ending is not what makes \
                      the cooking real — starting already did — only what \
                      stops it being In Progress.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({
                "type": "object",
                "properties": { "attempt_id": { "type": "string" } },
                "required": ["attempt_id"],
                "additionalProperties": false,
            }),
            output_schema: attempt_schema(),
            handler: crate::operations::finish_attempt,
        },
        Operation {
            name: "edit_attempt",
            summary: "Change an Attempt's free text or its five-star \
                      rating, whether it is still In Progress or long \
                      finished — an Attempt is freely editable by its \
                      cook, unlike the recipe it was cooked from.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "attempt_id": { "type": "string" },
                    "note": { "type": ["string", "null"] },
                    "rating": { "type": ["integer", "null"], "minimum": 1, "maximum": 5 },
                },
                "required": ["attempt_id"],
                "additionalProperties": false,
            }),
            output_schema: attempt_schema(),
            handler: crate::operations::edit_attempt,
        },
        Operation {
            name: "delete_attempt",
            summary: "Delete an Attempt outright — the explicit way a \
                      false start is undone, or any cooking record put \
                      away. Never soft-deleted: this is the whole of how \
                      an Attempt leaves.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({
                "type": "object",
                "properties": { "attempt_id": { "type": "string" } },
                "required": ["attempt_id"],
                "additionalProperties": false,
            }),
            output_schema: json!({
                "type": "object",
                "properties": { "deleted": { "type": "boolean" } },
                "required": ["deleted"],
                "additionalProperties": false,
            }),
            handler: crate::operations::delete_attempt,
        },
        Operation {
            name: "get_current_attempt",
            summary: "Read the caller's own In Progress Attempt for a \
                      Lineage, if any — how two devices cooking the same \
                      dish stay in step, and whether resuming should \
                      still be offered.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            input_schema: json!({
                "type": "object",
                "properties": { "lineage_id": { "type": "string" } },
                "required": ["lineage_id"],
                "additionalProperties": false,
            }),
            output_schema: json!({
                "type": "object",
                "properties": { "attempt": attempt_or_null_schema() },
                "required": ["attempt"],
                "additionalProperties": false,
            }),
            handler: crate::operations::get_current_attempt,
        },
        Operation {
            name: "list_foods",
            summary: "List every Food this instance knows, each shown in the \
                      reader's Reading Language where it has a name there.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            input_schema: empty_input(),
            output_schema: json!({
                "type": "object",
                "properties": { "foods": { "type": "array", "items": food_schema() } },
                "required": ["foods"],
                "additionalProperties": false,
            }),
            handler: crate::operations::list_foods,
        },
        Operation {
            name: "get_food",
            summary: "Read one Food: its names, its Cup Weight, and how many \
                      Readings currently point at it.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            input_schema: json!({ "type": "object", "properties": { "food_id": { "type": "string" } }, "required": ["food_id"], "additionalProperties": false }),
            output_schema: food_schema(),
            handler: crate::operations::get_food,
        },
        Operation {
            name: "set_food_name",
            summary: "Give a Food its name in one Language, or correct the \
                      one it has there. Any Person may — a Food is \
                      instance-wide, not a Kitchen's to guard.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({ "type": "object", "properties": { "food_id": { "type": "string" }, "language": { "enum": ["en", "fr", "es"] }, "name": { "type": "string" } }, "required": ["food_id", "language", "name"], "additionalProperties": false }),
            output_schema: food_schema(),
            handler: crate::operations::set_food_name,
        },
        Operation {
            name: "remove_food_name",
            summary: "Take a Food's name in one Language back off. A Food's \
                      last remaining name may not be removed this way.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({ "type": "object", "properties": { "food_id": { "type": "string" }, "language": { "enum": ["en", "fr", "es"] } }, "required": ["food_id", "language"], "additionalProperties": false }),
            output_schema: food_schema(),
            handler: crate::operations::remove_food_name,
        },
        Operation {
            name: "set_food_cup_weight",
            summary: "Set or clear a Food's Cup Weight — the one figure \
                      that turns a volume of it into a weight. Anyone may \
                      correct it.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "food_id": { "type": "string" },
                    "cup_weight_grams": { "type": ["number", "null"], "exclusiveMinimum": 0 },
                },
                "required": ["food_id", "cup_weight_grams"],
                "additionalProperties": false,
            }),
            output_schema: food_schema(),
            handler: crate::operations::set_food_cup_weight,
        },
        // Watching slow work: two ordinary Operations, so a browser polling an
        // import and an agent polling the same import use the identical shape.
        Operation {
            name: "get_job",
            summary: "Read one Job: its state, its progress, and its result or \
                      the reason it failed. Readable by the Person who asked, \
                      or by anyone when no Person did.",
            permission: Permission::Public,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            input_schema: json!({
                "type": "object",
                "properties": { "job_id": { "type": "string" } },
                "required": ["job_id"],
                "additionalProperties": false,
            }),
            output_schema: job_record_schema(),
            handler: crate::operations::get_job,
        },
        Operation {
            name: "cancel_job",
            summary: "Cancel a Job you asked for: acknowledged always, honoured \
                      while it still waits in line.",
            permission: Permission::Public,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: json!({
                "type": "object",
                "properties": { "job_id": { "type": "string" } },
                "required": ["job_id"],
                "additionalProperties": false,
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "cancelled": { "type": "boolean" },
                },
                "required": ["cancelled"],
                "additionalProperties": false,
            }),
            handler: crate::operations::cancel_job,
        },
        Operation {
            name: "list_jobs",
            summary: "List the Jobs this Person has asked for, newest first.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            input_schema: empty_input(),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "jobs": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": { "type": "string" },
                                "operation": { "type": "string" },
                                "status": { "enum": ["queued", "running", "completed", "failed", "cancelled"] },
                                "created_at": { "type": "string" },
                            },
                            "required": ["id", "operation", "status", "created_at"],
                            "additionalProperties": false,
                        },
                    },
                },
                "required": ["jobs"],
                "additionalProperties": false,
            }),
            handler: crate::operations::list_jobs,
        },
    ];
    // The demonstration Job exists only where tests can drive it end-to-end
    // through real Doors. Production instances ship no synthetic Operations;
    // their first Jobs arrive with Import, Sheet rendering and Meaning Search.
    #[cfg(feature = "test-jobs")]
    operations.push(Operation {
        name: "probe_job",
        summary: "A demonstration Job (test builds only): walks a few progress \
                  ticks over about a second, then finishes — or fails on purpose \
                  when asked to.",
        permission: Permission::Public,
        kind: Kind::Job,
        write: true,
        session_only: false,
        input_schema: json!({
            "type": "object",
            "properties": {
                "steps": { "type": "integer", "minimum": 1, "maximum": 60, "default": 3 },
                "delay_ms": { "type": "integer", "minimum": 1, "maximum": 500, "default": 100 },
                "fail": { "type": "boolean", "default": false },
            },
            "additionalProperties": false,
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "steps": { "type": "integer" },
                "message": { "type": "string" },
            },
            "required": ["steps", "message"],
            "additionalProperties": false,
        }),
        handler: crate::operations::probe_job,
    });

    operations
});

/// The shape `get_job` serves for any Job, whatever Operation runs behind it.
fn job_record_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string" },
            "operation": { "type": "string" },
            "status": { "enum": ["queued", "running", "completed", "failed", "cancelled"] },
            "progress": {
                "type": "object",
                "properties": {
                    "done": { "type": ["integer", "null"] },
                    "total": { "type": ["integer", "null"] },
                    "message": { "type": ["string", "null"] },
                },
                "additionalProperties": false,
            },
            "result": {},
            "error": { "type": ["string", "null"] },
            "errorCode": { "type": ["integer", "null"] },
            "created_at": { "type": "string" },
            "updated_at": { "type": "string" },
        },
        "required": [
            "id", "operation", "status", "progress", "result",
            "error", "errorCode", "created_at", "updated_at"
        ],
        "additionalProperties": false,
    })
}

/// The shape a Kitchen is served in: its shared Name and Hand, whether it is
/// the asking Person's Home Kitchen, their own Nickname for it (nobody
/// else's), and who else cooks in it.
fn kitchen_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string" },
            "name": { "type": "string" },
            "hand_id": { "type": "string" },
            "is_home": { "type": "boolean" },
            "nickname": { "type": ["string", "null"] },
            "members": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "person_id": { "type": "string" },
                        "name": { "type": "string" },
                    },
                    "required": ["person_id", "name"],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["id", "name", "hand_id", "is_home", "nickname", "members"],
        "additionalProperties": false,
    })
}

/// The shape a Tag is served in: the word to show this reader, the Language
/// that word is in, and every name the Tag has. `name` and `language` are null
/// for a Tag holding no name at all: no Operation here can leave one in that
/// state — `create_tag` demands a word and nothing removes a single name — but
/// that is true by omission rather than by anything enforcing it, so a screen
/// reading them still has to survive it.
fn tag_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string" },
            "kitchen_id": { "type": "string" },
            "name": { "type": ["string", "null"] },
            "language": { "type": ["string", "null"] },
            "names": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "language": { "type": "string" },
                        "name": { "type": "string" },
                    },
                    "required": ["language", "name"],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["id", "kitchen_id", "name", "language", "names"],
        "additionalProperties": false,
    })
}

/// A Related Recipe shown from one end: an ordinary name when the other
/// Lineage has left this Kitchen's shelf, otherwise its current Branch and
/// title. It is a shelf note and has no place inside a Version (#52).
fn related_recipe_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "lineage_id": { "type": "string" },
            "branch_id": { "type": ["string", "null"] },
            "title": { "type": "string" },
        },
        "required": ["lineage_id", "branch_id", "title"],
        "additionalProperties": false,
    })
}

/// The shape a Recipe is served in: the Branch as it stands and its whole
/// chain of Versions, oldest first — created by `create_recipe`, read back
/// by `get_recipe`.
fn recipe_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "branch_id": { "type": "string" },
            "lineage_id": { "type": "string" },
            "kitchen_id": { "type": "string" },
            "hand_id": { "type": "string" },
            "language": { "type": "string" },
            "origin_address": { "type": ["string", "null"] },
            "head_version_id": { "type": "string" },
            "versions": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "sequence": { "type": "integer" },
                        "version_id": { "type": "string" },
                        "parent_version_id": { "type": ["string", "null"] },
                        "hand_id": { "type": "string" },
                        "name": { "type": ["string", "null"] },
                        "change_note": { "type": ["string", "null"] },
                        "created_at": { "type": "string" },
                        "content": recipe_content_schema(),
                        "readings": reading_list_schema(),
                    },
                    "required": [
                        "sequence", "version_id", "parent_version_id", "hand_id",
                        "name", "change_note", "created_at", "content", "readings"
                    ],
                    "additionalProperties": false,
                },
            },
            // The Tags this Kitchen files the recipe under, as it stands now.
            // Outside `versions` on purpose: filing is not recipe content and
            // is named by no fingerprint (ADR 0035).
            "tags": { "type": "array", "items": tag_schema() },
            "related_recipes": { "type": "array", "items": related_recipe_schema() },
        },
        "required": [
            "branch_id", "lineage_id", "kitchen_id", "hand_id", "language",
            "origin_address", "head_version_id", "versions", "tags", "related_recipes"
        ],
        "additionalProperties": false,
    })
}

/// The fields of a Recipe a Version's fingerprint covers (ADR 0002, ADR 0019,
/// ADR 0021): the title, the Ingredient Lines and Steps — each list a flat,
/// ordered sequence where a `"section"` entry is a heading like "For the
/// sauce" and the other kind is the written line itself — the optional
/// Yield, Prep/Cook Time, Note and Source. A Reading is never part of this:
/// it is Kamosu's guess about a line, not the line (ADR 0021), and has no
/// slot here.
fn recipe_content_properties() -> Value {
    json!({
        "title": { "type": "string" },
        "yield": {
            "type": ["object", "null"],
            "properties": {
                "amount": { "type": "string" },
                // Named `noun`, not `unit`: CONTEXT.md's Yield ("4 servings",
                // "24 cookies") is a different concept from its Unit glossary
                // entry (grams, cups, spoons — a closed, convertible list).
                "noun": { "type": "string" },
            },
            "required": ["amount", "noun"],
            "additionalProperties": false,
        },
        "prep_time_minutes": {
            "type": ["integer", "null"],
            "description": "Whole minutes of active preparation.",
        },
        "cook_time_minutes": {
            "type": ["integer", "null"],
            "description": "Whole minutes of cooking, including resting, \
                             proving, marinating and chilling.",
        },
        "note": { "type": ["string", "null"] },
        // The single Photograph that stands for the recipe wherever it is
        // listed (CONTEXT.md, "Main Photo") — a reference only; storing the
        // picture itself is `upload_photograph` (#45, ADR 0017).
        "main_photo": { "type": ["string", "null"] },
        "source": {
            "type": ["object", "null"],
            "properties": {
                "text": { "type": "string" },
                "link": { "type": ["string", "null"] },
            },
            "required": ["text", "link"],
            "additionalProperties": false,
        },
        "ingredients": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "kind": { "enum": ["section", "ingredient"] },
                    "text": { "type": "string" },
                },
                "required": ["kind", "text"],
                "additionalProperties": false,
            },
        },
        "steps": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "kind": { "enum": ["section", "step"] },
                    "text": { "type": "string" },
                    "photo": { "type": ["string", "null"] },
                },
                "required": ["kind", "text", "photo"],
                "additionalProperties": false,
            },
        },
    })
}

/// A Version's content exactly as stored and read back: every field above,
/// always present — absent input normalises to `null` or `[]` rather than
/// being left out (see `parse_recipe_content` in `core.rs`).
fn recipe_content_schema() -> Value {
    json!({
        "type": "object",
        "properties": recipe_content_properties(),
        "required": [
            "title", "yield", "prep_time_minutes", "cook_time_minutes",
            "note", "main_photo", "source", "ingredients", "steps",
        ],
        "additionalProperties": false,
    })
}

/// `create_recipe`'s input: a Kitchen and a title are all a Recipe ever
/// needs — every other field of the recipe's content is optional here.
fn create_recipe_input_schema() -> Value {
    let mut properties = recipe_content_properties();
    let map = properties.as_object_mut().expect("object schema");
    map.insert("kitchen_id".to_string(), json!({ "type": "string" }));
    map.insert(
        "language".to_string(),
        json!({ "enum": ["en", "fr", "es"] }),
    );
    json!({
        "type": "object",
        "properties": properties,
        "required": ["kitchen_id", "title"],
        "additionalProperties": false,
    })
}

/// `save_recipe_version`'s input: the whole recipe as it now reads, replacing
/// what was on the Branch — a title is the one field that must be there.
/// `kitchen_id` names which of the caller's own Kitchens this save is on
/// behalf of, for when a Copy is about to start (CONTEXT.md, "Copy") — their
/// Home Kitchen unless they say otherwise, a question only ever put to
/// someone who cooks in more than one.
fn save_recipe_version_input_schema() -> Value {
    let mut properties = recipe_content_properties();
    let map = properties.as_object_mut().expect("object schema");
    map.insert("branch_id".to_string(), json!({ "type": "string" }));
    map.insert("name".to_string(), json!({ "type": "string" }));
    map.insert("change_note".to_string(), json!({ "type": "string" }));
    map.insert("kitchen_id".to_string(), json!({ "type": "string" }));
    json!({
        "type": "object",
        "properties": properties,
        "required": ["branch_id", "title"],
        "additionalProperties": false,
    })
}

/// `import`'s input: a source kind naming which ledger to match against, and
/// a batch of candidates already read from that source — each the same
/// recipe content `create_recipe` accepts, addressed by the foreign id the
/// source itself gave it (a Crouton UUID, a web page's own URL). Reading the
/// source — a `.crumb`, a page, a Bundle — happens before this: `import` is
/// the shared landing machinery every importer calls into, never the parser.
fn import_input_schema() -> Value {
    let mut properties = recipe_content_properties();
    let map = properties.as_object_mut().expect("object schema");
    map.insert(
        "foreign_id".to_string(),
        json!({
            "type": "string",
            "description": "The id this recipe had in the place it came \
                             from. Held in this Import's ledger, never on \
                             the recipe, so a re-run matches instead of \
                             doubling the library (ADR 0025).",
        }),
    );
    map.insert(
        "language".to_string(),
        json!({ "enum": ["en", "fr", "es"] }),
    );
    json!({
        "type": "object",
        "properties": {
            "source_kind": {
                "type": "string",
                "description": "Which outside source these candidates came \
                                 from. One ledger is kept per Kitchen per \
                                 source kind.",
            },
            "candidates": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": properties,
                    "required": ["foreign_id", "title"],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["source_kind", "candidates"],
        "additionalProperties": false,
    })
}

/// `import`'s eventual result: the Import Report, a ledger read by a person
/// rather than an error log (ADR 0025, CONTEXT.md "Import Report"). Every
/// candidate lands in exactly one bucket — `arrived` covers a recipe freshly
/// made and one already matched and found unchanged alike, told apart by
/// `status`; `offered` is a previously-seen recipe found changed, waiting for
/// a tap rather than written over; `unreadable` names what could not be
/// placed at all, and why.
fn import_report_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "import_id": { "type": "string" },
            "kitchen_id": { "type": "string" },
            "source_kind": { "type": "string" },
            "arrived": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "foreign_id": { "type": "string" },
                        "status": { "enum": ["created", "unchanged"] },
                        "lineage_id": { "type": "string" },
                        "branch_id": { "type": "string" },
                        "title": { "type": "string" },
                    },
                    "required": ["foreign_id", "status", "lineage_id", "branch_id", "title"],
                    "additionalProperties": false,
                },
            },
            "offered": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "foreign_id": { "type": "string" },
                        "lineage_id": { "type": "string" },
                        "branch_id": { "type": "string" },
                        "title": { "type": "string" },
                        "candidate_version_id": {
                            "type": "string",
                            "description": "The Version this candidate's \
                                             content became, held but not \
                                             yet on the Branch.",
                        },
                    },
                    "required": ["foreign_id", "lineage_id", "branch_id", "title", "candidate_version_id"],
                    "additionalProperties": false,
                },
            },
            "unreadable": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "foreign_id": { "type": ["string", "null"] },
                        "reason": { "type": "string" },
                    },
                    "required": ["foreign_id", "reason"],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["import_id", "kitchen_id", "source_kind", "arrived", "offered", "unreadable"],
        "additionalProperties": false,
    })
}

/// A Reading: Kamosu's interpretation of one Ingredient Line — an amount, a
/// Unit (whatever word was written; see #49) and a target, all optional and
/// `null` together wherever Kamosu has read nothing (ADR 0002, ADR 0021). A
/// target names a Food by the word alone; Foods (#47) carry no id here.
fn reading_schema() -> Value {
    json!({
        "type": ["object", "null"],
        "properties": {
            "amount": { "type": ["string", "null"] },
            "unit": { "type": ["string", "null"] },
            "target": { "type": ["string", "null"] },
        },
        "required": ["amount", "unit", "target"],
        "additionalProperties": false,
    })
}

/// One Reading slot per Ingredient Line in a Version's `ingredients` list,
/// in the same order — `null` at any index Kamosu has not read.
fn reading_list_schema() -> Value {
    json!({
        "type": "array",
        "items": reading_schema(),
    })
}

/// A Food as `list_foods` and `get_food` serve it: every name it has, the
/// one to show a reader, its Cup Weight, its nutrition slot — always `null`
/// in v1, a foundation left for the CIQUAL binding #12 deferred past v1, and
/// never carried in a Bundle — and how many Readings currently point at it.
/// Mirrors `tag_schema`'s reader-facing shape; a Food carries no `kitchen_id`
/// because it is instance-wide (ADR 0022).
fn food_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string" },
            "name": { "type": ["string", "null"] },
            "language": { "type": ["string", "null"] },
            "names": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "language": { "type": "string" },
                        "name": { "type": "string" },
                    },
                    "required": ["language", "name"],
                    "additionalProperties": false,
                },
            },
            "cup_weight_grams": { "type": ["number", "null"] },
            "nutrition": { "type": "null" },
            "reading_count": { "type": "integer", "minimum": 0 },
        },
        "required": ["id", "name", "language", "names", "cup_weight_grams", "nutrition", "reading_count"],
        "additionalProperties": false,
    })
}

/// The Yield an Attempt is cooking to — the same `{amount, noun}` shape a
/// recipe's own Yield takes, held on the Attempt as a fact about that
/// afternoon rather than a deviation (ADR 0010).
fn attempt_yield_schema() -> Value {
    json!({
        "type": ["object", "null"],
        "properties": {
            "amount": { "type": "string" },
            "noun": { "type": "string" },
        },
        "required": ["amount", "noun"],
        "additionalProperties": false,
    })
}

/// The shape an Attempt is served in — created by `start_attempt`, moved by
/// `advance_attempt`, ended by `finish_attempt`, corrected by `edit_attempt`,
/// and read back by `get_current_attempt`. `resumable` is computed at read
/// time: still In Progress and within three days of `last_action_at`
/// (ADR 0010) — the Attempt itself is never deleted by the window passing.
fn attempt_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string" },
            "lineage_id": { "type": "string" },
            "person_id": { "type": "string" },
            "version_id": { "type": "string" },
            "current_step_index": { "type": "integer", "minimum": 0 },
            "ticked_ingredients": {
                "type": "array",
                "items": { "type": "integer", "minimum": 0 },
            },
            "cooking_yield": attempt_yield_schema(),
            "note": { "type": ["string", "null"] },
            "rating": { "type": ["integer", "null"], "minimum": 1, "maximum": 5 },
            "finished_at": { "type": ["string", "null"] },
            "resumable": { "type": "boolean" },
            "created_at": { "type": "string" },
            "last_action_at": { "type": "string" },
        },
        "required": [
            "id", "lineage_id", "person_id", "version_id", "current_step_index",
            "ticked_ingredients", "cooking_yield", "note", "rating", "finished_at",
            "resumable", "created_at", "last_action_at",
        ],
        "additionalProperties": false,
    })
}

/// `attempt_schema`, nullable — `get_current_attempt` answers no Attempt at
/// all wherever the caller has none In Progress on that Lineage.
fn attempt_or_null_schema() -> Value {
    let mut schema = attempt_schema();
    schema["type"] = json!(["object", "null"]);
    schema
}

fn empty_input() -> Value {
    json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false,
    })
}

/// The Catalogue as JSON: every declaration, in the order both Doors walk them.
///
/// This is the one export the interface builds against. `kamosu catalogue`
/// prints it and `just client` turns it into the typed client every screen
/// calls through, so the frontend and the Core cannot disagree about an
/// Operation's shape — a renamed field is a failed build, not a blank screen.
pub fn declarations() -> Value {
    Value::Array(
        OPERATIONS
            .iter()
            .map(|op| {
                json!({
                    "name": op.name,
                    "summary": op.summary,
                    "permission": match op.permission {
                        Permission::Public => "public",
                        Permission::Person => "person",
                        Permission::Operator => "operator",
                    },
                    "kind": match op.kind {
                        Kind::Immediate => "immediate",
                        Kind::Job => "job",
                    },
                    "input_schema": op.input_schema.clone(),
                    "output_schema": op.output_schema.clone(),
                })
            })
            .collect(),
    )
}

/// Look up one Operation by name.
pub fn find(name: &str) -> Option<&'static Operation> {
    OPERATIONS.iter().find(|op| op.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique_and_rpc_shaped() {
        let mut names: Vec<_> = OPERATIONS.iter().map(|op| op.name).collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count, "duplicate operation name in Catalogue");
        assert!(OPERATIONS.iter().all(|op| !op.name.is_empty()));
        assert!(
            OPERATIONS
                .iter()
                .all(|op| op.name.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
        );
    }

    #[test]
    fn jobs_declared_in_the_catalogue_answer_the_job_envelope() {
        assert!(
            OPERATIONS
                .iter()
                .filter(|op| op.kind == Kind::Job)
                .all(|op| matches!(op.output_schema["type"], Value::String(_))),
            "a Job's declared output describes its eventual result, not the id envelope"
        );
    }

    #[test]
    fn declarations_carry_every_operation_verbatim() {
        let declared = declarations();
        let list = declared.as_array().expect("an array of declarations");
        assert_eq!(list.len(), OPERATIONS.len());
        for (entry, op) in list.iter().zip(OPERATIONS.iter()) {
            assert_eq!(entry["name"], op.name);
            assert_eq!(entry["input_schema"], op.input_schema);
            assert_eq!(entry["output_schema"], op.output_schema);
        }
        // The two enums are exported as words, not numbers: the interface's
        // generator reads them, and a number would be a silent renumbering.
        assert!(list.iter().all(|e| matches!(
            e["permission"].as_str(),
            Some("public" | "person" | "operator")
        )));
        assert!(
            list.iter()
                .all(|e| matches!(e["kind"].as_str(), Some("immediate" | "job")))
        );
    }
}
