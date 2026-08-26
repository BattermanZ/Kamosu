//! The Operations themselves. Each one is declared in `catalogue.rs`; this module
//! holds the functions those declarations name.

use serde_json::Value;
use serde_json::json;

use crate::core::{Caller, Core, OpError};

/// The one Operation the walking skeleton ships: an instance status saying the
/// version and whether setup has happened.
///
/// Input: `{}` (declared in the Catalogue). Output: `{ version, setup_complete }`.
pub fn instance_status(
    _core: &Core,
    _caller: &Option<Caller>,
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
        "setup_complete": _core.setup_complete()?,
    }))
}
