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

/// Which lane carries a Job (ADR 0032). Every Job but one shares the lane its
/// own caller would land in anyway — a Person's lane, or the single depth-one
/// lane when nobody is signed in — because lane assignment is ordinarily a
/// property of *who asked*, never of *what was asked*.
///
/// `AlwaysSingle` is the one deliberate exception: work whose risk lives in
/// the outbound fetch itself, not in who asked for it. A recipe's own page can
/// carry text telling an agent to fetch another URL (ADR 0033), so a signed-in
/// Person's web-link import is bounded exactly as a stranger's Sheet is —
/// never more than one fetch in flight at a time, instance-wide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobLane {
    /// The ordinary rule: the Person's lane, or the stranger lane with nobody
    /// signed in. Ignored on an Immediate Operation.
    ByCaller,
    /// Always the single depth-one lane, regardless of who asked.
    AlwaysSingle,
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
    /// Which lane carries this Operation's Job. Meaningless on an Immediate
    /// Operation, but declared on every entry so nothing here reads as an
    /// oversight rather than a choice.
    pub job_lane: JobLane,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
            input_schema: json!({ "type": "object", "properties": { "reading_language": { "enum": ["en", "fr", "es"] }, "reading_measures": { "enum": ["us", "metric", "as_written"] } }, "required": ["reading_language", "reading_measures"], "additionalProperties": false }),
            // The same two enums the input takes, and the same two the getter
            // answers with: one preference declared one way, so the generated
            // client cannot type the same field twice over (#49).
            output_schema: json!({ "type": "object", "properties": { "reading_language": { "enum": ["en", "fr", "es"] }, "reading_measures": { "enum": ["us", "metric", "as_written"] } }, "required": ["reading_language", "reading_measures"], "additionalProperties": false }),
            handler: crate::operations::set_reading_preferences,
        },
        Operation {
            name: "get_reading_preferences",
            summary: "The Language and measures this Person reads in. Reading \
                      Measures live on the account rather than in a browser, \
                      so every Door and every device reads the same recipe the \
                      same way; the default is American, a stated convention \
                      rather than a guess about anybody.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            // Declared as the same two enums `set_reading_preferences` accepts,
            // because those are the only answers there are — which is what
            // lets the generated client type them rather than calling them
            // strings.
            output_schema: json!({ "type": "object", "properties": { "reading_language": { "enum": ["en", "fr", "es"] }, "reading_measures": { "enum": ["us", "metric", "as_written"] } }, "required": ["reading_language", "reading_measures"], "additionalProperties": false }),
            handler: crate::operations::get_reading_preferences,
        },
        Operation {
            name: "rename_person",
            summary: "Change this Person's current reminder name.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
            input_schema: json!({ "type": "object", "properties": { "name": { "type": "string" } }, "required": ["name"], "additionalProperties": false }),
            output_schema: json!({ "type": "object", "properties": { "link": { "type": "string"} }, "required": ["link"], "additionalProperties": false }),
            handler: crate::operations::mint_recovery_link,
        },
        Operation {
            name: "sweep_photographs",
            summary: "Take away Photographs nothing has pointed at for a week, and their Display Copies with them. Runs daily on its own; this asks for it now.",
            permission: Permission::Operator,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: json!({ "type": "object", "properties": { "referenced": { "type": "integer" }, "newly_unreferenced": { "type": "integer" }, "back_in_use": { "type": "integer" }, "swept": { "type": "integer" }, "swept_photograph_ids": { "type": "array", "items": { "type": "string" } } }, "required": ["referenced", "newly_unreferenced", "back_in_use", "swept", "swept_photograph_ids"], "additionalProperties": false }),
            handler: crate::operations::sweep_photographs,
        },
        Operation {
            name: "list_sessions",
            summary: "List this Person's browser Sessions by device and last use.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
            input_schema: save_recipe_version_input_schema(),
            output_schema: saved_version_schema(),
            handler: crate::operations::save_recipe_version,
        },
        Operation {
            name: "start_translation",
            summary: "Translate a recipe: start an ordinary Branch of the same \
                      Lineage in another Language, whose first Version records \
                      which Version of the source it renders. There is no \
                      Translation object — what this makes is a Branch, and \
                      every Operation from here on is the ordinary one. Its \
                      chain starts fresh rather than carrying the source's, \
                      which is what separates it from a Copy: different words \
                      rendering the same dish, with a history of their own. An \
                      agent translating calls this under the Person's own \
                      Credential and is a scribe, not an author.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: start_translation_input_schema(),
            output_schema: recipe_schema(),
            handler: crate::operations::start_translation,
        },
        Operation {
            name: "set_recipe_language",
            summary: "Say what Language a recipe is written in. The only thing \
                      that acts on a save's language offer — Kamosu detects and \
                      offers, and never changes a Language without the cook \
                      saying so. Changing it makes a Version, so the change \
                      leaves a trace in the recipe's own history. Setting it to \
                      `unknown` says the recipe is honestly more than one \
                      Language: from then on it is offered nothing, marked \
                      nothing, and shown to every reader whatever they read in.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "branch_id": { "type": "string" },
                    "language": { "enum": ["en", "fr", "es", "unknown"] },
                },
                "required": ["branch_id", "language"],
                "additionalProperties": false,
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "branch_id": { "type": "string" },
                    "language": { "type": "string" },
                    "sequence": {
                        "type": ["integer", "null"],
                        "description": "The sequence of the Version this change made, or null when the recipe was already in that Language and nothing changed.",
                    },
                },
                "required": ["branch_id", "language", "sequence"],
                "additionalProperties": false,
            }),
            handler: crate::operations::set_recipe_language,
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
            job_lane: JobLane::ByCaller,
            input_schema: import_input_schema(),
            output_schema: import_report_schema(),
            handler: crate::operations::import,
        },
        Operation {
            name: "import_web_link",
            summary: "Bring in a recipe straight from a URL, as a Job. Reads \
                      the page's schema.org JSON-LD (#70) — no per-site \
                      scraping, no LLM fallback — and lands it in your Home \
                      Kitchen through the same ledger `import` uses, keyed by \
                      the page's own address. Fetching is bound to public \
                      addresses at the dialled address and at every redirect \
                      (ADR 0033), and — because a page's own text can tell an \
                      agent to fetch another URL — always takes the single \
                      depth-one lane, never more than one fetch in flight \
                      regardless of who is signed in.",
            permission: Permission::Person,
            kind: Kind::Job,
            write: true,
            session_only: false,
            job_lane: JobLane::AlwaysSingle,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The recipe page's address. Fetched \
                                         through the guarded client; only \
                                         http:// and https:// are accepted.",
                    },
                },
                "required": ["url"],
                "additionalProperties": false,
            }),
            output_schema: import_report_schema(),
            handler: crate::operations::import_web_link,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            name: "search_recipes",
            summary: "The shelf, and searching it. With no query: everything \
                      the Kitchens this Person cooks in hold, merged, \
                      alphabetical, one entry per Lineage, each titled in the \
                      reader's Reading Language with a marked fallback. With a \
                      query: the same shelf narrowed to what matched, an exact \
                      title first, every entry quoting the line that matched. \
                      One Operation either way — Meaning Search arrives here \
                      rather than beside it (ADR 0027, ADR 0029).",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    // Absent or empty is the whole shelf, not an error: the
                    // screen opens on it before anyone has typed anything.
                    "query": { "type": ["string", "null"] },
                    // The two filters, passed on every request and remembered
                    // nowhere. A filter that persists is a mode, and a mode you
                    // forgot you set is the Kitchen switcher wearing a hat.
                    "kitchen_id": { "type": ["string", "null"] },
                    // Created, branched or cooked by this Person — a history,
                    // not an ownership, and one that needs no curating ever.
                    "mine": { "type": "boolean" },
                },
                "additionalProperties": false,
            }),
            output_schema: shelf_schema(),
            handler: crate::operations::search_recipes,
        },
        Operation {
            name: "home_shelves",
            summary: "Home: the computed shelves that answer *show me \
                      something* rather than handing back a search box — \
                      cooked most, quick tonight, never cooked, recently \
                      opened. Each is one card per Lineage in the reader's \
                      Reading Language, in the same shape the library's shelf \
                      answers in. A shelf with nothing on it is left out \
                      rather than sent empty, so an instance holding no \
                      recipes answers with no shelves at all. All four are \
                      counted from recipes and Attempts that already exist, \
                      except *recently opened*, which reads what \
                      `note_recipe_opened` remembered (ADR 0011, ADR 0027).",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: json!({
                "type": "object",
                "properties": {
                    // The line *quick tonight* is drawn at, sent rather than
                    // written on the screen twice: the shelf's heading says
                    // "under 30 minutes" because the Core said 30, so the
                    // number a reader sees cannot drift from the number that
                    // chose the recipes under it.
                    "quick_tonight_minutes": { "type": "integer" },
                    "shelves": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                // What the shelf is, not what to call it: the
                                // wording is the screen's, in the reader's own
                                // language, and an agent at the MCP door gets
                                // the name rather than English prose.
                                "name": {
                                    "enum": [
                                        "cooked_most", "quick_tonight",
                                        "never_cooked", "recently_opened"
                                    ],
                                },
                                "recipes": {
                                    "type": "array",
                                    "items": shelf_entry_schema(),
                                },
                            },
                            "required": ["name", "recipes"],
                            "additionalProperties": false,
                        },
                    },
                },
                "required": ["quick_tonight_minutes", "shelves"],
                "additionalProperties": false,
            }),
            handler: crate::operations::home_shelves,
        },
        Operation {
            name: "note_recipe_opened",
            summary: "Remember that the caller opened this recipe, for Home's \
                      *recently opened* shelf. One fact per Person per \
                      Lineage — opening a recipe's French Branch and its \
                      English one is opening the same recipe — and opening it \
                      again moves the time rather than adding a row. It is \
                      private to the Person, never travels, and is in no \
                      fingerprint, Vault or Bundle: an instance that lost it \
                      would lose the order of one shelf and nothing else \
                      (ADR 0027).",
            permission: Permission::Person,
            kind: Kind::Immediate,
            // It writes, so a read-only Access Key cannot do it. That is the
            // deliberate cost of keeping `get_recipe` a read: a Credential
            // that may read every recipe must never be locked out of the
            // library by the act of reading one.
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": { "branch_id": { "type": "string" } },
                "required": ["branch_id"],
                "additionalProperties": false,
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    // The Lineage the opening was recorded against, which is
                    // never the Branch that was opened.
                    "lineage_id": { "type": "string" },
                    "opened_at": { "type": "string" },
                },
                "required": ["lineage_id", "opened_at"],
                "additionalProperties": false,
            }),
            handler: crate::operations::note_recipe_opened,
        },
        Operation {
            name: "meaning_search_status",
            summary: "Whether Meaning Search is on here, what model it would \
                      use, who accepted that model's terms — and whether this \
                      caller should be offered it. Answers on every instance, \
                      including the many that will never turn it on.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: meaning_status_schema(),
            handler: crate::operations::meaning_search_status,
        },
        Operation {
            name: "accept_meaning_search_terms",
            summary: "Accept the terms of the model Meaning Search needs. \
                      Kamosu ships no weights (ADR 0029): the person who \
                      accepts the terms is the person the terms are about, and \
                      the acceptance keeps the Hand that made it and whether it \
                      arrived by login or by Access Key. Available at both \
                      Doors — a web-only carve-out would be the first hole in \
                      Parity, and would stop nothing anyway.",
            permission: Permission::Operator,
            kind: Kind::Immediate,
            write: true,
            // Deliberately not session-only. An agent acts *as* a Person, and
            // minting it an Access Key was already the act of authorising that
            // (ADR 0029, ADR 0031). A read-only Key still cannot: accepting is
            // a write, so no new guard was needed for that half.
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: json!({
                "type": "object",
                "properties": { "state": meaning_state_enum() },
                "required": ["state"],
                "additionalProperties": false,
            }),
            handler: crate::operations::accept_meaning_search_terms,
        },
        Operation {
            name: "decline_meaning_search",
            summary: "Decline the model's terms. Meaning Search stays off and \
                      the offer is never made again on this instance — a \
                      question already answered, asked twice, is a nag.",
            permission: Permission::Operator,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: json!({
                "type": "object",
                "properties": { "state": meaning_state_enum() },
                "required": ["state"],
                "additionalProperties": false,
            }),
            handler: crate::operations::decline_meaning_search,
        },
        Operation {
            name: "download_meaning_model",
            summary: "Fetch the Meaning Search model into /data, as a Job. No \
                      weights ship in the image; this is the only way any \
                      arrive, and only after the terms have been accepted. The \
                      download is pinned to one revision and verified against a \
                      manifest, so a half-finished one is never mistaken for a \
                      model.",
            permission: Permission::Operator,
            kind: Kind::Job,
            write: true,
            session_only: false,
            // The ordinary rule: an Operator's own lane. The risk here is size,
            // not the fetch — the address is a constant in this binary, not
            // anything a recipe could talk Kamosu into dialling (ADR 0033).
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "model": { "type": "string" },
                    "repository": { "type": "string" },
                    "revision": { "type": "string" },
                },
                "required": ["model", "repository", "revision"],
                "additionalProperties": false,
            }),
            handler: crate::operations::download_meaning_model,
        },
        Operation {
            name: "build_meaning_index",
            summary: "Read the library into the Meaning Search index, as a Job, \
                      and turn Meaning Search on. Incremental: what is read is \
                      what the index does not already hold, so the first run is \
                      the whole library and every later one is whatever \
                      changed. The index is derived from the recipes and can be \
                      rebuilt at any time. Kamosu also does this by itself, \
                      within the minute, whenever a recipe changes.",
            permission: Permission::Operator,
            kind: Kind::Job,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: json!({
                "type": "object",
                "properties": { "indexed": { "type": "integer" } },
                "required": ["indexed"],
                "additionalProperties": false,
            }),
            handler: crate::operations::build_meaning_index,
        },
        Operation {
            name: "turn_off_meaning_search",
            summary: "Stop matching on meaning and throw the index away. \
                      Discards nothing that cannot be rebuilt — the index is \
                      derived from the recipes — and keeps both the acceptance, \
                      which is history, and the downloaded weights, so turning \
                      it back on is a rebuild rather than another download.",
            permission: Permission::Operator,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: json!({
                "type": "object",
                "properties": { "state": meaning_state_enum() },
                "required": ["state"],
                "additionalProperties": false,
            }),
            handler: crate::operations::turn_off_meaning_search,
        },
        Operation {
            name: "get_recipe",
            summary: "Read a Recipe: the Branch as it stands and its whole \
                      chain of Versions, oldest first.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
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
            name: "get_thread",
            summary: "Read the Thread: every Version of every Branch of \
                      one Lineage this Person can see, oldest first per \
                      Branch, with every Attempt hanging off it. \
                      branch_id is only the entry point — any Branch of \
                      the Lineage answers the same Thread.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": { "branch_id": { "type": "string" } },
                "required": ["branch_id"],
                "additionalProperties": false,
            }),
            output_schema: thread_schema(),
            handler: crate::operations::get_thread,
        },
        // ── Share Links (#65, ADR 0026, ADR 0018) ──────────────────────────
        //
        // There are two levels of visibility and no third: a recipe is seen by
        // its Kitchen, or by anyone holding its Share Link. Nothing here takes
        // a visibility argument, because there is no scale to set a point on.
        Operation {
            name: "share_recipe",
            summary: "Turn a Recipe's Share Link on, and answer the link. One \
                      permanent, unguessable address per Recipe, never \
                      expiring, freely passed on. Asking twice for a Recipe \
                      already shared answers the link it already has rather \
                      than minting a second one. The link's Secret is answered \
                      exactly once — here, at the moment it is minted — \
                      because only its hash is stored. The instance's public \
                      address is asked for at the first Share Link and stored \
                      once; a link is kept as a token rather than a URL, so \
                      setting the address later makes every link already \
                      minted render correctly.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "branch_id": { "type": "string" },
                    "public_address": {
                        "type": "string",
                        "description": "Where this instance is reachable from outside, \
                                         e.g. https://kamosu.example.com — asked at the \
                                         first Share Link and stored once. Ignored where \
                                         an address is already stored; `set_public_address` \
                                         is how one is changed.",
                    },
                },
                "required": ["branch_id"],
                "additionalProperties": false,
            }),
            output_schema: share_link_schema(),
            handler: crate::operations::share_recipe,
        },
        Operation {
            name: "end_share_link",
            summary: "End a Recipe's Share Link. Permanent: the link stops \
                      working and turning sharing back on mints a new one, so \
                      a withdrawn link stays dead. It reaches no copy already \
                      sent, and Kamosu says so rather than letting that be \
                      discovered.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": { "branch_id": { "type": "string" } },
                "required": ["branch_id"],
                "additionalProperties": false,
            }),
            output_schema: share_link_schema(),
            handler: crate::operations::end_share_link,
        },
        Operation {
            name: "get_share_link",
            summary: "Whether a Recipe is shared, and by whom. The link's URL \
                      is answered only at the moment it is minted, since only \
                      the Secret's hash is stored — so this says a link \
                      exists without being able to reprint it.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": { "branch_id": { "type": "string" } },
                "required": ["branch_id"],
                "additionalProperties": false,
            }),
            output_schema: share_link_schema(),
            handler: crate::operations::get_share_link,
        },
        Operation {
            name: "set_public_address",
            summary: "Set where this instance is reachable from outside. Kept \
                      in the database and never in an environment variable, so \
                      moving an instance is one act that every Share Link \
                      already minted follows.",
            permission: Permission::Operator,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": { "public_address": { "type": "string" } },
                "required": ["public_address"],
                "additionalProperties": false,
            }),
            output_schema: json!({
                "type": "object",
                "properties": { "public_address": { "type": "string" } },
                "required": ["public_address"],
                "additionalProperties": false,
            }),
            handler: crate::operations::set_public_address,
        },
        Operation {
            name: "read_shared_recipe",
            summary: "Read a Recipe through its Share Link token: the Recipe \
                      as it stands, its Translations, and its Thread complete \
                      back to the first Version with every name and *what \
                      changed* line. Never an Attempt, a rating or an Attempt \
                      photograph. Public, because holding the token is the \
                      whole of the permission — this is what the Share Link \
                      page consumes, and the page is not an Operation, so \
                      Parity is untouched.",
            permission: Permission::Public,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": { "token": { "type": "string" } },
                "required": ["token"],
                "additionalProperties": false,
            }),
            output_schema: shared_recipe_schema(),
            handler: crate::operations::read_shared_recipe,
        },
        Operation {
            name: "branch_point",
            summary: "The last Version two Branches share, found by \
                      walking both chains back until they meet — never \
                      declared, always computed. A chain that does not \
                      converge on a shared first Version answers a \
                      damaged-Bundle error rather than a guess.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "branch_a_id": { "type": "string" },
                    "branch_b_id": { "type": "string" },
                },
                "required": ["branch_a_id", "branch_b_id"],
                "additionalProperties": false,
            }),
            output_schema: json!({
                "type": "object",
                "properties": { "version_id": { "type": "string" } },
                "required": ["version_id"],
                "additionalProperties": false,
            }),
            handler: crate::operations::branch_point,
        },
        Operation {
            name: "divergence",
            summary: "Two Branches of one Lineage laid over each other, so a \
                      screen can show two whole recipes with a switch between \
                      them rather than a difference (ADR 0014). Every row \
                      carries both sides' own words; a line only one side has \
                      is a Ghost. Which line is which is read against the \
                      Branch Point, never by an id stapled to a line \
                      (ADR 0019), and an uncertain reading declines to pair \
                      rather than claiming a connection.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    // Named from where the caller stands, not a/b: `mine`
                    // throughout the answer is this Branch. The rows are
                    // symmetric, so crossing to the other recipe is reading
                    // them from the other side, not asking again.
                    "branch_id": { "type": "string" },
                    "other_branch_id": { "type": "string" },
                },
                "required": ["branch_id", "other_branch_id"],
                "additionalProperties": false,
            }),
            output_schema: divergence_schema(),
            handler: crate::operations::divergence,
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
            job_lane: JobLane::ByCaller,
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
                    // The one subordinate line this corrected Reading now
                    // produces for the Person who corrected it (#49), so the
                    // screen redraws the whole row from one answer.
                    "measured": { "type": ["string", "null"] },
                },
                "required": ["line_index", "reading", "measured"],
                "additionalProperties": false,
            }),
            handler: crate::operations::set_reading,
        },
        Operation {
            name: "read_ingredient_lines",
            summary: "Read every Ingredient Line in the library that nothing \
                      has read yet, as a Job, laying a Reading over each one \
                      Kamosu can make sense of. Touches no written line and \
                      makes no Version. A line already carrying a Reading is \
                      left alone, so a correction is never overwritten, and a \
                      line Kamosu cannot read is left unread, which is an \
                      ordinary state for a line rather than a failure. Kamosu \
                      also reads the lines of every recipe as it is written or \
                      imported, so this is for a library that predates it.",
            permission: Permission::Operator,
            kind: Kind::Job,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: json!({
                "type": "object",
                "properties": { "read": { "type": "integer" } },
                "required": ["read"],
                "additionalProperties": false,
            }),
            handler: crate::operations::read_ingredient_lines,
        },
        Operation {
            name: "start_attempt",
            summary: "Start cooking a Recipe: creates the Attempt, or hands \
                      back the one already In Progress for this Lineage — \
                      the cooking screen is that Attempt, never a second \
                      thing beside it. Pinned by fingerprint to the \
                      Branch's head Version at this moment, or to \
                      version_id — an older Version read back from the \
                      Thread — when one is given. Anyone who can see the \
                      recipe may.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "branch_id": { "type": "string" },
                    "version_id": { "type": "string" },
                },
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
            job_lane: JobLane::ByCaller,
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
            summary: "End an In Progress Attempt, taking the judgement that \
                      lands with it: a rating, a note and Photographs, all \
                      optional. Ending is not what makes the cooking real — \
                      starting already did — only what stops it being In \
                      Progress, so a cook who says nothing still cooked.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "attempt_id": { "type": "string" },
                    "note": { "type": ["string", "null"] },
                    "rating": rating_schema(),
                    "photographs": {
                        "type": ["array", "null"],
                        "items": { "type": "string" },
                    },
                },
                "required": ["attempt_id"],
                "additionalProperties": false,
            }),
            output_schema: attempt_schema(),
            handler: crate::operations::finish_attempt,
        },
        Operation {
            name: "edit_attempt",
            summary: "Change an Attempt's free text, its rating or its \
                      Photographs, whether it is still In Progress or long \
                      finished — an Attempt is freely editable by its \
                      cook, unlike the recipe it was cooked from.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "attempt_id": { "type": "string" },
                    "note": { "type": ["string", "null"] },
                    "rating": rating_schema(),
                    "photographs": {
                        "type": ["array", "null"],
                        "items": { "type": "string" },
                    },
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
            job_lane: JobLane::ByCaller,
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
            name: "promote_attempt_photograph",
            summary: "Make a picture taken while cooking the recipe's Main \
                      Photo, or a Step's photo — so the picture you actually \
                      took becomes the recipe's picture. This is an ordinary \
                      edit making a Version, with everything that follows \
                      from it: a rapid re-save folding into the Version \
                      already being shaped, and a Copy where the Branch \
                      belongs to another Kitchen. The Attempt keeps the \
                      picture too; promoting is not moving.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "attempt_id": { "type": "string" },
                    "photograph_id": {
                        "type": "string",
                        "description": "One of this Attempt's own Photographs. Any other Photograph is refused: this is not a second way to set the Main Photo.",
                    },
                    "branch_id": {
                        "type": "string",
                        "description": "Which Branch of the cooked Lineage to promote into. An Attempt belongs to a Lineage rather than a Branch, so this says where the picture lands.",
                    },
                    "step_index": {
                        "type": ["integer", "null"],
                        "minimum": 0,
                        "description": "The Step whose photo this becomes. Left out or null, the picture becomes the Main Photo.",
                    },
                    "change_note": { "type": ["string", "null"] },
                },
                "required": ["attempt_id", "photograph_id", "branch_id"],
                "additionalProperties": false,
            }),
            output_schema: saved_version_schema(),
            handler: crate::operations::promote_attempt_photograph,
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
            job_lane: JobLane::ByCaller,
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
            name: "list_attempts",
            summary: "The cooking diary: every Attempt the caller has made, \
                      newest first, across every recipe — sorted by date \
                      rather than by recipe, which is what makes *what did \
                      I cook that week* answerable. Unfinished and In \
                      Progress cookings are in it too, because starting is \
                      what makes a cooking real. Each entry names the \
                      recipe it was cooked from, and still names it after \
                      that recipe has left the caller's shelf.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "attempts": { "type": "array", "items": diary_entry_schema() },
                },
                "required": ["attempts"],
                "additionalProperties": false,
            }),
            handler: crate::operations::list_attempts,
        },
        Operation {
            name: "list_foods",
            summary: "List every Food this instance knows, each shown in the \
                      reader's Reading Language where it has a name there.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
        Operation {
            name: "list_merge_suggestions",
            summary: "The Operator's worklist: every note that two Foods are \
                      probably one thing, with the words that said so. \
                      Evidence, never an instruction — nothing merges itself.",
            permission: Permission::Operator,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "suggestions": { "type": "array", "items": merge_suggestion_schema() },
                },
                "required": ["suggestions"],
                "additionalProperties": false,
            }),
            handler: crate::operations::list_merge_suggestions,
        },
        Operation {
            name: "preview_food_merge",
            summary: "Say how many Ingredient Lines a Merge would move, and \
                      how many Reading rows, without moving any of them. A \
                      Merge cannot be undone and refuses to run until this \
                      figure is said back to it, so this saying is its safety \
                      net rather than a courtesy.",
            permission: Permission::Operator,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "survivor_food_id": { "type": "string" },
                    "absorbed_food_id": { "type": "string" },
                },
                "required": ["survivor_food_id", "absorbed_food_id"],
                "additionalProperties": false,
            }),
            output_schema: merge_preview_schema(),
            handler: crate::operations::preview_food_merge,
        },
        Operation {
            name: "merge_food",
            summary: "Join two Foods into one: the survivor takes every name \
                      both had, every Reading pointing at the other points at \
                      it instead, and every Merge Suggestion naming either is \
                      cleared. ingredient_lines is the figure \
                      preview_food_merge announced, said back — a Merge that \
                      does not match it is refused. Where the two disagree \
                      about Cup Weight, cup_weight_grams says which of the two \
                      figures survives. There is no un-merge in v1.",
            permission: Permission::Operator,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "survivor_food_id": { "type": "string" },
                    "absorbed_food_id": { "type": "string" },
                    "ingredient_lines": { "type": "integer", "minimum": 0 },
                    "cup_weight_grams": { "type": ["number", "null"], "exclusiveMinimum": 0 },
                },
                "required": ["survivor_food_id", "absorbed_food_id", "ingredient_lines"],
                "additionalProperties": false,
            }),
            output_schema: merge_result_schema(),
            handler: crate::operations::merge_food,
        },
        Operation {
            name: "delete_food",
            summary: "Delete a Food nothing points at. One a Reading still \
                      points at is refused: what a Food knows was expensive to \
                      learn and is never discarded by an unrelated act.",
            permission: Permission::Operator,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({ "type": "object", "properties": { "food_id": { "type": "string" } }, "required": ["food_id"], "additionalProperties": false }),
            output_schema: json!({ "type": "object", "properties": { "deleted": { "type": "boolean" } }, "required": ["deleted"], "additionalProperties": false }),
            handler: crate::operations::delete_food,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
            job_lane: JobLane::ByCaller,
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
        job_lane: JobLane::ByCaller,
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

/// The four states Meaning Search can be in, declared once so every Operation
/// that answers one types it the same way.
///
///   `unasked`  — nobody has been asked yet, so the offer is live.
///   `declined` — an Operator said no. The offer never appears again.
///   `accepted` — the terms are accepted; the weights may not be here yet.
///   `on`       — accepted, downloaded, indexed, and answering searches.
fn meaning_state_enum() -> Value {
    json!({ "enum": ["unasked", "declined", "accepted", "on"] })
}

/// What `meaning_search_status` answers: enough for a screen to decide whether
/// to offer Meaning Search, and enough for an agent to know what it would be
/// agreeing to before it agrees.
fn meaning_status_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "state": meaning_state_enum(),
            // Whether searches are actually matching on meaning right now. Not
            // the same as `state == "on"`: an instance whose model is still
            // loading, or whose weights were deleted, is honestly off.
            "on": { "type": "boolean" },
            // Whether to make the offer, answered here rather than on a screen:
            // only an Operator can act on it, and an offer somebody cannot act
            // on is worse than no offer.
            "offer": { "type": "boolean" },
            // Whether this caller could turn Meaning Search on or off at all.
            // A screen working that out for itself would be a permission check
            // living outside the Core, which is a bug (ADR 0001).
            "may_change": { "type": "boolean" },
            "model": { "type": "string" },
            "terms_url": { "type": "string" },
            "prohibited_use_policy_url": { "type": "string" },
            "terms_version": { "type": "string" },
            // The Hand that accepted, and how it arrived. Recorded, never
            // verified — the same thing Kamosu says out loud about every Hand
            // it keeps (ADR 0015, ADR 0029).
            "accepted_by": { "type": ["string", "null"] },
            "accepted_via_access_key": { "type": ["boolean", "null"] },
            "accepted_at": { "type": ["string", "null"] },
            "declined_at": { "type": ["string", "null"] },
            "model_present": { "type": "boolean" },
            "indexed_at": { "type": ["string", "null"] },
            // How far behind the library the index is. It catches up by itself
            // within the minute; saying so beats pretending an edit made a
            // second ago is already findable by meaning.
            "recipes_not_yet_indexed": { "type": "integer" },
        },
        "required": [
            "state", "on", "offer", "may_change", "model", "terms_url",
            "prohibited_use_policy_url", "terms_version", "accepted_by",
            "accepted_via_access_key", "accepted_at", "declined_at",
            "model_present", "indexed_at", "recipes_not_yet_indexed"
        ],
        "additionalProperties": false,
    })
}

