//! The Operations themselves. Each one is declared in `catalogue.rs`; this module
//! holds the functions those declarations name.
//!
//! # Reading input, after #85
//!
//! Shape is no longer checked here. `Core::execute` holds every input to the
//! Operation's own declaration before a handler runs, so a handler is never
//! reached with an unknown field, a missing required one, a wrong type or a
//! value outside a declared `enum`. The hand-written "takes no input" guards
//! that used to open twenty-one of these functions are gone with it: one check,
//! in one place, keyed on the one declaration.
//!
//! What remains, and reads like a shape check without being one, is the
//! `input.get("x").and_then(Value::as_str).ok_or_else(…)` that opens most
//! handlers. That is **extraction**, not validation: the value has to come out
//! of the envelope as a `&str` either way, and Rust offers no way to do that
//! without saying what happens when it is absent. Its error arm is now
//! ordinarily unreachable — the Catalogue refused the call first — and it stays
//! because the alternative is a panic, not because the check is wanted twice.
//!
//! **Semantic checks are a different thing entirely and belong here**: whether
//! a Branch exists, whether this Person may touch it, whether an amount parses.
//! No schema can know any of that, and none of it moved.

use serde_json::Value;
use serde_json::json;

use crate::core::{Caller, Core, Invocation, OpError, Reading, RelatedSide};
use crate::jobs;

/// The instance status: the version, whether setup has happened, and the
/// shortest password a sign-in screen should ask for (#138).
///
/// Input: `{}` (declared in the Catalogue). Output: `{ version, setup_complete,
/// password_minimum }`.
pub fn instance_status(
    core: &Core,
    _invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    Ok(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "setup_complete": core.setup_complete()?,
        "password_minimum": crate::core::PASSWORD_MINIMUM,
    }))
}

pub fn set_reading_preferences(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let language = input
        .get("reading_language")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request(
                "set_reading_preferences takes { reading_language, reading_measures }",
            )
        })?;
    let measures = input
        .get("reading_measures")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request(
                "set_reading_preferences takes { reading_language, reading_measures }",
            )
        })?;
    let caller = caller_of(invocation)?;
    core.set_reading_preferences(&caller.person_id, language, measures)?;
    Ok(json!({ "reading_language": language, "reading_measures": measures }))
}

pub fn get_reading_preferences(
    core: &Core,
    invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    let caller = caller_of(invocation)?;
    core.reading_preferences(&caller.person_id)
}

pub fn rename_person(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let name = input
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("rename_person takes { name }"))?;
    let caller = caller_of(invocation)?;
    // The name as stored, spaces trimmed, so the screen shows what it will
    // read back rather than what was typed.
    let name = core.rename_person(&caller.person_id, name)?;
    Ok(json!({ "name": name }))
}

pub fn get_person(core: &Core, invocation: &Invocation, _input: Value) -> Result<Value, OpError> {
    let caller = caller_of(invocation)?;
    core.person(&caller.person_id)
}

/// Who holds an account here (#103). `is_you` is answered by the Core rather
/// than left to the screen to work out by matching names, because a name is a
/// reminder and not identification (ADR 0015).
pub fn list_accounts(
    core: &Core,
    invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    let caller = caller_of(invocation)?;
    core.list_accounts(&caller.person_id)
}

/// Make a Person an Operator, or stand them down (#103). The refusal that
/// protects the last one lives in the Core, under both Doors.
pub fn set_operator(core: &Core, _invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let takes = "set_operator takes { name, is_operator }";
    let name = input
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let is_operator = input
        .get("is_operator")
        .and_then(Value::as_bool)
        .ok_or_else(|| OpError::bad_request(takes))?;
    core.set_operator(name, is_operator)
}

/// Mint a one-use Invite. The Operator receives the link once; its Secret is
/// stored only hashed and is spent when a stranger uses it.
pub fn mint_invite(core: &Core, _invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let is_operator = match input.get("is_operator") {
        None => false,
        Some(Value::Bool(value)) => *value,
        _ => {
            return Err(OpError::bad_request(
                "mint_invite takes optional { is_operator }",
            ));
        }
    };
    Ok(json!({ "link": core.mint_invite(is_operator)? }))
}

pub fn disable_account(
    core: &Core,
    _invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let name = input
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("disable_account takes { name }"))?;
    core.end_account(name, false)?;
    Ok(json!({ "disabled": true }))
}

pub fn delete_account(
    core: &Core,
    _invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let name = input
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("delete_account takes { name }"))?;
    core.end_account(name, true)?;
    Ok(json!({ "deleted": true }))
}

pub fn mint_recovery_link(
    core: &Core,
    _invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let name = input
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("mint_recovery_link takes { name }"))?;
    Ok(json!({ "link": core.mint_recovery_link(name)? }))
}

pub fn list_sessions(
    core: &Core,
    invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    let caller = caller_of(invocation)?;
    Ok(json!({
        "sessions": core.sessions_of(&caller.person_id, caller.session_id.as_deref())?
    }))
}

pub fn rename_session(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let (Some(session_id), Some(name)) = (
        input.get("session_id").and_then(Value::as_str),
        input.get("name").and_then(Value::as_str),
    ) else {
        return Err(OpError::bad_request(
            "rename_session takes { session_id, name }",
        ));
    };
    let caller = caller_of(invocation)?;
    let name = core.rename_session(&caller.person_id, session_id, name)?;
    Ok(json!({ "id": session_id, "name": name }))
}

pub fn revoke_session(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let session_id = input
        .get("session_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("revoke_session takes { session_id }"))?;
    let caller = caller_of(invocation)?;
    core.revoke_session(&caller.person_id, session_id)?;
    Ok(json!({ "revoked": true }))
}

pub fn mint_access_key(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let name = input
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("mint_access_key takes { name, read_only? }"))?;
    let read_only = input
        .get("read_only")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let caller = caller_of(invocation)?;
    let key = core.mint_access_key(&caller.person_id, name, read_only)?;
    Ok(json!({
        "id": key.id,
        "name": key.name,
        "read_only": key.read_only,
        "secret": key.secret,
    }))
}

pub fn list_access_keys(
    core: &Core,
    invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    let caller = caller_of(invocation)?;
    Ok(json!({ "access_keys": core.access_keys_of(&caller.person_id)? }))
}

