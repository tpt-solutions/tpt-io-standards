//! FHIRPath expression evaluator (subset).
//!
//! FHIRPath is a path-based navigation and extraction language for FHIR
//! resources. This module implements a useful subset:
//!
//! - Path navigation: `Patient.name.given`
//! - Filtering: `name.where(use = 'official')`
//! - Existence: `name.exists()`, `name.empty()`
//! - First/last: `name.first()`, `name.last()`
//! - Count: `name.count()`
//! - Logical: `and`, `or`, `not()`
//! - Comparison: `=`, `!=`, `<`, `>`, `<=`, `>=`
//! - Arithmetic: `+`, `-`, `*`, `/`
//!
//! The evaluator works over a [`FhirValue`] tree that mirrors the JSON structure
//! of a FHIR resource.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use crate::error::FhirError;
use crate::model::Resource;

/// A FHIRPath value in the evaluation tree.
#[derive(Debug, Clone, PartialEq)]
pub enum FhirValue {
    /// A primitive string.
    String(String),
    /// A primitive number (integer or decimal).
    Number(f64),
    /// A primitive boolean.
    Boolean(bool),
    /// An ordered collection of values.
    Collection(Vec<FhirValue>),
    /// A named object: key → value.
    Object(Vec<(String, FhirValue)>),
    /// The empty result.
    Empty,
}

impl fmt::Display for FhirValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FhirValue::String(s) => write!(f, "'{s}'"),
            FhirValue::Number(n) => {
                if *n == n.trunc() && n.abs() < 1e15 {
                    write!(f, "{:.0}", n)
                } else {
                    write!(f, "{n}")
                }
            }
            FhirValue::Boolean(b) => write!(f, "{b}"),
            FhirValue::Collection(items) => {
                write!(f, "[{}]", items.len())
            }
            FhirValue::Object(fields) => {
                write!(f, "{{{}}}", fields.len())
            }
            FhirValue::Empty => write!(f, "()"),
        }
    }
}

impl FhirValue {
    /// Convert a serde_json::Value into a FhirValue.
    pub fn from_json(value: &serde_json::Value) -> Self {
        match value {
            serde_json::Value::String(s) => FhirValue::String(s.clone()),
            serde_json::Value::Number(n) => FhirValue::Number(n.as_f64().unwrap_or(0.0)),
            serde_json::Value::Bool(b) => FhirValue::Boolean(*b),
            serde_json::Value::Array(arr) => {
                FhirValue::Collection(arr.iter().map(FhirValue::from_json).collect())
            }
            serde_json::Value::Object(map) => FhirValue::Object(
                map.iter().map(|(k, v)| (k.clone(), FhirValue::from_json(v))).collect(),
            ),
            serde_json::Value::Null => FhirValue::Empty,
        }
    }

    /// Get a field by name from an Object, or Empty.
    pub fn field(&self, name: &str) -> FhirValue {
        match self {
            FhirValue::Object(fields) => fields
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
                .unwrap_or(FhirValue::Empty),
            _ => FhirValue::Empty,
        }
    }

    /// Whether this value is empty (the FHIRPath `()`).
    pub fn is_empty(&self) -> bool {
        matches!(self, FhirValue::Empty) || matches!(self, FhirValue::Collection(c) if c.is_empty())
    }

    /// Whether this value is a collection.
    pub fn is_collection(&self) -> bool {
        matches!(self, FhirValue::Collection(_))
    }

    /// Flatten nested collections into a single collection.
    pub fn flatten_collection(self) -> FhirValue {
        match self {
            FhirValue::Collection(items) => {
                let mut flat = Vec::new();
                for item in items {
                    match item {
                        FhirValue::Collection(inner) => flat.extend(inner),
                        other => flat.push(other),
                    }
                }
                FhirValue::Collection(flat)
            }
            other => other,
        }
    }
}

/// A FHIRPath expression, parsed into an AST.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A literal value.
    Literal(FhirValue),
    /// A path: a sequence of navigation steps.
    Path(Vec<PathStep>),
    /// A binary operation.
    Binary(Box<Expr>, BinOp, Box<Expr>),
    /// A unary function call: name(argument expression).
    Function(String, Box<Expr>),
    /// Logical not.
    Not(Box<Expr>),
}

