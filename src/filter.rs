//! SCIM filtering ([RFC 7644, section 3.4.2.2]).
//!
//! Provides a parser and evaluator for SCIM filter expressions such as
//! `userName eq "bjensen" and not (emails co "example.com")`.
//!
//! Comparisons against an absent attribute evaluate to `false` (including
//! `ne`); string comparisons are case-insensitive, matching SCIM's
//! `caseIgnoreMatch` semantics.
//!
//! [RFC 7644, section 3.4.2.2]: https://datatracker.ietf.org/doc/html/rfc7644#section-3.4.2.2

use crate::error::ScimError;
use serde::Serialize;
use serde_json::Value;

/// A comparison operator used in [`Filter::Compare`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    /// `eq` — equal.
    Eq,
    /// `ne` — not equal.
    Ne,
    /// `co` — contains.
    Co,
    /// `sw` — starts with.
    Sw,
    /// `ew` — ends with.
    Ew,
    /// `gt` — greater than.
    Gt,
    /// `ge` — greater than or equal.
    Ge,
    /// `lt` — less than.
    Lt,
    /// `le` — less than or equal.
    Le,
}

impl CompareOp {
    fn from_word(word: &str) -> Option<Self> {
        match word.to_ascii_lowercase().as_str() {
            "eq" => Some(Self::Eq),
            "ne" => Some(Self::Ne),
            "co" => Some(Self::Co),
            "sw" => Some(Self::Sw),
            "ew" => Some(Self::Ew),
            "gt" => Some(Self::Gt),
            "ge" => Some(Self::Ge),
            "lt" => Some(Self::Lt),
            "le" => Some(Self::Le),
            _ => None,
        }
    }
}

/// A parsed SCIM filter expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Filter {
    /// `attr Pr` — the attribute has a value.
    Present(String),
    /// `attr op value` — an attribute comparison.
    Compare {
        /// Dotted attribute path (e.g. `name.givenName`).
        attr: String,
        /// The comparison operator.
        op: CompareOp,
        /// The comparison value.
        value: Value,
    },
    /// `left and right`.
    And(Box<Filter>, Box<Filter>),
    /// `left or right`.
    Or(Box<Filter>, Box<Filter>),
    /// `not inner`.
    Not(Box<Filter>),
}

impl Filter {
    /// Parses a SCIM filter expression.
    ///
    /// # Errors
    ///
    /// Returns [`ScimError::BadRequest`] if the expression is malformed.
    pub fn parse(input: &str) -> Result<Self, ScimError> {
        parse_filter(input)
    }

    /// Evaluates the filter against a resource represented as JSON.
    ///
    /// Multi-valued attributes match if any element matches.
    pub fn matches(&self, resource: &Value) -> bool {
        match self {
            Self::Present(attr) => resolve(resource, attr).into_iter().any(|v| !v.is_null()),
            Self::Compare { attr, op, value } => resolve(resource, attr)
                .into_iter()
                .any(|v| compare(v, *op, value)),
            Self::And(a, b) => a.matches(resource) && b.matches(resource),
            Self::Or(a, b) => a.matches(resource) || b.matches(resource),
            Self::Not(inner) => !inner.matches(resource),
        }
    }

    /// Evaluates the filter against any serializable resource.
    pub fn matches_serialized<S: Serialize>(&self, resource: &S) -> bool {
        match serde_json::to_value(resource) {
            Ok(value) => self.matches(&value),
            Err(_) => false,
        }
    }
}