pub fn revoke_access_key(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let access_key_id = input
        .get("access_key_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("revoke_access_key takes { access_key_id }"))?;
    let caller = caller_of(invocation)?;
    core.revoke_access_key(&caller.person_id, access_key_id)?;
    Ok(json!({ "revoked": true }))
}

/// Read one Job back: its state, its progress, and its result or the reason it
/// failed. This is where watching slow work happens — at both Doors alike.
pub fn get_job(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let job_id = input
        .get("job_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("get_job takes { job_id }"))?;

    let record = core.job(job_id)?.ok_or_else(|| jobs::no_such_job(job_id))?;
    jobs::ensure_reader(&record, &invocation.caller, || jobs::no_such_job(job_id))?;

    Ok(jobs::to_value(&record))
}

/// List the Jobs one Person has asked for, newest first. A stranger has asked
/// for nothing here: anonymous Jobs belong to no Person to list them by.
pub fn list_jobs(core: &Core, invocation: &Invocation, _input: Value) -> Result<Value, OpError> {
    let caller = caller_of(invocation)?;
    let records = core.jobs_of(&caller.person_id)?;
    Ok(json!({
        "jobs": records.iter().map(jobs::to_summary).collect::<Vec<_>>(),
    }))
}

/// Cancel a Job: the same ownership rule as reading it, and the cancellation
/// itself lives here in the Core — the MCP door's tasks/cancel is decoration
/// over this Operation, exactly as ADR 0001 asks of anything state-changing.
pub fn cancel_job(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let job_id = input
        .get("job_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("cancel_job takes { job_id }"))?;

    let record = core.job(job_id)?.ok_or_else(|| jobs::no_such_job(job_id))?;
    jobs::ensure_reader(&record, &invocation.caller, || jobs::no_such_job(job_id))?;

    let cancelled = core.cancel_job_if_queued(job_id)?;
    Ok(json!({ "cancelled": cancelled }))
}

pub fn create_kitchen(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let name = input
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("create_kitchen takes { name }"))?;
    let caller = caller_of(invocation)?;
    core.create_kitchen(&caller.person_id, name)
}

pub fn list_kitchens(
    core: &Core,
    invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    let caller = caller_of(invocation)?;
    Ok(json!({ "kitchens": core.list_kitchens(&caller.person_id)? }))
}

pub fn rename_kitchen(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let kitchen_id = input
        .get("kitchen_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("rename_kitchen takes { kitchen_id, name }"))?;
    let name = input
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("rename_kitchen takes { kitchen_id, name }"))?;
    let caller = caller_of(invocation)?;
    core.rename_kitchen(&caller.person_id, kitchen_id, name)?;
    Ok(json!({ "name": name }))
}

pub fn set_kitchen_nickname(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let kitchen_id = input
        .get("kitchen_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request("set_kitchen_nickname takes { kitchen_id, nickname }")
        })?;
    if !input
        .get("nickname")
        .is_some_and(|v| v.is_string() || v.is_null())
    {
        return Err(OpError::bad_request(
            "set_kitchen_nickname takes { kitchen_id, nickname }",
        ));
    }
    let nickname = input.get("nickname").and_then(Value::as_str);
    let caller = caller_of(invocation)?;
    core.set_kitchen_nickname(&caller.person_id, kitchen_id, nickname)?;
    Ok(json!({ "nickname": nickname }))
}

pub fn invite_to_kitchen(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let kitchen_id = input
        .get("kitchen_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("invite_to_kitchen takes { kitchen_id }"))?;
    let caller = caller_of(invocation)?;
    let (id, secret) = core.invite_to_kitchen(&caller.person_id, kitchen_id)?;
    Ok(json!({ "invite_id": id, "secret": secret }))
}

pub fn accept_kitchen_invite(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let secret = input
        .get("secret")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("accept_kitchen_invite takes { secret }"))?;
    let caller = caller_of(invocation)?;
    core.accept_kitchen_invite(&caller.person_id, secret)
}

pub fn remove_kitchen_member(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let kitchen_id = input
        .get("kitchen_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request("remove_kitchen_member takes { kitchen_id, person_id }")
        })?;
    let person_id = input
        .get("person_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request("remove_kitchen_member takes { kitchen_id, person_id }")
        })?;
    let caller = caller_of(invocation)?;
    core.remove_kitchen_member(&caller.person_id, kitchen_id, person_id)?;
    Ok(json!({ "removed": true }))
}

pub fn delete_kitchen(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let kitchen_id = input
        .get("kitchen_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("delete_kitchen takes { kitchen_id }"))?;
    let caller = caller_of(invocation)?;
    core.delete_kitchen(&caller.person_id, kitchen_id)?;
    Ok(json!({ "deleted": true }))
}

/// What removing a Person from a Kitchen would leave each side, before
/// anybody does it (#131, screen choice 4). `person_id` defaults to the
/// caller, which is leaving.
pub fn preview_leaving_kitchen(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let kitchen_id = input
        .get("kitchen_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request("preview_leaving_kitchen takes { kitchen_id, person_id? }")
        })?;
    let caller = caller_of(invocation)?;
    let person_id = input
        .get("person_id")
        .and_then(Value::as_str)
        .unwrap_or(&caller.person_id);
    core.preview_leaving_kitchen(&caller.person_id, kitchen_id, person_id)
}

pub fn get_cookbook(core: &Core, invocation: &Invocation, _input: Value) -> Result<Value, OpError> {
    let caller = caller_of(invocation)?;
    core.get_cookbook(&caller.person_id)
}

pub fn rename_cookbook(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let caller = caller_of(invocation)?;
    core.rename_cookbook(&caller.person_id, input.get("name").and_then(Value::as_str))
}

pub fn invite_to_cookbook(
    core: &Core,
    invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    let caller = caller_of(invocation)?;
    let (id, secret) = core.invite_to_cookbook(&caller.person_id)?;
    Ok(json!({ "invite_id": id, "secret": secret }))
}

pub fn cancel_cookbook_invite(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let invite_id = input
        .get("invite_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("cancel_cookbook_invite takes { invite_id }"))?;
    let caller = caller_of(invocation)?;
    core.cancel_cookbook_invite(&caller.person_id, invite_id)?;
    Ok(json!({ "ended": true }))
}

pub fn read_cookbook_invite(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let secret = input
        .get("secret")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("read_cookbook_invite takes { secret }"))?;
    let caller = caller_of(invocation)?;
    core.read_cookbook_invite(&caller.person_id, secret)
}