/// A single step in a path navigation.
#[derive(Debug, Clone, PartialEq)]
pub enum PathStep {
    /// Navigate to a child field by name.
    Field(String),
    /// Filter with a predicate: `.where(expr)`.
    Where(Box<Expr>),
    /// Select the first item: `.first()`.
    First,
    /// Select the last item: `.last()`.
    Last,
    /// Count items: `.count()`.
    Count,
    /// Check existence: `.exists()`.
    Exists,
    /// Check emptiness: `.empty()`.
    Empty,
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    And,
    Or,
}

/// A simple recursive-descent parser for FHIRPath expressions.
pub struct FhirPathParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> FhirPathParser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    pub fn parse(&mut self) -> Result<Expr, FhirError> {
        self.skip_ws();
        self.parse_or()
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn parse_or(&mut self) -> Result<Expr, FhirError> {
        let mut left = self.parse_and()?;
        self.skip_ws();
        while self.match_keyword("or") {
            let right = self.parse_and()?;
            left = Expr::Binary(Box::new(left), BinOp::Or, Box::new(right));
            self.skip_ws();
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, FhirError> {
        let mut left = self.parse_comparison()?;
        self.skip_ws();
        while self.match_keyword("and") {
            let right = self.parse_comparison()?;
            left = Expr::Binary(Box::new(left), BinOp::And, Box::new(right));
            self.skip_ws();
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, FhirError> {
        let left = self.parse_addition()?;
        self.skip_ws();
        let op = match self.peek() {
            Some('=') => {
                self.advance();
                BinOp::Eq
            }
            Some('!') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    BinOp::Ne
                } else {
                    return Err(FhirError::FhirPath("expected '!='".to_string()));
                }
            }
            Some('<') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    BinOp::Le
                } else {
                    BinOp::Lt
                }
            }
            Some('>') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    BinOp::Ge
                } else {
                    BinOp::Gt
                }
            }
            _ => return Ok(left),
        };
        let right = self.parse_addition()?;
        Ok(Expr::Binary(Box::new(left), op, Box::new(right)))
    }

    fn parse_addition(&mut self) -> Result<Expr, FhirError> {
        let mut left = self.parse_unary()?;
        self.skip_ws();
        while let Some(c) = self.peek() {
            if c == '+' {
                self.advance();
                let right = self.parse_unary()?;
                left = Expr::Binary(Box::new(left), BinOp::Add, Box::new(right));
            } else if c == '-' {
                self.advance();
                let right = self.parse_unary()?;
                left = Expr::Binary(Box::new(left), BinOp::Sub, Box::new(right));
            } else {
                break;
            }
            self.skip_ws();
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, FhirError> {
        self.skip_ws();
        if self.match_keyword("not") {
            let expr = self.parse_primary()?;
            return Ok(Expr::Not(Box::new(expr)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, FhirError> {
        self.skip_ws();
        match self.peek() {
            Some('(') => {
                self.advance();
                let expr = self.parse_or()?;
                self.skip_ws();
                if self.peek() == Some(')') {
                    self.advance();
                }
                Ok(expr)
            }
            Some('\'') => self.parse_string_literal(),
            Some('"') => self.parse_string_literal(),
            Some(c) if c.is_ascii_digit() => self.parse_number_literal(),
            Some(c) if c.is_alphabetic() || c == '_' => self.parse_identifier_or_path(),
            _ => Err(FhirError::FhirPath("unexpected token".to_string())),
        }
    }

    fn parse_identifier_or_path(&mut self) -> Result<Expr, FhirError> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }
        let ident = self.input[start..self.pos].to_string();

        // Boolean literals.
        if ident == "true" {
            return Ok(Expr::Literal(FhirValue::Boolean(true)));
        }
        if ident == "false" {
            return Ok(Expr::Literal(FhirValue::Boolean(false)));
        }

        // Check for function call or path step.
        self.skip_ws();
        match self.peek() {
            Some('(') => {
                // Function call.
                self.advance();
                self.skip_ws();
                let arg = if self.peek() == Some(')') {
                    Expr::Literal(FhirValue::Empty)
                } else {
                    self.parse_or()?
                };
                self.skip_ws();
                if self.peek() == Some(')') {
                    self.advance();
                }
                Ok(Expr::Function(ident, Box::new(arg)))
            }
            Some('.') => {
                // Path navigation.
                let mut steps = vec![PathStep::Field(ident)];
                while self.peek() == Some('.') {
                    self.advance();
                    self.skip_ws();
                    let step_start = self.pos;
                    while let Some(c) = self.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let step_name = self.input[step_start..self.pos].to_string();
                    self.skip_ws();
                    if self.peek() == Some('(') {
                        // Function-like step: .where(...), .first(), etc.
                        self.advance();
                        self.skip_ws();
                        let step = match step_name.as_str() {
                            "where" => {
                                let arg = self.parse_or()?;
                                self.skip_ws();
                                if self.peek() == Some(')') {
                                    self.advance();
                                }
                                PathStep::Where(Box::new(arg))
                            }
                            "first" => {
                                if self.peek() == Some(')') {
                                    self.advance();
                                }
                                PathStep::First
                            }
                            "last" => {
                                if self.peek() == Some(')') {
                                    self.advance();
                                }
                                PathStep::Last
                            }
                            "count" => {
                                if self.peek() == Some(')') {
                                    self.advance();
                                }
                                PathStep::Count
                            }
                            "exists" => {
                                if self.peek() == Some(')') {
                                    self.advance();
                                }
                                PathStep::Exists
                            }
                            "empty" => {
                                if self.peek() == Some(')') {
                                    self.advance();
                                }
                                PathStep::Empty
                            }
                            _ => {
                                return Err(FhirError::FhirPath(format!(
                                    "unknown path function: {step_name}"
                                )));
                            }
                        };
                        steps.push(step);
                    } else {
                        steps.push(PathStep::Field(step_name));
                    }
                }
                Ok(Expr::Path(steps))
            }
            _ => {
                // Bare identifier — treat as single-field path.
                Ok(Expr::Path(vec![PathStep::Field(ident)]))
            }
        }
    }

    fn parse_string_literal(&mut self) -> Result<Expr, FhirError> {
        let quote = self.advance().unwrap();
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == quote {
                let s = &self.input[start..self.pos];
                self.advance();
                return Ok(Expr::Literal(FhirValue::String(s.to_string())));
            }
            self.advance();
        }
        Err(FhirError::FhirPath("unterminated string".to_string()))
    }

    fn parse_number_literal(&mut self) -> Result<Expr, FhirError> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' {
                self.advance();
            } else {
                break;
            }
        }
        let num_str = &self.input[start..self.pos];
        let num: f64 = num_str
            .parse()
            .map_err(|_| FhirError::FhirPath(format!("invalid number: {num_str}")))?;
        Ok(Expr::Literal(FhirValue::Number(num)))
    }

    fn match_keyword(&mut self, kw: &str) -> bool {
        let rest = &self.input[self.pos..];
        if let Some(after) = rest.strip_prefix(kw) {
            // Ensure it's not part of a longer identifier.
            if after.is_empty() || !after.starts_with(|c: char| c.is_alphanumeric() || c == '_') {
                self.pos += kw.len();
                self.skip_ws();
                return true;
            }
        }
        false
    }
}

