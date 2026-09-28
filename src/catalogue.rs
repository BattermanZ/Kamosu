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

impl Operation {
    /// What a caller that reads text is told this Operation does: its summary,
    /// and for a Job also how the answer is followed (#146). A Door that reads
    /// the Catalogue out as text says this rather than phrasing its own. The
    /// Job sentence is the one `Kind::Job` is documented with, then the
    /// summaries of the two Operations that watch a Job, read from their
    /// declarations so it cannot promise more than they do.
    pub fn description(&self) -> String {
        match self.kind {
            Kind::Immediate => self.summary.to_string(),
            Kind::Job => {
                let summary = |name| {
                    find(name)
                        .expect("the Operations that watch a Job are declared")
                        .summary
                };
                format!(
                    "{} A Job: asking returns a job id at once; state, progress and result \
                     are ordinary Operations. Where a task answers instead, its taskId is \
                     that job id. get_job: {} cancel_job: {}",
                    self.summary,
                    summary("get_job"),
                    summary("cancel_job"),
                )
            }
        }
    }
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
            summary: "The version of this Kamosu, whether setup has happened, and the shortest password it accepts.",
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
                    "password_minimum": { "type": "integer", "minimum": 1 },
                },
                "required": ["version", "setup_complete", "password_minimum"],
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
            name: "get_person",
            summary: "Who this Credential names: the Person's permanent id, \
                      which is also their Hand, and the name they currently \
                      go by — the name every Version they wrote shows here, \
                      and the one they sign in with.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: json!({ "type": "object", "properties": { "person_id": { "type": "string" }, "name": { "type": "string" } }, "required": ["person_id", "name"], "additionalProperties": false }),
            handler: crate::operations::get_person,
        },
        Operation {
            name: "rename_person",
            summary: "Change this Person's current reminder name. Every \
                      Version they ever wrote shows the new one on this \
                      instance, since a Hand is named live and nothing is \
                      keyed on the name; no id or fingerprint moves. It is \
                      also the name they sign in with, so a name somebody \
                      else here signs in with is refused. A Bundle already \
                      sent keeps the name it left with.",
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
            name: "list_accounts",
            summary: "Who holds an account on this instance: their name, \
                      whether they administer it, and whether the account is \
                      disabled. Nothing about what they cook — the Operator \
                      administers and does not read (ADR 0007), so no recipe, \
                      Attempt, Cookbook or Kitchen of theirs is reachable \
                      from here.",
            permission: Permission::Operator,
            kind: Kind::Immediate,
            write: false,
            // The same rule the rest of this family follows: administering
            // accounts is something a person does signed in, never something a
            // borrowed Access Key does on their behalf (ADR 0007).
            session_only: true,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "accounts": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" },
                                "is_operator": { "type": "boolean" },
                                "disabled": { "type": "boolean" },
                                "is_you": { "type": "boolean" },
                                "created_at": { "type": "string" },
                            },
                            "required": [
                                "name", "is_operator", "disabled", "is_you", "created_at",
                            ],
                            "additionalProperties": false,
                        },
                    },
                },
                "required": ["accounts"],
                "additionalProperties": false,
            }),
            handler: crate::operations::list_accounts,
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
            summary: "Delete an account while preserving its Hand in history. \
                      The Person's name is freed for somebody new to sign in \
                      with; what they wrote keeps their Hand and the name \
                      they had. Disabling an account keeps the name.",
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
            name: "set_operator",
            summary: "Make a Person an Operator, or stop them being one. The \
                      last Operator cannot be demoted (ADR 0007): an instance \
                      with nobody to administer it can never get one back, so \
                      the refusal is the point rather than a nicety.",
            permission: Permission::Operator,
            kind: Kind::Immediate,
            write: true,
            session_only: true,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "is_operator": { "type": "boolean" },
                },
                "required": ["name", "is_operator"],
                "additionalProperties": false,
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "is_operator": { "type": "boolean" },
                },
                "required": ["name", "is_operator"],
                "additionalProperties": false,
            }),
            handler: crate::operations::set_operator,
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
            name: "take_backup",
            summary: "Take a Backup now, as a Job: one archive holding a consistent copy of the database and every Photograph, written beside the database under /data. Kamosu keeps three — one taken daily, one weekly, one monthly — and takes them on its own; this asks for one now. A Job because an archive is the size of the library. It is never sent anywhere: fetch the bytes at GET /api/backups/<name>.",
            permission: Permission::Operator,
            kind: Kind::Job,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: json!({ "type": "object", "properties": { "taken": { "type": "array", "items": { "type": "string" } }, "backups": backups_schema() }, "required": ["taken", "backups"], "additionalProperties": false }),
            handler: crate::operations::take_backup,
        },
        Operation {
            name: "list_backups",
            summary: "The Backups this instance holds, newest first. Each can be fetched at GET /api/backups/<name>, under the same Credential as any Operation.",
            permission: Permission::Operator,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: json!({ "type": "object", "properties": { "backups": backups_schema() }, "required": ["backups"], "additionalProperties": false }),
            handler: crate::operations::list_backups,
        },
        Operation {
            name: "list_sessions",
            summary: "List this Person's browser Sessions by device and last use. \
                      `current` marks the Session asking, so it is never set \
                      when an Access Key asks.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: json!({ "type": "object", "properties": { "sessions": { "type": "array", "items": { "type": "object", "properties": { "id": {"type":"string"}, "name": {"type":"string"}, "created_at": {"type":"string"}, "last_used_at": {"type":["string","null"]}, "revoked": {"type":"boolean"}, "current": {"type":"boolean"} }, "required":["id","name","created_at","last_used_at","revoked","current"], "additionalProperties": false } } }, "required":["sessions"], "additionalProperties": false }),
            handler: crate::operations::list_sessions,
        },
        Operation {
            name: "rename_session",
            summary: "Rename one of your browser Sessions. A Session is named \
                      for its device when it signs in; this corrects the guess \
                      or names an older one. An ended Session is not renamed.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({ "type": "object", "properties": { "session_id": { "type": "string" }, "name": { "type": "string" } }, "required": ["session_id", "name"], "additionalProperties": false }),
            output_schema: json!({ "type": "object", "properties": { "id": { "type": "string" }, "name": { "type": "string" } }, "required": ["id", "name"], "additionalProperties": false }),
            handler: crate::operations::rename_session,
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
            summary: "Create a Kitchen: a group of People who see and cook from \
                      each other's Cookbooks. Its creator is its first member.",
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
                      leave. Their Cookbook leaves with them; each member who \
                      stays keeps a Branch of every recipe of theirs they \
                      cooked, and they keep one of every recipe they cooked \
                      from the others.",
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
            name: "preview_leaving_kitchen",
            summary: "What removing a Person from a Kitchen would leave each \
                      side, before anybody does it: how many recipes the \
                      members who stay keep, and how many the one leaving \
                      keeps. `person_id` defaults to you.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({ "type": "object", "properties": { "kitchen_id": { "type": "string" }, "person_id": { "type": "string" } }, "required": ["kitchen_id"], "additionalProperties": false }),
            output_schema: json!({ "type": "object", "properties": { "they_keep": { "type": "integer" }, "you_keep": { "type": "integer" } }, "required": ["they_keep", "you_keep"], "additionalProperties": false }),
            handler: crate::operations::preview_leaving_kitchen,
        },
        Operation {
            name: "get_cookbook",
            summary: "Your own Cookbook: its name, who writes it, how many \
                      recipes it holds, the Kitchens that see it and the \
                      Invites still waiting.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: cookbook_schema(),
            handler: crate::operations::get_cookbook,
        },
        Operation {
            name: "rename_cookbook",
            summary: "Give your Cookbook a name of its own, or clear it back to \
                      its Co-authors' names with an empty or null one. Any \
                      Co-author may.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({ "type": "object", "properties": { "name": { "type": ["string", "null"] } }, "required": ["name"], "additionalProperties": false }),
            output_schema: cookbook_schema(),
            handler: crate::operations::rename_cookbook,
        },
        Operation {
            name: "invite_to_cookbook",
            summary: "Mint a one-use Invite for somebody to write your Cookbook \
                      with you. When they accept, their recipes and yours become \
                      one Cookbook either of you changes.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: json!({ "type": "object", "properties": { "invite_id": { "type": "string" }, "secret": { "type": "string" } }, "required": ["invite_id", "secret"], "additionalProperties": false }),
            handler: crate::operations::invite_to_cookbook,
        },
        Operation {
            name: "cancel_cookbook_invite",
            summary: "End a Cookbook Invite nobody has used yet.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({ "type": "object", "properties": { "invite_id": { "type": "string" } }, "required": ["invite_id"], "additionalProperties": false }),
            output_schema: json!({ "type": "object", "properties": { "ended": { "type": "boolean" } }, "required": ["ended"], "additionalProperties": false }),
            handler: crate::operations::cancel_cookbook_invite,
        },
        Operation {
            name: "read_cookbook_invite",
            summary: "What accepting a Cookbook Invite would do, before you say \
                      yes: whose Cookbook it is, and how many recipes on each \
                      side become one.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({ "type": "object", "properties": { "secret": { "type": "string" } }, "required": ["secret"], "additionalProperties": false }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "cookbook": cookbook_schema(),
                    "invited_by": person_ref_schema(),
                    "their_recipes": { "type": "integer" },
                    "your_recipes": { "type": "integer" },
                    "together_recipes": {
                        "type": "integer",
                        "description": "How many recipes the one Cookbook holds once joined: fewer than the two counts added up wherever both already hold a version of the same recipe.",
                    },
                    "already_yours": { "type": "boolean" },
                    "asks": {
                        "type": "array",
                        "items": person_ref_schema(),
                        "description": "Who else must say yes before the two Cookbooks become one: everyone writing either, less you and the sender. Empty when accepting joins them at once.",
                    },
                    "waiting": {
                        "type": "boolean",
                        "description": "True when you already accepted this Invite and the join waits on `asks`.",
                    },
                },
                "required": ["cookbook", "invited_by", "their_recipes", "your_recipes", "together_recipes", "already_yours", "asks", "waiting"],
                "additionalProperties": false,
            }),
            handler: crate::operations::read_cookbook_invite,
        },
        Operation {
            name: "accept_cookbook_invite",
            summary: "Open a Cookbook Invite: your Cookbook joins the one it \
                      names, and every recipe in either becomes one Cookbook \
                      you all change. Where either Cookbook has other writers, \
                      the join waits until each of them says yes, and the \
                      answer is your own Cookbook with the join in `joins`. \
                      Spent on use.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({ "type": "object", "properties": { "secret": { "type": "string" } }, "required": ["secret"], "additionalProperties": false }),
            output_schema: cookbook_schema(),
            handler: crate::operations::accept_cookbook_invite,
        },
        Operation {
            name: "answer_cookbook_join",
            summary: "Say yes or no to a Cookbook join that waits on you, as \
                      your Cookbook's `joins` lists it. The last yes joins the \
                      two Cookbooks. A no from anybody writing either calls it \
                      off and opens its Invite again; from the one who \
                      accepted it, that takes the acceptance back.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({ "type": "object", "properties": { "join_id": { "type": "string" }, "yes": { "type": "boolean" } }, "required": ["join_id", "yes"], "additionalProperties": false }),
            output_schema: cookbook_schema(),
            handler: crate::operations::answer_cookbook_join,
        },
        Operation {
            name: "leave_cookbook",
            summary: "Leave the Cookbook you write with others, taking your own \
                      Branch of every recipe in it with its whole history. \
                      Whoever started a recipe keeps the original; everyone \
                      else a copy.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: cookbook_schema(),
            handler: crate::operations::leave_cookbook,
        },
        Operation {
            name: "remove_cookbook_author",
            summary: "Separate another Co-author from your Cookbook. They leave \
                      with a Branch of every recipe in it, as though they had \
                      left.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({ "type": "object", "properties": { "person_id": { "type": "string" } }, "required": ["person_id"], "additionalProperties": false }),
            output_schema: cookbook_schema(),
            handler: crate::operations::remove_cookbook_author,
        },
        Operation {
            name: "create_tag",
            summary: "Create a Tag in your own Cookbook, named in one Language. \
                      A word the Cookbook already files under returns the Tag \
                      it already has rather than making a second.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({ "type": "object", "properties": { "language": { "enum": ["en", "fr", "es"] }, "name": { "type": "string" }, "kitchen_id": ignored_kitchen_id() }, "required": ["language", "name"], "additionalProperties": false }),
            output_schema: tag_schema(),
            handler: crate::operations::create_tag,
        },
        Operation {
            name: "list_tags",
            summary: "List every Tag your own Cookbook files by, each shown in \
                      the reader's Reading Language where it has a name there. \
                      With `everywhere`, every word any Cookbook you may see \
                      files by, one entry per word — what a shelf filters by. \
                      With a `kitchen_id`, the same for the Cookbooks seen in \
                      that one Kitchen of yours.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({ "type": "object", "properties": { "everywhere": { "type": "boolean" }, "kitchen_id": { "type": "string", "description": "One of your Kitchens: every word its Cookbooks file by, one entry per word." } }, "additionalProperties": false }),
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
            summary: "Relate one of your Cookbook's Recipes to any Recipe you \
                      may see, or take that single two-way, untyped link back \
                      off. Your Cookbook keeps the link. It never \
                      changes either Recipe or travels in a Bundle or Share. \
                      Name the far end with `related_branch_id`, or with \
                      `related_lineage_id` where the Recipe there has since \
                      been deleted — exactly one of the two.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "branch_id": { "type": "string" },
                    "related_branch_id": {
                        "type": "string",
                        "description": "The Recipe at the far end, named by its \
                                        Branch. The ordinary way to say it.",
                    },
                    // A deleted Branch leaves its Lineage behind, and the link
                    // is stored between two Lineages, so this is what makes a
                    // link breakable after the recipe at the far end is gone
                    // (#105). It cannot MAKE a link to a Lineage this Kitchen
                    // no longer holds: there would be no recipe to point at.
                    "related_lineage_id": {
                        "type": "string",
                        "description": "The far end named by its Lineage instead. \
                                        Use this to take off a link to a Recipe \
                                        that has been deleted, which has no \
                                        Branch left to name.",
                    },
                    "related": { "type": "boolean" },
                },
                "required": ["branch_id", "related"],
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
            // The collapse window is COLLAPSE_WINDOW_SECONDS, which a test
            // below holds this wording to.
            summary: "Save a new state of a Recipe onto a Branch — the whole \
                      recipe as written, replacing what was there: every field \
                      left out is erased. For a partial change use edit_recipe \
                      instead. A re-save by the same Hand within 60 minutes of \
                      the last one collapses into the Version already being \
                      shaped rather than starting a new one, unless another \
                      Branch or a Translation already holds that Version. A \
                      collapsed save \
                      keeps the name and change_note the Version already has \
                      unless it sends new ones. A save that changes nothing \
                      but sends a new name or change_note writes them onto \
                      the Version being shaped, and is refused once that \
                      Version is no longer being shaped. \
                      Changing a recipe your Cookbook did not write — a \
                      Kitchen-mate's, or one that arrived — is a Copy: it \
                      starts a new Branch of the same Lineage in your own \
                      Cookbook, starting at the Version you changed and \
                      carrying the whole chain behind it — the Branch you \
                      changed is left untouched. The Branch must be one you \
                      may see.",
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
            name: "edit_recipe",
            // The collapse window is COLLAPSE_WINDOW_SECONDS, which a test
            // below holds this wording to. An unchanged line keeping its
            // Reading is #166: that sentence goes when the issue is fixed.
            summary: "Change some fields of a Recipe and leave the rest as they \
                      are: send only the fields that change. A field left out \
                      keeps what the recipe has, and null clears it; the title \
                      alone can be changed but never cleared. Ingredients and steps are each replaced whole, \
                      so changing one line means sending that whole list, \
                      but not the other one. Otherwise exactly \
                      save_recipe_version: the result is saved as the \
                      recipe's new state, a re-edit by the same Hand within \
                      60 minutes collapses into the Version being shaped \
                      (unless another Branch or a Translation already holds \
                      it), and changing a recipe your Cookbook did not write is \
                      a Copy. A collapsed edit keeps the name and change_note \
                      the Version already has unless it sends new ones, and \
                      an edit sending only a name or change_note writes them \
                      onto the Version being shaped, or is refused once that \
                      Version is no longer being shaped. An Ingredient Line the edit \
                      leaves word for word as it was, in the same place, keeps \
                      its Reading as it was, a misreading included; correct \
                      one with set_reading.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: edit_recipe_input_schema(),
            output_schema: saved_version_schema(),
            handler: crate::operations::edit_recipe,
        },
        Operation {
            name: "start_variation",
            summary: "Start a variation of a recipe: a Branch of it, unchanged, \
                      in your own Cookbook, under a name you give it \
                      (\"Vegetarian\"). Changing one never changes the other.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({ "type": "object", "properties": { "branch_id": { "type": "string" }, "name": { "type": "string" } }, "required": ["branch_id", "name"], "additionalProperties": false }),
            output_schema: recipe_schema(),
            handler: crate::operations::start_variation,
        },
        Operation {
            name: "rename_branch",
            summary: "Name one of your Cookbook's Branches of a recipe, or clear \
                      its name. A Cookbook keeps one unnamed Branch of a recipe \
                      in each Language, so a second one needs a name.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({ "type": "object", "properties": { "branch_id": { "type": "string" }, "name": { "type": ["string", "null"] } }, "required": ["branch_id", "name"], "additionalProperties": false }),
            output_schema: json!({ "type": "object", "properties": { "branch_id": { "type": "string" }, "name": { "type": ["string", "null"] } }, "required": ["branch_id", "name"], "additionalProperties": false }),
            handler: crate::operations::rename_branch,
        },
        Operation {
            name: "delete_recipe",
            summary: "Take one recipe off the shelf for good. It is gone from \
                      the shelf, from search and from every member of its \
                      Kitchen, and nothing brings it back. **One Branch**: a \
                      translation is an ordinary Branch, so deleting the \
                      English one leaves the French one whole, and another \
                      Kitchen's copy of the same recipe is untouched. **The \
                      cooking history stays.** Every Attempt ever made from \
                      this recipe keeps its rating, its note and its \
                      Photographs, and the Cooked diary keeps each entry \
                      under the name the recipe was known by. So does a \
                      Shopping List holding it, which says it can no longer \
                      be read rather than quietly dropping it. No Version is \
                      ever deleted, by this or by anything else. A live Share \
                      Link stops working.",
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
            output_schema: json!({
                "type": "object",
                "properties": { "deleted": { "type": "boolean" } },
                "required": ["deleted"],
                "additionalProperties": false,
            }),
            handler: crate::operations::delete_recipe,
        },
        Operation {
            name: "read_pasted_recipe",
            summary: "Read a whole recipe pasted as text into a title, an \
                      ingredient list and a method. Decides only what each \
                      line IS — an Ingredient Line, a Step, a Section — and \
                      never \
                      what it says: every line comes back exactly as pasted, \
                      with no amount extracted, no rewording and no \
                      reordering (ADR 0002). Nothing is guessed beyond the \
                      split and the title: no Yield, no times, no Source, and \
                      no Component (ADR 0008). It writes nothing anywhere — \
                      what comes back is shown to whoever pasted it, who \
                      moves the boundary if it landed wrong, and only then is \
                      a recipe saved by an ordinary create_recipe or \
                      save_recipe_version. The boundary is the index in \
                      `lines` where the method starts, so moving it re-splits \
                      the same answer without asking again.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": { "text": { "type": "string" } },
                "required": ["text"],
                "additionalProperties": false,
            }),
            output_schema: pasted_recipe_schema(),
            handler: crate::operations::read_pasted_recipe,
        },
        Operation {
            name: "read_recipe_pdf",
            summary: "Read a recipe PDF — one printed from a web page or a \
                      word processor — the way read_pasted_recipe reads \
                      pasted text, and answer the same shape. A printed line \
                      that wrapped is joined back into one, and the first \
                      line is taken as the title. It writes nothing anywhere: \
                      what comes back is shown to whoever sent the PDF, who \
                      moves the boundary if it landed wrong, and only then is \
                      a recipe saved by an ordinary create_recipe. A scan or \
                      a photograph of a page holds no text and is refused \
                      with reason `pdf_has_no_text`; text is never read out \
                      of a picture. Send the file to POST /api/uploads and \
                      pass the `upload_id` it answers, or pass it \
                      base64-encoded as `data`.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "upload_id": {
                        "type": "string",
                        "description": "The id POST /api/uploads answered for the \
                                         PDF. Used once, then deleted.",
                    },
                    "data": {
                        "type": "string",
                        "description": "The PDF itself, base64-encoded — for a \
                                         Door that can send only JSON.",
                    },
                },
                "additionalProperties": false,
            }),
            output_schema: pasted_recipe_schema(),
            handler: crate::operations::read_recipe_pdf,
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
            summary: "Bring a batch of already-read recipes into your own \
                      Cookbook, as a Job. Matched by foreign id against this \
                      Cookbook's ledger for the source kind, so re-running \
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
            name: "import_crouton",
            summary: "Bring in a Crouton library, as a Job: the whole export \
                      (a zip of .crumb files) or one .crumb. Each recipe lands \
                      in your own Cookbook through the same ledger `import` \
                      uses, keyed by its Crouton id, so running it again \
                      matches instead of doubling the library. Ingredient \
                      Lines are rebuilt from Crouton's split fields; the \
                      site's favicon and Crouton's nutrition text are left \
                      out. Send the file to POST /api/uploads and pass the \
                      `upload_id` it answers, or pass it base64-encoded as \
                      `data`.",
            permission: Permission::Person,
            kind: Kind::Job,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "upload_id": {
                        "type": "string",
                        "description": "The id POST /api/uploads answered for the \
                                         export. Used once, then deleted.",
                    },
                    "data": {
                        "type": "string",
                        "description": "The export itself, base64-encoded — for a \
                                         Door that can send only JSON.",
                    },
                },
                "additionalProperties": false,
            }),
            output_schema: import_report_schema(),
            handler: crate::operations::import_crouton,
        },
        Operation {
            name: "list_imports",
            summary: "What has been brought into your Kitchens from outside, \
                      and what happened each time. One entry per source — a \
                      Crouton library, recipe files, web pages — each holding \
                      how many recipes its ledger remembers and every arrival \
                      you asked for, newest first. An arrival names the Job \
                      whose Report `get_job` serves, so what happened is read \
                      back long after the screen that started it closed. \
                      Listed is an event, never a mark on a recipe: an \
                      imported recipe is an ordinary recipe and says nothing \
                      about where it came from (ADR 0025).",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "imports": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "import_id": {
                                    "type": ["string", "null"],
                                    "description": "The ledger's id, which `forget_import` \
                                                     takes. Null once it has been forgotten: \
                                                     the arrivals and their Reports outlive \
                                                     the ledger, because forgetting throws \
                                                     away what became what, not what happened.",
                                },
                                "source_kind": {
                                    "type": "string",
                                    "description": "Which outside source this is — `crouton`, \
                                                     `bundle`, `web`, or whatever an `import` \
                                                     caller named.",
                                },
                                "created_at": { "type": "string" },
                                "remembered": {
                                    "type": "integer",
                                    "description": "How many recipes this ledger can still \
                                                     match on a re-run. The Kitchen's figure, \
                                                     not the caller's.",
                                },
                                "arrivals": {
                                    "type": "array",
                                    "description": "Every run of this source the caller asked \
                                                     for, newest first.",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "job_id": {
                                                "type": "string",
                                                "description": "Pass to `get_job` for the \
                                                                 Report itself.",
                                            },
                                            "status": { "enum": ["queued", "running", "completed", "failed", "cancelled"] },
                                            "created_at": { "type": "string" },
                                            "arrived": { "type": "integer" },
                                            "created": {
                                                "type": "integer",
                                                "description": "Of those that arrived, how many \
                                                                 were new rather than already \
                                                                 held.",
                                            },
                                            "offered": { "type": "integer" },
                                            "unreadable": { "type": "integer" },
                                        },
                                        "required": ["job_id", "status", "created_at", "arrived", "created", "offered", "unreadable"],
                                        "additionalProperties": false,
                                    },
                                },
                            },
                            "required": ["import_id", "source_kind", "created_at", "remembered", "arrivals"],
                            "additionalProperties": false,
                        },
                    },
                },
                "required": ["imports"],
                "additionalProperties": false,
            }),
            handler: crate::operations::list_imports,
        },
        Operation {
            name: "forget_import",
            summary: "Throw an Import's ledger away whole — the memory of \
                      which outside recipe became which of yours. Every \
                      recipe it made stays exactly as it is. Once forgotten, \
                      importing the same file again brings everything in as \
                      new, so do this when the place it came from is gone.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": { "import_id": { "type": "string" } },
                "required": ["import_id"],
                "additionalProperties": false,
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "import_id": { "type": "string" },
                    "forgotten": {
                        "type": "integer",
                        "description": "How many ledger entries were thrown away.",
                    },
                },
                "required": ["import_id", "forgotten"],
                "additionalProperties": false,
            }),
            handler: crate::operations::forget_import,
        },
        Operation {
            name: "import_web_link",
            summary: "Bring in a recipe straight from a URL, as a Job. Reads \
                      the page's schema.org JSON-LD (#70) — no per-site \
                      scraping, no LLM fallback — and lands it in your own \
                      Cookbook through the same ledger `import` uses, keyed by \
                      the page's own address. A Kamosu Share Link, one this \
                      instance minted or one from another Kamosu at a public \
                      address, is not read as a page: it arrives \
                      whole as the recipe file it serves, exactly as \
                      `import_bundle` receives one, with every Version and its \
                      original Source; an ended link is refused and lands \
                      nothing (#169). Fetching is bound to public \
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
            name: "preview_shared_recipe",
            summary: "Say what importing a Kamosu Share Link would do, before \
                      anything is written, as a Job (#170). Reaches the recipe \
                      file exactly as `import_web_link` does — this \
                      instance's own link locally, another Kamosu's at a \
                      public address through the guarded client — and answers \
                      the shared recipe's title, Source, writer, how many \
                      Versions it carries and a small picture, and whether \
                      your own Cookbook holds it already, with how many newer \
                      Versions the file carries past yours. The file is \
                      staged: pass `upload_id` to `import_bundle` to import \
                      exactly what was previewed without fetching it again. \
                      An ended link, or an address that is no Share Link, is \
                      refused.",
            permission: Permission::Person,
            kind: Kind::Job,
            // It writes nothing to the library, but it fetches another site
            // and stages the file on disk — work only a Credential that may
            // import has any use for.
            write: true,
            session_only: false,
            // The same lane `import_web_link` takes, for the same reason: the
            // address is somebody else's, and a page can ask for another.
            job_lane: JobLane::AlwaysSingle,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The Share Link: https://<instance>/s/<token>, \
                                         or one of its Translations' pages.",
                    },
                },
                "required": ["url"],
                "additionalProperties": false,
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "upload_id": {
                        "type": "string",
                        "description": "The recipe file, staged for you. Pass it to \
                                         `import_bundle`; unused, it is swept after a day.",
                    },
                    "title": { "type": "string" },
                    "shared_by": {
                        "type": ["string", "null"],
                        "description": "Who shared it, as the Share Link's page names \
                                         them; null where the page does not say.",
                    },
                    "written_by": {
                        "type": ["string", "null"],
                        "description": "The name of the Hand that writes the shared recipe.",
                    },
                    "source": {
                        "type": ["object", "null"],
                        "properties": {
                            "text": { "type": ["string", "null"] },
                            "link": { "type": ["string", "null"] },
                        },
                        "required": ["text", "link"],
                        "additionalProperties": false,
                    },
                    "versions": {
                        "type": "integer",
                        "description": "How many Versions of the recipe the file carries.",
                    },
                    "photo": {
                        "type": ["string", "null"],
                        "description": "A small copy of its picture, as a data: address.",
                    },
                    "held": {
                        "type": ["object", "null"],
                        "description": "Your Cookbook's copy, when it holds one already.",
                        "properties": {
                            "branch_id": { "type": "string" },
                            "arrived": {
                                "type": "boolean",
                                "description": "Whether it came from elsewhere, rather \
                                                 than being written in your Cookbook.",
                            },
                            "since": {
                                "type": "string",
                                "description": "When your Cookbook first held it.",
                            },
                            "newer": {
                                "type": "integer",
                                "description": "How many Versions importing would add.",
                            },
                            "diverged": {
                                "type": "boolean",
                                "description": "Whether your copy and the file have gone \
                                                 different ways, so importing would change \
                                                 nothing: the sender rewrote a Version you \
                                                 already hold.",
                            },
                        },
                        "required": ["branch_id", "arrived", "since", "newer", "diverged"],
                        "additionalProperties": false,
                    },
                },
                "required": ["upload_id", "title", "shared_by", "written_by", "source", "versions", "photo", "held"],
                "additionalProperties": false,
            }),
            handler: crate::operations::preview_shared_recipe,
        },
        Operation {
            name: "rename_version",
            summary: "Rename a Version — the one thing about it that can \
                      change later. An absent or empty name clears it. \
                      Targeted by the Branch's own sequence number, since \
                      the same content can recur more than once on one \
                      Branch, each occurrence named on its own. Only the \
                      Person who saved that Version may rename it.",
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
                    // A Kitchen narrows to the Cookbooks seen in it.
                    "kitchen_id": { "type": ["string", "null"] },
                    // Created, branched or cooked by this Person — a history,
                    // not an ownership, and one that needs no curating ever.
                    // Not the `mine` each entry answers with (#177).
                    "mine": {
                        "type": "boolean",
                        "description": "Only recipes this Person created, branched or cooked: a history, not ownership. Not the same as the `mine` on each entry, which says whether their own Cookbook holds the recipe.",
                    },
                    // The third filter, and the one that makes a Tag something
                    // a person can browse by rather than a word that happens to
                    // match (#104). Held nowhere, like the other two. It
                    // narrows the shelf before the query runs, so a Tag and a
                    // search compose rather than compete.
                    "tag_id": { "type": ["string", "null"] },
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
                      recently added (newest first), cooked most, quick \
                      tonight, never cooked (shuffled each time it is asked \
                      for, so it is not the newest again), recently opened. \
                      Each is one card per Lineage in the reader's Reading \
                      Language, in the same shape the library's shelf \
                      answers in. A shelf with nothing on it is left out \
                      rather than sent empty, so an instance holding no \
                      recipes answers with no shelves at all. All five are \
                      counted from recipes and Attempts that already exist, \
                      except *recently opened*, which reads what \
                      `note_recipe_opened` remembered (ADR 0011, ADR 0027, \
                      ADR 0042).",
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
                                        "recently_added", "cooked_most",
                                        "quick_tonight", "never_cooked",
                                        "recently_opened"
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
                      chain of Versions, oldest first. Each Version's \
                      `measured` lines are scaled to `wanted_yield` where one \
                      is given (null for the recipe as written), and otherwise \
                      to the Yield the caller's own In Progress Attempt is \
                      cooking to; `scaled_to` says which, or is null where the \
                      amounts are as written. Nothing is stored.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "branch_id": { "type": "string" },
                    "wanted_yield": wanted_yield_schema(),
                },
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
                      than minting a second one, with its `url` again. The \
                      instance's public address is asked for at the first \
                      Share Link and stored once; a link is kept as a token \
                      rather than a URL, so `url` is always built against the \
                      address stored now.",
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
                      discovered. Answers `shared: false` and no `url`.",
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
            summary: "Whether a Recipe is shared, by whom, and at what \
                      address. A live link answers its `url` for as long as it \
                      lives, to send again. A link minted before Kamosu kept \
                      its address answers none: ending it and sharing again \
                      mints one that does.",
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
            name: "get_public_address",
            summary: "Where this instance currently says it is reachable from \
                      outside, or nothing if it has never been asked. The \
                      Operator's half of `set_public_address`: changing an \
                      address you cannot see is a guess.",
            permission: Permission::Operator,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: json!({
                "type": "object",
                "properties": { "public_address": { "type": ["string", "null"] } },
                "required": ["public_address"],
                "additionalProperties": false,
            }),
            handler: crate::operations::get_public_address,
        },
        Operation {
            name: "set_public_address",
            summary: "Change where this instance says it is reachable from \
                      outside. Kept in the database and never in an \
                      environment variable, so moving an instance is one act \
                      rather than a redeployment. It fixes the future, not the \
                      past: Share Links minted after it carry the new address, \
                      while a link already sent stays the text it was sent as. \
                      A live link's `url`, as `get_share_link` answers it, is \
                      built against the new address, so the owner can send the \
                      one that opens now. A link minted before Kamosu kept its \
                      address has none to show.",
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
            name: "export_bundle",
            summary: "Write a Bundle of one recipe: a plain zip holding a readable Markdown note \
                      per recipe with its Thread beneath it, its Photographs, and a hidden \
                      .kamosu/ sidecar carrying every Version complete back to the first, the \
                      Readings and the ids. It carries the Branch named, its Translations, and \
                      every Component it needs as a Passenger. This answers what the Bundle \
                      holds; fetch its bytes at GET /api/bundles/<branch_id> under the same \
                      Credential. Nothing is sent anywhere and nothing is changed.",
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
            output_schema: json!({
                "type": "object",
                "properties": {
                    "file_name": { "type": "string" },
                    "fetch_at": { "type": "string" },
                    "subjects": bundle_recipes_schema(),
                    "passengers": bundle_recipes_schema(),
                    "notes": { "type": "array", "items": { "type": "string" } },
                    "photographs": { "type": "integer" },
                    "missing_photographs": { "type": "array", "items": { "type": "string" } },
                },
                "required": ["file_name", "fetch_at", "subjects", "passengers", "notes", "photographs", "missing_photographs"],
                "additionalProperties": false,
            }),
            handler: crate::operations::export_bundle,
        },
        // ── Sheets (#75, ADR 0023) ─────────────────────────────────────────
        //
        // A Sheet is one recipe set for paper, as a Job. Two ways in, as there
        // are two ways to read a recipe: a Person's own, and a stranger's
        // through a Share Link — and a Person reading through someone's link
        // is a stranger to it, which is why the second is Public. Either way
        // the PDF waits under the Job's id at GET /api/sheets/<job_id>.
        Operation {
            name: "make_sheet",
            summary: "Set a Sheet of one recipe: the Branch as it stands on this Person's \
                      screen, set for paper as a PDF. It carries the recipe and not the \
                      library — no Tags, Attempts, Thread or past Versions. Written \
                      Ingredient Lines are printed and Readings are not, except the amount \
                      beneath a line when a cooking has scaled the recipe; Components \
                      unfold after it, parent first, each already scaled. Letter for US \
                      Reading Measures, A4 otherwise. `wanted_yield` is the Yield the \
                      screen is scaled to, as `get_recipe` takes it. When the Job \
                      completes, fetch the PDF at GET /api/sheets/<job_id> under the \
                      same Credential. Nothing is changed.",
            permission: Permission::Person,
            kind: Kind::Job,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "branch_id": { "type": "string" },
                    "wanted_yield": wanted_yield_schema(),
                },
                "required": ["branch_id"],
                "additionalProperties": false,
            }),
            output_schema: sheet_schema(),
            handler: crate::operations::make_sheet,
        },
        Operation {
            name: "make_shared_sheet",
            summary: "Set a Sheet of the recipe a Share Link shows, for anyone holding the \
                      link — no account needed. The recipe is printed as written, with its \
                      Components unfolded after it at the amount each line asks for. \
                      `language` picks one of the link's Translations; `locale` is the \
                      reader's locale (a US or Canadian one prints Letter, anything else A4) and decides \
                      nothing but the paper. When the Job completes, fetch the PDF at \
                      GET /api/sheets/<job_id>.",
            permission: Permission::Public,
            kind: Kind::Job,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "token": { "type": "string" },
                    "language": { "type": "string" },
                    "locale": { "type": "string" },
                },
                "required": ["token"],
                "additionalProperties": false,
            }),
            output_schema: sheet_schema(),
            handler: crate::operations::make_shared_sheet,
        },
        Operation {
            name: "import_bundle",
            summary: "Receive a Bundle into your own Cookbook, as a Job. Every recipe it \
                      carries is placed under the sender's Hands and travels on under the \
                      sender's ids, its Versions, Readings and Photographs exactly as they \
                      were sent, while your Cookbook holds it under an id of this instance's \
                      own; one your Cookbook already holds is extended by whatever the Bundle \
                      carries past it, so the same friend's next Bundle continues their \
                      recipe. Another Cookbook here holding it is no part of the question: \
                      each Cookbook receives its own copy. Receiving makes nothing of your \
                      own — changing what arrived does. A recipe whose history is damaged \
                      arrives as a new recipe of your own with no history, and the Import \
                      Report says so. Send the file to POST /api/uploads and pass the \
                      `upload_id` it answers, or pass it base64-encoded as `data`.",
            permission: Permission::Person,
            kind: Kind::Job,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            // Two ways in, the pair `import_crouton` already takes (#69, #93):
            // one recipe carrying photographs is megabytes, and base64 inside a
            // JSON body is not how that travels from a browser.
            input_schema: json!({
                "type": "object",
                "properties": {
                    "upload_id": {
                        "type": "string",
                        "description": "The id POST /api/uploads answered for the \
                                         Bundle. Used once, then deleted.",
                    },
                    "data": {
                        "type": "string",
                        "description": "The Bundle's zip, base64-encoded — for a \
                                         Door that can send only JSON.",
                    },
                },
                "additionalProperties": false,
            }),
            output_schema: import_report_schema(),
            handler: crate::operations::import_bundle,
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
            // A save never re-reading an unchanged line is #166: the last
            // sentence goes when that issue is fixed.
            summary: "Correct the Reading on one Ingredient Line of a Recipe's \
                      current state — an amount, a Unit and a target, sent \
                      together as the whole new Reading (never a per-field \
                      patch, the same convention save_recipe_version uses \
                      for the whole recipe). Mints no Version and appears in \
                      no history (ADR 0021). All of them left out together \
                      clears the Reading, taking the line back to fully \
                      unread. The target is either a Food's written word or — \
                      as `lineage_id` — the Recipe this line names, which \
                      makes the Ingredient a Component (ADR 0008); never \
                      both, and a Lineage this instance does not hold is \
                      accepted, because a Component goes on naming its recipe \
                      when the recipe is gone. A save carries the Reading of \
                      each line left word for word as it was, in the same \
                      place, onto the new Version and never reads that line \
                      again, so this is how to correct a misreading without \
                      rewording the line.",
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
                    // **The other kind of target** (ADR 0008): the Lineage of a
                    // Recipe, which makes this Ingredient a Component. It is
                    // exclusive with `target` — a Reading points at a Food or
                    // at a Recipe, never both — and it may name a Lineage this
                    // instance does not hold, because a Component has to go on
                    // naming its recipe when the recipe is gone.
                    "lineage_id": { "type": ["string", "null"] },
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
            name: "reread_ingredient_lines",
            summary: "Read every Ingredient Line in the library again with \
                      the reader as it stands today, as a Job, so a fix to the \
                      reader reaches lines it already misread. Covers every \
                      Version a recipe holds, its history too, and makes no \
                      Version. A Reading a person set, and every Component, is \
                      left exactly as it is; a line the reader can no longer \
                      read loses the reader's old guess. Reports each line \
                      whose Reading changed, before and after, once per recipe \
                      as it reads on the head, with how many older Versions \
                      changed the same way, and each line changed only in \
                      older Versions; the Foods now left with nothing \
                      pointing at them, which delete_food will take; and how \
                      many lines it left alone because a person set them.",
            permission: Permission::Operator,
            kind: Kind::Job,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "changed": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "branch_id": { "type": "string" },
                                "title": { "type": ["string", "null"] },
                                "line_index": { "type": "integer" },
                                "line": { "type": "string" },
                                "before": reread_reading_schema(),
                                "after": reread_reading_schema(),
                                "on_head": {
                                    "type": "boolean",
                                    "description": "Whether the line changed on the recipe as it \
                                                     stands, rather than only in older Versions.",
                                },
                                "older_versions": { "type": "integer" },
                            },
                            "required": ["branch_id", "title", "line_index", "line", "before", "after", "on_head", "older_versions"],
                            "additionalProperties": false,
                        },
                    },
                    "older_versions_changed": { "type": "integer" },
                    "emptied_foods": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "food_id": { "type": "string" },
                                "name": { "type": ["string", "null"] },
                            },
                            "required": ["food_id", "name"],
                            "additionalProperties": false,
                        },
                    },
                    "kept_by_hand": { "type": "integer" },
                },
                "required": ["changed", "older_versions_changed", "emptied_foods", "kept_by_hand"],
                "additionalProperties": false,
            }),
            handler: crate::operations::reread_ingredient_lines,
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
                    "attempt_id": {
                        "type": "string",
                        "description": "The id to give this cooking, for one started with no network (#77): at_ and sixteen lower-case hex digits. Sending the same start twice answers the same Attempt; where the Lineage already has one In Progress, that one is answered instead.",
                    },
                    "started_at": {
                        "type": "string",
                        "description": "When cooking really started, for a start that waited on a phone with no network (#77). The same form as written_at.",
                    },
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
                    "cooking_yield": wanted_yield_schema(),
                    "written_at": written_at_schema(),
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
                    "add_photographs": {
                        "type": ["array", "null"],
                        "items": { "type": "string" },
                        "description": "Photographs to put beside those the cooking already holds, rather than replacing them: what a device sends when it takes a picture, so two devices never erase each other's (#77).",
                    },
                    "written_at": written_at_schema(),
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
                    "add_photographs": {
                        "type": ["array", "null"],
                        "items": { "type": "string" },
                        "description": "Photographs to put beside those the cooking already holds, rather than replacing them: what a device sends when it takes a picture, so two devices never erase each other's (#77).",
                    },
                    "written_at": written_at_schema(),
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
                      already being shaped, and a Copy in your own \
                      Cookbook where you do not write the Branch's. The \
                      Branch must be one you may see. The Attempt keeps the \
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
                    "kitchen_id": ignored_kitchen_id(),
                },
                "required": ["attempt_id", "photograph_id", "branch_id"],
                "additionalProperties": false,
            }),
            output_schema: saved_version_schema(),
            handler: crate::operations::promote_attempt_photograph,
        },
        Operation {
            name: "set_as_cooked",
            summary: "Write down what you actually cooked, where it differed \
                      from the recipe: the whole recipe as you cooked it, in \
                      ordinary Ingredient Lines and ordinary Step text — a \
                      line reworded, one added, one dropped, a step grown. \
                      Not a record of differences; the same shape a Version \
                      takes. Sending back exactly what the recipe says, or \
                      null, stores nothing at all, because cooking a recipe \
                      as it is written changes nothing. Changes no recipe and makes no \
                      Version: that is Promotion, and it is a separate act.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: set_as_cooked_input_schema(),
            output_schema: attempt_schema(),
            handler: crate::operations::set_as_cooked,
        },
        Operation {
            name: "decline_promotion",
            summary: "Say that the words a cooking used belong in the diary \
                      and not in the recipe — or take that back. It answers \
                      the offer and nothing else: what was cooked stays on \
                      the cooking, whole. Remembered, because a question \
                      already answered, asked twice, is a nag.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "attempt_id": { "type": "string" },
                    "declined": { "type": "boolean" },
                },
                "required": ["attempt_id", "declined"],
                "additionalProperties": false,
            }),
            output_schema: attempt_schema(),
            handler: crate::operations::decline_promotion,
        },
        Operation {
            name: "promote_as_cooked",
            summary: "Promotion: turn what you cooked into a real Version of \
                      the recipe. Mechanical — the As Cooked is already a \
                      whole recipe, so nothing is retyped and nothing is \
                      reconciled. It is an ordinary edit and inherits all of \
                      one: a rapid re-save folds into the Version being \
                      shaped, and a Branch whose Cookbook you do not write \
                      becomes a Copy in your own. The Branch must be one \
                      you may see. Promoting a cooking of an older Version \
                      appends onto wherever the Branch stands now — a \
                      Version, never a merge. The Attempt is left exactly as \
                      it was, still saying which Version it cooked.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "attempt_id": { "type": "string" },
                    "branch_id": {
                        "type": "string",
                        "description": "Which Branch of the cooked Lineage to promote into. An Attempt belongs to a Lineage rather than a Branch, so this says where the words land.",
                    },
                    "name": { "type": ["string", "null"] },
                    "change_note": { "type": ["string", "null"] },
                    "kitchen_id": ignored_kitchen_id(),
                },
                "required": ["attempt_id", "branch_id"],
                "additionalProperties": false,
            }),
            output_schema: saved_version_schema(),
            handler: crate::operations::promote_as_cooked,
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
        // ── The Shopping List (#73, ADR 0024) ────────────────────────────────
        //
        // Every Operation here that changes the choosing answers the WHOLE
        // list, computed again. That is not a convenience: the rows are
        // computed and stored nowhere, so there is no smaller answer that
        // would still be true, and a screen redrawing from one answer cannot
        // hold a row the Core has since worked out differently.
        Operation {
            name: "get_shopping_list",
            summary: "Your Shopping List: the recipes you chose, and the rows \
                      worked out from them. Everyone has exactly one; it has \
                      no name and is never archived. The rows are computed on \
                      every read and stored nowhere, so editing a chosen \
                      recipe or correcting a Reading changes the list at once. \
                      A row names a Food in your Reading Language and merges \
                      every mention of it; amounts add where the Units \
                      honestly convert, saying about, and ride side by side \
                      where they do not. Nothing here is ticked off.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: shopping_list_schema(),
            handler: crate::operations::get_shopping_list,
        },
        Operation {
            name: "shopping_basis",
            summary: "What one recipe puts on a Shopping List before anything \
                      is added up: each Ingredient Line, the Food it was read \
                      as and the name that Food goes by for you, how much it \
                      said, its Unit, and what a cup of the Food weighs. Every \
                      recipe this one includes is unfolded to the bottom and \
                      its lines are here too, already carrying their share, so \
                      a pizza's flour and its dough's flour add up to one \
                      thing to buy. Always the Branch's latest Version. It is \
                      how a phone with no network works out the list's rows \
                      itself for the recipes it holds (#77); get_shopping_list \
                      is the list itself.",
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
            output_schema: shopping_basis_schema(),
            handler: crate::operations::shopping_basis,
        },
        Operation {
            name: "add_to_shopping_list",
            summary: "Choose a recipe to shop for, at a Yield, a multiplier \
                      (a Yield with an empty noun) or as it is written. It holds the Branch at its latest Version, \
                      never a Lineage and never pinned, so a recipe edited \
                      between the planning and the shopping is right in the \
                      shop. Choosing one already on the list is not an error \
                      and makes no second entry: it moves that entry to the \
                      Yield given here, or back to the recipe as written when \
                      none is. Answers the whole list.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "branch_id": { "type": "string" },
                    // Named to match the Shopping List's own answer, and to
                    // match `cooking_yield` on `advance_attempt` next door
                    // (#85). It was `yield`, which agreed with neither — and
                    // because it is optional and its absence legitimately
                    // means "back to the recipe as written", a misspelling and
                    // a deliberate reset were the same request.
                    "shopping_yield": wanted_yield_schema(),
                    "written_at": written_at_schema(),
                },
                "required": ["branch_id"],
                "additionalProperties": false,
            }),
            output_schema: shopping_list_schema(),
            handler: crate::operations::add_to_shopping_list,
        },
        Operation {
            name: "remove_from_shopping_list",
            summary: "Take a recipe off your Shopping List. Works whether or \
                      not it can still be read, which is exactly the entry \
                      somebody most wants gone. Answers the whole list.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "branch_id": { "type": "string" },
                    "written_at": written_at_schema(),
                },
                "required": ["branch_id"],
                "additionalProperties": false,
            }),
            output_schema: shopping_list_schema(),
            handler: crate::operations::remove_from_shopping_list,
        },
        Operation {
            name: "set_shopping_yield",
            summary: "Say how much of a chosen recipe you are shopping for — \
                      an amount and its noun, a multiplier (an amount with an \
                      empty noun: twice the recipe is `2`), or null for the \
                      recipe as written. Every amount it contributes moves with \
                      it. Answers the whole list.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "branch_id": { "type": "string" },
                    // Named as on `add_to_shopping_list` above, and for the
                    // same reason (#85).
                    "shopping_yield": wanted_yield_schema(),
                    "written_at": written_at_schema(),
                },
                "required": ["branch_id"],
                "additionalProperties": false,
            }),
            output_schema: shopping_list_schema(),
            handler: crate::operations::set_shopping_yield,
        },
        Operation {
            name: "shopping_list_as_text",
            summary: "Your Shopping List as plain text, ready to be carried \
                      out of Kamosu. Nothing is ticked off here, because the \
                      list leaves and something else holds the ticks — Apple \
                      Notes, through a Shortcut. The text opens with a header \
                      line, the date and the recipes it was built from (and \
                      any that can no longer be read), because a note \
                      accumulates and three trips appended with no divider are \
                      a wall. Under it, one flat alphabetical list with one \
                      Markdown checklist line (`- [ ] `) per thing to buy, so \
                      each line becomes one checkbox; a row whose amounts \
                      could not be added stays on its one line, naming the \
                      dish behind each amount. This only reads: emptying the \
                      list afterwards is a separate Operation, offered and \
                      never done on the way out.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: false,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: empty_input(),
            output_schema: json!({
                "type": "object",
                "properties": { "text": { "type": "string" } },
                "required": ["text"],
                "additionalProperties": false,
            }),
            handler: crate::operations::shopping_list_as_text,
        },
        Operation {
            name: "empty_shopping_list",
            summary: "Empty your Shopping List — every recipe chosen and \
                      every typed line at once. Offered after the list has \
                      left as text and never done on the way out: a list that \
                      emptied itself when it was sent would be silent and \
                      unrecoverable. Answers the whole list.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": { "written_at": written_at_schema(), },
                "additionalProperties": false,
            }),
            output_schema: shopping_list_schema(),
            handler: crate::operations::empty_shopping_list,
        },
        Operation {
            name: "add_loose_item",
            summary: "Type a line straight onto your Shopping List — bin \
                      bags, coffee. Kept exactly as typed and never read, so \
                      it carries no amount and merges with nothing: typing \
                      flour beside a recipe that wants flour gives two lines. \
                      Answers the whole list.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string" },
                    "item_id": {
                        "type": "string",
                        "description": "The id to give this line, for one typed with no network (#77): i_ and sixteen lower-case hex digits. Sending the same line twice adds it once.",
                    },
                    "written_at": written_at_schema(),
                },
                "required": ["text"],
                "additionalProperties": false,
            }),
            output_schema: shopping_list_schema(),
            handler: crate::operations::add_loose_item,
        },
        Operation {
            name: "remove_loose_item",
            summary: "Take one typed line off your Shopping List. Answers the \
                      whole list.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "item_id": { "type": "string" },
                    "written_at": written_at_schema(),
                },
                "required": ["item_id"],
                "additionalProperties": false,
            }),
            output_schema: shopping_list_schema(),
            handler: crate::operations::remove_loose_item,
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
            name: "set_food_names",
            summary: "Say every name a Food answers to in one Language, in \
                      order: the list replaces the names it had there. A \
                      line naming any of them reads as this Food, so \
                      \"œufs\" and \"œuf\" can both be the eggs. The first \
                      is the one a reader is shown. An empty list takes the \
                      Language off, but a Food's last name may not go. Any \
                      Person may — a Food is instance-wide, not a Kitchen's \
                      to guard.",
            permission: Permission::Person,
            kind: Kind::Immediate,
            write: true,
            session_only: false,
            job_lane: JobLane::ByCaller,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "food_id": { "type": "string" },
                    "language": { "enum": ["en", "fr", "es"] },
                    "names": { "type": "array", "items": { "type": "string" } },
                },
                "required": ["food_id", "language", "names"],
                "additionalProperties": false,
            }),
            output_schema: food_schema(),
            handler: crate::operations::set_food_names,
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
            summary: "Join two Foods into one. The survivor keeps its own \
                      names and answers to every one of the other's as well, \
                      after its own in each Language. Every Reading \
                      pointing at the other points at it instead, and every \
                      Merge Suggestion naming either is cleared. ingredient_lines is the figure \
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
            summary: "Delete a Food nothing points at. One a Reading on some \
                      recipe still points at is refused: what a Food knows was \
                      expensive to learn and is never discarded by an unrelated \
                      act. Readings only a deleted recipe held count for nothing \
                      and go with the Food.",
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
                  when asked to. It can stop reporting early, or report no total.",
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
                "stop_after": { "type": "integer", "minimum": 0, "maximum": 60 },
                "without_total": { "type": "boolean", "default": false },
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