pub fn accept_cookbook_invite(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let secret = input
        .get("secret")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("accept_cookbook_invite takes { secret }"))?;
    let caller = caller_of(invocation)?;
    core.accept_cookbook_invite(&caller.person_id, secret)
}

pub fn leave_cookbook(
    core: &Core,
    invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    let caller = caller_of(invocation)?;
    core.leave_cookbook(&caller.person_id)
}

pub fn remove_cookbook_author(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let person_id = input
        .get("person_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("remove_cookbook_author takes { person_id }"))?;
    let caller = caller_of(invocation)?;
    core.remove_cookbook_author(&caller.person_id, person_id)
}

/// Start a Branch of a recipe, unchanged, in the caller's own Cookbook, under
/// a name they give it (ADR 0041).
pub fn start_variation(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let takes = "start_variation takes { branch_id, name }";
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let name = input
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let caller = caller_of(invocation)?;
    core.start_variation(&caller.person_id, branch_id, name)
}

pub fn rename_branch(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("rename_branch takes { branch_id, name }"))?;
    let caller = caller_of(invocation)?;
    core.rename_branch(
        &caller.person_id,
        branch_id,
        input.get("name").and_then(Value::as_str),
    )
}

pub fn create_tag(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let takes = "create_tag takes { language, name }";
    let language = input
        .get("language")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let name = input
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let caller = caller_of(invocation)?;
    core.create_tag(&caller.person_id, language, name)
}

pub fn list_tags(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let everywhere = input
        .get("everywhere")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let kitchen_id = input.get("kitchen_id").and_then(Value::as_str);
    let caller = caller_of(invocation)?;
    Ok(json!({ "tags": core.list_tags(&caller.person_id, everywhere, kitchen_id)? }))
}