/// The shape the shelf is served in — one entry per Lineage, read back by
/// `search_recipes` whether or not anything was searched for (ADR 0027).
///
/// Deliberately thin: what a card needs and not one field more. In
/// particular it carries **no Kitchen name and no Kitchen id**, so a card has
/// nothing to print a fence with; the Kitchen a Branch belongs to is
/// machinery, and it appears where it means something — in the Thread, and
/// where two Branches sit side by side.
/// One card on a shelf — the most-repeated object in Kamosu, and therefore
/// declared exactly once. The library's shelf (`search_recipes`) and Home's
/// four (`home_shelves`) both answer in this shape, so a recipe is the same
/// object on both screens rather than two treatments of one thing that drift.
fn shelf_entry_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "lineage_id": { "type": "string" },
            // The Branch this entry opens: the one in the reader's
            // own Language where the Lineage has one.
            "branch_id": { "type": "string" },
            "title": { "type": "string" },
            "language": { "type": "string" },
            // True where the reader is being shown a Language they
            // did not ask for. The mark exists so a preference can
            // never hide a recipe from its owner (ADR 0006): the
            // recipe is shown either way, and says which it is.
            "language_fallback": { "type": "boolean" },
            "main_photo": { "type": ["string", "null"] },
            "yield": yield_schema(),
            // The line that matched, for a result that can explain
            // itself (ADR 0027). Null on an unsearched shelf, and
            // on a title match `line` is the title itself.
            "matched": {
                "type": ["object", "null"],
                "properties": {
                    "where": {
                        "type": "string",
                        // A Section header — "For the sauce" — is
                        // searched with the rest of the recipe but
                        // is neither an ingredient nor a step, so
                        // it answers as itself rather than being
                        // mislabelled as one.
                        "enum": [
                            "title", "tag", "ingredient", "step",
                            "section", "note", "attempt"
                        ],
                    },
                    "line": { "type": "string" },
                    // Which half of searching found this. A reader
                    // cannot tell the two apart by looking at the
                    // line, and the surprising result is exactly
                    // where trust is won or lost — so the answer
                    // says which it was rather than leaving the
                    // screen to guess (ADR 0027).
                    "by": { "enum": ["words", "meaning"] },
                    // Which step this is, counted as the recipe
                    // page counts them — over the Steps alone,
                    // Sections taking no number — so a result can
                    // say "step 4" and mean the step so numbered.
                    // Null for every other kind of match.
                    "step_number": { "type": ["integer", "null"] },
                },
                "required": ["where", "line", "step_number", "by"],
                "additionalProperties": false,
            },
        },
        "required": [
            "lineage_id", "branch_id", "title", "language",
            "language_fallback", "main_photo", "yield", "matched"
        ],
        "additionalProperties": false,
    })
}