/// The shape a Kitchen is served in: its shared Name, the asking Person's own
/// Nickname for it (nobody else's), who cooks in it, and whose Cookbooks it
/// sees (ADR 0041).
fn kitchen_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string" },
            "name": { "type": "string" },
            "nickname": { "type": ["string", "null"] },
            // Whose Cookbooks this Kitchen sees: one per member, or fewer
            // where members write one together (ADR 0041). The reader's own
            // first.
            "cookbooks": { "type": "array", "items": cookbook_label_schema() },
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
        "required": ["id", "name", "nickname", "members", "cookbooks"],
        "additionalProperties": false,
    })
}

/// A Cookbook as a screen labels a recipe by it (ADR 0041): its id, the name
/// its Co-authors gave it — null until they give one, when a screen names it
/// after `authors` in the reader's own Language — and who writes it.
fn cookbook_label_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string" },
            "name": { "type": ["string", "null"] },
            "authors": { "type": "array", "items": person_ref_schema() },
        },
        "required": ["id", "name", "authors"],
        "additionalProperties": false,
    })
}

/// A Cookbook's settings card: its label, how many recipes it holds, the
/// Kitchens that see it, the Invites still waiting to be opened, and the
/// joins waiting on answers (#135).
fn cookbook_schema() -> Value {
    let mut schema = cookbook_label_schema();
    let properties = schema["properties"].as_object_mut().expect("object schema");
    properties.insert("recipe_count".to_string(), json!({ "type": "integer" }));
    properties.insert(
        "kitchens".to_string(),
        json!({
            "type": "array",
            "items": {
                "type": "object",
                "properties": { "id": { "type": "string" }, "name": { "type": "string" } },
                "required": ["id", "name"],
                "additionalProperties": false,
            },
        }),
    );
    properties.insert(
        "invites".to_string(),
        json!({
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "invite_id": { "type": "string" },
                    "created_at": { "type": "string" },
                },
                "required": ["invite_id", "created_at"],
                "additionalProperties": false,
            },
        }),
    );
    properties.insert(
        "joins".to_string(),
        json!({ "type": "array", "items": cookbook_join_schema() }),
    );
    schema["required"] = json!([
        "id",
        "name",
        "authors",
        "recipe_count",
        "kitchens",
        "invites",
        "joins"
    ]);
    schema
}