pub fn rename_tag(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let takes = "rename_tag takes { tag_id, language, name }";
    let tag_id = input
        .get("tag_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let language = input
        .get("language")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let name = input
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let caller = caller_of(invocation)?;
    core.rename_tag(&caller.person_id, tag_id, language, name)
}

pub fn merge_tags(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let takes = "merge_tags takes { keep_tag_id, merge_tag_id }";
    let keep_tag_id = input
        .get("keep_tag_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let merge_tag_id = input
        .get("merge_tag_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let caller = caller_of(invocation)?;
    core.merge_tags(&caller.person_id, keep_tag_id, merge_tag_id)
}

pub fn delete_tag(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let tag_id = input
        .get("tag_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("delete_tag takes { tag_id }"))?;
    let caller = caller_of(invocation)?;
    core.delete_tag(&caller.person_id, tag_id)?;
    Ok(json!({ "deleted": true }))
}

pub fn set_recipe_tag(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let takes = "set_recipe_tag takes { branch_id, tag_id, carried }";
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let tag_id = input
        .get("tag_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let carried = input
        .get("carried")
        .and_then(Value::as_bool)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let caller = caller_of(invocation)?;
    core.set_recipe_tag(&caller.person_id, branch_id, tag_id, carried)
}

pub fn set_related_recipe(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let takes = "set_related_recipe takes { branch_id, related_branch_id or \
                 related_lineage_id, related }";
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let related = input
        .get("related")
        .and_then(Value::as_bool)
        .ok_or_else(|| OpError::bad_request(takes))?;
    // Exactly one of the two names the far end. The Catalogue's schema checks
    // shape and cannot say "one of these", so the rule is stated here — where
    // both Doors inherit it, since both arrive through this one handler.
    let other = match (
        input.get("related_branch_id").and_then(Value::as_str),
        input.get("related_lineage_id").and_then(Value::as_str),
    ) {
        (Some(branch), None) => RelatedSide::Branch(branch),
        (None, Some(lineage)) => RelatedSide::Lineage(lineage),
        (Some(_), Some(_)) => {
            return Err(OpError::bad_request(
                "set_related_recipe takes related_branch_id or related_lineage_id, not both",
            ));
        }
        (None, None) => return Err(OpError::bad_request(takes)),
    };
    let caller = caller_of(invocation)?;
    core.set_related_recipe(&caller.person_id, branch_id, other, related)
}

pub fn create_recipe(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let language = input.get("language").and_then(Value::as_str);
    let caller = caller_of(invocation)?;
    core.create_recipe(caller, &input, language)
}

pub fn save_recipe_version(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request(
                "save_recipe_version takes { branch_id, title, name?, change_note?, \
             translates_version_id?, yield?, prep_time_minutes?, \
             cook_time_minutes?, note?, source?, ingredients?, steps? }",
            )
        })?;
    let name = input.get("name").and_then(Value::as_str);
    let change_note = input.get("change_note").and_then(Value::as_str);
    let translates_version_id = input.get("translates_version_id").and_then(Value::as_str);
    let caller = caller_of(invocation)?;
    core.save_recipe_version(
        caller,
        branch_id,
        &input,
        name,
        change_note,
        translates_version_id,
    )
}

/// **Read a whole recipe pasted as text** (#94).
///
/// Pure, and the only handler in this file that touches neither the database
/// nor the Person who called it: the parse is a function over a string, and
/// what comes back is shown to whoever pasted it before anything is saved by
/// an ordinary `create_recipe` or `save_recipe_version`.
///
/// It is an Operation rather than TypeScript in `ui/` for three reasons, the
/// open call #94 left to whoever implemented it. The split rests on
/// [`crate::reading`], whose Unit vocabulary is [`crate::units`]'s in three
/// Languages, and a second copy of that in a second language is exactly the
/// objection ADR 0036 raised against the two ingredient-parsing libraries. An
/// agent at the MCP door pastes recipes as readily as a browser does, and a
/// parser only the web could reach is the web-only route ADR 0001 exists to
/// make unrepresentable. And the measurement that says the split works at all
/// lives in `tests/pasting_corpus.rs`, against the real export.
///
/// What it costs is one round trip, paid once when the paste arrives. Moving
/// the boundary afterwards costs none: every line comes back already read, so
/// the split is an index into an answer the screen already holds.
pub fn read_pasted_recipe(
    _core: &Core,
    _invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let text = input
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("read_pasted_recipe takes { text }"))?;

    // A semantic check rather than a shape one, so it belongs here (#85): the
    // Catalogue's declaration says a string and cannot say how long a recipe
    // somebody typed plausibly is.
    // `str::len` is bytes, and so is the bound — a limit on how much text
    // Kamosu will hold, not on how many letters a recipe may have. Said in
    // bytes rather than reported as characters, which it is not: an accented
    // paste would otherwise be refused well before the number it was given.
    if text.len() > crate::pasting::LONGEST_PASTE {
        return Err(OpError::bad_request(format!(
            "a pasted recipe is at most {} bytes of text, and that is {}",
            crate::pasting::LONGEST_PASTE,
            text.len()
        )));
    }

    let paste = crate::pasting::read_paste(text);
    Ok(serde_json::json!({
        "title": paste.title,
        "lines": paste
            .lines
            .iter()
            .map(|line| serde_json::json!({ "text": line.text, "kind": line.kind.as_str() }))
            .collect::<Vec<_>>(),
        "boundary": paste.boundary,
    }))
}

/// Start a Translation: an ordinary Branch of the same Lineage in another
/// Language, whose first Version records what it renders (#56, ADR 0006).
pub fn start_translation(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let takes = "start_translation takes { branch_id, language, title, \
                 translates_version_id?, name?, change_note?, yield?, \
                 prep_time_minutes?, cook_time_minutes?, note?, source?, ingredients?, steps? }";
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let language = input
        .get("language")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let name = input.get("name").and_then(Value::as_str);
    let change_note = input.get("change_note").and_then(Value::as_str);
    let translates_version_id = input.get("translates_version_id").and_then(Value::as_str);
    let caller = caller_of(invocation)?;
    core.start_translation(
        caller,
        branch_id,
        language,
        &input,
        name,
        change_note,
        translates_version_id,
    )
}

/// Say what Language a recipe is written in — the only thing that acts on a
/// save's `language_offer`, and it makes a Version (#56, ADR 0006).
pub fn set_recipe_language(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let takes = "set_recipe_language takes { branch_id, language }";
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let language = input
        .get("language")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let caller = caller_of(invocation)?;
    core.set_recipe_language(caller, branch_id, language)
}

/// Import: land a batch of candidates already read from an outside source
/// into the caller's own Cookbook. Reading the source itself is each
/// importer's own job (#69, #70); this Operation is the shared machinery
/// they land through (#68, ADR 0025).
pub fn import(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let source_kind = input
        .get("source_kind")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("import takes { source_kind, candidates }"))?;
    let candidates = input
        .get("candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| OpError::bad_request("import takes { source_kind, candidates }"))?;
    let caller = caller_of(invocation)?;
    core.import(caller, source_kind, candidates, invocation.job.as_ref())
}

/// Import a recipe straight from a web link (#70): fetch the page through the
/// guarded client (ADR 0033), read its schema.org JSON-LD, and land the one
/// candidate through the same ledgered machinery every importer shares
/// (`import`, #68, ADR 0025). The page's own address — after redirects — is
/// both the ledger's foreign id and the Version's Source link, so re-running
/// the import on the same page matches instead of doubling the library.
pub fn import_web_link(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let url = input
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("import_web_link takes { url }"))?;
    let caller = caller_of(invocation)?;
    let progress = invocation.job.as_ref();

    if let Some(progress) = progress {
        progress.report(0, None, "reading the page".to_string());
    }
    let (html, effective_url) = crate::web_import::fetch_page(url)?;
    let parsed = crate::web_import::extract_recipe(&html, &effective_url);

    // A photo that failed to fetch, decode or remake never fails the recipe
    // (#45's rule for an uploaded picture, applied the same way to one
    // arriving by URL): the recipe still lands, simply without a photo.
    let main_photo = parsed.image_url.as_deref().and_then(|image_url| {
        if let Some(progress) = progress {
            progress.report(0, None, "fetching the photo".to_string());
        }
        let bytes = crate::web_import::fetch_photo(image_url).ok()?;
        let stored = core.store_photograph(&bytes).ok()?;
        stored
            .get("photograph_id")
            .and_then(Value::as_str)
            .map(str::to_string)
    });

    let ingredients: Vec<Value> = parsed
        .ingredients
        .iter()
        .map(|text| json!({ "kind": "ingredient", "text": text }))
        .collect();
    let steps: Vec<Value> = parsed
        .steps
        .iter()
        .map(|step| match step {
            crate::web_import::StepEntry::Section(text) => {
                json!({ "kind": "section", "text": text, "photo": null })
            }
            crate::web_import::StepEntry::Step(text) => {
                json!({ "kind": "step", "text": text, "photo": null })
            }
        })
        .collect();
    let recipe_yield = parsed
        .yield_amount_noun
        .as_ref()
        .map(|(amount, noun)| json!({ "amount": amount, "noun": noun }));
    // Nutrition rides only where the page's own structured data stated a
    // number (#72, ADR 0025) — never worked out from the Ingredient Lines
    // just read, because a plausible-but-wrong calorie figure is worse than
    // an empty field.
    //
    // The basis is the one thing here not read off the page: schema.org
    // defines every `NutritionInformation` value as being for one serving, so
    // `per_serving` is what the vocabulary the page chose to speak already
    // means. A page that states a number states a per-serving number whether
    // or not it says so, and the alternative — landing the figure with no
    // basis — is not available, since a figure that does not say what it
    // counts is refused (`parse_nutrition`).
    let nutrition = parsed
        .nutrition_calories
        .map(|calories| json!({ "calories": calories, "basis": "per_serving" }));

    let candidate = json!({
        "foreign_id": effective_url,
        "title": parsed.title,
        "yield": recipe_yield,
        "prep_time_minutes": parsed.prep_time_minutes,
        "cook_time_minutes": parsed.cook_time_minutes,
        "note": Value::Null,
        "main_photo": main_photo,
        "source": { "text": parsed.source_text, "link": effective_url },
        "nutrition": nutrition,
        "ingredients": ingredients,
        "steps": steps,
    });

    core.import(caller, "web", &[candidate], progress)
}

pub fn rename_version(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request("rename_version takes { branch_id, sequence, name }")
        })?;
    let sequence = input
        .get("sequence")
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            OpError::bad_request("rename_version takes { branch_id, sequence, name }")
        })?;
    if !input
        .get("name")
        .is_some_and(|v| v.is_string() || v.is_null())
    {
        return Err(OpError::bad_request(
            "rename_version takes { branch_id, sequence, name }",
        ));
    }
    let name = input.get("name").and_then(Value::as_str);
    let caller = caller_of(invocation)?;
    let stored = core.rename_version(&caller.person_id, branch_id, sequence, name)?;
    Ok(json!({ "name": stored }))
}

