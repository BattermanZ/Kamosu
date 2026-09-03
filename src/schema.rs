//! The Catalogue's declared input, compiled into the thing that checks it (#85).
//!
//! Every Operation declares an `input_schema`. That declaration is already
//! load-bearing in three directions — the MCP door publishes it to agents, the
//! typed client is generated from it, `just check` fails when the generated
//! copy drifts — and until this module existed it was load-bearing in none of
//! the direction that matters when a call actually arrives.
//!
//! What is checked here is **shape**: unknown fields, missing required fields,
//! wrong types, values outside a declared `enum`, numbers under a declared
//! bound. What is not checked here, and never moves here, is **meaning**:
//! whether a Branch exists, whether a Person may touch it, whether an amount
//! parses. A schema cannot know any of that.
//!
//! The check runs in the Core at dispatch, so both Doors inherit it and a new
//! Operation cannot forget it any more than it can forget to appear at both
//! Doors. A validation check written inside a Door is a bug for the same reason
//! a permission check inside a Door is one: it is the second copy that goes
//! stale.
//!
//! # Why this is not a JSON Schema crate
//!
//! Because **a JSON Schema validator is required by its own specification to
//! ignore a keyword it does not recognise**, so it cannot be the thing that
//! catches a typo in the Catalogue. Checked against `boon` 0.6.1: a schema
//! written `"requird": ["branch_id"]` compiles without complaint and silently
//! drops the requirement — this module's own failure mode, moved from the
//! caller's input to Kamosu's declaration.
//!
//! So [`compile`] refuses every keyword it does not implement, and
//! [`INPUT_SCHEMAS`] compiles all of them at startup. A misspelt keyword, an
//! unknown type name or an `additionalProperties` that is not `false` is a
//! failed build (`tests/parity.rs`), not a check that quietly stopped running.
//! The whole of the Catalogue's input surface is the nine keywords
//! [`IMPLEMENTED`] names, plus the two that only annotate.
//!
//! The full argument, the crates weighed against it and what the measurement
//! before turning enforcement on turned up: ADR 0037.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde_json::Value;

use crate::catalogue;
use crate::core::OpError;

/// The JSON types a declaration may name. Spelt out rather than borrowed from a
/// string so an unknown type name in the Catalogue fails to compile rather than
/// matching nothing at run time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JsonType {
    Null,
    Boolean,
    Integer,
    Number,
    String,
    Array,
    Object,
}

impl JsonType {
    fn parse(name: &str) -> Option<JsonType> {
        Some(match name {
            "null" => JsonType::Null,
            "boolean" => JsonType::Boolean,
            "integer" => JsonType::Integer,
            "number" => JsonType::Number,
            "string" => JsonType::String,
            "array" => JsonType::Array,
            "object" => JsonType::Object,
            _ => return None,
        })
    }

    /// What to call this type in a sentence addressed to whoever called.
    fn describe(self) -> &'static str {
        match self {
            JsonType::Null => "null",
            JsonType::Boolean => "a true or false",
            JsonType::Integer => "a whole number",
            JsonType::Number => "a number",
            JsonType::String => "a string",
            JsonType::Array => "a list",
            JsonType::Object => "an object",
        }
    }

    /// Whether a value is of this type. `integer` accepts a JSON number with no
    /// fractional part, which is what a caller writing `8.0` means and what
    /// every JSON Schema implementation agrees on.
    fn matches(self, value: &Value) -> bool {
        match self {
            JsonType::Null => value.is_null(),
            JsonType::Boolean => value.is_boolean(),
            JsonType::Integer => value.as_f64().is_some_and(|n| n.fract() == 0.0),
            JsonType::Number => value.is_number(),
            JsonType::String => value.is_string(),
            JsonType::Array => value.is_array(),
            JsonType::Object => value.is_object(),
        }
    }
}

