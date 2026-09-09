//! SCIM patch operations ([RFC 7644, section 3.5.2]).
//!
//! Supports `add`, `replace`, and `remove` operations targeting attribute
//! paths with optional sub-attributes and value filters, e.g.
//! `members[value eq "2819c223-..."]`.
//!
//! [RFC 7644, section 3.5.2]: https://datatracker.ietf.org/doc/html/rfc7644#section-3.5.2

use crate::error::ScimError;
use crate::filter::{parse_filter, Filter};
use crate::schema::PATCH_OP_URN;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The `op` of a [`PatchOperation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    /// `add` — add or merge a value at the target path.
    Add,
    /// `remove` — remove the value at the target path.
    Remove,
    /// `replace` — overwrite the value at the target path.
    Replace,
}

/// A single operation within a [`PatchOp`] request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchOperation {
    /// The operation kind.
    pub op: Operation,
    /// Target attribute path, optionally narrowed by a value filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// The value to add or replace with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
}

/// A SCIM patch request (`urn:ietf:params:scim:api:messages:2.0:PatchOp`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchOp {
    /// Schema URIs; defaults to [`PATCH_OP_URN`].
    #[serde(rename = "schemas", default = "patch_schemas_default")]
    pub schemas: Vec<String>,
    /// The operations, applied in order.
    #[serde(rename = "Operations")]
    pub operations: Vec<PatchOperation>,
}

fn patch_schemas_default() -> Vec<String> {
    vec![PATCH_OP_URN.to_string()]
}

impl PatchOp {
    /// Builds a patch request with the default schema URN.
    pub fn new(operations: Vec<PatchOperation>) -> Self {
        Self {
            schemas: patch_schemas_default(),
            operations,
        }
    }

    /// Applies this patch request to a JSON document in place.
    ///
    /// # Errors
    ///
    /// Returns [`ScimError::BadRequest`] for malformed paths, missing
    /// values, or targets of the wrong shape.
    pub fn apply_to(&self, doc: &mut Value) -> Result<(), ScimError> {
        apply_patch(doc, self)
    }
}

/// Applies a [`PatchOp`] to a JSON document in place.
///
/// # Errors
///
/// Returns [`ScimError::BadRequest`] for malformed paths, missing values,
/// or targets of the wrong shape.
pub fn apply_patch(doc: &mut Value, patch: &PatchOp) -> Result<(), ScimError> {
    if !doc.is_object() {
        return Err(bad("patch target must be a JSON object"));
    }
    for operation in &patch.operations {
        apply_operation(doc, operation)?;
    }
    Ok(())
}

fn bad(msg: &str) -> ScimError {
    ScimError::BadRequest(msg.to_string())
}

fn apply_operation(doc: &mut Value, operation: &PatchOperation) -> Result<(), ScimError> {
    match operation.op {
        Operation::Add | Operation::Replace => {
            let Some(value) = &operation.value else {
                return Err(bad("add/replace operation requires a value"));
            };
            match &operation.path {
                None => merge_into(doc, value),
                Some(path) => {
                    let segments = parse_path(path)?;
                    apply_at(doc, &segments, value, mode_of(operation.op))
                }
            }
        }
        Operation::Remove => {
            let Some(path) = &operation.path else {
                return Err(bad("remove operation requires a path"));
            };
            let segments = parse_path(path)?;
            apply_at(doc, &segments, &Value::Null, Mode::Remove)
        }
    }
}

fn mode_of(op: Operation) -> Mode {
    match op {
        Operation::Add => Mode::Add,
        Operation::Remove => Mode::Remove,
        Operation::Replace => Mode::Replace,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Add,
    Replace,
    Remove,
}

#[derive(Debug, Clone)]
struct PathSegment {
    attr: String,
    filter: Option<Filter>,
}

fn parse_path(path: &str) -> Result<Vec<PathSegment>, ScimError> {
    let mut raw_segments = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    for c in path.chars() {
        match c {
            '[' => {
                depth += 1;
                current.push(c);
            }
            ']' => {
                if depth == 0 {
                    return Err(bad("unbalanced brackets in path"));
                }
                depth -= 1;
                current.push(c);
            }
            '.' if depth == 0 => {
                raw_segments.push(std::mem::take(&mut current));
            }
            _ => current.push(c),
        }
    }
    if depth != 0 {
        return Err(bad("unbalanced brackets in path"));
    }
    raw_segments.push(current);

    raw_segments
        .into_iter()
        .map(|segment| match segment.find('[') {
            None => {
                let attr = segment.trim().to_string();
                if attr.is_empty() {
                    return Err(bad("empty attribute in path"));
                }
                Ok(PathSegment { attr, filter: None })
            }
            Some(index) => {
                let attr = segment[..index].trim().to_string();
                if attr.is_empty() || !segment.ends_with(']') {
                    return Err(bad(&format!("invalid path segment {segment:?}")));
                }
                let inner = &segment[index + 1..segment.len() - 1];
                let filter = parse_filter(inner)
                    .map_err(|e| ScimError::BadRequest(format!("invalid path filter: {e}")))?;
                Ok(PathSegment {
                    attr,
                    filter: Some(filter),
                })
            }
        })
        .collect()
}

fn apply_at(
    target: &mut Value,
    segments: &[PathSegment],
    value: &Value,
    mode: Mode,
) -> Result<(), ScimError> {
    let Some((segment, rest)) = segments.split_first() else {
        return Err(bad("empty path"));
    };
    let Some(map) = target.as_object_mut() else {
        return Err(bad("path traverses a non-object value"));
    };
    match rest {
        [] => apply_leaf(map, segment, value, mode),
        _ => {
            if let Some(filter) = &segment.filter {
                let Some(array) = map.get_mut(&segment.attr).and_then(Value::as_array_mut) else {
                    return Err(bad("filtered path must target an array"));
                };
                for element in array.iter_mut() {
                    if filter.matches(element) {
                        apply_at(element, rest, value, mode)?;
                    }
                }
            } else {
                match mode {
                    Mode::Remove => {
                        if let Some(entry) = map.get_mut(&segment.attr) {
                            apply_at(entry, rest, value, mode)?;
                        }
                    }
                    _ => {
                        let entry = map
                            .entry(segment.attr.clone())
                            .or_insert_with(|| Value::Object(Map::new()));
                        apply_at(entry, rest, value, mode)?;
                    }
                }
            }
            Ok(())
        }
    }
}

fn apply_leaf(
    map: &mut Map<String, Value>,
    segment: &PathSegment,
    value: &Value,
    mode: Mode,
) -> Result<(), ScimError> {
    if let Some(filter) = &segment.filter {
        let Some(array) = map.get_mut(&segment.attr).and_then(Value::as_array_mut) else {
            return Err(bad("filtered path must target an array"));
        };
        match mode {
            Mode::Remove => array.retain(|element| !filter.matches(element)),
            Mode::Add => {
                for element in array.iter_mut() {
                    if filter.matches(element) {
                        merge_into(element, value)?;
                    }
                }
            }
            Mode::Replace => {
                for element in array.iter_mut() {
                    if filter.matches(element) {
                        *element = value.clone();
                    }
                }
            }
        }
        return Ok(());
    }
    match mode {
        Mode::Add => match map.get_mut(&segment.attr) {
            Some(Value::Array(array)) => match value {
                Value::Array(values) => array.extend(values.iter().cloned()),
                v => array.push(v.clone()),
            },
            Some(Value::Object(_)) if value.is_object() => {
                let existing = map.get_mut(&segment.attr).expect("checked above");
                merge_into(existing, value)?;
            }
            _ => {
                map.insert(segment.attr.clone(), value.clone());
            }
        },
        Mode::Replace => {
            map.insert(segment.attr.clone(), value.clone());
        }
        Mode::Remove => {
            map.remove(&segment.attr);
        }
    }
    Ok(())
}

fn merge_into(target: &mut Value, value: &Value) -> Result<(), ScimError> {
    match (target, value) {
        (Value::Object(dst), Value::Object(src)) => {
            for (k, v) in src {
                dst.insert(k.clone(), v.clone());
            }
            Ok(())
        }
        (target, Value::Object(_)) => {
            *target = value.clone();
            Ok(())
        }
        _ => Err(bad("add value must be a JSON object")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group_doc() -> Value {
        serde_json::json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
            "displayName": "Developers",
            "members": [
                { "value": "2819c223-7f76-453a-919d-413861904646", "display": "Babs" },
                { "value": "9e2c1f62-99ab-4c8f-a0f5-3f1c64d53b53", "display": "Jay" }
            ]
        })
    }

    #[test]
    fn test_deserialize_rfc7644_example() {
        let json = r#"{
            "schemas": ["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
            "Operations": [{
                "op": "remove",
                "path": "members[value eq \"2819c223-7f76-453a-919d-413861904646\"]"
            }]
        }"#;
        let patch: PatchOp = serde_json::from_str(json).unwrap();
        assert_eq!(patch.schemas, vec![PATCH_OP_URN]);
        assert_eq!(patch.operations.len(), 1);
        assert_eq!(patch.operations[0].op, Operation::Remove);
    }

    #[test]
    fn test_default_schemas() {
        let patch = PatchOp::new(vec![PatchOperation {
            op: Operation::Add,
            path: Some("active".into()),
            value: Some(Value::Bool(true)),
        }]);
        assert_eq!(patch.schemas, vec![PATCH_OP_URN]);
    }

    #[test]
    fn test_add_simple() {
        let mut doc = serde_json::json!({});
        PatchOp::new(vec![PatchOperation {
            op: Operation::Add,
            path: Some("active".into()),
            value: Some(Value::Bool(true)),
        }])
        .apply_to(&mut doc)
        .unwrap();
        assert_eq!(doc["active"], Value::Bool(true));
    }

    #[test]
    fn test_add_nested_creates_intermediates() {
        let mut doc = serde_json::json!({});
        PatchOp::new(vec![PatchOperation {
            op: Operation::Add,
            path: Some("name.givenName".into()),
            value: Some(Value::String("Barbara".into())),
        }])
        .apply_to(&mut doc)
        .unwrap();
        assert_eq!(doc["name"]["givenName"], "Barbara");
    }

    #[test]
    fn test_add_merges_into_object() {
        let mut doc = serde_json::json!({ "name": { "givenName": "B" } });
        PatchOp::new(vec![PatchOperation {
            op: Operation::Add,
            path: Some("name".into()),
            value: Some(serde_json::json!({ "familyName": "J" })),
        }])
        .apply_to(&mut doc)
        .unwrap();
        assert_eq!(doc["name"]["givenName"], "B");
        assert_eq!(doc["name"]["familyName"], "J");
    }

    #[test]
    fn test_add_appends_to_array() {
        let mut doc = serde_json::json!({ "emails": [{ "value": "a@x.org" }] });
        PatchOp::new(vec![PatchOperation {
            op: Operation::Add,
            path: Some("emails".into()),
            value: Some(serde_json::json!({ "value": "b@x.org" })),
        }])
        .apply_to(&mut doc)
        .unwrap();
        assert_eq!(doc["emails"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_replace_simple() {
        let mut doc = serde_json::json!({ "active": true });
        PatchOp::new(vec![PatchOperation {
            op: Operation::Replace,
            path: Some("active".into()),
            value: Some(Value::Bool(false)),
        }])
        .apply_to(&mut doc)
        .unwrap();
        assert_eq!(doc["active"], Value::Bool(false));
    }

    #[test]
    fn test_remove_simple() {
        let mut doc = serde_json::json!({ "nickname": "babs" });
        PatchOp::new(vec![PatchOperation {
            op: Operation::Remove,
            path: Some("nickname".into()),
            value: None,
        }])
        .apply_to(&mut doc)
        .unwrap();
        assert!(doc.get("nickname").is_none());
    }

    #[test]
    fn test_remove_missing_is_noop() {
        let mut doc = serde_json::json!({});
        PatchOp::new(vec![PatchOperation {
            op: Operation::Remove,
            path: Some("nickname.x".into()),
            value: None,
        }])
        .apply_to(&mut doc)
        .unwrap();
    }

    #[test]
    fn test_remove_with_value_filter() {
        let mut doc = group_doc();
        PatchOp::new(vec![PatchOperation {
            op: Operation::Remove,
            path: Some("members[value eq \"2819c223-7f76-453a-919d-413861904646\"]".into()),
            value: None,
        }])
        .apply_to(&mut doc)
        .unwrap();
        let members = doc["members"].as_array().unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0]["value"], "9e2c1f62-99ab-4c8f-a0f5-3f1c64d53b53");
    }

    #[test]
    fn test_add_with_value_filter_merges() {
        let mut doc = group_doc();
        PatchOp::new(vec![PatchOperation {
            op: Operation::Add,
            path: Some("members[value eq \"2819c223-7f76-453a-919d-413861904646\"]".into()),
            value: Some(serde_json::json!({ "type": "direct" })),
        }])
        .apply_to(&mut doc)
        .unwrap();
        assert_eq!(doc["members"][0]["type"], "direct");
        assert_eq!(doc["members"][0]["display"], "Babs");
    }

    #[test]
    fn test_replace_with_value_filter() {
        let mut doc = group_doc();
        PatchOp::new(vec![PatchOperation {
            op: Operation::Replace,
            path: Some("members[value eq \"2819c223-7f76-453a-919d-413861904646\"]".into()),
            value: Some(serde_json::json!({ "value": "x" })),
        }])
        .apply_to(&mut doc)
        .unwrap();
        assert_eq!(doc["members"][0]["value"], "x");
    }

    #[test]
    fn test_add_requires_value() {
        let mut doc = serde_json::json!({});
        let err = PatchOp::new(vec![PatchOperation {
            op: Operation::Add,
            path: Some("active".into()),
            value: None,
        }])
        .apply_to(&mut doc)
        .unwrap_err();
        assert!(matches!(err, ScimError::BadRequest(_)));
    }

    #[test]
    fn test_remove_requires_path() {
        let mut doc = serde_json::json!({});
        let err = PatchOp::new(vec![PatchOperation {
            op: Operation::Remove,
            path: None,
            value: None,
        }])
        .apply_to(&mut doc)
        .unwrap_err();
        assert!(matches!(err, ScimError::BadRequest(_)));
    }

    #[test]
    fn test_invalid_path_filter() {
        let mut doc = serde_json::json!({ "members": [] });
        let err = PatchOp::new(vec![PatchOperation {
            op: Operation::Remove,
            path: Some("members[value eq]".into()),
            value: None,
        }])
        .apply_to(&mut doc)
        .unwrap_err();
        assert!(matches!(err, ScimError::BadRequest(_)));
    }

    #[test]
    fn test_apply_to_non_object_fails() {
        let mut doc = Value::Array(vec![]);
        let err = PatchOp::new(vec![PatchOperation {
            op: Operation::Add,
            path: Some("a".into()),
            value: Some(Value::Bool(true)),
        }])
        .apply_to(&mut doc)
        .unwrap_err();
        assert!(matches!(err, ScimError::BadRequest(_)));
    }
}