/// A Person as a join names them: who accepted, who sent, who is still asked.
fn person_ref_schema() -> Value {
    person_ref_schema_of(json!("object"))
}

/// The same shape, declared nullable: `refused_by` is null while nobody has
/// said no. Written as a `type` array rather than an `anyOf`, for the reason
/// `nullable_shared_version_schema` gives.
fn nullable_person_ref_schema() -> Value {
    person_ref_schema_of(json!(["object", "null"]))
}

fn person_ref_schema_of(type_: Value) -> Value {
    json!({
        "type": type_,
        "properties": {
            "person_id": { "type": "string" },
            "name": { "type": "string" },
        },
        "required": ["person_id", "name"],
        "additionalProperties": false,
    })
}

/// A join a Cookbook's card shows one of its writers (#135): one waiting on
/// answers, or, to the one who accepted it, one somebody said no to while its
/// Invite is open to try again.
fn cookbook_join_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "join_id": { "type": "string" },
            "state": { "type": "string", "enum": ["waiting", "refused"] },
            "accepted_by": person_ref_schema(),
            "invited_by": person_ref_schema(),
            "joining": cookbook_label_schema(),
            "into": cookbook_label_schema(),
            "together_recipes": { "type": "integer" },
            "waiting_on": {
                "type": "array",
                "items": person_ref_schema(),
                "description": "Who has still to say yes, worked out afresh: everyone writing either Cookbook, less the sender, the one who accepted and whoever already said yes.",
            },
            "you": {
                "type": "string",
                "enum": ["accepted", "invited", "asked", "answered"],
                "description": "Your part in it: you accepted the Invite, sent it, have still to answer, or already said yes.",
            },
            "refused_by": nullable_person_ref_schema(),
            "refused_by_co_author": {
                "type": "boolean",
                "description": "Whether whoever said no still writes the accepting Cookbook, so leaving it first would let the Invite be opened alone.",
            },
        },
        "required": [
            "join_id", "state", "accepted_by", "invited_by", "joining", "into",
            "together_recipes", "waiting_on", "you", "refused_by",
            "refused_by_co_author"
        ],
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
            "cookbook_id": { "type": "string" },
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
            // How many recipes carry it, counted in Lineages so that this
            // number and the shelf's own agree (#104). A screen offering to
            // delete a Tag has to say what it is taking the Tag off.
            "recipes": { "type": "integer" },
            // Whether `name` is in a Language the reader did not ask for, in
            // the same word a shelf entry already uses for the same fact
            // (#104, ADR 0006). The Reading Language is held on the account, so
            // this is the Core's to answer: a screen comparing against the
            // interface locale would mark every Tag wrongly for a reader whose
            // two settings differ.
            "language_fallback": { "type": "boolean" },
        },
        "required": [
            "id", "cookbook_id", "name", "language", "names", "recipes", "language_fallback"
        ],
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
            // So a screen can draw a related recipe wearing the face it wears
            // on the shelf without searching the whole library to find it
            // (#105). Null where the recipe has no Main Photo — which is a
            // third of the real corpus, and wears a Cover — and null again
            // where the Lineage has left the shelf, along with `branch_id`.
            "main_photo": { "type": ["string", "null"] },
            // The Language of the Branch this names, and whether that is one
            // the reader did not ask for. A shelf entry carries both for the
            // same reason: a Language preference may never hide a recipe from
            // its owner, so the recipe is shown and the difference is said
            // (ADR 0006). Null and false once the Lineage has left the shelf,
            // where there is no Branch to have a Language at all.
            "language": { "type": ["string", "null"] },
            "language_fallback": { "type": "boolean" },
        },
        "required": [
            "lineage_id", "branch_id", "title", "main_photo", "language", "language_fallback"
        ],
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
/// five (`home_shelves`) both answer in this shape, so a recipe is the same
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
            // Whose recipe this is, and what the reader may do with it: the
            // same three answers `get_recipe` gives for the Branch this entry
            // opens, so a caller can read the whole shelf once and know in
            // advance what the Core will allow (#177).
            "cookbook": cookbook_label_schema(),
            "writes": {
                "type": "boolean",
                "description": "Whether an edit by the reader lands on this Branch. Where false, an edit is not refused: it starts a Branch of the reader's own instead.",
            },
            "mine": {
                "type": "boolean",
                "description": "Whether the reader's own Cookbook holds this Branch, written there or arrived there. Changes made to the Cookbook rather than the recipe, such as putting a Tag on it, are allowed exactly where this is true. Not the same as the `mine` filter on search_recipes, which is a history.",
            },
        },
        "required": [
            "lineage_id", "branch_id", "title", "language",
            "language_fallback", "main_photo", "yield", "matched",
            "cookbook", "writes", "mine"
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

/// What a finished Sheet Job answers: where to fetch the PDF, and what it
/// came to.
fn sheet_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "file_name": { "type": "string" },
            "fetch_at": { "type": "string" },
            "paper": { "enum": ["a4", "us-letter"] },
            "pages": { "type": "integer", "minimum": 1 },
            // The kept file this Job's PDF is read from. The same page asked
            // for twice is set once and kept (ADR 0032), so two Jobs can name
            // the same one.
            "kept_as": { "type": "string" },
        },
        "required": ["file_name", "fetch_at", "paper", "pages", "kept_as"],
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
            // Whose recipe this is (ADR 0041).
            "cookbook": cookbook_label_schema(),
            // A variation's name ("Vegetarian"), null on every other Branch.
            "name": { "type": ["string", "null"] },
            // Whether a save by the reader lands on this Branch. False on
            // anybody else's recipe and on one that arrived from elsewhere,
            // where a save starts a Branch of the reader's own instead.
            "writes": { "type": "boolean" },
            // Why not, where the reader does not write it (#132): `mine` where
            // their own Cookbook holds it, `arrived` where it was sent from
            // elsewhere — as on a Thread's Branch.
            "mine": { "type": "boolean" },
            "arrived": { "type": "boolean" },
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
                        // The Components this Version composes, unfolded
                        // (ADR 0008). Beside the content rather than inside it:
                        // what is stored is a Reading pointing at a Lineage,
                        // and everything here is worked out from that at
                        // display time.
                        "components": { "type": "array", "items": component_schema() },
                        "translates_version_id": { "type": ["string", "null"] },
                        "language": { "type": ["string", "null"] },
                        // The Yield `measured` and `components` were scaled
                        // to, or null where they are as written (#109).
                        "scaled_to": wanted_yield_schema(),
                    },
                    "required": [
                        "sequence", "version_id", "parent_version_id", "hand_id",
                        "name", "change_note", "created_at", "content", "readings",
                        "measured", "cooking", "components", "translates_version_id", "language",
                        "scaled_to"
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
            "branch_id", "lineage_id", "cookbook", "name", "writes", "mine", "arrived", "hand_id",
            "language",
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
            // Who wrote it, named live where this instance minted the Hand
            // (ADR 0015) — so a rename reaches every past Version. Null for a
            // Hand nothing here has a name for.
            "hand_name": { "type": ["string", "null"] },
            "name": { "type": ["string", "null"] },
            "change_note": { "type": ["string", "null"] },
            "created_at": { "type": "string" },
            "translates_version_id": { "type": ["string", "null"] },
            "language": { "type": ["string", "null"] },
        },
        "required": [
            "branch_id", "sequence", "version_id", "parent_version_id",
            "hand_id", "hand_name", "name", "change_note", "created_at",
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
            // Whose Cookbook holds it (ADR 0041).
            "cookbook": cookbook_label_schema(),
            // A variation's name, null on every other Branch.
            "name": { "type": ["string", "null"] },
            // In the reader's own Cookbook.
            "mine": { "type": "boolean" },
            // Somebody else's writing that reached its Cookbook in a Bundle or
            // from a Share Link, labelled by its sender's Hand.
            "arrived": { "type": "boolean" },
            "hand_id": { "type": "string" },
            // The Hand's name, the same way a Version's writer is named.
            "hand_name": { "type": ["string", "null"] },
            "language": { "type": "string" },
            "head_version_id": { "type": "string" },
            "translation": translation_schema(),
        },
        "required": [
            "branch_id", "cookbook", "name", "mine", "arrived", "hand_id", "hand_name",
            "language", "head_version_id", "translation"
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
        // v1's whole of nutrition (CONTEXT.md, "Nutrition"): a figure the cook
        // types, or one a source page's own structured data stated — never
        // computed from the Ingredient Lines or the Foods they name. The basis
        // is required alongside the number because 308 says nothing until it
        // says what it counts, and the two bases do not convert into each
        // other without a weight the recipe does not carry.
        "nutrition": {
            "type": ["object", "null"],
            "properties": {
                "calories": {
                    "type": "number",
                    "minimum": 0,
                    "description": "Calories, zero or more.",
                },
                "basis": {
                    "enum": ["per_serving", "per_100g"],
                    "description": "What the figure counts: one serving of the \
                                     Yield as written, or 100 g.",
                },
            },
            "required": ["calories", "basis"],
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
            "cookbook": cookbook_label_schema(),
            "name": { "type": ["string", "null"] },
            "mine": { "type": "boolean" },
            "arrived": { "type": "boolean" },
            "hand_id": { "type": "string" },
            "hand_name": { "type": ["string", "null"] },
            "language": { "type": "string" },
            "head_version_id": { "type": "string" },
            "content": recipe_content_schema(),
            "readings": reading_list_schema(),
            "measured": measured_schema(),
            // Both sides carry their own Components (#50, ADR 0008). ADR 0014's
            // whole shape is two WHOLE recipes with a switch between them, so a
            // dough that unfolds on one side and not the other would make one
            // of them the lesser recipe.
            "components": { "type": "array", "items": component_schema() },
        },
        "required": [
            "branch_id", "cookbook", "name", "mine", "arrived", "hand_id", "hand_name",
            "language", "head_version_id", "content", "readings", "measured", "components",
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
            // Title, Yield, times, Source, Note, Nutrition and the Main Photo
            // are single values. Tags are absent on purpose: they are how a
            // Kitchen files, not what a recipe is, and marking them would put
            // a "take theirs" offer under a difference between two filing
            // systems (ADR 0035).
            "fields": {
                "type": "object",
                "properties": {
                    "title": divergence_field_schema(),
                    "yield": divergence_field_schema(),
                    "prep_time_minutes": divergence_field_schema(),
                    "cook_time_minutes": divergence_field_schema(),
                    "source": divergence_field_schema(),
                    "note": divergence_field_schema(),
                    "nutrition": divergence_field_schema(),
                    "main_photo": divergence_field_schema(),
                },
                "required": [
                    "title", "yield", "prep_time_minutes", "cook_time_minutes",
                    "source", "note", "nutrition", "main_photo",
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
/// being left out (see `parse_recipe_content` in `core/recipes.rs`).
fn recipe_content_schema() -> Value {
    json!({
        "type": "object",
        "properties": recipe_content_properties(),
        "required": [
            "title", "yield", "prep_time_minutes", "cook_time_minutes",
            "note", "main_photo", "source", "nutrition", "ingredients", "steps",
        ],
        "additionalProperties": false,
    })
}

/// The same content properties, as an Operation *accepts* them rather than as
/// it answers them.
///
/// One field differs, and only one: a step's `photo`. The shared definition
/// requires it, which is true of every step Kamosu answers with —
/// `parse_step_list` normalises an absent photo to `null` and then always
/// writes the field. It was never true of a step Kamosu is *given*: the same
/// function reads an absent `photo` as "no photograph", which is what every
/// caller has always sent, Kamosu's own frontend included.
///
/// Measured while wiring #85's enforcement up in report-only mode: 80 calls
/// across the behaviour suite and the interface send a step with no `photo`,
/// and zero send one with. Requiring it on input would have broken every
/// recipe save in the program — so the declaration was wrong, not the callers.
/// This is the same shared-properties-different-required pattern the top level
/// already uses; it simply never reached inside `steps`.
fn recipe_content_input_properties() -> Value {
    let mut properties = recipe_content_properties();
    // Navigated rather than indexed: `Value`'s `IndexMut` *creates* a missing
    // key, so a renamed `steps` would quietly grow a bogus property here
    // instead of failing — which is the very "check that silently stopped
    // running" #85 exists to close.
    properties
        .get_mut("steps")
        .and_then(|steps| steps.get_mut("items"))
        .and_then(Value::as_object_mut)
        .expect("the shared content properties declare steps as a list of objects")
        .insert("required".to_string(), json!(["kind", "text"]));
    properties
}

/// `kitchen_id`, accepted and ignored since #131 (ADR 0041): a recipe is
/// written in a Cookbook, and a new recipe, a save, a Copy, a Translation or
/// a Tag always lands in the caller's own. Kept so a client that still sends
/// it, a phone running the interface it cached before the upgrade among
/// them, is not refused: the precedent #100 set for the two promotions.
fn ignored_kitchen_id() -> Value {
    json!({
        "type": "string",
        "description": "Ignored. Everything you write lands in your own Cookbook (ADR 0041), so there is no Kitchen to name. Accepted so that a client which still sends it is not refused.",
    })
}

/// `create_recipe`'s input: a Kitchen and a title are all a Recipe ever
/// needs — every other field of the recipe's content is optional here.
fn create_recipe_input_schema() -> Value {
    let mut properties = recipe_content_input_properties();
    let map = properties.as_object_mut().expect("object schema");
    map.insert("kitchen_id".to_string(), ignored_kitchen_id());
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
        "required": ["title"],
        "additionalProperties": false,
    })
}

/// `save_recipe_version`'s input: the whole recipe as it now reads, replacing
/// what was on the Branch — a title is the one field that must be there. A
/// save by anybody who does not write the Branch's Cookbook starts a Copy in
/// their own (CONTEXT.md, "Copy"; ADR 0041), so there is never a place to
/// name.
fn save_recipe_version_input_schema() -> Value {
    let mut properties = recipe_content_input_properties();
    let map = properties.as_object_mut().expect("object schema");
    map.insert("kitchen_id".to_string(), ignored_kitchen_id());
    map.insert("branch_id".to_string(), json!({ "type": "string" }));
    map.insert("name".to_string(), json!({ "type": "string" }));
    map.insert("change_note".to_string(), json!({ "type": "string" }));
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

/// `edit_recipe`'s input: a Branch and only the fields of its recipe that
/// change (#164). Nothing is required but the Branch, and `{ branch_id }` alone
/// changes nothing and mints nothing. Every field but the title takes `null`
/// to clear it, the two lists included, which a whole save never needs: there
/// a list is cleared by leaving it out. No `kitchen_id`, since no client ever
/// sent one to an Operation this new.
fn edit_recipe_input_schema() -> Value {
    let mut schema = save_recipe_version_input_schema();
    let map = schema.as_object_mut().expect("object schema");
    map.insert("required".to_string(), json!(["branch_id"]));
    let properties = map
        .get_mut("properties")
        .and_then(Value::as_object_mut)
        .expect("save_recipe_version declares its properties");
    properties.remove("kitchen_id");
    for list in ["ingredients", "steps"] {
        properties
            .get_mut(list)
            .and_then(Value::as_object_mut)
            .expect("the content properties declare both lists")
            .insert("type".to_string(), json!(["array", "null"]));
    }
    schema
}

/// `start_translation`'s input: the recipe as it now reads in the new Language,
/// the Branch it is a rendering of, and — optionally — which Version of that
/// Branch it renders, defaulting to wherever the source stands now.
fn start_translation_input_schema() -> Value {
    let mut properties = recipe_content_input_properties();
    let map = properties.as_object_mut().expect("object schema");
    map.insert("kitchen_id".to_string(), ignored_kitchen_id());
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
    let mut properties = recipe_content_input_properties();
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
                                 from. One ledger is kept per Cookbook per \
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

/// What `read_pasted_recipe` and `read_recipe_pdf` both answer: a pasted
/// recipe read into a title, its lines and where the method starts (#94,
/// #176). One shape, so a PDF goes to the screen a paste already goes to.
fn pasted_recipe_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            // The first line where it stands alone above a blank line (a
            // PDF's first line, blank or none), or nothing. A guess,
            // corrected by typing in the title.
            "title": { "type": ["string", "null"] },
            "lines": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "text": { "type": "string" },
                        // A Section is a Section in either block, and
                        // is spelt the way a recipe's own content
                        // spells it, so a row goes onto the page as
                        // itself. Every other line is an Ingredient
                        // Line above the boundary and a Step below
                        // it, which is the whole of why the boundary
                        // can move without anything being read a
                        // second time.
                        "kind": { "enum": ["line", "section"] },
                    },
                    "required": ["text", "kind"],
                    "additionalProperties": false,
                },
            },
            "boundary": { "type": "integer", "minimum": 0 },
            // What the paste said about the recipe rather than in it, every
            // line as it came (#176): the block under `Description`, `Notes`
            // or `Suggestions de service :`. It goes to the recipe's own
            // note, and is in neither list.
            "note": { "type": ["string", "null"] },
        },
        "required": ["title", "lines", "boundary", "note"],
        "additionalProperties": false,
    })
}