/// One declaration, compiled. Every field is what some part of the Catalogue
/// actually declares; nothing here is speculative.
#[derive(Debug)]
pub struct Schema {
    /// The types named by `type`. Empty means the declaration named none, which
    /// is how `finish_attempt`'s `rating` is written — constrained by its `enum`
    /// alone.
    types: Vec<JsonType>,
    /// The values named by `enum`, compared verbatim.
    allowed: Option<Vec<Value>>,
    /// Declared properties, in the Catalogue's own order so a sentence listing
    /// them reads the way the declaration does.
    properties: Vec<(String, Schema)>,
    required: Vec<String>,
    /// Whether the declaration named an object's fields at all. Where it did,
    /// [`compile`] has already insisted on `additionalProperties: false`, so the
    /// list is closed and any other field is refused.
    ///
    /// The insisting is the point. JSON Schema's default for a missing
    /// `additionalProperties` is *permissive*, and unlike a misspelt keyword an
    /// omission looks like nothing at all — so a nested object declared without
    /// it would silently accept unknown fields and break no build. That is this
    /// module's own failure mode on the one keyword it can least afford it, so
    /// the omission is refused instead of defaulted.
    closed: bool,
    /// The schema every element of an array is held to.
    items: Option<Box<Schema>>,
    minimum: Option<f64>,
    maximum: Option<f64>,
    exclusive_minimum: Option<f64>,
}

/// The keywords this module implements, plus the two that only annotate.
///
/// `description` and `default` are carried into the MCP door's published tool
/// list and the generated client; neither constrains a value, and JSON Schema
/// agrees they do not. Every other keyword is a compile error until it is
/// implemented here — which is the point.
const ANNOTATIONS: [&str; 2] = ["description", "default"];
const IMPLEMENTED: [&str; 9] = [
    "type",
    "enum",
    "properties",
    "required",
    "additionalProperties",
    "items",
    "minimum",
    "maximum",
    "exclusiveMinimum",
];

/// Compile one declaration, refusing anything this module does not implement.
///
/// The error is addressed to whoever is writing the Catalogue, not to a caller:
/// it names the path inside the schema and says what to do about it.
pub fn compile(schema: &Value, path: &str) -> Result<Schema, String> {
    let object = schema
        .as_object()
        .ok_or_else(|| format!("{path}: a schema must be an object"))?;

    for key in object.keys() {
        if !IMPLEMENTED.contains(&key.as_str()) && !ANNOTATIONS.contains(&key.as_str()) {
            return Err(format!(
                "{path}: '{key}' is not a keyword src/schema.rs implements. \
                 Implement it there, or declare the input without it — a keyword \
                 silently ignored is the bug #85 closed."
            ));
        }
    }

    let types = match object.get("type") {
        None => Vec::new(),
        Some(Value::String(name)) => vec![parse_type(name, path)?],
        Some(Value::Array(names)) => names
            .iter()
            .map(|name| {
                name.as_str()
                    .ok_or_else(|| format!("{path}: every entry in 'type' must be a string"))
                    .and_then(|name| parse_type(name, path))
            })
            .collect::<Result<Vec<_>, _>>()?,
        Some(_) => return Err(format!("{path}: 'type' must be a string or a list of them")),
    };

    let allowed = match object.get("enum") {
        None => None,
        Some(Value::Array(values)) if !values.is_empty() => Some(values.clone()),
        Some(_) => return Err(format!("{path}: 'enum' must be a non-empty list")),
    };

    let mut properties = Vec::new();
    if let Some(declared) = object.get("properties") {
        let declared = declared
            .as_object()
            .ok_or_else(|| format!("{path}: 'properties' must be an object"))?;
        for (name, sub) in declared {
            properties.push((name.clone(), compile(sub, &join(path, name))?));
        }
    }

    let required = match object.get("required") {
        None => Vec::new(),
        Some(Value::Array(names)) => names
            .iter()
            .map(|name| {
                name.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| format!("{path}: every entry in 'required' must be a string"))
            })
            .collect::<Result<Vec<_>, _>>()?,
        Some(_) => return Err(format!("{path}: 'required' must be a list of field names")),
    };
    // A required field nobody declared is a typo in one of the two lists, and
    // the schema below it would never be applied. Only worth checking where
    // properties are declared at all: `empty_input()` declares none and
    // requires none.
    if object.contains_key("properties") {
        for name in &required {
            if !properties.iter().any(|(declared, _)| declared == name) {
                return Err(format!(
                    "{path}: '{name}' is required but never declared in 'properties'"
                ));
            }
        }
    }

    // An object must say what it holds, and must close the list. Neither half
    // may be left to a default: JSON Schema's default here is permissive, and a
    // missing keyword is invisible in a way a misspelt one is not.
    let closed = object.contains_key("properties");
    if !closed && types.contains(&JsonType::Object) {
        return Err(format!(
            "{path}: an object must declare its 'properties'. Left out, it \
             accepts anything — which is what #85 stopped Kamosu doing."
        ));
    }
    match object.get("additionalProperties") {
        Some(Value::Bool(false)) => {}
        None if !closed => {}
        None => {
            return Err(format!(
                "{path}: declares 'properties' but not \
                 `\"additionalProperties\": false`. Without it a field nobody \
                 declared is accepted, silently and with no failed build — the \
                 bug #85 closed."
            ));
        }
        Some(_) => {
            return Err(format!(
                "{path}: 'additionalProperties' may only be declared `false`. \
                 Every declaration in the Catalogue writes that; a schema here \
                 would be read as permission by a validator that does not \
                 implement it."
            ));
        }
    }

    let items = match object.get("items") {
        None => None,
        Some(sub) => Some(Box::new(compile(sub, &format!("{path}[]"))?)),
    };

    let minimum = number_keyword(object.get("minimum"), "minimum", path)?;
    let maximum = number_keyword(object.get("maximum"), "maximum", path)?;
    let exclusive_minimum =
        number_keyword(object.get("exclusiveMinimum"), "exclusiveMinimum", path)?;

    Ok(Schema {
        types,
        allowed,
        properties,
        required,
        closed,
        items,
        minimum,
        maximum,
        exclusive_minimum,
    })
}