fn shelf_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            // What was searched for, echoed back: null is the whole shelf.
            // The screen needs it to name the query in "nothing matched X"
            // without trusting that its own field still says what it asked.
            "query": { "type": ["string", "null"] },
            // True where nothing was close enough and what follows is the
            // nearest anyway. Meaning-matching always has a nearest neighbour,
            // so "nothing found" means "nothing close enough" — Kamosu says
            // exactly that and shows the closest under that label, rather than
            // letting a weak match pass as a good one (ADR 0027). Always false
            // where Meaning Search is off: with no model there is no nearest.
            "closest": { "type": "boolean" },
            "recipes": { "type": "array", "items": shelf_entry_schema() },
        },
        "required": ["query", "closest", "recipes"],
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
                        "measured": measured_schema(),
                        "cooking": cooking_schema(),
                        "translates_version_id": { "type": ["string", "null"] },
                        "language": { "type": ["string", "null"] },
                    },
                    "required": [
                        "sequence", "version_id", "parent_version_id", "hand_id",
                        "name", "change_note", "created_at", "content", "readings",
                        "measured", "cooking", "translates_version_id", "language"
                    ],
                    "additionalProperties": false,
                },
            },
            "translation": translation_schema(),
            // The Tags this Kitchen files the recipe under, as it stands now.
            // Outside `versions` on purpose: filing is not recipe content and
            // is named by no fingerprint (ADR 0035).
            "tags": { "type": "array", "items": tag_schema() },
            "related_recipes": { "type": "array", "items": related_recipe_schema() },
            // Beside the Versions, never inside one. `recipe_content_schema`
            // — what a Version *is*, and the whole of what a fingerprint names
            // — carries no rating, no note and no Photograph of anybody's
            // cooking, and `a_version_carries_nothing_of_a_cooking` holds that
            // line. This is what a Share Link page (#65) must render from: a
            // share renders a Version, and no path leads from one back to an
            // Attempt.
            "cooked": cooking_record_schema(),
        },
        "required": [
            "branch_id", "lineage_id", "kitchen_id", "hand_id", "language",
            "origin_address", "head_version_id", "versions", "translation",
            "tags", "related_recipes", "cooked"
        ],
        "additionalProperties": false,
    })
}