/// Evaluate a parsed FHIRPath expression against a [`FhirValue`] context.
pub fn evaluate(expr: &Expr, context: &FhirValue) -> Result<FhirValue, FhirError> {
    match expr {
        Expr::Literal(v) => Ok(v.clone()),
        Expr::Path(steps) => {
            let mut current = context.clone();
            for step in steps {
                current = eval_step(step, &current)?;
            }
            Ok(current)
        }
        Expr::Binary(left, op, right) => {
            let l = evaluate(left, context)?;
            let r = evaluate(right, context)?;
            eval_binary(op, &l, &r)
        }
        Expr::Function(name, arg) => {
            let arg_val = evaluate(arg, context)?;
            eval_function(name, &arg_val)
        }
        Expr::Not(expr) => {
            let v = evaluate(expr, context)?;
            Ok(FhirValue::Boolean(!is_truthy(&v)))
        }
    }
}

fn eval_step(step: &PathStep, context: &FhirValue) -> Result<FhirValue, FhirError> {
    // Flatten the context into a collection of items to process.
    let items = match context {
        FhirValue::Collection(c) => c.clone(),
        FhirValue::Empty => Vec::new(),
        other => vec![other.clone()],
    };

    match step {
        PathStep::Field(name) => {
            let mut result = Vec::new();
            for item in &items {
                let val = item.field(name);
                match val {
                    FhirValue::Collection(c) => result.extend(c),
                    FhirValue::Empty => {}
                    other => result.push(other),
                }
            }
            if result.is_empty() {
                Ok(FhirValue::Empty)
            } else if result.len() == 1 {
                Ok(result.into_iter().next().unwrap())
            } else {
                Ok(FhirValue::Collection(result))
            }
        }
        PathStep::Where(pred) => {
            let mut result = Vec::new();
            for item in items {
                let pred_result = evaluate(pred, &item)?;
                if is_truthy(&pred_result) {
                    result.push(item);
                }
            }
            Ok(FhirValue::Collection(result))
        }
        PathStep::First => {
            if items.is_empty() {
                Ok(FhirValue::Empty)
            } else {
                Ok(items.into_iter().next().unwrap())
            }
        }
        PathStep::Last => {
            if items.is_empty() {
                Ok(FhirValue::Empty)
            } else {
                Ok(items.into_iter().last().unwrap())
            }
        }
        PathStep::Count => Ok(FhirValue::Number(items.len() as f64)),
        PathStep::Exists => Ok(FhirValue::Boolean(!items.is_empty())),
        PathStep::Empty => Ok(FhirValue::Boolean(items.is_empty())),
    }
}

