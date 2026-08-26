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
