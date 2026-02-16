//! condition parser - converts JSON to condition AST
//!
//! supports:
//! - logical operators: all, any, not
//! - comparison operators: ==, !=, >, >=, <, <= (multiple forms)
//! - set operator: in
//! - implicit AND when multiple fields in one object
//! - $ref for referencing named conditions

use std::collections::HashMap;

use serde_json::Value as JsonValue;

use super::types::{CompareOp, Condition, FieldCondition, Value};

/// error type for parsing conditions
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub path: String,
}

impl ParseError {
    pub fn new(message: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            path: path.into(),
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.path.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{}: {}", self.path, self.message)
        }
    }
}

impl std::error::Error for ParseError {}

/// type alias for condition definitions map
pub type ConditionDefinitions = HashMap<String, Condition>;

/// parse a JSON value into a condition AST
///
/// # Arguments
/// * `json` - the JSON value to parse
/// * `definitions` - named condition definitions for $ref resolution
///
/// # Returns
/// * `Ok(Condition)` - the parsed condition
/// * `Err(ParseError)` - if parsing fails
pub fn parse_condition(
    json: &JsonValue,
    definitions: &ConditionDefinitions,
) -> Result<Condition, ParseError> {
    parse_condition_internal(json, definitions, "")
}

fn parse_condition_internal(
    json: &JsonValue,
    definitions: &ConditionDefinitions,
    path: &str,
) -> Result<Condition, ParseError> {
    match json {
        JsonValue::Object(obj) => parse_object(obj, definitions, path),
        JsonValue::Bool(b) => {
            // bare boolean: true = always, false = never
            if *b {
                Ok(Condition::All(vec![])) // empty AND = true
            } else {
                Ok(Condition::Any(vec![])) // empty OR = false
            }
        }
        JsonValue::String(s) => {
            // bare string could be a $ref shorthand
            if let Some(name) = s.strip_prefix('$') {
                resolve_ref(name, definitions, path)
            } else {
                Err(ParseError::new(
                    format!("unexpected string value: {}", s),
                    path,
                ))
            }
        }
        _ => Err(ParseError::new(
            format!("expected object, got {:?}", json),
            path,
        )),
    }
}

fn parse_object(
    obj: &serde_json::Map<String, JsonValue>,
    definitions: &ConditionDefinitions,
    path: &str,
) -> Result<Condition, ParseError> {
    // check for logical operators first
    if let Some(value) = obj.get("all") {
        return parse_all(value, definitions, path);
    }
    if let Some(value) = obj.get("any") {
        return parse_any(value, definitions, path);
    }
    if let Some(value) = obj.get("not") {
        return parse_not(value, definitions, path);
    }
    if let Some(value) = obj.get("$ref") {
        return parse_ref(value, definitions, path);
    }

    // multiple fields = implicit AND
    let mut conditions = Vec::new();

    for (key, value) in obj {
        let field_path = if path.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", path, key)
        };

        // skip logical operators (already handled)
        if key == "all" || key == "any" || key == "not" || key == "$ref" {
            continue;
        }

        let condition = parse_field_condition(key, value, definitions, &field_path)?;
        conditions.push(condition);
    }

    match conditions.len() {
        0 => Ok(Condition::All(vec![])), // empty object = true
        1 => Ok(conditions.remove(0)),
        _ => Ok(Condition::All(conditions)),
    }
}

fn parse_all(
    value: &JsonValue,
    definitions: &ConditionDefinitions,
    path: &str,
) -> Result<Condition, ParseError> {
    let arr = value
        .as_array()
        .ok_or_else(|| ParseError::new("'all' must be an array", path))?;

    let conditions: Result<Vec<Condition>, ParseError> = arr
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let item_path = format!("{}all[{}]", if path.is_empty() { "" } else { "." }, i);
            parse_condition_internal(v, definitions, &item_path)
        })
        .collect();

    Ok(Condition::All(conditions?))
}

fn parse_any(
    value: &JsonValue,
    definitions: &ConditionDefinitions,
    path: &str,
) -> Result<Condition, ParseError> {
    let arr = value
        .as_array()
        .ok_or_else(|| ParseError::new("'any' must be an array", path))?;

    let conditions: Result<Vec<Condition>, ParseError> = arr
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let item_path = format!("{}any[{}]", if path.is_empty() { "" } else { "." }, i);
            parse_condition_internal(v, definitions, &item_path)
        })
        .collect();

    Ok(Condition::Any(conditions?))
}