fn parse_type(name: &str, path: &str) -> Result<JsonType, String> {
    JsonType::parse(name).ok_or_else(|| format!("{path}: '{name}' is not a JSON type"))
}

fn number_keyword(value: Option<&Value>, keyword: &str, path: &str) -> Result<Option<f64>, String> {
    match value {
        None => Ok(None),
        Some(value) => value
            .as_f64()
            .map(Some)
            .ok_or_else(|| format!("{path}: '{keyword}' must be a number")),
    }
}

/// Where a field sits, written the way a caller wrote it: `candidates[0].kind`.
fn join(path: &str, field: &str) -> String {
    if path.is_empty() {
        field.to_string()
    } else {
        format!("{path}.{field}")
    }
}

impl Schema {
    /// Hold one value to this declaration. The first thing wrong is the whole
    /// answer: a caller fixes one field and asks again, and a list of six
    /// complaints about an envelope whose first field was misspelt is noise.
    ///
    /// `operation` names the Operation in every sentence, because at the MCP
    /// door the error arrives detached from the call that caused it.
    fn check(&self, operation: &str, value: &Value, path: &str) -> Result<(), String> {
        if !self.types.is_empty() && !self.types.iter().any(|t| t.matches(value)) {
            return Err(format!(
                "{operation}: {} must be {}",
                at(path),
                or_list(&self.types.iter().map(|t| t.describe()).collect::<Vec<_>>()),
            ));
        }

        if let Some(allowed) = &self.allowed
            && !allowed.contains(value)
        {
            return Err(format!(
                "{operation}: {} must be one of {}",
                at(path),
                or_list(&allowed.iter().map(render).collect::<Vec<_>>()),
            ));
        }

        if let Some(object) = value.as_object() {
            if self.closed {
                for name in object.keys() {
                    if !self.properties.iter().any(|(declared, _)| declared == name) {
                        return Err(self.undeclared(operation, path, name));
                    }
                }
            }
            for name in &self.required {
                if !object.contains_key(name) {
                    return Err(format!(
                        "{operation} requires the field '{}'",
                        join(path, name)
                    ));
                }
            }
            for (name, sub) in &self.properties {
                if let Some(field) = object.get(name) {
                    sub.check(operation, field, &join(path, name))?;
                }
            }
        }

        if let (Some(items), Some(list)) = (&self.items, value.as_array()) {
            for (index, element) in list.iter().enumerate() {
                items.check(operation, element, &format!("{path}[{index}]"))?;
            }
        }

        if let Some(number) = value.as_f64() {
            if let Some(minimum) = self.minimum
                && number < minimum
            {
                return Err(format!(
                    "{operation}: {} must be at least {minimum}",
                    at(path)
                ));
            }
            if let Some(maximum) = self.maximum
                && number > maximum
            {
                return Err(format!(
                    "{operation}: {} must be at most {maximum}",
                    at(path)
                ));
            }
            if let Some(bound) = self.exclusive_minimum
                && number <= bound
            {
                return Err(format!(
                    "{operation}: {} must be greater than {bound}",
                    at(path)
                ));
            }
        }

        Ok(())
    }

