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
            }),
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
            summary: "End an In Progress Attempt. Ending is not what makes \
                      the cooking real — starting already did — only what \
                      stops it being In Progress.",
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
            job_lane: JobLane::ByCaller,
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

/// The shape the shelf is served in — one entry per Lineage, read back by
/// `search_recipes` whether or not anything was searched for (ADR 0027).
///
/// Deliberately thin: what a card needs and not one field more. In
/// particular it carries **no Kitchen name and no Kitchen id**, so a card has
/// nothing to print a fence with; the Kitchen a Branch belongs to is
/// machinery, and it appears where it means something — in the Thread, and
/// where two Branches sit side by side.
fn shelf_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            // What was searched for, echoed back: null is the whole shelf.
            // The screen needs it to name the query in "nothing matched X"
            // without trusting that its own field still says what it asked.
            "query": { "type": ["string", "null"] },
            "recipes": {
                "type": "array",
                "items": {
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
                                // Which step this is, counted as the recipe
                                // page counts them — over the Steps alone,
                                // Sections taking no number — so a result can
                                // say "step 4" and mean the step so numbered.
                                // Null for every other kind of match.
                                "step_number": { "type": ["integer", "null"] },
                            },
                            "required": ["where", "line", "step_number"],
                            "additionalProperties": false,
                        },
                    },
                    "required": [
                        "lineage_id", "branch_id", "title", "language",
                        "language_fallback", "main_photo", "yield", "matched"
                    ],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["query", "recipes"],
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
                        "translates_version_id": { "type": ["string", "null"] },
                        "language": { "type": ["string", "null"] },
                    },
                    "required": [
                        "sequence", "version_id", "parent_version_id", "hand_id",
                        "name", "change_note", "created_at", "content", "readings",
                        "translates_version_id", "language"
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
        },
        "required": [
            "branch_id", "lineage_id", "kitchen_id", "hand_id", "language",
            "origin_address", "head_version_id", "versions", "translation",
            "tags", "related_recipes"
        ],
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
        },
        "required": [
            "branch_id", "kitchen_id", "kitchen_name", "hand_id", "language",
            "head_version_id", "content", "readings",
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
