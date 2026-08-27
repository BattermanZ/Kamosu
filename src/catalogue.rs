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
                      already being shaped rather than starting a new one.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            input_schema: save_recipe_version_input_schema(),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "version_id": { "type": "string" },
                    "parent_version_id": { "type": ["string", "null"] },
                    "sequence": { "type": "integer" },
                    "collapsed": { "type": "boolean" },
                },
                "required": ["version_id", "parent_version_id", "sequence", "collapsed"],
                "additionalProperties": false,
            }),
            handler: crate::operations::save_recipe_version,
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
            "note", "source", "ingredients", "steps",
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
fn save_recipe_version_input_schema() -> Value {
    let mut properties = recipe_content_properties();
    let map = properties.as_object_mut().expect("object schema");
    map.insert("branch_id".to_string(), json!({ "type": "string" }));
    map.insert("name".to_string(), json!({ "type": "string" }));
    map.insert("change_note".to_string(), json!({ "type": "string" }));
    json!({
        "type": "object",
        "properties": properties,
        "required": ["branch_id", "title"],
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