/// Parses a SCIM filter expression.
///
/// # Errors
///
/// Returns [`ScimError::BadRequest`] if the expression is malformed.
pub fn parse_filter(input: &str) -> Result<Filter, ScimError> {
    let bad = |msg: &str| ScimError::BadRequest(format!("invalid filter: {msg}"));
    let tokens = tokenize(input).map_err(|msg| bad(&msg))?;
    let mut parser = Parser { tokens, pos: 0 };
    let filter = parser.parse_or().map_err(|msg| bad(&msg))?;
    if parser.pos != parser.tokens.len() {
        return Err(bad("unexpected trailing input"));
    }
    Ok(filter)
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    Str(String),
    LParen,
    RParen,
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            c if c.is_whitespace() => {
                chars.next();
            }
            '(' => {
                tokens.push(Token::LParen);
                chars.next();
            }
            ')' => {
                tokens.push(Token::RParen);
                chars.next();
            }
            '"' => {
                chars.next();
                let mut s = String::new();
                loop {
                    match chars.next() {
                        None => return Err("unterminated string".to_string()),
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some(escaped @ ('"' | '\\')) => s.push(escaped),
                            other => {
                                s.push('\\');
                                if let Some(c) = other {
                                    s.push(c);
                                }
                            }
                        },
                        Some(c) => s.push(c),
                    }
                }
                tokens.push(Token::Str(s));
            }
            _ => {
                let mut word = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '$') {
                        word.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if word.is_empty() {
                    return Err(format!("unexpected character {c:?}"));
                }
                tokens.push(Token::Word(word));
            }
        }
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned();
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    fn parse_or(&mut self) -> Result<Filter, String> {
        let mut left = self.parse_and()?;
        loop {
            match self.peek() {
                Some(Token::Word(w)) if w.eq_ignore_ascii_case("or") => {
                    self.next();
                    let right = self.parse_and()?;
                    left = Filter::Or(Box::new(left), Box::new(right));
                }
                _ => return Ok(left),
            }
        }
    }

    fn parse_and(&mut self) -> Result<Filter, String> {
        let mut left = self.parse_unary()?;
        loop {
            match self.peek() {
                Some(Token::Word(w)) if w.eq_ignore_ascii_case("and") => {
                    self.next();
                    let right = self.parse_unary()?;
                    left = Filter::And(Box::new(left), Box::new(right));
                }
                _ => return Ok(left),
            }
        }
    }

    fn parse_unary(&mut self) -> Result<Filter, String> {
        match self.peek() {
            Some(Token::Word(w)) if w.eq_ignore_ascii_case("not") => {
                self.next();
                let inner = self.parse_unary()?;
                Ok(Filter::Not(Box::new(inner)))
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Filter, String> {
        match self.next() {
            None => Err("unexpected end of input".to_string()),
            Some(Token::LParen) => {
                let inner = self.parse_or()?;
                match self.next() {
                    Some(Token::RParen) => Ok(inner),
                    _ => Err("expected closing parenthesis".to_string()),
                }
            }
            Some(Token::RParen) => Err("unexpected closing parenthesis".to_string()),
            Some(Token::Str(_)) => Err("expected attribute name".to_string()),
            Some(Token::Word(attr)) => {
                if attr.contains('.') && attr.split('.').any(str::is_empty) {
                    return Err(format!("invalid attribute path {attr:?}"));
                }
                match self.peek() {
                    Some(Token::Word(w)) if w.eq_ignore_ascii_case("pr") => {
                        self.next();
                        Ok(Filter::Present(attr))
                    }
                    Some(Token::Word(w)) => {
                        let Some(op) = CompareOp::from_word(w) else {
                            return Err(format!("expected operator after {attr:?}"));
                        };
                        self.next();
                        let value = self.parse_value()?;
                        Ok(Filter::Compare { attr, op, value })
                    }
                    _ => Err(format!("expected operator after {attr:?}")),
                }
            }
        }
    }

    fn parse_value(&mut self) -> Result<Value, String> {
        match self.next() {
            Some(Token::Str(s)) => Ok(Value::String(s)),
            Some(Token::Word(w)) => match w.to_ascii_lowercase().as_str() {
                "true" => Ok(Value::Bool(true)),
                "false" => Ok(Value::Bool(false)),
                "null" => Ok(Value::Null),
                _ => {
                    if w.starts_with('-') || w.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                        w.parse::<f64>()
                            .map(|n| {
                                serde_json::Number::from_f64(n)
                                    .map(Value::Number)
                                    .unwrap_or(Value::Null)
                            })
                            .map_err(|_| format!("invalid numeric value {w:?}"))
                    } else {
                        Err(format!("invalid value {w:?}"))
                    }
                }
            },
            _ => Err("expected comparison value".to_string()),
        }
    }
}

fn resolve<'a>(root: &'a Value, path: &str) -> Vec<&'a Value> {
    let mut out = Vec::new();
    resolve_all(root, path, &mut out);
    out
}