fn eval_binary(op: &BinOp, left: &FhirValue, right: &FhirValue) -> Result<FhirValue, FhirError> {
    match op {
        BinOp::Eq => Ok(FhirValue::Boolean(values_equal(left, right))),
        BinOp::Ne => Ok(FhirValue::Boolean(!values_equal(left, right))),
        BinOp::Lt => Ok(FhirValue::Boolean(
            left.as_f64().zip(right.as_f64()).map(|(l, r)| l < r).unwrap_or(false),
        )),
        BinOp::Gt => Ok(FhirValue::Boolean(
            left.as_f64().zip(right.as_f64()).map(|(l, r)| l > r).unwrap_or(false),
        )),
        BinOp::Le => Ok(FhirValue::Boolean(
            left.as_f64().zip(right.as_f64()).map(|(l, r)| l <= r).unwrap_or(false),
        )),
        BinOp::Ge => Ok(FhirValue::Boolean(
            left.as_f64().zip(right.as_f64()).map(|(l, r)| l >= r).unwrap_or(false),
        )),
        BinOp::Add => {
            Ok(FhirValue::Number(left.as_f64().unwrap_or(0.0) + right.as_f64().unwrap_or(0.0)))
        }
        BinOp::Sub => {
            Ok(FhirValue::Number(left.as_f64().unwrap_or(0.0) - right.as_f64().unwrap_or(0.0)))
        }
        BinOp::Mul => {
            Ok(FhirValue::Number(left.as_f64().unwrap_or(0.0) * right.as_f64().unwrap_or(0.0)))
        }
        BinOp::Div => {
            Ok(FhirValue::Number(left.as_f64().unwrap_or(0.0) / right.as_f64().unwrap_or(1.0)))
        }
        BinOp::And => Ok(FhirValue::Boolean(is_truthy(left) && is_truthy(right))),
        BinOp::Or => Ok(FhirValue::Boolean(is_truthy(left) || is_truthy(right))),
    }
}

fn eval_function(name: &str, arg: &FhirValue) -> Result<FhirValue, FhirError> {
    match name {
        "exists" => Ok(FhirValue::Boolean(!arg.is_empty())),
        "empty" => Ok(FhirValue::Boolean(arg.is_empty())),
        "count" => match arg {
            FhirValue::Collection(c) => Ok(FhirValue::Number(c.len() as f64)),
            FhirValue::Empty => Ok(FhirValue::Number(0.0)),
            _ => Ok(FhirValue::Number(1.0)),
        },
        "first" => match arg {
            FhirValue::Collection(c) => Ok(c.first().cloned().unwrap_or(FhirValue::Empty)),
            other => Ok(other.clone()),
        },
        "last" => match arg {
            FhirValue::Collection(c) => Ok(c.last().cloned().unwrap_or(FhirValue::Empty)),
            other => Ok(other.clone()),
        },
        _ => Err(FhirError::FhirPath(format!("unknown function: {name}"))),
    }
}

fn is_truthy(value: &FhirValue) -> bool {
    match value {
        FhirValue::Boolean(b) => *b,
        FhirValue::Empty => false,
        FhirValue::Collection(c) => !c.is_empty(),
        FhirValue::Number(n) => *n != 0.0,
        FhirValue::String(s) => !s.is_empty(),
        FhirValue::Object(_) => true,
    }
}

fn values_equal(left: &FhirValue, right: &FhirValue) -> bool {
    match (left, right) {
        (FhirValue::String(a), FhirValue::String(b)) => a == b,
        (FhirValue::Number(a), FhirValue::Number(b)) => (a - b).abs() < 1e-9,
        (FhirValue::Boolean(a), FhirValue::Boolean(b)) => a == b,
        (FhirValue::Empty, FhirValue::Empty) => true,
        _ => false,
    }
}

