//! core types for the condition system

use std::fmt;

/// comparison operators supported in conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    /// equality: ==, eq, equals
    Eq,
    /// inequality: !=, ne, not_equals
    Ne,
    /// greater than: >, gt, greater_than
    Gt,
    /// greater than or equal: >=, gte, greater_than_or_equal
    Gte,
    /// less than: <, lt, less_than
    Lt,
    /// less than or equal: <=, lte, less_than_or_equal
    Lte,
    /// set membership: in
    In,
}

impl CompareOp {
    /// parse operator from string (supports all forms)
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "==" | "eq" | "equals" => Some(CompareOp::Eq),
            "!=" | "ne" | "not_equals" => Some(CompareOp::Ne),
            ">" | "gt" | "greater_than" => Some(CompareOp::Gt),
            ">=" | "gte" | "greater_than_or_equal" => Some(CompareOp::Gte),
            "<" | "lt" | "less_than" => Some(CompareOp::Lt),
            "<=" | "lte" | "less_than_or_equal" => Some(CompareOp::Lte),
            "in" => Some(CompareOp::In),
            _ => None,
        }
    }
}

impl fmt::Display for CompareOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompareOp::Eq => write!(f, "=="),
            CompareOp::Ne => write!(f, "!="),
            CompareOp::Gt => write!(f, ">"),
            CompareOp::Gte => write!(f, ">="),
            CompareOp::Lt => write!(f, "<"),
            CompareOp::Lte => write!(f, "<="),
            CompareOp::In => write!(f, "in"),
        }
    }
}

/// a value that can be used in comparisons
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// string value
    String(String),
    /// integer value
    Number(i64),
    /// floating point value
    Float(f64),
    /// boolean value
    Bool(bool),
    /// list of values (for 'in' operator)
    List(Vec<Value>),
}

#[allow(dead_code)]
impl Value {
    /// try to get as string
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// try to get as integer
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Number(n) => Some(*n),
            Value::Float(f) => Some(*f as i64),
            _ => None,
        }
    }

    /// try to get as float
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// try to get as bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// try to get as list
    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Value::List(l) => Some(l),
            _ => None,
        }
    }

    /// check if value is truthy
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::List(l) => !l.is_empty(),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Number(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::List(l) => {
                write!(f, "[")?;
                for (i, v) in l.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
        }
    }
}

/// a single field comparison condition
#[derive(Debug, Clone, PartialEq)]
pub struct FieldCondition {
    /// field name (e.g., "app", "display.count", "time.hour")
    pub field: String,
    /// comparison operator
    pub op: CompareOp,
    /// value to compare against
    pub value: Value,
}

impl FieldCondition {
    /// create a new field condition
    pub fn new(field: impl Into<String>, op: CompareOp, value: Value) -> Self {
        Self {
            field: field.into(),
            op,
            value,
        }
    }

    /// create an equality condition
    pub fn eq(field: impl Into<String>, value: Value) -> Self {
        Self::new(field, CompareOp::Eq, value)
    }

    /// create an 'in' condition
    pub fn is_in(field: impl Into<String>, values: Vec<Value>) -> Self {
        Self::new(field, CompareOp::In, Value::List(values))
    }
}

impl fmt::Display for FieldCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.field, self.op, self.value)
    }
}

/// the condition AST - represents a parsed condition
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    /// all conditions must be true (AND)
    All(Vec<Condition>),
    /// any condition must be true (OR)
    Any(Vec<Condition>),
    /// negate a condition (NOT)
    Not(Box<Condition>),
    /// a field comparison
    Field(FieldCondition),
    /// reference to a named condition (resolved during parsing)
    Ref(String),
}

impl Condition {
    /// create an AND condition
    #[allow(dead_code)]
    pub fn all(conditions: Vec<Condition>) -> Self {
        Condition::All(conditions)
    }

    /// create an OR condition
    #[allow(dead_code)]
    pub fn any(conditions: Vec<Condition>) -> Self {
        Condition::Any(conditions)
    }

    /// create a NOT condition
    #[allow(dead_code)]
    #[allow(clippy::should_implement_trait)]
    pub fn negate(condition: Condition) -> Self {
        Condition::Not(Box::new(condition))
    }

    /// create a field condition
    #[allow(dead_code)]
    pub fn field(fc: FieldCondition) -> Self {
        Condition::Field(fc)
    }

    /// create a reference condition
    #[allow(dead_code)]
    pub fn reference(name: impl Into<String>) -> Self {
        Condition::Ref(name.into())
    }

