//! The Operations themselves. Each one is declared in `catalogue.rs`; this module
//! holds the functions those declarations name.

use serde_json::Value;
use serde_json::json;

use crate::core::{Caller, Core, Invocation, OpError};
use crate::jobs;

/// The instance status: the version and whether setup has happened.
///
/// Input: `{}` (declared in the Catalogue). Output: `{ version, setup_complete }`.
pub fn instance_status(
    core: &Core,
    _invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    // The Catalogue declares an empty envelope; hold that line even if a Door
    // somehow let something through.
    match &input {
        Value::Object(map) if !map.is_empty() => {
            return Err(OpError::bad_request("instance_status takes no input"));
        }
        _ => {}
    }

    Ok(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "setup_complete": core.setup_complete()?,
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

pub fn rename_person(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let name = input
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("rename_person takes { name }"))?;
    let caller = caller_of(invocation)?;
    core.rename_person(&caller.person_id, name)?;
    Ok(json!({ "name": name }))
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

pub fn list_sessions(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    if !input.as_object().is_some_and(|map| map.is_empty()) {
        return Err(OpError::bad_request("list_sessions takes no input"));
    }
    let caller = caller_of(invocation)?;
    Ok(json!({ "sessions": core.sessions_of(&caller.person_id)? }))
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
    input: Value,
) -> Result<Value, OpError> {
    if !input.as_object().is_some_and(|map| map.is_empty()) {
        return Err(OpError::bad_request("list_access_keys takes no input"));
    }
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

    let record = core
        .job(job_id)?
        .ok_or_else(|| OpError::not_found(format!("no Job with id '{job_id}'")))?;
    jobs::ensure_reader(&record, &invocation.caller)?;

    Ok(jobs::to_value(&record))
}

/// List the Jobs one Person has asked for, newest first. A stranger has asked
/// for nothing here: anonymous Jobs belong to no Person to list them by.
pub fn list_jobs(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    match &input {
        Value::Object(map) if !map.is_empty() => {
            return Err(OpError::bad_request("list_jobs takes no input"));
        }
        _ => {}
    }

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

    let record = core
        .job(job_id)?
        .ok_or_else(|| OpError::not_found(format!("no Job with id '{job_id}'")))?;
    jobs::ensure_reader(&record, &invocation.caller)?;

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

pub fn list_kitchens(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    if !input.as_object().is_some_and(|map| map.is_empty()) {
        return Err(OpError::bad_request("list_kitchens takes no input"));
    }
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

pub fn create_tag(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let takes = "create_tag takes { kitchen_id, language, name }";
    let kitchen_id = input
        .get("kitchen_id")
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
    core.create_tag(&caller.person_id, kitchen_id, language, name)
}

pub fn list_tags(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let kitchen_id = input
        .get("kitchen_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("list_tags takes { kitchen_id }"))?;
    let caller = caller_of(invocation)?;
    Ok(json!({ "tags": core.list_tags(&caller.person_id, kitchen_id)? }))
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
    let takes = "set_related_recipe takes { branch_id, related_branch_id, related }";
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let related_branch_id = input
        .get("related_branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let related = input
        .get("related")
        .and_then(Value::as_bool)
        .ok_or_else(|| OpError::bad_request(takes))?;
    let caller = caller_of(invocation)?;
    core.set_related_recipe(&caller.person_id, branch_id, related_branch_id, related)
}

pub fn create_recipe(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let kitchen_id = input
        .get("kitchen_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request(
                "create_recipe takes { kitchen_id, title, language?, yield?, prep_time_minutes?, \
             cook_time_minutes?, note?, source?, ingredients?, steps? }",
            )
        })?;
    let language = input.get("language").and_then(Value::as_str);
    let caller = caller_of(invocation)?;
    core.create_recipe(caller, kitchen_id, &input, language)
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
             kitchen_id?, yield?, prep_time_minutes?, cook_time_minutes?, note?, source?, \
             ingredients?, steps? }",
            )
        })?;
    let name = input.get("name").and_then(Value::as_str);
    let change_note = input.get("change_note").and_then(Value::as_str);
    let kitchen_id = input.get("kitchen_id").and_then(Value::as_str);
    let caller = caller_of(invocation)?;
    core.save_recipe_version(caller, branch_id, &input, name, change_note, kitchen_id)
}

/// Import: land a batch of candidates already read from an outside source
/// into the caller's Home Kitchen. Reading the source itself is each
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
    _invocation: &Invocation,
    input: Value,
) -> Result<Value, OpError> {
    let data = input.get("data").and_then(Value::as_str).ok_or_else(|| {
        OpError::bad_request("upload_photograph takes { data } — the picture, base64-encoded")
    })?;
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|e| OpError::bad_request(format!("data is not valid base64: {e}")))?;
    core.store_photograph(&bytes)
}

pub fn get_recipe(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("get_recipe takes { branch_id }"))?;
    let caller = caller_of(invocation)?;
    core.get_recipe(&caller.person_id, branch_id)
}

pub fn set_reading(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request(
                "set_reading takes { branch_id, line_index, amount?, unit?, target? }",
            )
        })?;
    let line_index = input
        .get("line_index")
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            OpError::bad_request(
                "set_reading takes { branch_id, line_index, amount?, unit?, target? }",
            )
        })?;
    let amount = input.get("amount").and_then(Value::as_str);
    let unit = input.get("unit").and_then(Value::as_str);
    let target = input.get("target").and_then(Value::as_str);
    let caller = caller_of(invocation)?;
    core.set_reading(
        &caller.person_id,
        branch_id,
        line_index,
        amount,
        unit,
        target,
    )
}

pub fn start_attempt(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let branch_id = input
        .get("branch_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("start_attempt takes { branch_id }"))?;
    let caller = caller_of(invocation)?;
    core.start_attempt(&caller.person_id, branch_id)
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
        .ok_or_else(|| OpError::bad_request("finish_attempt takes { attempt_id }"))?;
    let caller = caller_of(invocation)?;
    core.finish_attempt(&caller.person_id, attempt_id)
}

pub fn edit_attempt(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    let attempt_id = input
        .get("attempt_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("edit_attempt takes { attempt_id, note?, rating? }"))?;
    let note = input.get("note");
    let rating = input.get("rating");
    let caller = caller_of(invocation)?;
    core.edit_attempt(&caller.person_id, attempt_id, note, rating)
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

pub fn list_foods(core: &Core, invocation: &Invocation, input: Value) -> Result<Value, OpError> {
    if !input.as_object().is_some_and(|map| map.is_empty()) {
        return Err(OpError::bad_request("list_foods takes no input"));
    }
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