/// What saving a Version answers. Shared by `save_recipe_version` and by
/// `promote_attempt_photograph`, because promoting a picture *is* an ordinary
/// save (#59) — a caller that can read one answer can read the other, Copy and
/// collapse included.
fn saved_version_schema() -> Value {
    json!({
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
            "language": {
                "type": "string",
                "description": "The Language this recipe still carries. A save never changes it.",
            },
            "language_offer": {
                "type": ["string", "null"],
                "description": "The Language this text reads as, when that disagrees with the one the recipe carries — an offer to put to the cook, never a change. Null when they agree, when there is too little text to tell, and always when the Language is unknown.",
            },
            "translates_version_id": {
                "type": ["string", "null"],
                "description": "The Version of the source this Version renders, for a Translation. Carried forward from the Version replaced unless this save named a new one; null on a recipe that translates nothing.",
            },
        },
        "required": [
            "branch_id", "version_id", "parent_version_id", "sequence", "collapsed",
            "copied", "language", "language_offer", "translates_version_id"
        ],
        "additionalProperties": false,
    })
}

/// How a recipe has been cooked, as the recipe screen shows it (#59).
///
/// There is deliberately **no average, mean or aggregate score in this shape,
/// and no way to derive one** (ADR 0015). A rating is a word rather than a
/// number, and what is served is one row per Person — their most recent
/// verdict, with their name — rather than a distribution. A superseded verdict
/// never reaches a reader at all, so it cannot permanently drag down a recipe
/// that has since been fixed.
fn cooking_record_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            // Every Attempt, unfinished ones included (ADR 0010): a cooking is
            // real from the moment it starts.
            "count": { "type": "integer", "minimum": 0 },
            "last_cooked_at": { "type": ["string", "null"] },
            "ratings": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "person_id": { "type": "string" },
                        "name": { "type": "string" },
                        "rating": { "type": "string", "enum": ["again", "tweak", "no"] },
                        "at": { "type": "string" },
                    },
                    "required": ["person_id", "name", "rating", "at"],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["count", "last_cooked_at", "ratings"],
        "additionalProperties": false,
    })
}