    /// The sentence for a field nobody declared — the one this whole module was
    /// built for. It names the offending field and then says what the Operation
    /// does take, because the commonest cause is a misspelling and the answer is
    /// usually visible in that list.
    fn undeclared(&self, operation: &str, path: &str, name: &str) -> String {
        let offender = join(path, name);
        if self.properties.is_empty() {
            return format!("{operation} takes no input, and was given '{offender}'");
        }
        let declared: Vec<&str> = self
            .properties
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        format!(
            "{operation} does not take a field named '{offender}' — {} takes {{ {} }}",
            if path.is_empty() { "it" } else { path },
            declared.join(", "),
        )
    }
}

/// How to refer to a place inside the envelope. The root is the envelope itself.
fn at(path: &str) -> String {
    if path.is_empty() {
        "its input".to_string()
    } else {
        format!("'{path}'")
    }
}

/// A JSON value as a caller would recognise it in an error message.
fn render(value: &Value) -> String {
    match value {
        Value::String(text) => format!("\"{text}\""),
        other => other.to_string(),
    }
}

/// `a`, `a or b`, `a, b or c` — the Oxford-free form Kamosu's other sentences use.
fn or_list(parts: &[impl AsRef<str>]) -> String {
    match parts {
        [] => String::new(),
        [only] => only.as_ref().to_string(),
        [rest @ .., last] => format!(
            "{} or {}",
            rest.iter()
                .map(AsRef::as_ref)
                .collect::<Vec<_>>()
                .join(", "),
            last.as_ref()
        ),
    }
}

/// Every Operation's input declaration, compiled once.
///
/// Compiled at first dispatch rather than per request, next to where the
/// Catalogue is already walked to build both Doors. A declaration this module
/// cannot compile panics here — and `tests/parity.rs` reaches it first, so the
/// build breaks before any instance can.
pub static INPUT_SCHEMAS: LazyLock<HashMap<&'static str, Schema>> = LazyLock::new(|| {
    catalogue::OPERATIONS
        .iter()
        .map(|op| {
            let compiled = compile(&op.input_schema, "").unwrap_or_else(|reason| {
                panic!("{}'s declared input does not compile: {reason}", op.name)
            });
            (op.name, compiled)
        })
        .collect()
});