impl FhirValue {
    fn as_f64(&self) -> Option<f64> {
        match self {
            FhirValue::Number(n) => Some(*n),
            FhirValue::String(s) => s.parse().ok(),
            _ => None,
        }
    }
}

/// Parse and evaluate a FHIRPath expression against a FHIR resource (JSON).
pub fn evaluate_resource(expr_str: &str, resource_json: &str) -> Result<FhirValue, FhirError> {
    let value: serde_json::Value =
        serde_json::from_str(resource_json).map_err(|e| FhirError::Json(e.to_string()))?;
    let context = FhirValue::from_json(&value);
    let mut parser = FhirPathParser::new(expr_str);
    let expr = parser.parse()?;
    evaluate(&expr, &context)
}

/// Parse and evaluate a FHIRPath expression against a [`Resource`].
pub fn evaluate_resource_value(
    expr_str: &str,
    resource: &Resource,
) -> Result<FhirValue, FhirError> {
    let json = serde_json::to_string(resource).map_err(|e| FhirError::Json(e.to_string()))?;
    evaluate_resource(expr_str, &json)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patient_json() -> &'static str {
        r#"{
            "resourceType": "Patient",
            "id": "example",
            "gender": "male",
            "birthDate": "1974-12-25",
            "name": [
                {"use": "official", "family": "Chalmers", "given": ["Peter", "James"]},
                {"use": "maiden", "family": "Windsor", "given": ["Peter", "James"]}
            ],
            "active": true
        }"#
    }

    #[test]
    fn parse_path_field() {
        let mut parser = FhirPathParser::new("Patient.name");
        let expr = parser.parse().unwrap();
        assert!(matches!(expr, Expr::Path(_)));
    }

    #[test]
    fn eval_field_navigation() {
        let result = evaluate_resource("name", patient_json()).unwrap();
        assert!(result.is_collection());
    }

    #[test]
    fn eval_nested_field() {
        let result = evaluate_resource("name.family", patient_json()).unwrap();
        assert!(result.is_collection());
    }

    #[test]
    fn eval_where_filter() {
        let result =
            evaluate_resource("name.where(use = 'official').family", patient_json()).unwrap();
        assert_eq!(result, FhirValue::String("Chalmers".to_string()));
    }

    #[test]
    fn eval_exists() {
        let result = evaluate_resource("name.exists()", patient_json()).unwrap();
        assert_eq!(result, FhirValue::Boolean(true));
    }

    #[test]
    fn eval_empty() {
        let result = evaluate_resource("name.empty()", patient_json()).unwrap();
        assert_eq!(result, FhirValue::Boolean(false));
    }

    #[test]
    fn eval_first() {
        let result = evaluate_resource("name.first().family", patient_json()).unwrap();
        assert_eq!(result, FhirValue::String("Chalmers".to_string()));
    }

    #[test]
    fn eval_count() {
        let result = evaluate_resource("name.count()", patient_json()).unwrap();
        assert_eq!(result, FhirValue::Number(2.0));
    }

    #[test]
    fn eval_comparison() {
        let result = evaluate_resource("gender = 'male'", patient_json()).unwrap();
        assert_eq!(result, FhirValue::Boolean(true));
    }

    #[test]
    fn eval_not() {
        let result = evaluate_resource("not(gender = 'female')", patient_json()).unwrap();
        assert_eq!(result, FhirValue::Boolean(true));
    }

    #[test]
    fn eval_and() {
        let result =
            evaluate_resource("gender = 'male' and active = true", patient_json()).unwrap();
        assert_eq!(result, FhirValue::Boolean(true));
    }

    #[test]
    fn eval_arithmetic() {
        let result = evaluate_resource("1 + 2", patient_json()).unwrap();
        assert_eq!(result, FhirValue::Number(3.0));
    }

    #[test]
    fn eval_literal_string() {
        let mut parser = FhirPathParser::new("'hello'");
        let expr = parser.parse().unwrap();
        let result = evaluate(&expr, &FhirValue::Empty).unwrap();
        assert_eq!(result, FhirValue::String("hello".to_string()));
    }

    #[test]
    fn eval_bare_identifier() {
        let result = evaluate_resource("gender", patient_json()).unwrap();
        assert_eq!(result, FhirValue::String("male".to_string()));
    }
}