/// How a recipe stands as a Translation, or null — which the overwhelming
/// majority of recipes are. Nothing here is stored as a fact of its own: the
/// original is simply the Branch that translates nothing, and how far behind
/// this one has fallen is counted off the source Branch as it stands right now
/// (ADR 0006).
fn translation_schema() -> Value {
    json!({
        "type": ["object", "null"],
        "properties": {
            "translates_version_id": {
                "type": "string",
                "description": "The Version of the source this recipe's newest Version renders.",
            },
            "source_branch_id": {
                "type": ["string", "null"],
                "description": "The recipe this one translates. Null where that recipe is not on this instance — a Translation may arrive on its own, and how far behind it has fallen is then unanswerable rather than zero.",
            },
            "versions_behind": {
                "type": ["integer", "null"],
                "description": "How many Versions the source has moved on since the one this translates. Zero means up to date.",
            },
        },
        "required": ["translates_version_id", "source_branch_id", "versions_behind"],
        "additionalProperties": false,
    })
}

/// One occurrence of one Version on one Branch, as the Thread shows it —
/// deliberately lighter than a `recipe_schema` entry: no content, no
/// Readings, since the Thread reads back names and *what changed* lines, not
/// the recipe itself (CONTEXT.md, "Thread").
fn thread_version_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "branch_id": { "type": "string" },
            "sequence": { "type": "integer" },
            "version_id": { "type": "string" },
            "parent_version_id": { "type": ["string", "null"] },
            "hand_id": { "type": "string" },
            "name": { "type": ["string", "null"] },
            "change_note": { "type": ["string", "null"] },
            "created_at": { "type": "string" },
            "translates_version_id": { "type": ["string", "null"] },
            "language": { "type": ["string", "null"] },
        },
        "required": [
            "branch_id", "sequence", "version_id", "parent_version_id",
            "hand_id", "name", "change_note", "created_at",
            "translates_version_id", "language",
        ],
        "additionalProperties": false,
    })
}