/// The base64 fallback for a Door that cannot carry raw bytes (ADR 0001). A
/// browser instead calls the out-of-band `POST /api/photographs`; both reach
/// the identical `Core::store_photograph`.
pub fn upload_photograph(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let data = input.get("data").and_then(Value::as_str).ok_or_else(|| {
        OpError::bad_request("upload_photograph takes { data } — the picture, base64-encoded")
    })?;
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|e| OpError::bad_request(format!("data is not valid base64: {e}")))?;
    core.upload_photograph(caller_of(invocation)?, &bytes)
}

/// Ask for the orphan sweep now rather than waiting for the daily one (#46).
///
/// The sweep takes no arguments and reads nothing from the caller: what it
/// deletes is decided entirely by what the Versions point at. The Operator
/// permission is about who may cause the work, never about what it does.
pub fn sweep_photographs(
    core: &Core,
    _invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    core.sweep_photographs()
}

/// Take a Backup now (#78). The archive itself never travels through here:
/// this answers what was written and what the instance now holds, and the
/// bytes are fetched out of band at `GET /api/backups/<name>`.
pub fn take_backup(core: &Core, invocation: &Invocation, _input: Value) -> Result<Value, OpError> {
    core.take_backup(invocation.job.as_ref())
}

/// What Backups this instance holds (#78).
pub fn list_backups(
    core: &Core,
    _invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    core.list_backups()
}

pub fn search_recipes(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let query = input.get("query").and_then(Value::as_str);
    let kitchen_id = input.get("kitchen_id").and_then(Value::as_str);
    let mine = input.get("mine").and_then(Value::as_bool).unwrap_or(false);
    let tag_id = input.get("tag_id").and_then(Value::as_str);
    let caller = caller_of(invocation)?;
    core.search_recipes(&caller.person_id, query, kitchen_id, mine, tag_id)
}

/// Home's computed shelves (#64). Like the diary it takes no input: whose Home
/// is not a question, and a `person_id` here would be a permission question
/// wearing a parameter's clothes.
pub fn home_shelves(core: &Core, invocation: &Invocation, _input: Value) -> Result<Value, OpError> {
    let caller = caller_of(invocation)?;
    core.home_shelves(&caller.person_id)
}

pub fn note_recipe_opened(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("note_recipe_opened takes { branch_id }"))?;
    let caller = caller_of(invocation)?;
    core.note_recipe_opened(&caller.person_id, branch_id)
}

pub fn get_recipe(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("get_recipe takes { branch_id, wanted_yield? }"))?;
    let caller = caller_of(invocation)?;
    core.get_recipe(&caller.person_id, branch_id, input.get("wanted_yield"))
}

/// Receive a Bundle (#67), landed in the caller's own Cookbook as a Job
/// answering the Import Report.
///
/// The file arrives one of two ways, both carrying the same bytes to the same
/// reader (ADR 0001), and the pair is `import_crouton`'s (#69, #93):
/// `upload_id` names a file already sent out of band to `POST /api/uploads`,
/// which is how a browser sends one recipe's worth of photographs; `data`
/// carries it base64-encoded, the fallback for a Door that can send nothing but
/// JSON. A staged upload is used once — deleted whatever the import's outcome,
/// so a retry sends the file again.
pub fn import_bundle(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    const USAGE: &str = "import_bundle takes { upload_id } or { data }: the Bundle's zip \
                       sent to POST /api/uploads, or base64-encoded";
    let caller = caller_of(invocation)?;
    let progress = invocation.job.as_ref();
    match (
        input.get("upload_id").and_then(Value::as_str),
        input.get("data").and_then(Value::as_str),
    ) {
        (Some(upload_id), None) => {
            let path = core.staged_upload(&caller.person_id, upload_id)?;
            let outcome = std::fs::read(&path)
                .map_err(|e| OpError::internal(format!("cannot open the upload: {e}")))
                .and_then(|bytes| core.import_bundle(caller, &bytes, progress));
            let _ = std::fs::remove_file(&path);
            outcome
        }
        (None, Some(data)) => {
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(data)
                .map_err(|e| OpError::bad_request(format!("data is not valid base64: {e}")))?;
            core.import_bundle(caller, &bytes, progress)
        }
        _ => Err(OpError::bad_request(USAGE)),
    }
}

/// Bring in a Crouton export (#69) — the whole library as a zip of `.crumb`
/// files, or one `.crumb`. The file arrives one of two ways, both carrying the
/// same bytes to the same reader (ADR 0001): `upload_id` names a file already
/// sent out of band to `POST /api/uploads`, which is how a browser sends a
/// 114 MB library; `data` carries it base64-encoded, the fallback for a Door
/// that can send nothing but JSON. A staged upload is used once: it is deleted
/// whatever the import's outcome, so a retry sends the file again.
pub fn import_crouton(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    const USAGE: &str = "import_crouton takes { upload_id } or { data }: the export \
                       sent to POST /api/uploads, or base64-encoded";
    let caller = caller_of(invocation)?;
    let progress = invocation.job.as_ref();
    match (
        input.get("upload_id").and_then(Value::as_str),
        input.get("data").and_then(Value::as_str),
    ) {
        (Some(upload_id), None) => {
            let path = core.staged_upload(&caller.person_id, upload_id)?;
            let outcome = std::fs::File::open(&path)
                .map_err(|e| OpError::internal(format!("cannot open the upload: {e}")))
                .and_then(crate::crouton::Export::open)
                .and_then(|export| core.import_crouton(caller, export, progress));
            let _ = std::fs::remove_file(&path);
            outcome
        }
        (None, Some(data)) => {
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(data)
                .map_err(|e| OpError::bad_request(format!("data is not valid base64: {e}")))?;
            let export = crate::crouton::Export::open(std::io::Cursor::new(bytes))?;
            core.import_crouton(caller, export, progress)
        }
        _ => Err(OpError::bad_request(USAGE)),
    }
}