    /// check if this condition is empty (always true)
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        match self {
            Condition::All(v) => v.is_empty(),
            Condition::Any(v) => v.is_empty(),
            _ => false,
        }
    }
}

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Condition::All(conditions) => {
                write!(f, "all(")?;
                for (i, c) in conditions.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", c)?;
                }
                write!(f, ")")
            }
            Condition::Any(conditions) => {
                write!(f, "any(")?;
                for (i, c) in conditions.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", c)?;
                }
                write!(f, ")")
            }
            Condition::Not(inner) => write!(f, "not({})", inner),
            Condition::Field(fc) => write!(f, "{}", fc),
            Condition::Ref(name) => write!(f, "$ref({})", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_op_parse() {
        assert_eq!(CompareOp::parse("=="), Some(CompareOp::Eq));
        assert_eq!(CompareOp::parse("eq"), Some(CompareOp::Eq));
        assert_eq!(CompareOp::parse("equals"), Some(CompareOp::Eq));

        assert_eq!(CompareOp::parse("!="), Some(CompareOp::Ne));
        assert_eq!(CompareOp::parse("ne"), Some(CompareOp::Ne));
        assert_eq!(CompareOp::parse("not_equals"), Some(CompareOp::Ne));

        assert_eq!(CompareOp::parse(">"), Some(CompareOp::Gt));
        assert_eq!(CompareOp::parse("gt"), Some(CompareOp::Gt));
        assert_eq!(CompareOp::parse("greater_than"), Some(CompareOp::Gt));

        assert_eq!(CompareOp::parse(">="), Some(CompareOp::Gte));
        assert_eq!(CompareOp::parse("gte"), Some(CompareOp::Gte));
        assert_eq!(
            CompareOp::parse("greater_than_or_equal"),
            Some(CompareOp::Gte)
        );

        assert_eq!(CompareOp::parse("<"), Some(CompareOp::Lt));
        assert_eq!(CompareOp::parse("lt"), Some(CompareOp::Lt));
        assert_eq!(CompareOp::parse("less_than"), Some(CompareOp::Lt));

        assert_eq!(CompareOp::parse("<="), Some(CompareOp::Lte));
        assert_eq!(CompareOp::parse("lte"), Some(CompareOp::Lte));
        assert_eq!(CompareOp::parse("less_than_or_equal"), Some(CompareOp::Lte));

        assert_eq!(CompareOp::parse("in"), Some(CompareOp::In));

        assert_eq!(CompareOp::parse("invalid"), None);
    }

    #[test]
    fn test_value_conversions() {
        let s = Value::String("test".to_string());
        assert_eq!(s.as_str(), Some("test"));
        assert_eq!(s.as_i64(), None);

        let n = Value::Number(42);
        assert_eq!(n.as_i64(), Some(42));
        assert_eq!(n.as_f64(), Some(42.0));
        assert_eq!(n.as_str(), None);

        let f = Value::Float(3.14);
        assert_eq!(f.as_f64(), Some(3.14));
        assert_eq!(f.as_i64(), Some(3));

        let b = Value::Bool(true);
        assert_eq!(b.as_bool(), Some(true));

        let l = Value::List(vec![Value::Number(1), Value::Number(2)]);
        assert!(l.as_list().is_some());
        assert_eq!(l.as_list().unwrap().len(), 2);
    }

    #[test]
    fn test_value_truthy() {
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Bool(false).is_truthy());

        assert!(Value::Number(1).is_truthy());
        assert!(!Value::Number(0).is_truthy());

        assert!(Value::String("hello".to_string()).is_truthy());
        assert!(!Value::String("".to_string()).is_truthy());

        assert!(Value::List(vec![Value::Number(1)]).is_truthy());
        assert!(!Value::List(vec![]).is_truthy());
    }

    #[test]
    fn test_field_condition_display() {
        let fc = FieldCondition::eq("app", Value::String("Firefox".to_string()));
        assert_eq!(format!("{}", fc), "app == \"Firefox\"");

        let fc = FieldCondition::new("display.count", CompareOp::Gte, Value::Number(2));
        assert_eq!(format!("{}", fc), "display.count >= 2");
    }

    #[test]
    fn test_condition_display() {
        let c = Condition::field(FieldCondition::eq(
            "app",
            Value::String("Firefox".to_string()),
        ));
        assert_eq!(format!("{}", c), "app == \"Firefox\"");

        let c = Condition::negate(Condition::field(FieldCondition::eq(
            "fullscreen",
            Value::Bool(true),
        )));
        assert_eq!(format!("{}", c), "not(fullscreen == true)");

        let c = Condition::all(vec![
            Condition::field(FieldCondition::eq(
                "app",
                Value::String("Firefox".to_string()),
            )),
            Condition::field(FieldCondition::new(
                "display.count",
                CompareOp::Gte,
                Value::Number(2),
            )),
        ]);
        assert_eq!(
            format!("{}", c),
            "all(app == \"Firefox\", display.count >= 2)"
        );
    }
}