fn parse_not(
    value: &JsonValue,
    definitions: &ConditionDefinitions,
    path: &str,
) -> Result<Condition, ParseError> {
    let inner = parse_condition_internal(value, definitions, &format!("{}.not", path))?;
    Ok(Condition::Not(Box::new(inner)))
}

fn parse_ref(
    value: &JsonValue,
    definitions: &ConditionDefinitions,
    path: &str,
) -> Result<Condition, ParseError> {
    let name = value
        .as_str()
        .ok_or_else(|| ParseError::new("'$ref' must be a string", path))?;

    resolve_ref(name, definitions, path)
}

fn resolve_ref(
    name: &str,
    definitions: &ConditionDefinitions,
    path: &str,
) -> Result<Condition, ParseError> {
    definitions
        .get(name)
        .cloned()
        .ok_or_else(|| ParseError::new(format!("undefined condition reference: '{}'", name), path))
}

fn parse_field_condition(
    field: &str,
    value: &JsonValue,
    definitions: &ConditionDefinitions,
    path: &str,
) -> Result<Condition, ParseError> {
    match value {
        // simple value: { "app": "Firefox" } or { "display.count": 2 }
        JsonValue::String(s) => Ok(Condition::Field(FieldCondition::eq(
            field,
            Value::String(s.clone()),
        ))),
        JsonValue::Number(n) => {
            let val = if let Some(i) = n.as_i64() {
                Value::Number(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                return Err(ParseError::new("invalid number", path));
            };
            Ok(Condition::Field(FieldCondition::eq(field, val)))
        }
        JsonValue::Bool(b) => Ok(Condition::Field(FieldCondition::eq(field, Value::Bool(*b)))),

        // object value: { "display.count": { ">=": 2 } } or { "app": { "in": [...] } }
        JsonValue::Object(obj) => parse_field_with_operators(field, obj, definitions, path),

        // array value: shorthand for 'in'
        JsonValue::Array(arr) => {
            let values = parse_value_array(arr, path)?;
            Ok(Condition::Field(FieldCondition::is_in(field, values)))
        }

        JsonValue::Null => Err(ParseError::new("null values not supported", path)),
    }
}

fn parse_field_with_operators(
    field: &str,
    obj: &serde_json::Map<String, JsonValue>,
    _definitions: &ConditionDefinitions,
    path: &str,
) -> Result<Condition, ParseError> {
    let mut conditions = Vec::new();

    for (op_str, value) in obj {
        let op = CompareOp::parse(op_str)
            .ok_or_else(|| ParseError::new(format!("unknown operator: '{}'", op_str), path))?;

        let val = parse_value(value, path)?;
        conditions.push(Condition::Field(FieldCondition::new(field, op, val)));
    }

    match conditions.len() {
        0 => Err(ParseError::new("empty operator object", path)),
        1 => Ok(conditions.remove(0)),
        _ => Ok(Condition::All(conditions)), // multiple operators = AND
    }
}

fn parse_value(json: &JsonValue, path: &str) -> Result<Value, ParseError> {
    match json {
        JsonValue::String(s) => Ok(Value::String(s.clone())),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Number(i))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::Float(f))
            } else {
                Err(ParseError::new("invalid number", path))
            }
        }
        JsonValue::Bool(b) => Ok(Value::Bool(*b)),
        JsonValue::Array(arr) => {
            let values = parse_value_array(arr, path)?;
            Ok(Value::List(values))
        }
        JsonValue::Null => Err(ParseError::new("null values not supported", path)),
        JsonValue::Object(_) => Err(ParseError::new(
            "nested objects not supported as values",
            path,
        )),
    }
}

fn parse_value_array(arr: &[JsonValue], path: &str) -> Result<Vec<Value>, ParseError> {
    arr.iter()
        .enumerate()
        .map(|(i, v)| parse_value(v, &format!("{}[{}]", path, i)))
        .collect()
}