/// Every importer's eventual result: the Import Report, a ledger read by a
/// person rather than an error log (ADR 0025, CONTEXT.md "Import Report").
/// Every candidate lands in exactly one bucket — `arrived` covers a recipe
/// freshly made, one already matched and found unchanged, and a Branch a
/// Bundle extended, told apart by `status`; `offered` is a previously-seen
/// recipe found changed, waiting for a tap rather than written over;
/// `unreadable` names what could not be placed at all, and why.
///
/// A damaged Bundle's recipe is one row in `unreadable` (#67, ADR 0020): its
/// history could not be read, and `kept_as` names the new recipe of your own
/// its words became.
fn import_report_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "import_id": { "type": "string" },
            // Always the importing Person's own Cookbook (ADR 0041).
            "cookbook_id": { "type": "string" },
            "source_kind": { "type": "string" },
            "arrived": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "foreign_id": {
                            "type": "string",
                            "description": "What the source called this recipe: from a \
                                             Bundle, the Branch id it travels under, which \
                                             is the sender's.",
                        },
                        "status": { "enum": ["created", "extended", "unchanged"] },
                        "lineage_id": { "type": "string" },
                        "branch_id": {
                            "type": "string",
                            "description": "The Branch as this instance holds it — the id \
                                             every other Operation and every URL takes. On a \
                                             recipe received from elsewhere it is not the id \
                                             it travelled under.",
                        },
                        "title": { "type": "string" },
                        "subject": {
                            "type": "boolean",
                            "description": "From a Bundle: whether this recipe is what the \
                                             Bundle is about, rather than a Passenger that \
                                             travelled because something needed it.",
                        },
                        "main_photo": {
                            "type": ["string", "null"],
                            "description": "The Photograph the recipe arrived with as its \
                                             Main Photo, or null.",
                        },
                        "bare": {
                            "type": "boolean",
                            "description": "It came as a bare name (and usually a link): no \
                                             Ingredient Line and no Step. A real recipe that \
                                             arrived correctly, not a failure.",
                        },
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
                        "name": {
                            "type": "string",
                            "description": "What the source called it, where even its own \
                                             id could not be read — a Crouton file's name.",
                        },
                        "reason": { "type": "string" },
                        "kept_as": {
                            "type": "object",
                            "description": "From a damaged Bundle: the new recipe of your \
                                             own its words were kept as, with no history and \
                                             no link to the recipe it came from.",
                            "properties": {
                                "lineage_id": { "type": "string" },
                                "branch_id": { "type": "string" },
                                "title": { "type": "string" },
                            },
                            "required": ["lineage_id", "branch_id", "title"],
                            "additionalProperties": false,
                        },
                    },
                    "required": ["foreign_id", "reason"],
                    "additionalProperties": false,
                },
            },
            "left_out": {
                "type": "array",
                "description": "What the importer read and deliberately did not bring in, \
                                 recipe by recipe: `site_icon` is a source site's favicon, \
                                 which is not a photograph of the dish (ADR 0017); \
                                 `extra_photos` are pictures past the first, since a recipe \
                                 keeps one Main Photo and Kamosu does not guess which Step \
                                 another shows; `unreadable_photos` could not be decoded.",
                "items": {
                    "type": "object",
                    "properties": {
                        "foreign_id": { "type": "string" },
                        "branch_id": { "type": "string" },
                        "title": { "type": "string" },
                        "what": { "enum": ["site_icon", "extra_photos", "unreadable_photos"] },
                        "count": { "type": "integer" },
                        "icon": {
                            "type": "string",
                            "description": "For a `site_icon`: the icon itself as a \
                                             `data:` URI, so the Report can show what \
                                             was left out. Carried only here, never \
                                             stored as a Photograph.",
                        },
                    },
                    "required": ["foreign_id", "branch_id", "title", "what", "count"],
                    "additionalProperties": false,
                },
            },
            "related_candidates": {
                "type": "array",
                "description": "Pairs of recipes this Import landed that share a name or a \
                                 web page, offered as Related Recipes to tick with \
                                 `set_related_recipe` — never joined into one Lineage \
                                 (ADR 0025). A pair already related is not offered.",
                "items": {
                    "type": "object",
                    "properties": {
                        "recipes": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "lineage_id": { "type": "string" },
                                    "branch_id": { "type": "string" },
                                    "title": { "type": "string" },
                                    "main_photo": { "type": ["string", "null"] },
                                    "ingredients": { "type": "integer" },
                                },
                                "required": ["lineage_id", "branch_id", "title", "main_photo", "ingredients"],
                                "additionalProperties": false,
                            },
                        },
                        "shared": {
                            "type": "array",
                            "items": { "enum": ["name", "page"] },
                        },
                    },
                    "required": ["recipes", "shared"],
                    "additionalProperties": false,
                },
            },
        },
        "required": [
            "import_id", "cookbook_id", "source_kind", "arrived", "offered", "unreadable",
            "left_out", "related_candidates"
        ],
        "additionalProperties": false,
    })
}