fn resolve_all<'a>(current: &'a Value, path: &str, out: &mut Vec<&'a Value>) {
    match path.split_once('.') {
        Some((head, tail)) => match current {
            Value::Object(map) => {
                if let Some(v) = map.get(head) {
                    resolve_all(v, tail, out);
                }
            }
            Value::Array(items) => {
                for item in items {
                    resolve_all(item, path, out);
                }
            }
            _ => {}
        },
        None => match current {
            Value::Object(map) => {
                if let Some(v) = map.get(path) {
                    out.push(v);
                }
            }
            Value::Array(items) => {
                for item in items {
                    resolve_all(item, path, out);
                }
            }
            _ => {}
        },
    }
}

fn compare(actual: &Value, op: CompareOp, expected: &Value) -> bool {
    match op {
        CompareOp::Eq => value_eq(actual, expected),
        CompareOp::Ne => !value_eq(actual, expected),
        CompareOp::Co => match (actual, expected) {
            (Value::String(a), Value::String(b)) => a.to_lowercase().contains(&b.to_lowercase()),
            (Value::Array(items), e) => items.iter().any(|i| value_eq(i, e)),
            _ => false,
        },
        CompareOp::Sw => string_binop(actual, expected, |a, b| a.starts_with(b)),
        CompareOp::Ew => string_binop(actual, expected, |a, b| a.ends_with(b)),
        CompareOp::Gt => cmp_values(actual, expected) == Some(std::cmp::Ordering::Greater),
        CompareOp::Ge => matches!(
            cmp_values(actual, expected),
            Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
        ),
        CompareOp::Lt => cmp_values(actual, expected) == Some(std::cmp::Ordering::Less),
        CompareOp::Le => matches!(
            cmp_values(actual, expected),
            Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
        ),
    }
}

fn value_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(x), Value::String(y)) => x.eq_ignore_ascii_case(y),
        (Value::Number(x), Value::Number(y)) => match (x.as_f64(), y.as_f64()) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        },
        _ => a == b,
    }
}

fn string_binop(a: &Value, b: &Value, f: impl Fn(&str, &str) -> bool) -> bool {
    match (a, b) {
        (Value::String(x), Value::String(y)) => f(&x.to_lowercase(), &y.to_lowercase()),
        _ => false,
    }
}