/// Hold one Operation's input to what the Catalogue declares it takes.
///
/// Called by `Core::execute` for every Operation, at both Doors, before any
/// handler runs.
pub fn validate_input(operation: &str, input: &Value) -> Result<(), OpError> {
    let Some(schema) = INPUT_SCHEMAS.get(operation) else {
        // Unreachable through `execute`, which looks the Operation up first.
        return Ok(());
    };
    schema
        .check(operation, input, "")
        .map_err(OpError::bad_request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn check(schema: Value, input: Value) -> Result<(), String> {
        compile(&schema, "")?.check("an_operation", &input, "")
    }

    #[test]
    fn an_undeclared_field_is_named_in_the_refusal() {
        let refusal = check(
            json!({ "type": "object", "properties": { "branch_id": { "type": "string" } }, "required": ["branch_id"], "additionalProperties": false }),
            json!({ "branch_id": "b_1", "shopping_yield": { "amount": "8" } }),
        )
        .expect_err("an undeclared field is refused");
        assert!(refusal.contains("shopping_yield"), "{refusal}");
        assert!(refusal.contains("an_operation"), "{refusal}");
    }

    #[test]
    fn an_operation_taking_no_input_says_so() {
        let refusal = check(
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
            json!({ "anything": 1 }),
        )
        .expect_err("no input means no input");
        assert!(refusal.contains("takes no input"), "{refusal}");
        assert!(refusal.contains("anything"), "{refusal}");
        assert!(
            check(
                json!({ "type": "object", "properties": {}, "additionalProperties": false }),
                json!({}),
            )
            .is_ok()
        );
    }

    #[test]
    fn a_missing_required_field_a_wrong_type_and_a_bad_enum_are_each_refused() {
        let schema = json!({
            "type": "object",
            "properties": {
                "branch_id": { "type": "string" },
                "rating": { "enum": ["again", "tweak", null] },
                "steps": { "type": "integer", "minimum": 0 },
            },
            "required": ["branch_id"],
            "additionalProperties": false,
        });
        assert!(
            check(schema.clone(), json!({}))
                .expect_err("missing required")
                .contains("branch_id")
        );
        assert!(
            check(schema.clone(), json!({ "branch_id": 7 }))
                .expect_err("wrong type")
                .contains("must be a string")
        );
        assert!(
            check(
                schema.clone(),
                json!({ "branch_id": "b", "rating": "lovely" })
            )
            .expect_err("outside the enum")
            .contains("must be one of")
        );
        assert!(
            check(schema.clone(), json!({ "branch_id": "b", "steps": -1 }))
                .expect_err("under the minimum")
                .contains("at least 0")
        );
        assert!(
            check(
                schema,
                json!({ "branch_id": "b", "rating": null, "steps": 2 })
            )
            .is_ok()
        );
    }

    #[test]
    fn nesting_names_the_path_a_caller_would_recognise() {
        let schema = json!({
            "type": "object",
            "properties": {
                "candidates": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": { "kind": { "enum": ["section", "step"] } },
                        "additionalProperties": false,
                    },
                },
            },
            "additionalProperties": false,
        });
        let refusal = check(
            schema.clone(),
            json!({ "candidates": [{ "kind": "step" }, { "knid": "step" }] }),
        )
        .expect_err("an undeclared field inside a list element");
        assert!(refusal.contains("candidates[1].knid"), "{refusal}");
        assert!(
            check(schema, json!({ "candidates": [{ "kind": "nope" }] }))
                .expect_err("a bad enum inside a list element")
                .contains("candidates[0].kind")
        );
    }

    #[test]
    fn a_nullable_field_still_refuses_the_wrong_shape() {
        let schema = json!({
            "type": "object",
            "properties": { "shopping_yield": { "type": ["object", "null"], "properties": { "amount": { "type": "string" } }, "required": ["amount"], "additionalProperties": false } },
            "additionalProperties": false,
        });
        assert!(check(schema.clone(), json!({ "shopping_yield": null })).is_ok());
        assert!(
            check(
                schema.clone(),
                json!({ "shopping_yield": { "amount": "8" } })
            )
            .is_ok()
        );
        assert!(check(schema, json!({ "shopping_yield": { "amount": 8 } })).is_err());
    }

    #[test]
    fn a_keyword_this_module_does_not_implement_fails_to_compile() {
        // The exact drift a spec-compliant JSON Schema validator must ignore.
        let reason = compile(
            &json!({ "type": "object", "properties": { "a": { "type": "string" } }, "requird": ["a"], "additionalProperties": false }),
            "",
        )
        .expect_err("a misspelt keyword is a failed build");
        assert!(reason.contains("requird"), "{reason}");

        assert!(compile(&json!({ "type": "objct" }), "").is_err());
        assert!(
            compile(
                &json!({ "type": "object", "properties": {}, "additionalProperties": { "type": "string" } }),
                ""
            )
            .is_err()
        );
        // The omission that would otherwise look like nothing at all: JSON
        // Schema defaults it to permissive, so it must be refused rather than
        // defaulted.
        assert!(
            compile(
                &json!({ "type": "object", "properties": { "a": { "type": "string" } } }),
                ""
            )
            .expect_err("an object that does not close its field list")
            .contains("additionalProperties")
        );
        assert!(
            compile(&json!({ "type": "object" }), "")
                .expect_err("an object that never says what it holds")
                .contains("properties")
        );
        assert!(
            compile(
                &json!({ "type": "object", "properties": { "a": { "type": "string" } }, "required": ["b"] }),
                ""
            )
            .expect_err("required names a field nobody declared")
            .contains('b')
        );
    }

    #[test]
    fn an_integer_accepts_a_whole_number_written_as_a_decimal() {
        assert!(check(json!({ "type": "integer" }), json!(8.0)).is_ok());
        assert!(check(json!({ "type": "integer" }), json!(8.5)).is_err());
    }
}