/// A Reading: Kamosu's interpretation of one Ingredient Line — an amount, a
/// Unit (whatever word was written; see #49) and a target, all optional and
/// `null` together wherever Kamosu has read nothing (ADR 0002, ADR 0021). A
/// target names a Food by the word alone; Foods (#47) carry no id here.
/// Whether a Recipe is shared, and by whom (#65, ADR 0026).
///
/// `url` is answered for as long as a live link's Secret is kept, which is
/// every link minted since #171: a Share Link is the one Secret kept readable
/// (ADR 0031 as amended). It is null once the link is ended, and on a link
/// minted before #171, whose Secret was only ever stored as a hash.
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
                "description": "The link itself, for as long as it is live. Null once \
                                 it is ended, and on a link minted before Kamosu kept \
                                 its address (#171), which still opens but cannot be \
                                 shown again.",
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
            // **The Passengers** (ADR 0008): every Component this recipe
            // composes, carried through the link because a recipe that cannot
            // tell you how to make its own dough is incomplete. The dough's own
            // Visibility is untouched by travelling — it gets no page and no
            // link of its own, and is read only through this one.
            //
            // The same shape `get_recipe` answers, minus `measured`: a Share
            // Link's rows carry no subordinate line at all, because Kamosu
            // converts to a kitchen and a stranger holding a link has none.
            "components": {
                "type": "array",
                "items": passenger_schema(),
            },
        },
        "required": [
            "branch_id", "lineage_id", "version_id", "language",
            "content", "readings", "components",
        ],
        "additionalProperties": false,
    })
}