fn cmp_values(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    use std::cmp::Ordering;
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x
            .as_f64()
            .partial_cmp(&y.as_f64())
            .or(Some(Ordering::Equal)),
        (Value::String(x), Value::String(y)) => Some(x.to_lowercase().cmp(&y.to_lowercase())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{ScimEmail, ScimMeta, ScimUser, USER_SCHEMA_URN};
    use chrono::Utc;

    fn user_json() -> Value {
        serde_json::json!({
            "userName": "bjensen",
            "active": true,
            "age": 32,
            "name": { "givenName": "Barbara", "familyName": "Jensen" },
            "emails": [
                { "value": "bjensen@example.com", "type": "work" },
                { "value": "babs@home.org", "type": "home" }
            ]
        })
    }

    #[test]
    fn test_eq_string_case_insensitive() {
        let f = Filter::parse(r#"userName eq "BJENSEN""#).unwrap();
        assert!(f.matches(&user_json()));
    }

    #[test]
    fn test_eq_number_and_bool() {
        assert!(Filter::parse("age eq 32").unwrap().matches(&user_json()));
        assert!(Filter::parse("active eq true")
            .unwrap()
            .matches(&user_json()));
        assert!(!Filter::parse("age eq 99").unwrap().matches(&user_json()));
    }

    #[test]
    fn test_ne() {
        assert!(Filter::parse("age ne 99").unwrap().matches(&user_json()));
        assert!(!Filter::parse("userName ne \"bjensen\"")
            .unwrap()
            .matches(&user_json()));
    }

    #[test]
    fn test_missing_attribute_is_false() {
        assert!(!Filter::parse("nickname eq \"x\"")
            .unwrap()
            .matches(&user_json()));
        assert!(!Filter::parse("nickname ne \"x\"")
            .unwrap()
            .matches(&user_json()));
        assert!(!Filter::parse("nickname pr").unwrap().matches(&user_json()));
    }

    #[test]
    fn test_contains() {
        assert!(Filter::parse(r#"userName co "jen""#)
            .unwrap()
            .matches(&user_json()));
        assert!(!Filter::parse(r#"userName co "zzz""#)
            .unwrap()
            .matches(&user_json()));
    }

    #[test]
    fn test_starts_with_ends_with() {
        assert!(Filter::parse(r#"userName sw "bj""#)
            .unwrap()
            .matches(&user_json()));
        assert!(Filter::parse(r#"userName ew "sen""#)
            .unwrap()
            .matches(&user_json()));
        assert!(!Filter::parse(r#"userName sw "zz""#)
            .unwrap()
            .matches(&user_json()));
    }

    #[test]
    fn test_ordering_operators() {
        assert!(Filter::parse("age gt 30").unwrap().matches(&user_json()));
        assert!(Filter::parse("age ge 32").unwrap().matches(&user_json()));
        assert!(Filter::parse("age lt 40").unwrap().matches(&user_json()));
        assert!(!Filter::parse("age le 31").unwrap().matches(&user_json()));
    }

    #[test]
    fn test_present() {
        assert!(Filter::parse("userName pr").unwrap().matches(&user_json()));
    }

    #[test]
    fn test_and_or_not() {
        assert!(Filter::parse("age gt 30 and active eq true")
            .unwrap()
            .matches(&user_json()));
        assert!(Filter::parse("age gt 99 or active eq true")
            .unwrap()
            .matches(&user_json()));
        assert!(Filter::parse("not (age gt 99)")
            .unwrap()
            .matches(&user_json()));
        assert!(!Filter::parse("age gt 30 and not (active eq true)")
            .unwrap()
            .matches(&user_json()));
    }

    #[test]
    fn test_precedence_and_parens() {
        // `and` binds tighter than `or`.
        let f = Filter::parse("age gt 99 or active eq true and userName co \"jen\"").unwrap();
        match &f {
            Filter::Or(_, right) => assert!(matches!(**right, Filter::And(_, _))),
            other => panic!("unexpected shape: {other:?}"),
        }
        assert!(f.matches(&user_json()));
    }

    #[test]
    fn test_dotted_path_over_array() {
        assert!(Filter::parse(r#"emails.value co "example.com""#)
            .unwrap()
            .matches(&user_json()));
        assert!(Filter::parse(r#"name.givenName eq "barbara""#)
            .unwrap()
            .matches(&user_json()));
    }

    #[test]
    fn test_escaped_string() {
        let json = serde_json::json!({ "note": "say \"hi\"" });
        assert!(Filter::parse(r#"note eq "say \"hi\"""#)
            .unwrap()
            .matches(&json));
    }

    #[test]
    fn test_null_literal() {
        let json = serde_json::json!({ "middleName": null });
        assert!(!Filter::parse("middleName pr").unwrap().matches(&json));
        assert!(Filter::parse("middleName eq null").unwrap().matches(&json));
    }

    #[test]
    fn test_parse_errors() {
        assert!(Filter::parse("userName eq").is_err());
        assert!(Filter::parse("(userName eq \"x\"").is_err());
        assert!(Filter::parse("userName eq \"x\" garbage").is_err());
        assert!(matches!(
            Filter::parse("userName eq"),
            Err(ScimError::BadRequest(_))
        ));
    }

    #[test]
    fn test_matches_serialized() {
        let user = ScimUser {
            schemas: vec![USER_SCHEMA_URN.into()],
            id: "u1".into(),
            external_id: None,
            user_name: "bjensen".into(),
            name: None,
            display_name: None,
            emails: vec![ScimEmail {
                value: "b@x.org".into(),
                email_type: Some("work".into()),
                primary: true,
            }],
            active: true,
            groups: vec![],
            meta: ScimMeta {
                resource_type: "User".into(),
                created: Utc::now(),
                last_modified: Utc::now(),
                location: "/scim/v2/Users/u1".into(),
            },
        };
        assert!(Filter::parse(r#"userName eq "BJENSEN""#)
            .unwrap()
            .matches_serialized(&user));
        assert!(!Filter::parse(r#"emails.value co "example.com""#)
            .unwrap()
            .matches_serialized(&user));
    }
}