/// One Branch as the Thread lists it — brief, since its Versions are carried
/// in `versions` rather than nested here.
fn thread_branch_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "branch_id": { "type": "string" },
            "kitchen_id": { "type": "string" },
            "hand_id": { "type": "string" },
            "language": { "type": "string" },
            "head_version_id": { "type": "string" },
            "translation": translation_schema(),
        },
        "required": [
            "branch_id", "kitchen_id", "hand_id", "language", "head_version_id", "translation"
        ],
        "additionalProperties": false,
    })
}

/// The shape a Thread is served in — read back by `get_thread`. Every
/// Branch of the Lineage the caller can see, every Version of each (oldest
/// first, forking read out of `parent_version_id` or asked of
/// `branch_point`), and every Attempt hanging off it.
fn thread_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "lineage_id": { "type": "string" },
            "branches": { "type": "array", "items": thread_branch_schema() },
            "versions": { "type": "array", "items": thread_version_schema() },
            "attempts": { "type": "array", "items": attempt_schema() },
        },
        "required": ["lineage_id", "branches", "versions", "attempts"],
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
/// A Yield: one amount and what it is an amount of. Written once here because
/// a recipe's own content and a shelf entry must not be able to disagree about
/// its shape.
fn yield_schema() -> Value {
    json!({
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
    })
}

fn recipe_content_properties() -> Value {
    json!({
        "title": { "type": "string" },
        "yield": yield_schema(),
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

/// One place in two recipes laid over each other. Both sides' own words are
/// always here — there is no summary of a change anywhere in this schema,
/// because there is no such object as "the difference" (ADR 0014).
fn divergence_row_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "kind": { "type": "string" },
            // same: both sides have it and the words are identical.
            // changed: both sides have it and the words are not.
            // only-mine / only-theirs: one side has it — a Ghost, seen from
            // the other. There is deliberately no confidence value anywhere:
            // an uncertain reading declines to pair, and shows as two
            // unjoined rows rather than one row wearing a hedge (ADR 0019).
            "state": { "enum": ["same", "changed", "only-mine", "only-theirs"] },
            // Whether this line was already at the Branch Point. It separates
            // "they took this out" from "they never had it" — the same Ghost,
            // a different sentence, and only the first can be carried across
            // as a removal.
            "from_branch_point": { "type": "boolean" },
            "mine": divergence_line_schema(),
            "theirs": divergence_line_schema(),
        },
        "required": ["kind", "state", "from_branch_point", "mine", "theirs"],
        "additionalProperties": false,
    })
}

/// One side's line at one row. `index` is where it sits in that side's own list
/// — how its Reading is found, never how it is matched.
fn divergence_line_schema() -> Value {
    json!({
        "type": ["object", "null"],
        "properties": {
            "kind": { "type": "string" },
            "text": { "type": "string" },
            "index": { "type": "integer" },
        },
        "required": ["kind", "text", "index"],
        "additionalProperties": false,
    })
}

/// A single value on both sides: same or not, with nothing to pair.
fn divergence_field_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "same": { "type": "boolean" },
            "mine": {},
            "theirs": {},
        },
        "required": ["same", "mine", "theirs"],
        "additionalProperties": false,
    })
}

/// One Branch as one side of the switch: the Kitchen you stand in when you are
/// reading it, its whole current content, and its Readings.
fn divergence_branch_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "branch_id": { "type": "string" },
            "kitchen_id": { "type": "string" },
            "kitchen_name": { "type": "string" },
            "hand_id": { "type": "string" },
            "language": { "type": "string" },
            "head_version_id": { "type": "string" },
            "content": recipe_content_schema(),
            "readings": reading_list_schema(),
            "measured": measured_schema(),
        },
        "required": [
            "branch_id", "kitchen_id", "kitchen_name", "hand_id", "language",
            "head_version_id", "content", "readings", "measured",
        ],
        "additionalProperties": false,
    })
}