/// One Passenger: a Component as a Share Link carries it — [`component_schema`]
/// with its subordinate lines declared as the empty slot they are.
///
/// Declared `null` rather than removed, the way a Food's `nutrition` is
/// (`get_food`): the shape a Passenger has is the shape a Component has, with
/// one slot this page never fills. Removing the key would make two shapes where
/// there is one, and a reader would have to know which of them they held.
fn passenger_schema() -> Value {
    let mut schema = component_schema();
    schema["properties"]["measured"] = json!({ "type": "null" });
    schema
}

/// The recipes a Bundle names, subjects or Passengers: one per Lineage.
fn bundle_recipes_schema() -> Value {
    json!({
        "type": "array",
        "items": {
            "type": "object",
            "properties": {
                "lineage_id": { "type": "string" },
                "title": { "type": ["string", "null"] },
            },
            "required": ["lineage_id", "title"],
            "additionalProperties": false,
        },
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
            // **A Reading's target is either a Food or a Lineage** (ADR 0008,
            // ADR 0021), and these two properties are that one slot. `target`
            // is the Food's written word and carries no id, because two
            // instances mint their *farine* separately and an id's only
            // confident statement about two identical Foods would be "not the
            // same". A Lineage id is the opposite: global by construction
            // (ADR 0004), so it needs nothing added and travels as it is.
            "target": { "type": ["string", "null"] },
            "lineage_id": {
                "type": ["string", "null"],
                "description": "The Recipe this line names, which makes the Ingredient a \
                                 Component. It may name a Lineage this instance does not \
                                 hold — deleted, never received, or held by nobody here — \
                                 and the line still reads correctly, because the written \
                                 line was always the truth.",
            },
        },
        "required": ["amount", "unit", "target", "lineage_id"],
        "additionalProperties": false,
    })
}

