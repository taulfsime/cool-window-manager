//! condition evaluation system for cwm rules
//!
//! provides a flexible condition system supporting:
//! - logical operators: all (AND), any (OR), not (NOT)
//! - comparison operators: ==, !=, >, >=, <, <= (multiple forms)
//! - set operator: in
//! - implicit AND when multiple fields in one object
//!
//! conditions can be used in shortcuts and app_rules via the `when` field.

mod eval;
mod parser;
mod time;
mod types;

pub use eval::{evaluate, EvalContext, WindowState};
pub use parser::parse_condition;
pub use types::Condition;

// re-export for potential future use
#[allow(unused_imports)]
pub use parser::ParseError;
#[allow(unused_imports)]
pub use types::{CompareOp, FieldCondition, Value};