/// What has been brought in from outside, by source, with every arrival (#108).
pub fn list_imports(core: &Core, invocation: &Invocation, _input: Value) -> Result<Value, OpError> {
    let caller = caller_of(invocation)?;
    core.list_imports(&caller.person_id)
}

/// Throw an Import's ledger away whole (ADR 0025); every recipe it made stays.
pub fn forget_import(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let import_id = input
        .get("import_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("forget_import takes { import_id }"))?;
    let caller = caller_of(invocation)?;
    core.forget_import(&caller.person_id, import_id)
}

/// Write a Bundle of one recipe and say where to fetch it (#66, ADR 0020).
pub fn export_bundle(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("export_bundle takes { branch_id }"))?;
    let caller = caller_of(invocation)?;
    core.export_bundle(&caller.person_id, branch_id)
}

/// Set a Sheet of one recipe as this Person sees it (#75, ADR 0023).
pub fn make_sheet(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("make_sheet takes { branch_id, wanted_yield? }"))?;
    let caller = caller_of(invocation)?;
    core.make_sheet(
        &caller.person_id,
        branch_id,
        input.get("wanted_yield"),
        invocation.job.as_ref(),
    )
}

/// Set a Sheet of what a Share Link shows, for whoever holds it (#75).
pub fn make_shared_sheet(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let token = input
        .get("token")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("make_shared_sheet takes { token }"))?;
    core.make_shared_sheet(
        token,
        input.get("language").and_then(Value::as_str),
        input.get("locale").and_then(Value::as_str),
        invocation.job.as_ref(),
    )
}

pub fn set_reading(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request(
                "set_reading takes { branch_id, line_index, amount?, unit?, target?, lineage_id? }",
            )
        })?;
    let line_index = input
        .get("line_index")
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            OpError::bad_request(
                "set_reading takes { branch_id, line_index, amount?, unit?, target?, lineage_id? }",
            )
        })?;
    let amount = input.get("amount").and_then(Value::as_str);
    let unit = input.get("unit").and_then(Value::as_str);
    let target = input.get("target").and_then(Value::as_str);
    let lineage_id = input.get("lineage_id").and_then(Value::as_str);
    let caller = caller_of(invocation)?;
    core.set_reading(
        &caller.person_id,
        branch_id,
        line_index,
        Reading {
            amount,
            unit,
            target,
            lineage_id,
        },
    )
}

pub fn start_attempt(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request(
                "start_attempt takes { branch_id, version_id?, attempt_id?, started_at? }",
            )
        })?;
    let version_id = input.get("version_id").and_then(Value::as_str);
    let attempt_id = input.get("attempt_id").and_then(Value::as_str);
    let started_at = input.get("started_at").and_then(Value::as_str);
    let caller = caller_of(invocation)?;
    core.start_attempt(
        &caller.person_id,
        branch_id,
        version_id,
        attempt_id,
        started_at,
    )
}

pub fn get_thread(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("get_thread takes { branch_id }"))?;
    let caller = caller_of(invocation)?;
    core.get_thread(&caller.person_id, branch_id)
}

// ── Share Links (#65, ADR 0026, ADR 0018) ────────────────────────────────────

pub fn share_recipe(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("share_recipe takes { branch_id, public_address? }"))?;
    let address = input.get("public_address").and_then(Value::as_str);
    let caller = caller_of(invocation)?;
    core.share_recipe(&caller.person_id, branch_id, address)
}

pub fn end_share_link(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("end_share_link takes { branch_id }"))?;
    let caller = caller_of(invocation)?;
    core.end_share_link(&caller.person_id, branch_id)
}

pub fn get_share_link(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("get_share_link takes { branch_id }"))?;
    let caller = caller_of(invocation)?;
    core.get_share_link(&caller.person_id, branch_id)
}

/// Where this instance says it is reachable from outside (#103). The read
/// half of `set_public_address`, so the screen that changes it can show what
/// it is changing.
pub fn get_public_address(
    core: &Core,
    _invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    core.get_public_address()
}

pub fn set_public_address(
    core: &Core,
    _invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let address = input
        .get("public_address")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("set_public_address takes { public_address }"))?;
    core.set_public_address(address)
}

/// What the Share Link page consumes. Public: holding the token is the whole
/// of the permission, so there is no Caller to resolve here at all.
pub fn read_shared_recipe(
    core: &Core,
    _invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let token = input
        .get("token")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("read_shared_recipe takes { token }"))?;
    core.read_shared_recipe(token)
}

pub fn branch_point(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let takes = "branch_point takes { branch_a_id, branch_b_id }";
    let branch_a_id = input
        .get("branch_a_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let branch_b_id = input
        .get("branch_b_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let caller = caller_of(invocation)?;
    core.branch_point(&caller.person_id, branch_a_id, branch_b_id)
}

pub fn divergence(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let takes = "divergence takes { branch_id, other_branch_id }";
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let other_branch_id = input
        .get("other_branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let caller = caller_of(invocation)?;
    core.divergence(&caller.person_id, branch_id, other_branch_id)
}

pub fn advance_attempt(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let takes = "advance_attempt takes { attempt_id, current_step_index?, \
                 ticked_ingredients?, cooking_yield? }";
    let attempt_id = input
        .get("attempt_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let current_step_index = input.get("current_step_index").and_then(Value::as_i64);
    let ticked_ingredients = match input.get("ticked_ingredients") {
        None => None,
        Some(Value::Array(items)) => {
            let indices = items
                .iter()
                .map(|item| item.as_i64().ok_or_else(|| OpError::bad_request(takes)))
                .collect::<Result<Vec<i64>, OpError>>()?;
            Some(indices)
        }
        Some(_) => return Err(OpError::bad_request(takes)),
    };
    let cooking_yield = input.get("cooking_yield");
    let caller = caller_of(invocation)?;
    core.advance_attempt(
        &caller.person_id,
        attempt_id,
        current_step_index,
        ticked_ingredients.as_deref(),
        cooking_yield,
        written_at(&input),
    )
}

pub fn finish_attempt(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let attempt_id = input
        .get("attempt_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request(
                "finish_attempt takes { attempt_id, note?, rating?, photographs? }",
            )
        })?;
    let caller = caller_of(invocation)?;
    core.finish_attempt(
        &caller.person_id,
        attempt_id,
        judgement(&input),
        written_at(&input),
    )
}

/// What finishing or correcting a cooking says, as `finish_attempt` and
/// `edit_attempt` both take it.
fn judgement(input: &Value) -> crate::core::Judgement<'_> {
    crate::core::Judgement {
        note: input.get("note"),
        rating: input.get("rating"),
        photographs: input.get("photographs"),
        add_photographs: input.get("add_photographs"),
    }
}

/// When a write was made, where a phone that held it offline says (#77).
fn written_at(input: &Value) -> Option<&str> {
    input.get("written_at").and_then(Value::as_str)
}

pub fn edit_attempt(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let attempt_id = input
        .get("attempt_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request(
                "edit_attempt takes { attempt_id, note?, rating?, photographs?, add_photographs? }",
            )
        })?;
    let caller = caller_of(invocation)?;
    core.edit_attempt(
        &caller.person_id,
        attempt_id,
        judgement(&input),
        written_at(&input),
    )
}