/// **The Components of one Version, unfolded** — one entry per Ingredient Line
/// whose Reading names a Lineage rather than a Food (ADR 0008), depth first, in
/// the order the page meets them. Empty for nearly every recipe.
///
/// **It is a flat list carrying a `path`, not a tree.** A Component may hold
/// Components and ADR 0008 sets no depth limit beyond the repeat guard, so a
/// nested declaration would either lie about the depth or stop being a
/// declaration — and `generate-client.mjs` deliberately throws on a shape it
/// cannot render rather than degrading it to `unknown`, which is exactly the
/// silent loss of checking ADR 0012 bought Svelte to avoid. A `path` of line
/// indexes says where each entry sits at any depth, in a shape that is fully
/// declared. It also happens to be the shape the page wants: a Component's
/// Steps are set at the foot in document order, and that is this list.
///
/// **Nothing here is stored.** The whole structure is worked out at display
/// time from the Readings and the inner recipes' own Yields, which is why
/// editing a dough reaches every recipe using it without minting a Version of
/// any of them.
fn component_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            // Line indexes from the recipe being read down to this Component:
            // `[3]` is the fourth line of the recipe, `[3, 1]` the second line
            // of the recipe THAT line names.
            "path": { "type": "array", "items": { "type": "integer" } },
            "lineage_id": { "type": "string" },
            // Whether this instance has the recipe at all. False is an
            // ordinary state and not an error — deleted, never received, or
            // held by nobody here are one case. Everything below is then null
            // and the line reads as written (ADR 0002).
            "held": { "type": "boolean" },
            // Unfolding stopped here because this Recipe is already open
            // further up. A cycle is never refused (ADR 0008): it stops, and
            // says so.
            "stopped": { "type": "boolean" },
            "branch_id": { "type": ["string", "null"] },
            "title": { "type": ["string", "null"] },
            // **How much of the inner recipe is wanted**, computed from the
            // Reading's quantity over that recipe's Yield and stored nowhere.
            // Null means Kamosu could not compare the two — `2 poignées` of a
            // dough that yields `1 kg`, or a recipe with no Yield at all — and
            // the inner recipe is then handed over as written rather than
            // silently mis-scaled.
            "share": { "type": ["number", "null"] },
            // **The one line beneath the Component's written line**, already
            // worded — which recipe it names and how much of it, or the one
            // sentence saying why there is no unfolding. Worded in the Core
            // for the reason `measured` is (#49): the recipe page is rendered
            // twice, once as a Svelte screen and once as a Rust Share Link,
            // and an agent reads the same recipe at the MCP door. One sentence
            // worded three times would be three chances to drift.
            "said": { "type": "string" },
            "content": {
                "type": ["object", "null"],
                "properties": recipe_content_properties(),
                "required": [
                    "title", "yield", "prep_time_minutes", "cook_time_minutes",
                    "note", "main_photo", "source", "nutrition", "ingredients", "steps",
                ],
                "additionalProperties": false,
            },
            "readings": { "type": ["array", "null"], "items": reading_schema() },
            // The inner recipe's own subordinate lines, already scaled by
            // `share` and converted to this reader's measures — the same one
            // slot every Ingredient Line has (#49, ADR 0016), so a Component's
            // lines are worded by the same code as everything else.
            "measured": {
                "type": ["object", "null"],
                "properties": {
                    "ingredients": { "type": "array", "items": { "type": ["string", "null"] } },
                    "steps": { "type": "array", "items": measured_step_schema() },
                },
                "required": ["ingredients", "steps"],
                "additionalProperties": false,
            },
        },
        "required": [
            "path", "lineage_id", "held", "stopped", "branch_id", "title",
            "share", "said", "content", "readings", "measured"
        ],
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
            "steps": { "type": "array", "items": measured_step_schema() },
        },
        "required": ["ingredients", "steps"],
        "additionalProperties": false,
    })
}