fn divergence_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "lineage_id": { "type": "string" },
            "branch_point_version_id": { "type": "string" },
            "mine": divergence_branch_schema(),
            "theirs": divergence_branch_schema(),
            "ingredients": { "type": "array", "items": divergence_row_schema() },
            "steps": { "type": "array", "items": divergence_row_schema() },
            // Title, Yield, times, Source, Note and the Main Photo are single
            // values. Tags are absent on purpose: they are how a Kitchen files,
            // not what a recipe is, and marking them would put a "take theirs"
            // offer under a difference between two filing systems (ADR 0035).
            "fields": {
                "type": "object",
                "properties": {
                    "title": divergence_field_schema(),
                    "yield": divergence_field_schema(),
                    "prep_time_minutes": divergence_field_schema(),
                    "cook_time_minutes": divergence_field_schema(),
                    "source": divergence_field_schema(),
                    "note": divergence_field_schema(),
                    "main_photo": divergence_field_schema(),
                },
                "required": [
                    "title", "yield", "prep_time_minutes", "cook_time_minutes",
                    "source", "note", "main_photo",
                ],
                "additionalProperties": false,
            },
        },
        "required": [
            "lineage_id", "branch_point_version_id", "mine", "theirs",
            "ingredients", "steps", "fields",
        ],
        "additionalProperties": false,
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
        json!({
            "enum": ["en", "fr", "es", "unknown"],
            "description": "The Language this recipe is written in. Left out, it \
                             is detected from the recipe's own text, falling back \
                             to the writer's Reading Language where there is too \
                             little text to tell. `unknown` says the recipe is \
                             honestly more than one Language.",
        }),
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
    map.insert(
        "translates_version_id".to_string(),
        json!({
            "type": "string",
            "description": "For a Translation, the Version of the source this save \
                             now renders — how bringing a Translation up to date is \
                             said. Left out, whatever the Version being replaced \
                             pointed at is carried forward, so editing a \
                             Translation's wording never claims it has caught up.",
        }),
    );
    json!({
        "type": "object",
        "properties": properties,
        "required": ["branch_id", "title"],
        "additionalProperties": false,
    })
}

/// `start_translation`'s input: the recipe as it now reads in the new Language,
/// the Branch it is a rendering of, and — optionally — which Version of that
/// Branch it renders, defaulting to wherever the source stands now.
fn start_translation_input_schema() -> Value {
    let mut properties = recipe_content_properties();
    let map = properties.as_object_mut().expect("object schema");
    map.insert(
        "branch_id".to_string(),
        json!({
            "type": "string",
            "description": "The recipe being translated.",
        }),
    );
    map.insert(
        "language".to_string(),
        json!({
            "enum": ["en", "fr", "es"],
            "description": "The Language this rendering is written in — necessarily \
                             a different one from the recipe it translates. Never \
                             `unknown`: a recipe that is honestly more than one \
                             Language can neither be a Translation nor have one.",
        }),
    );
    map.insert(
        "translates_version_id".to_string(),
        json!({
            "type": "string",
            "description": "Which Version of the source this renders. Defaults to \
                             where the source stands now.",
        }),
    );
    map.insert("name".to_string(), json!({ "type": "string" }));
    map.insert("change_note".to_string(), json!({ "type": "string" }));
    map.insert(
        "kitchen_id".to_string(),
        json!({
            "type": "string",
            "description": "Which of your own Kitchens holds the Translation. \
                             Defaults to the one holding the recipe translated.",
        }),
    );
    json!({
        "type": "object",
        "properties": properties,
        "required": ["branch_id", "language", "title"],
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
        json!({
            "enum": ["en", "fr", "es", "unknown"],
            "description": "The Language this recipe is written in. Left out, it \
                             is detected from the recipe's own text, falling back \
                             to the writer's Reading Language where there is too \
                             little text to tell. `unknown` says the recipe is \
                             honestly more than one Language.",
        }),
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
/// Whether a Recipe is shared, and by whom (#65, ADR 0026).
///
/// `url` is answered **only at the moment a link is minted**, and is null
/// every other time. Only the Secret's hash is stored, so there is nothing to
/// reprint later — the same bargain every Secret in Kamosu makes (ADR 0031).
/// The share screen therefore knows a link exists without being able to show
/// it again, which is honest about what an instance actually holds.
fn share_link_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "shared": {
                "type": "boolean",
                "description": "Whether a live Share Link exists. This is the whole of \
                                 Visibility: there is no scale and no third audience.",
            },
            "share_id": { "type": ["string", "null"] },
            "url": {
                "type": ["string", "null"],
                "description": "The link itself, answered once, at the moment it is minted.",
            },
            "shared_by": {
                "type": ["string", "null"],
                "description": "The Name of the Person who turned the link on, looked up \
                                 live. Never a Kitchen: a Kitchen's Nickname is private to \
                                 the member who set it and could not appear on a public page.",
            },
            "created_at": { "type": ["string", "null"] },
            "public_address": { "type": ["string", "null"] },
        },
        "required": ["shared", "share_id", "url", "shared_by", "created_at", "public_address"],
        "additionalProperties": false,
    })
}

/// One Branch as a stranger holding a Share Link sees it: the words, the
/// Readings, and the one subordinate line each produces.
///
/// There is no `measured` slot: Kamosu converts to the kitchen (ADR 0016) and
/// a stranger holding a link has no kitchen, so there is no reader to convert
/// for. The Reading still travels, as structure for an agent reading the same
/// share at the MCP door.
fn shared_version_schema() -> Value {
    shared_version_schema_of(json!("object"))
}

/// The same shape, declared nullable — `recipe` is absent on an ended link.
/// Written as a `type` array rather than an `anyOf` because that is the subset
/// of JSON Schema the Catalogue declares and the typed client renders; a shape
/// outside it is one the interface would silently stop checking.
fn nullable_shared_version_schema() -> Value {
    shared_version_schema_of(json!(["object", "null"]))
}

fn shared_version_schema_of(type_: Value) -> Value {
    json!({
        "type": type_,
        "properties": {
            "branch_id": { "type": "string" },
            "lineage_id": {
                "type": "string",
                "description": "What a Cover is drawn from, for a recipe with no \
                                 photograph — the Lineage id and nothing else (#46).",
            },
            "version_id": { "type": "string" },
            "language": { "type": "string" },
            "content": recipe_content_schema(),
            "readings": reading_list_schema(),
        },
        "required": [
            "branch_id", "lineage_id", "version_id", "language",
            "content", "readings",
        ],
        "additionalProperties": false,
    })
}

/// Everything the public Share Link page draws.
///
/// **No Attempt appears here in any form** — not a rating, not a note, not an
/// Attempt photograph (ADR 0005, ADR 0026). There is no field for one.
///
/// The Thread is complete back to the first Version, names and *what changed*
/// lines included: a share cannot be made to start part-way along, so there is
/// no argument anywhere that could shorten it (ADR 0018).
fn shared_recipe_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "ended": {
                "type": "boolean",
                "description": "Whether this link has been ended. A token nobody minted is \
                                 not found; a real token whose link was withdrawn answers \
                                 here, because 'no such page' reads as a mistake to retry.",
            },
            "share_id": { "type": "string" },
            "shared_by": { "type": ["string", "null"] },
            "public_address": { "type": ["string", "null"] },
            "recipe": nullable_shared_version_schema(),
            "translations": {
                "type": "array",
                "items": shared_version_schema(),
                "description": "The Branches of this Lineage in another Language that \
                                 translate the Branch shared (ADR 0006). Carried whole \
                                 rather than as links: each is a Branch of its own, and a \
                                 token per Translation would be a second link to end.",
            },
            "thread": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "sequence": { "type": "integer" },
                        "name": { "type": ["string", "null"] },
                        "change_note": { "type": ["string", "null"] },
                        "hand": {
                            "type": "string",
                            "description": "The Name of the Person who wrote this Version. \
                                             What makes credit travel with a recipe.",
                        },
                        "created_at": { "type": "string" },
                    },
                    "required": ["sequence", "name", "change_note", "hand", "created_at"],
                    "additionalProperties": false,
                },
            },
        },
        "required": [
            "ended", "share_id", "shared_by", "public_address",
            "recipe", "translations", "thread",
        ],
        "additionalProperties": false,
    })
}

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