/// Make a picture taken while cooking the recipe's Main Photo or a Step's
/// photo (#59) — an ordinary edit making a Version, never a special move.
pub fn promote_attempt_photograph(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let takes = "promote_attempt_photograph takes { attempt_id, photograph_id, \
                 branch_id, step_index?, change_note? }";
    let attempt_id = input
        .get("attempt_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let photograph_id = input
        .get("photograph_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let step_index = match input.get("step_index") {
        None | Some(Value::Null) => None,
        Some(value) => Some(
            value
                .as_i64()
                .filter(|index| *index >= 0)
                .ok_or_else(|| OpError::bad_request("step_index must be zero or more"))?,
        ),
    };
    let change_note = input.get("change_note").and_then(Value::as_str);
    let caller = caller_of(invocation)?;
    core.promote_attempt_photograph(
        caller,
        attempt_id,
        photograph_id,
        branch_id,
        step_index,
        change_note,
    )
}

pub fn set_as_cooked(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let takes = "set_as_cooked takes { attempt_id, as_cooked }, where as_cooked is \
                 the whole recipe as it was cooked, or null";
    let attempt_id = input
        .get("attempt_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    // Required by the declaration, so the Core's dispatch has already refused
    // an input without it (#85). Absent and explicitly null mean the same
    // thing here — this cooking followed the recipe.
    let as_cooked = input.get("as_cooked");
    let caller = caller_of(invocation)?;
    core.set_as_cooked(&caller.person_id, attempt_id, as_cooked, written_at(&input))
}

pub fn decline_promotion(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let takes = "decline_promotion takes { attempt_id, declined }";
    let attempt_id = input
        .get("attempt_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let declined = input
        .get("declined")
        .and_then(Value::as_bool)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let caller = caller_of(invocation)?;
    core.decline_promotion(&caller.person_id, attempt_id, declined)
}

pub fn promote_as_cooked(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let takes = "promote_as_cooked takes { attempt_id, branch_id, name?, change_note? }";
    let attempt_id = input
        .get("attempt_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let caller = caller_of(invocation)?;
    core.promote_as_cooked(
        caller,
        attempt_id,
        branch_id,
        input.get("name").and_then(Value::as_str),
        input.get("change_note").and_then(Value::as_str),
    )
}

/// Take one Branch off the shelf for good (#120). The Lineage's other
/// Branches, and every Attempt ever cooked from it, are untouched.
pub fn delete_recipe(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("delete_recipe takes { branch_id }"))?;
    let caller = caller_of(invocation)?;
    core.delete_recipe(&caller.person_id, branch_id)?;
    Ok(json!({ "deleted": true }))
}

pub fn delete_attempt(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let attempt_id = input
        .get("attempt_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("delete_attempt takes { attempt_id }"))?;
    let caller = caller_of(invocation)?;
    core.delete_attempt(&caller.person_id, attempt_id)?;
    Ok(json!({ "deleted": true }))
}

pub fn get_current_attempt(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let lineage_id = input
        .get("lineage_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("get_current_attempt takes { lineage_id }"))?;
    let caller = caller_of(invocation)?;
    core.get_current_attempt(&caller.person_id, lineage_id)
}

/// The cooking diary (#60). It takes no input because there is no whose to
/// ask: a diary is the caller's own, and a `person_id` here would be a
/// permission question wearing a parameter's clothes.
pub fn list_attempts(
    core: &Core,
    invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    let caller = caller_of(invocation)?;
    core.list_attempts(&caller.person_id)
}

pub fn list_foods(core: &Core, invocation: &Invocation, _input: Value) -> Result<Value, OpError> {
    let caller = caller_of(invocation)?;
    Ok(json!({ "foods": core.list_foods(&caller.person_id)? }))
}

pub fn get_food(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let food_id = input
        .get("food_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("get_food takes { food_id }"))?;
    let caller = caller_of(invocation)?;
    core.get_food(&caller.person_id, food_id)
}

pub fn set_food_name(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let takes = "set_food_name takes { food_id, language, name }";
    let food_id = input
        .get("food_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let language = input
        .get("language")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let name = input
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let caller = caller_of(invocation)?;
    core.set_food_name(&caller.person_id, food_id, language, name)
}

pub fn remove_food_name(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let takes = "remove_food_name takes { food_id, language }";
    let food_id = input
        .get("food_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let language = input
        .get("language")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let caller = caller_of(invocation)?;
    core.remove_food_name(&caller.person_id, food_id, language)
}

pub fn set_food_cup_weight(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let food_id = input
        .get("food_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request("set_food_cup_weight takes { food_id, cup_weight_grams }")
        })?;
    if !input
        .get("cup_weight_grams")
        .is_some_and(|v| v.is_number() || v.is_null())
    {
        return Err(OpError::bad_request(
            "set_food_cup_weight takes { food_id, cup_weight_grams }",
        ));
    }
    let cup_weight_grams = input.get("cup_weight_grams").and_then(Value::as_f64);
    let caller = caller_of(invocation)?;
    core.set_food_cup_weight(&caller.person_id, food_id, cup_weight_grams)
}

pub fn list_merge_suggestions(
    core: &Core,
    invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    let caller = caller_of(invocation)?;
    Ok(json!({ "suggestions": core.list_merge_suggestions(&caller.person_id)? }))
}

pub fn preview_food_merge(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let takes = "preview_food_merge takes { survivor_food_id, absorbed_food_id }";
    let survivor_food_id = input
        .get("survivor_food_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let absorbed_food_id = input
        .get("absorbed_food_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let caller = caller_of(invocation)?;
    core.preview_food_merge(&caller.person_id, survivor_food_id, absorbed_food_id)
}

pub fn merge_food(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let takes = "merge_food takes { survivor_food_id, absorbed_food_id, ingredient_lines, \
                 cup_weight_grams? }";
    let survivor_food_id = input
        .get("survivor_food_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let absorbed_food_id = input
        .get("absorbed_food_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    // The figure preview_food_merge announced, said back: a Merge cannot be
    // undone, so it may not be reached without having read what it moves.
    let ingredient_lines = input
        .get("ingredient_lines")
        .and_then(Value::as_i64)
        .ok_or_else(|| OpError::bad_request(takes))?;
    if !input
        .get("cup_weight_grams")
        .is_none_or(|v| v.is_number() || v.is_null())
    {
        return Err(OpError::bad_request(takes));
    }
    let cup_weight_grams = input.get("cup_weight_grams").and_then(Value::as_f64);
    let caller = caller_of(invocation)?;
    core.merge_food(
        &caller.person_id,
        survivor_food_id,
        absorbed_food_id,
        ingredient_lines,
        cup_weight_grams,
    )
}

pub fn delete_food(core: &Core, _invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let food_id = input
        .get("food_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("delete_food takes { food_id }"))?;
    core.delete_food(food_id)?;
    Ok(json!({ "deleted": true }))
}

// ── Meaning Search (#63, ADR 0029) ───────────────────────────────────────────
//
// All six exist on every instance, whether or not that instance has a model.
// The Catalogue is the sole source of what Kamosu can do, and one that changed
// shape per server would make an agent's first question unanswerable.

pub fn meaning_search_status(
    core: &Core,
    invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    core.meaning_search_status(caller_of(invocation)?)
}

pub fn accept_meaning_search_terms(
    core: &Core,
    invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    core.accept_meaning_search_terms(caller_of(invocation)?)
}

pub fn decline_meaning_search(
    core: &Core,
    invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    core.decline_meaning_search(caller_of(invocation)?)
}

pub fn download_meaning_model(
    core: &Core,
    invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    core.download_meaning_model(invocation.job.as_ref())
}

pub fn build_meaning_index(
    core: &Core,
    invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    core.build_meaning_index(invocation.job.as_ref())
}

pub fn read_ingredient_lines(
    core: &Core,
    invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    core.read_ingredient_lines(invocation.job.as_ref())
}

pub fn turn_off_meaning_search(
    core: &Core,
    _invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    core.turn_off_meaning_search()
}

fn caller_of(invocation: &Invocation) -> Result<&Caller, OpError> {
    invocation.caller.as_ref().ok_or_else(|| {
        OpError::unauthorized("this Operation requires a Credential naming a Person")
    })
}
/// A demonstration Job, present only in test builds (`--features test-jobs`).
/// It walks a few progress ticks slowly enough to watch, then finishes — or
/// fails on purpose, so both endings are provable through real Doors.
#[cfg(feature = "test-jobs")]
pub fn probe_job(_core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let steps = input
        .get("steps")
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .clamp(1, 60);
    let delay_ms = input
        .get("delay_ms")
        .and_then(Value::as_u64)
        .unwrap_or(100)
        .clamp(1, 500);
    let fail = input.get("fail").and_then(Value::as_bool).unwrap_or(false);

    let job = invocation
        .job
        .as_ref()
        .ok_or_else(|| OpError::internal("probe_job can only run as a Job"))?;

    for done in 1..=steps {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        job.report(done, Some(steps), format!("tick {done} of {steps}"));
    }

    if fail {
        return Err(OpError::bad_request(
            "the probe was asked to fail, and failed",
        ));
    }

    Ok(json!({ "steps": steps, "message": "finished" }))
}

// ── The Shopping List (#73, ADR 0024) ────────────────────────────────────────
//
// Every one of these answers the whole list. The choosing is what is stored;
// the rows are worked out from it on every read and kept nowhere, so the list
// a change produced is the only honest thing to hand back.

pub fn get_shopping_list(
    core: &Core,
    invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    let caller = caller_of(invocation)?;
    core.shopping_list(&caller.person_id)
}

pub fn shopping_basis(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("shopping_basis takes { branch_id }"))?;
    let caller = caller_of(invocation)?;
    core.shopping_basis(&caller.person_id, branch_id)
}

pub fn shopping_list_as_text(
    core: &Core,
    invocation: &Invocation,
    _input: Value,
) -> Result<Value, OpError> {
    let caller = caller_of(invocation)?;
    core.shopping_list_as_text(&caller.person_id)
}

pub fn empty_shopping_list(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let caller = caller_of(invocation)?;
    core.empty_shopping_list(&caller.person_id, written_at(&input))
}

pub fn add_to_shopping_list(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("add_to_shopping_list takes { branch_id }"))?;
    let caller = caller_of(invocation)?;
    core.add_to_shopping_list(
        &caller.person_id,
        branch_id,
        input.get("shopping_yield"),
        written_at(&input),
    )
}

pub fn remove_from_shopping_list(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("remove_from_shopping_list takes { branch_id }"))?;
    let caller = caller_of(invocation)?;
    core.remove_from_shopping_list(&caller.person_id, branch_id, written_at(&input))
}

pub fn set_shopping_yield(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("set_shopping_yield takes { branch_id }"))?;
    let caller = caller_of(invocation)?;
    core.set_shopping_yield(
        &caller.person_id,
        branch_id,
        input.get("shopping_yield"),
        written_at(&input),
    )
}

pub fn add_loose_item(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let text = input
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("add_loose_item takes { text }"))?;
    let caller = caller_of(invocation)?;
    core.add_loose_item(
        &caller.person_id,
        text,
        input.get("item_id").and_then(Value::as_str),
        written_at(&input),
    )
}

pub fn remove_loose_item(
    core: &Core,
    invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let item_id = input
        .get("item_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("remove_loose_item takes { item_id }"))?;
    let caller = caller_of(invocation)?;
    core.remove_loose_item(&caller.person_id, item_id, written_at(&input))
}