/// **What a Step carries beside its text** (#150, ADR 0016), or `null` where
/// there is nothing to add — a Section row, or a Step already in this reader's
/// measures at this Yield.
///
/// Each conversion the Step offers, in the order written: its oven in the
/// other system, and each amount it writes, converted and scaled as an
/// Ingredient Line is. `written` is the temperature or the amount and Unit
/// exactly as they stand in the Step's text, so a screen finds them by
/// searching forward from the last one and puts `measured` straight after.
/// The text itself is never rewritten.
fn measured_step_schema() -> Value {
    json!({
        "type": ["array", "null"],
        "items": {
            "type": "object",
            "properties": {
                "written": { "type": "string" },
                "measured": { "type": "string" },
            },
            "required": ["written", "measured"],
            "additionalProperties": false,
        },
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

/// **How much of a recipe somebody means**: the Yield an Attempt is cooking
/// to, held on it as a fact about that afternoon rather than a deviation (ADR
/// 0010), and the Yield a recipe page is read at (#109). The same `{amount,
/// noun}` shape a recipe's own Yield takes — except that an empty noun makes it
/// a multiplier (`{"amount": "2", "noun": ""}` is twice the recipe), which is
/// how a recipe that never said what it makes is scaled.
fn wanted_yield_schema() -> Value {
    json!({
        "type": ["object", "null"],
        "properties": {
            "amount": { "type": "string" },
            "noun": {
                "type": "string",
                "description": "What the amount counts, as the recipe's own Yield \
                                names it. Empty makes the amount a multiplier of \
                                the recipe as written.",
            },
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
            "cooking_yield": wanted_yield_schema(),
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
            "as_cooked": as_cooked_schema(),
        },
        "required": [
            "id", "lineage_id", "person_id", "version_id", "current_step_index",
            "ticked_ingredients", "cooking_yield", "note", "rating", "finished_at",
            "resumable", "created_at", "last_action_at", "photographs", "as_cooked",
        ],
        "additionalProperties": false,
    })
}

/// **The As Cooked** an Attempt holds, or `null` where the cooking followed the
/// recipe (#58, ADR 0005).
///
/// It carries the whole recipe rather than a list of differences, because that
/// is what an As Cooked *is*: "a complete recipe state, structurally identical
/// to a Version, differing only in that it never joined a Branch". Served as
/// `recipe_content_schema` — the same definition a Version answers with — so a
/// screen that can draw a recipe can draw this with nothing new, and the day a
/// field is added to a recipe it cannot go missing here.
///
/// `version_id` is the fingerprint of that content, and it is the whole of how
/// **already promoted** is answered: it is either in the Branch's chain or it
/// is not. No flag says so, because a flag could disagree with the chain.
fn as_cooked_schema() -> Value {
    json!({
        "type": ["object", "null"],
        "properties": {
            "version_id": { "type": "string" },
            "content": recipe_content_schema(),
            // The cook answered the offer: these words belong in the diary and
            // not in the recipe. It says nothing about the As Cooked, which
            // stays whole — declining is a decision about the recipe.
            "promotion_declined": { "type": "boolean" },
            // The As Cooked laid over the Version it was cooked from, by the
            // same Pairing two Branches are laid over each other with —
            // exactly what ADR 0019 said an Attempt's deviations would need.
            // Read in the Core so both Doors and every client agree: a
            // frontend comparing by index would be wrong the moment a cook
            // adds or drops a line, which they may.
            //
            // `same` where the line was left alone, `changed` where it was
            // rewritten, `only-mine` where the cook dropped it and
            // `only-theirs` where they added one.
            "against": {
                "type": "object",
                "properties": {
                    "ingredients": { "type": "array", "items": divergence_row_schema() },
                    "steps": { "type": "array", "items": divergence_row_schema() },
                },
                "required": ["ingredients", "steps"],
                "additionalProperties": false,
            },
        },
        "required": ["version_id", "content", "against", "promotion_declined"],
        "additionalProperties": false,
    })
}

/// `set_as_cooked`'s input: which cooking, and the whole recipe as it was
/// actually cooked — or `null` to say it was cooked as written after all.
///
/// The recipe is nested under one key rather than spread across the top level
/// the way `save_recipe_version` spreads it, because this Operation says two
/// separate things: *which cooking*, and *what it was*. Flattening them would
/// put `attempt_id` in among the Ingredient Lines.
fn set_as_cooked_input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "attempt_id": { "type": "string" },
            "as_cooked": {
                "type": ["object", "null"],
                "properties": recipe_content_input_properties(),
                "required": ["title"],
                "additionalProperties": false,
            },
            "written_at": written_at_schema(),
        },
        "required": ["attempt_id", "as_cooked"],
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
            // What the Version cooked says it makes, so a cooking at another
            // amount can say both (#109).
            "written_yield": yield_schema(),
        },
        "required": ["branch_id", "title", "written_yield"],
        "additionalProperties": false,
    });
    schema["required"]
        .as_array_mut()
        .expect("attempt_schema declares its required fields as an array")
        .push(json!("recipe"));
    schema
}

/// **The whole Shopping List** — the only shape any of its six Operations
/// answers (#73, ADR 0024).
///
/// One schema rather than six, because there is one answer: the choosing as
/// stored, and the rows worked out from it. Every Operation that changes the
/// choosing hands back the list it produced, so no screen and no agent ever
/// has to guess at what its own change did.
///
/// **Nothing here carries a tick.** A row is derived from a Food and however
/// many recipes mention it, and it changes shape the moment a Yield moves, so
/// there is no name to staple a tick to (ADR 0019, ADR 0024). The list leaves
/// as text and something else carries it round the shop.
fn shopping_list_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            // The choosing, in the order it was made.
            "chosen": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "branch_id": { "type": "string" },
                        // The live title while the recipe can be read, and
                        // otherwise the name it was known by when it was
                        // chosen (ADR 0024).
                        "title": { "type": "string" },
                        // Whether it can no longer be read — deleted, or in a
                        // Cookbook this Person may no longer see. The entry
                        // stays and contributes nothing, because a thing that
                        // quietly disappears from a shopping list is a thing
                        // that does not get bought.
                        "gone": { "type": "boolean" },
                        "shopping_yield": wanted_yield_schema(),
                        "written_yield": yield_schema(),
                    },
                    "required": [
                        "branch_id", "title", "gone", "shopping_yield", "written_yield"
                    ],
                    "additionalProperties": false,
                },
            },
            // The rows, computed. One list in one order: a Loose Item and a
            // line nobody read sort among the Foods rather than into a block
            // of their own.
            "rows": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        // `food` merges every mention of one Food. `line` is
                        // an Ingredient Line with no Food to merge under, kept
                        // exactly as written. `loose` is a line typed straight
                        // onto the list. The last two merge with nothing.
                        "kind": { "enum": ["food", "line", "loose"] },
                        "name": { "type": "string" },
                        // The Language the name is in, so a Food named only in
                        // another Language can be marked as borrowed. Null on
                        // a row that is somebody's own words.
                        "name_language": { "type": ["string", "null"] },
                        // Between one and several amounts. More than one is
                        // the row the arithmetic could not close: two true
                        // amounts beat one wrong one.
                        "parts": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    // `about` is an amount Kamosu converted
                                    // and rounded; `count` is a number of
                                    // things; `as_written` is a cook's own
                                    // Unit, untouched; `no_amount` is a line
                                    // nobody put a number on.
                                    "kind": {
                                        "enum": ["about", "count", "as_written", "no_amount"]
                                    },
                                    "text": { "type": "string" },
                                    // The recipes that fed this amount, so a
                                    // row that had to break open can say which
                                    // dish wants which.
                                    "sources": {
                                        "type": "array",
                                        "items": { "type": "string" },
                                    },
                                },
                                "required": ["kind", "text", "sources"],
                                "additionalProperties": false,
                            },
                        },
                        // The written lines this row was made from, always one
                        // tap away (ADR 0002, ADR 0019). Empty on a Loose Item,
                        // which was made from nothing.
                        "lines": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "branch_id": { "type": "string" },
                                    "recipe": { "type": "string" },
                                    "text": { "type": "string" },
                                },
                                "required": ["branch_id", "recipe", "text"],
                                "additionalProperties": false,
                            },
                        },
                        // **Why a row that buys nothing buys nothing**, where
                        // its line stands for a Component Kamosu could not
                        // open (ADR 0008, #86): a recipe it does not have, or
                        // one already open above it. Without it such a row
                        // reads exactly like a line Kamosu could not
                        // interpret, and nothing on the list tells the two
                        // apart. Worded in the Core so the recipe page and
                        // the list say it the same way. Null everywhere else,
                        // a Loose Item included — words typed at the door are
                        // never interpreted, deliberately (ADR 0024).
                        "said": { "type": ["string", "null"] },
                    },
                    "required": [
                        "id", "kind", "name", "name_language", "parts", "lines", "said"
                    ],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["chosen", "rows"],
        "additionalProperties": false,
    })
}

/// **What one recipe puts on a Shopping List** (#77): `shopping_basis`'s
/// answer. Every figure in it is one `shopping_list` reads for itself, so a
/// phone adding them up gets the rows the server would.
fn shopping_basis_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "branch_id": { "type": "string" },
            "title": { "type": "string" },
            "written_yield": yield_schema(),
            // The Ingredient Lines in the recipe's order, Sections left out:
            // a Section heads a list and is not a thing to buy. **Every line
            // of every recipe this one composes is here too**, unfolded to the
            // bottom and already scaled by its Component's factor (ADR 0008,
            // #86) — a Component contributes its inner recipe's Foods rather
            // than a line of its own, so the pizza's flour and its dough's
            // flour merge into one row.
            "lines": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        // Where the line sits, as the chain of line indexes
                        // that reaches it: `[3]` in this recipe, `[3, 1]` for
                        // the second line of the recipe its fourth line
                        // composes. It is what names a row that merges with
                        // nothing, and no two lines share one.
                        "path": {
                            "type": "array",
                            "items": { "type": "integer", "minimum": 0 },
                        },
                        // The recipe the line is really from, where that is
                        // not this one: a line that arrived by unfolding a
                        // Component names the inner recipe, so a row can say
                        // the flour is the dough's. Null on this recipe's own
                        // lines.
                        "from": {
                            "type": ["object", "null"],
                            "properties": {
                                "branch_id": { "type": "string" },
                                "title": { "type": "string" },
                            },
                            "required": ["branch_id", "title"],
                            "additionalProperties": false,
                        },
                        "text": { "type": "string" },
                        // **Why this line buys nothing**, where it stands for
                        // a Component Kamosu could not open (ADR 0008, #86):
                        // a recipe it does not have, or one already open above
                        // it. Worded once in the Core, in the reader's
                        // Language, so the recipe page and the list say it the
                        // same way. Null on every other line.
                        "said": { "type": ["string", "null"] },
                        // Null where no Food was read: the line then stands
                        // on the list exactly as written and merges with
                        // nothing (ADR 0024).
                        "food": {
                            "type": ["object", "null"],
                            "properties": {
                                "id": { "type": "string" },
                                // The name this reader sees, and its Language.
                                // Null where the Food has no name at all.
                                "name": { "type": ["string", "null"] },
                                "name_language": { "type": ["string", "null"] },
                                // The amount as a number, for the recipe as
                                // written and already carrying its Component's
                                // factor where it came through one. Null where
                                // nobody put a number on the line, Kamosu
                                // could not read one, or it could work out no
                                // factor for the Component this line arrived
                                // through (#86) — the line then rides as one
                                // more thing that will not add, rather than as
                                // a figure nobody computed.
                                "amount": { "type": ["number", "null"] },
                                // The Unit exactly as the cook wrote it.
                                "unit": { "type": ["string", "null"] },
                                // Which of Kamosu's own Units that word is,
                                // or null for a Unit it does not convert.
                                "unit_id": { "type": ["string", "null"] },
                                // The word as two spellings of one Unit are
                                // matched: `clove` and `cloves` alike.
                                "unit_key": { "type": ["string", "null"] },
                                "cup_weight_grams": { "type": ["number", "null"] },
                            },
                            "required": [
                                "id", "name", "name_language", "amount", "unit",
                                "unit_id", "unit_key", "cup_weight_grams"
                            ],
                            "additionalProperties": false,
                        },
                    },
                    "required": ["path", "from", "said", "text", "food"],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["branch_id", "title", "written_yield", "lines"],
        "additionalProperties": false,
    })
}

/// **When a write was really made** (#77, ADR 0013), on every write a phone
/// may make with no network: the Attempt's and the Shopping List's. Absent is
/// now. A time later than now is taken as now.
fn written_at_schema() -> Value {
    json!({
        "type": "string",
        "description": "When this was really written, for a write a phone held while it had no network and sent later (#77): an ISO 8601 time such as 2026-09-19T14:05:00.000Z. Absent means now. Where the cook has since moved on, or the Shopping List has since been written, on another device, a write older than that changes nothing and the answer is how things stand. On a cooking already finished, a move changes nothing and a finish keeps the first finish's time (its rating, note and Photographs still land), whenever either was written: a finished cooking is final.",
    })
}

/// A Reading as `reread_ingredient_lines` reports one side of a change: the
/// reader's three fields, or null where the line had or now has none (#166).
fn reread_reading_schema() -> Value {
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

fn empty_input() -> Value {
    json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false,
    })
}

/// One Backup as both Operations that mention one describe it, declared once
/// so `take_backup` and `list_backups` cannot drift into two shapes for the
/// same thing (#78).
fn backups_schema() -> Value {
    json!({
        "type": "array",
        "items": {
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                // `slot`, not `tier`: what `backups::Archive` serialises
                // (src/backups.rs) and what ADR 0039 calls it throughout. The
                // declaration said `tier` until #103, so the generated client
                // promised a field the server has never sent — a Backup list
                // would have rendered a blank where "daily" belongs, while a
                // stand-in test built on the declaration passed.
                "slot": { "enum": ["daily", "weekly", "monthly"] },
                "taken_at": { "type": "string" },
                "size_bytes": { "type": "integer" },
            },
            "required": ["name", "slot", "taken_at", "size_bytes"],
            "additionalProperties": false,
        },
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
                    // Carried out so the interface's service worker can tell a
                    // read it may answer from the phone from a write it must
                    // never replay (#76, ADR 0013) without a list of its own.
                    "write": op.write,
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

    /// The two saves say how long a re-save collapses as a figure written
    /// into their summaries, which are literal text. This holds that figure
    /// to the Core's window, so changing one fails the build until the other
    /// follows (#168).
    #[test]
    fn the_save_summaries_say_the_collapse_window_the_core_uses() {
        let window = format!("{} minutes", crate::core::COLLAPSE_WINDOW_SECONDS / 60);
        for name in ["save_recipe_version", "edit_recipe"] {
            let summary = find(name).expect("declared").summary;
            assert!(summary.contains(&window), "{name} says {window}: {summary}");
        }
    }

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