/// parse condition definitions from a JSON object
#[allow(dead_code)]
pub fn parse_definitions(json: &JsonValue) -> Result<ConditionDefinitions, ParseError> {
    let obj = json
        .as_object()
        .ok_or_else(|| ParseError::new("conditions must be an object", "conditions"))?;

    let mut definitions = ConditionDefinitions::new();

    // first pass: create placeholder refs for all definitions
    // this allows forward references
    for name in obj.keys() {
        definitions.insert(name.clone(), Condition::Ref(name.clone()));
    }

    // second pass: parse all definitions
    let mut parsed = ConditionDefinitions::new();
    for (name, value) in obj {
        let condition =
            parse_condition_internal(value, &definitions, &format!("conditions.{}", name))?;
        parsed.insert(name.clone(), condition);
    }

    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_simple_equality() {
        let json = json!({ "app": "Firefox" });
        let cond = parse_condition(&json, &ConditionDefinitions::new()).unwrap();

        match cond {
            Condition::Field(fc) => {
                assert_eq!(fc.field, "app");
                assert_eq!(fc.op, CompareOp::Eq);
                assert_eq!(fc.value, Value::String("Firefox".to_string()));
            }
            _ => panic!("expected Field condition"),
        }
    }

    #[test]
    fn test_parse_number_equality() {
        let json = json!({ "display.count": 2 });
        let cond = parse_condition(&json, &ConditionDefinitions::new()).unwrap();

        match cond {
            Condition::Field(fc) => {
                assert_eq!(fc.field, "display.count");
                assert_eq!(fc.op, CompareOp::Eq);
                assert_eq!(fc.value, Value::Number(2));
            }
            _ => panic!("expected Field condition"),
        }
    }

    #[test]
    fn test_parse_comparison_operators() {
        let json = json!({ "display.count": { ">=": 2 } });
        let cond = parse_condition(&json, &ConditionDefinitions::new()).unwrap();

        match cond {
            Condition::Field(fc) => {
                assert_eq!(fc.field, "display.count");
                assert_eq!(fc.op, CompareOp::Gte);
                assert_eq!(fc.value, Value::Number(2));
            }
            _ => panic!("expected Field condition"),
        }
    }

    #[test]
    fn test_parse_all_operator_forms() {
        let defs = ConditionDefinitions::new();

        // test all forms of equality
        for op in ["==", "eq", "equals"] {
            let json = json!({ "x": { op: 1 } });
            let cond = parse_condition(&json, &defs).unwrap();
            match cond {
                Condition::Field(fc) => assert_eq!(fc.op, CompareOp::Eq),
                _ => panic!("expected Field"),
            }
        }

        // test all forms of greater than
        for op in [">", "gt", "greater_than"] {
            let json = json!({ "x": { op: 1 } });
            let cond = parse_condition(&json, &defs).unwrap();
            match cond {
                Condition::Field(fc) => assert_eq!(fc.op, CompareOp::Gt),
                _ => panic!("expected Field"),
            }
        }
    }

    #[test]
    fn test_parse_in_operator() {
        let json = json!({ "app": { "in": ["Firefox", "Chrome"] } });
        let cond = parse_condition(&json, &ConditionDefinitions::new()).unwrap();

        match cond {
            Condition::Field(fc) => {
                assert_eq!(fc.field, "app");
                assert_eq!(fc.op, CompareOp::In);
                match &fc.value {
                    Value::List(l) => {
                        assert_eq!(l.len(), 2);
                        assert_eq!(l[0], Value::String("Firefox".to_string()));
                        assert_eq!(l[1], Value::String("Chrome".to_string()));
                    }
                    _ => panic!("expected List value"),
                }
            }
            _ => panic!("expected Field condition"),
        }
    }

    #[test]
    fn test_parse_array_shorthand_for_in() {
        let json = json!({ "app": ["Firefox", "Chrome"] });
        let cond = parse_condition(&json, &ConditionDefinitions::new()).unwrap();

        match cond {
            Condition::Field(fc) => {
                assert_eq!(fc.op, CompareOp::In);
            }
            _ => panic!("expected Field condition"),
        }
    }

    #[test]
    fn test_parse_all() {
        let json = json!({
            "all": [
                { "app": "Firefox" },
                { "display.count": { ">=": 2 } }
            ]
        });
        let cond = parse_condition(&json, &ConditionDefinitions::new()).unwrap();

        match cond {
            Condition::All(conditions) => {
                assert_eq!(conditions.len(), 2);
            }
            _ => panic!("expected All condition"),
        }
    }

    #[test]
    fn test_parse_any() {
        let json = json!({
            "any": [
                { "app": "Firefox" },
                { "app": "Chrome" }
            ]
        });
        let cond = parse_condition(&json, &ConditionDefinitions::new()).unwrap();

        match cond {
            Condition::Any(conditions) => {
                assert_eq!(conditions.len(), 2);
            }
            _ => panic!("expected Any condition"),
        }
    }

    #[test]
    fn test_parse_not() {
        let json = json!({
            "not": { "app.fullscreen": true }
        });
        let cond = parse_condition(&json, &ConditionDefinitions::new()).unwrap();

        match cond {
            Condition::Not(inner) => match inner.as_ref() {
                Condition::Field(fc) => {
                    assert_eq!(fc.field, "app.fullscreen");
                    assert_eq!(fc.value, Value::Bool(true));
                }
                _ => panic!("expected Field inside Not"),
            },
            _ => panic!("expected Not condition"),
        }
    }

    #[test]
    fn test_parse_implicit_and() {
        let json = json!({
            "app": "Firefox",
            "display.count": 2
        });
        let cond = parse_condition(&json, &ConditionDefinitions::new()).unwrap();

        match cond {
            Condition::All(conditions) => {
                assert_eq!(conditions.len(), 2);
            }
            _ => panic!("expected All condition (implicit AND)"),
        }
    }

    #[test]
    fn test_parse_multiple_comparison_operators() {
        // { "time.hour": { ">=": 9, "<": 17 } } should become AND of two conditions
        let json = json!({ "time.hour": { ">=": 9, "<": 17 } });
        let cond = parse_condition(&json, &ConditionDefinitions::new()).unwrap();

        match cond {
            Condition::All(conditions) => {
                assert_eq!(conditions.len(), 2);
            }
            _ => panic!("expected All condition for multiple operators"),
        }
    }

    #[test]
    fn test_parse_ref() {
        let mut defs = ConditionDefinitions::new();
        defs.insert(
            "work_hours".to_string(),
            Condition::Field(FieldCondition::eq(
                "time",
                Value::String("9AM-5PM".to_string()),
            )),
        );

        let json = json!({ "$ref": "work_hours" });
        let cond = parse_condition(&json, &defs).unwrap();

        match cond {
            Condition::Field(fc) => {
                assert_eq!(fc.field, "time");
            }
            _ => panic!("expected resolved Field condition"),
        }
    }

    #[test]
    fn test_parse_ref_undefined() {
        let json = json!({ "$ref": "undefined_condition" });
        let result = parse_condition(&json, &ConditionDefinitions::new());

        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("undefined"));
    }

    #[test]
    fn test_parse_nested_complex() {
        let json = json!({
            "all": [
                { "$ref": "work_hours" },
                {
                    "any": [
                        { "display.connected": "external" },
                        { "display.count": { ">=": 2 } }
                    ]
                },
                { "not": { "app.fullscreen": true } }
            ]
        });

        let mut defs = ConditionDefinitions::new();
        defs.insert(
            "work_hours".to_string(),
            Condition::Field(FieldCondition::eq(
                "time",
                Value::String("9AM-5PM".to_string()),
            )),
        );

        let cond = parse_condition(&json, &defs).unwrap();

        match cond {
            Condition::All(conditions) => {
                assert_eq!(conditions.len(), 3);
                // second should be Any
                match &conditions[1] {
                    Condition::Any(inner) => assert_eq!(inner.len(), 2),
                    _ => panic!("expected Any"),
                }
                // third should be Not
                match &conditions[2] {
                    Condition::Not(_) => {}
                    _ => panic!("expected Not"),
                }
            }
            _ => panic!("expected All condition"),
        }
    }

    #[test]
    fn test_parse_definitions() {
        let json = json!({
            "work_hours": {
                "time": "9AM-5PM",
                "time.day": "mon-fri"
            },
            "docked": {
                "display.count": { ">=": 2 }
            }
        });

        let defs = parse_definitions(&json).unwrap();

        assert!(defs.contains_key("work_hours"));
        assert!(defs.contains_key("docked"));

        // work_hours should be an All (implicit AND of two fields)
        match defs.get("work_hours").unwrap() {
            Condition::All(conditions) => {
                assert_eq!(conditions.len(), 2);
            }
            _ => panic!("expected All for work_hours"),
        }
    }

    #[test]
    fn test_parse_bool_true() {
        let json = json!(true);
        let cond = parse_condition(&json, &ConditionDefinitions::new()).unwrap();

        // true should be empty All (always true)
        match cond {
            Condition::All(v) => assert!(v.is_empty()),
            _ => panic!("expected empty All"),
        }
    }

    #[test]
    fn test_parse_bool_false() {
        let json = json!(false);
        let cond = parse_condition(&json, &ConditionDefinitions::new()).unwrap();

        // false should be empty Any (always false)
        match cond {
            Condition::Any(v) => assert!(v.is_empty()),
            _ => panic!("expected empty Any"),
        }
    }
}