/// **The one subordinate line** under each Ingredient Line and each Step, for
/// the Person asking — scaling and conversion in a single slot (ADR 0016, #49).
///
/// One slot per line, in the same order as the list it belongs to, and `null`
/// wherever there is nothing to say: no quantity read, a quantity Kamosu could
/// not read, a Step with no temperature in it, or — the common case — a line
/// already in this reader's measures at the Yield as written, where a line
/// would only repeat what is above it.
///
/// Rendered rather than structured, in the reader's Reading Language and
/// always saying *about*, so an agent asked *how much flour in grams* gets the
/// answer a cook would read (ADR 0001). It is computed, never stored: Reading
/// Measures is a preference and changing it makes no Version.
fn measured_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "ingredients": { "type": "array", "items": { "type": ["string", "null"] } },
            "steps": { "type": "array", "items": { "type": ["string", "null"] } },
        },
        "required": ["ingredients", "steps"],
        "additionalProperties": false,
    })
}

/// **What the cooking screen reads out of each Step**, and Kamosu stores
/// nowhere (ADR 0011, CONTEXT.md "Step"): which Ingredient Lines the Step uses,
/// and the duration it offers as a timer.
///
/// One slot per row of the Version's `steps`, in the same order `measured`
/// takes, and `null` on a Section row — a Section is neither a Step nor
/// somewhere a cook stands.
///
/// `uses` names Ingredient Lines by their index into the Version's own
/// `ingredients`, and an empty list is a real answer rather than a gap: it is
/// the step that adds nothing new, where the panel says so. Which lines those
/// are is worked out from their **Readings** on every read — nothing points at
/// anything and nobody types a link (ADR 0019) — so a recipe Kamosu has read
/// nothing on has a panel that is simply empty, per ADR 0002's rule that
/// anything built on a Reading degrades politely.
///
/// `timer_seconds` is read out of the Step's own text at display time. Nothing
/// is typed beside the sentence and nothing is written down: a range answers
/// with its lower end, because a timer that goes off early sends you to look
/// and one that goes off late has already let it burn.
fn cooking_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "steps": {
                "type": "array",
                "items": {
                    "type": ["object", "null"],
                    "properties": {
                        "uses": {
                            "type": "array",
                            "items": { "type": "integer", "minimum": 0 },
                        },
                        "timer_seconds": { "type": ["integer", "null"], "minimum": 1 },
                    },
                    "required": ["uses", "timer_seconds"],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["steps"],
        "additionalProperties": false,
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

/// A Merge Suggestion as the Operator's worklist serves it: the two Foods in
/// full, the reason the note was made, and the words that made it. Evidence,
/// never an instruction (ADR 0022) — reading this list merges nothing.
fn merge_suggestion_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "foods": { "type": "array", "items": food_schema(), "minItems": 2, "maxItems": 2 },
            "reason": { "enum": ["arrived_as_one", "name_typed_onto_another"] },
            "words": {
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
            "created_at": { "type": "string" },
        },
        "required": ["foods", "reason", "words", "created_at"],
        "additionalProperties": false,
    })
}

/// What a Merge is about to move, with both Foods in full so the Operator can
/// read what they are agreeing to. `ingredient_lines` is what a cook would see
/// move — the Readings lying on recipes as they stand today — and is the
/// figure `merge_food` requires said back. `readings` is every row that
/// changes hands, past Versions included, which is larger whenever a recipe
/// has been edited since its line was read. `cup_weight_conflict` is the one
/// question a Merge may have to ask.
fn merge_preview_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "survivor": food_schema(),
            "absorbed": food_schema(),
            "ingredient_lines": { "type": "integer", "minimum": 0 },
            "readings": { "type": "integer", "minimum": 0 },
            "cup_weight_conflict": { "type": "boolean" },
        },
        "required": [
            "survivor", "absorbed", "ingredient_lines", "readings",
            "cup_weight_conflict",
        ],
        "additionalProperties": false,
    })
}

/// What a Merge did: the surviving Food, and the same two numbers the preview
/// announced — which is how "the announced count matches what moves" is
/// checkable rather than merely promised.
fn merge_result_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "food": food_schema(),
            "ingredient_lines": { "type": "integer", "minimum": 0 },
            "readings": { "type": "integer", "minimum": 0 },
        },
        "required": ["food", "ingredient_lines", "readings"],
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
            "rating": rating_schema(),
            "finished_at": { "type": ["string", "null"] },
            "resumable": { "type": "boolean" },
            "created_at": { "type": "string" },
            "last_action_at": { "type": "string" },
            // The Photographs taken during this cooking, in the order they
            // were added. They belong to the Attempt and to no Version, which
            // is why a Share Link cannot reach them: a share renders a
            // Version, and nothing leads from a Version back to an Attempt.
            "photographs": { "type": "array", "items": { "type": "string" } },
        },
        "required": [
            "id", "lineage_id", "person_id", "version_id", "current_step_index",
            "ticked_ingredients", "cooking_yield", "note", "rating", "finished_at",
            "resumable", "created_at", "last_action_at", "photographs",
        ],
        "additionalProperties": false,
    })
}

/// What a rating is (#59): the cook's decision about next time, in one of
/// three words, or absent.
///
/// Deliberately not a number. ADR 0015 refuses to average ratings, and a scale
/// out of five invites a reader to do that arithmetic in their own head even
/// where Kamosu never does; three words make the refusal self-evident. The
/// scale was Aurélien's choice, recorded on #59 before this was written.
fn rating_schema() -> Value {
    json!({
        "type": ["string", "null"],
        "enum": ["again", "tweak", "no", null],
    })
}

/// `attempt_schema`, nullable — `get_current_attempt` answers no Attempt at
/// all wherever the caller has none In Progress on that Lineage.
fn attempt_or_null_schema() -> Value {
    let mut schema = attempt_schema();
    schema["type"] = json!(["object", "null"]);
    schema
}

/// One line of the cooking diary (#60): an ordinary Attempt, plus the recipe
/// it was cooked from.
///
/// Grown from `attempt_schema` rather than written out beside it, so an
/// Attempt has exactly one declared shape wherever it is served and a field
/// added there cannot go missing here.
///
/// **No mark says whether a cooking is finished, In Progress or walked away
/// from**, because `finished_at` and `resumable` already say it between them
/// and a third field could only ever disagree with them: not finished and
/// still offered is In Progress, not finished and no longer offered is a
/// cooking somebody walked away from — which happened all the same (ADR 0010).
fn diary_entry_schema() -> Value {
    let mut schema = attempt_schema();
    schema["properties"]["recipe"] = json!({
        "type": "object",
        "properties": {
            // The Branch this entry opens, chosen the way the shelf chooses
            // which Branch a card opens. **Null** where the recipe has left
            // the caller's shelf — they left the Kitchen holding it, say. The
            // Attempt is theirs and stays; the pointer is the part that goes
            // (the rule #52 settled for Related Recipes).
            "branch_id": { "type": ["string", "null"] },
            // The name the recipe goes by now where it is still on the shelf,
            // and the name it was known by — off the Version actually cooked —
            // where it is not. Text is better than a broken pointer.
            "title": { "type": "string" },
        },
        "required": ["branch_id", "title"],
        "additionalProperties": false,
    });
    schema["required"]
        .as_array_mut()
        .expect("attempt_schema declares its required fields as an array")
        .push(json!("recipe"));
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
