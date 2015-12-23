extern crate rustc_serialize;

use std::string::ToString;
use std::cmp::{max, Ordering};
use std::rc::Rc;
use std::collections::BTreeMap;
use std::iter::Iterator;
use self::rustc_serialize::json::Json;

use super::IntoJMESPath;
use super::ast::{Ast, Comparator};

/// JMESPath variable.
///
/// Note: this enum and implementation is based on rustc-serialize:
/// https://github.com/rust-lang-nursery/rustc-serialize
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub enum Variable {
    Null,
    String(String),
    Boolean(bool),
    Number(f64),
    Array(Vec<Rc<Variable>>),
    Object(BTreeMap<String, Rc<Variable>>),
    Expref(Ast)
}

impl Eq for Variable {}

impl Ord for Variable {
    fn cmp(&self, other: &Self) -> Ordering {
        let var_type = self.get_type();
        // Variables of different types are considered equal.
        if var_type != other.get_type() {
            Ordering::Equal
        } else {
            match var_type {
                "string" => self.as_string().unwrap().cmp(other.as_string().unwrap()),
                "number" => self.as_number().unwrap()
                    .partial_cmp(&other.as_number().unwrap())
                    .unwrap_or(Ordering::Less),
                _ => Ordering::Equal
            }
        }
    }
}

impl Variable {
    /// Create a new JMESPath variable from a JSON value.
    pub fn from_json(value: &Json) -> Self {
        match value {
            &Json::Null => Variable::Null,
            &Json::Boolean(value) => Variable::Boolean(value),
            &Json::String(ref s) => Variable::String(s.to_string()),
            &Json::I64(n) => Variable::Number(n as f64),
            &Json::U64(n) => Variable::Number(n as f64),
            &Json::F64(f) => Variable::Number(f),
            &Json::Array(ref array) => {
                let mut result: Vec<Rc<Variable>> = vec![];
                for value in array.iter() {
                    result.push(Rc::new(Variable::from_json(value)));
                }
                Variable::Array(result)
            },
            &Json::Object(ref map) => {
                let mut result: BTreeMap<String, Rc<Variable>> = BTreeMap::new();
                for (key, value) in map.iter() {
                    result.insert(key.clone(), Rc::new(Variable::from_json(value)));
                }
                Variable::Object(result)
            }
        }
    }

    /// Create a JMESPath Variable from a JSON encoded string.
    pub fn from_str(s: &str) -> Result<Self, String> {
        Json::from_str(s)
            .map(|value| Self::from_json(&value))
            .or_else(|err| Err(format!("{:?}", err).to_string()))
    }

    /// Converts the Variable value to a JSON value.
    /// If any value in the Variable is an Expref, None is returned.
    pub fn to_json(&self) -> Option<Json> {
        match self {
            &Variable::Null => Some(Json::Null),
            &Variable::Boolean(value) => Some(Json::Boolean(value)),
            &Variable::String(ref s) => Some(Json::String(s.to_string())),
            &Variable::Number(f) => Some(Json::F64(f)),
            &Variable::Array(ref array) => {
                let mut result: Vec<Json> = vec![];
                for value in array.iter() {
                    let json_value = Variable::to_json(value);
                    if json_value.is_none() { return None };
                    result.push(json_value.unwrap());
                }
                Some(Json::Array(result))
            },
            &Variable::Object(ref map) => {
                let mut result: BTreeMap<String, Json> = BTreeMap::new();
                for (key, value) in map.iter() {
                    let json_value = Variable::to_json(value);
                    if json_value.is_none() { return None };
                    result.insert(key.clone(), json_value.unwrap());
                }
                Some(Json::Object(result))
            },
            &Variable::Expref(_) => None
        }
    }

    /// Converts the Variable value to a JSON encoded string value.
    /// If any value in the Variable is an Expref, None is returned.
    pub fn to_string(&self) -> Option<String> {
        self.to_json().map(|v| v.to_string())
    }

    /// Returns true if the Variable is an Array. Returns false otherwise.
    pub fn is_array<'a>(&'a self) -> bool {
        self.as_array().is_some()
    }

    /// If the Variable value is an Array, returns the associated vector.
    /// Returns None otherwise.
    pub fn as_array<'a>(&'a self) -> Option<&'a Vec<Rc<Variable>>> {
        match self {
            &Variable::Array(ref array) => Some(&*array),
            _ => None
        }
    }

    /// Returns true if the value is an Object.
    pub fn is_object<'a>(&'a self) -> bool {
        self.as_object().is_some()
    }

    /// If the value is an Object, returns the associated BTreeMap.
    /// Returns None otherwise.
    pub fn as_object<'a>(&'a self) -> Option<&'a BTreeMap<String, Rc<Variable>>> {
        match self {
            &Variable::Object(ref map) => Some(&*map),
            _ => None
        }
    }

    /// Returns true if the value is a String. Returns false otherwise.
    pub fn is_string(&self) -> bool {
        self.as_string().is_some()
    }

    /// If the value is a String, returns the associated str.
    /// Returns None otherwise.
    pub fn as_string(&self) -> Option<&String> {
        match *self {
            Variable::String(ref s) => Some(s),
            _ => None
        }
    }

    /// Returns true if the value is a Number. Returns false otherwise.
    pub fn is_number(&self) -> bool {
        match *self {
            Variable::Number(_) => true,
            _ => false,
        }
    }

    /// If the value is a number, return or cast it to a f64.
    /// Returns None otherwise.
    pub fn as_number(&self) -> Option<f64> {
        match *self {
            Variable::Number(f) => Some(f),
            _ => None
        }
    }

    /// Returns true if the value is a Boolean. Returns false otherwise.
    pub fn is_boolean(&self) -> bool {
        self.as_boolean().is_some()
    }

    /// If the value is a Boolean, returns the associated bool.
    /// Returns None otherwise.
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            &Variable::Boolean(b) => Some(b),
            _ => None
        }
    }

    /// Returns true if the value is a Null. Returns false otherwise.
    pub fn is_null(&self) -> bool {
        self.as_null().is_some()
    }

    /// If the value is a Null, returns ().
    /// Returns None otherwise.
    pub fn as_null(&self) -> Option<()> {
        match self {
            &Variable::Null => Some(()),
            _ => None
        }
    }

    /// Returns true if the value is an expression reference.
    /// Returns false otherwise.
    pub fn is_expref(&self) -> bool {
        self.as_expref().is_some()
    }

    /// If the value is an expression reference, returns the associated Ast node.
    /// Returns None otherwise.
    pub fn as_expref(&self) -> Option<&Ast> {
        match self {
            &Variable::Expref(ref ast) => Some(ast),
            _ => None
        }
    }

    /// Retrieves an index from the Variable if the Variable is an array.
    /// Returns None if not an array or if the index is not present.
    pub fn get_index(&self, index: usize) -> Option<Rc<Variable>> {
        self.as_array()
            .and_then(|array| array.get(index))
            .map(|value| value.clone().clone())
    }

    /// Retrieves an index from the end of a Variable if the Variable is an array.
    /// Returns None if not an array or if the index is not present.
    /// The formula for determining the index position is length - index (i.e., an
    /// index of 0 or 1 is treated as the end of the array).
    pub fn get_negative_index(&self, index: usize) -> Option<Rc<Variable>> {
        self.as_array()
            .and_then(|array| {
                let adjusted_index = max(index, 1);
                if array.len() < adjusted_index {
                    None
                } else {
                    array.get(array.len() - adjusted_index)
                }
            })
            .map(|value| value.clone().clone())
    }

    /// Retrieves a key value from a Variable if the Variable is an object.
    /// Returns None if the Variable is not an object or if the field is not present.
    pub fn get_value(&self, key: &str) -> Option<Rc<Variable>> {
        self.as_object()
            .and_then(|map| map.get(key))
            .map(|value| value.clone().clone())
    }

    /// Returns true or false based on if the Variable value is considered truthy.
    pub fn is_truthy(&self) -> bool {
        match self {
            &Variable::Boolean(true) => true,
            &Variable::String(ref s) if s.len() > 0 => true,
            &Variable::Array(ref a) if a.len() > 0 => true,
            &Variable::Object(ref o) if o.len() > 0 => true,
            &Variable::Number(_) => true,
            _ => false
        }
    }

    /// Returns the JMESPath type name of a Variable value.
    pub fn get_type(&self) -> &str {
        match self {
            &Variable::Boolean(_) => "boolean",
            &Variable::String(_) => "string",
            &Variable::Number(_) => "number",
            &Variable::Array(_) => "array",
            &Variable::Object(_) => "object",
            &Variable::Null => "null",
            &Variable::Expref(_) => "expref"
        }
    }

    /// Compares two Variable values using a comparator.
    pub fn compare(&self, cmp: &Comparator, value: &Variable) -> Option<bool> {
        match cmp {
            &Comparator::Eq => Some(*self == *value),
            &Comparator::Ne => Some(*self != *value),
            &Comparator::Lt if self.is_number() && value.is_number() => Some(*self < *value),
            &Comparator::Lte if self.is_number() && value.is_number() => Some(*self <= *value),
            &Comparator::Gt if self.is_number() && value.is_number() => Some(*self > *value),
            &Comparator::Gte if self.is_number() && value.is_number() => Some(*self >= *value),
            _ => None
        }
    }
}

/// Handles the allocation of runtime Variables.
/// Currently only used for common static values like null, true, false, etc.
/// TODO: test out and benchmark interned object key strings.
#[derive(Clone)]
pub struct VariableArena {
    true_bool: Rc<Variable>,
    false_bool: Rc<Variable>,
    null: Rc<Variable>
}

impl VariableArena {
    /// Create a new variable arena.
    pub fn new() -> VariableArena {
        VariableArena {
            true_bool: Rc::new(Variable::Boolean(true)),
            false_bool: Rc::new(Variable::Boolean(false)),
            null: Rc::new(Variable::Null)
        }
    }

    /// Allocate a boolean value using one of the shared references.
    #[inline]
    pub fn alloc_bool(&self, value: bool) -> Rc<Variable> {
        match value {
            true => self.true_bool.clone(),
            false => self.false_bool.clone()
        }
    }

    /// Allocate a null value (uses the shared null value reference).
    #[inline]
    pub fn alloc_null(&self) -> Rc<Variable> {
        self.null.clone()
    }

    /// Convenience method to allocates a Variable.
    #[inline]
    pub fn alloc<S>(&self, s: S) -> Rc<Variable> where S: IntoJMESPath {
        s.into_jmespath()
    }
}


#[cfg(test)]
mod tests {
    extern crate rustc_serialize;

    use std::rc::Rc;

    use self::rustc_serialize::json::Json;

    use super::*;
    use ast::{Ast, Comparator};

    #[test]
    fn creates_variable_from_json() {
        assert_eq!(Variable::String("Foo".to_string()),
                   Variable::from_json(&Json::String("Foo".to_string())));
        assert_eq!(Variable::Null, Variable::from_json(&Json::Null));
        assert_eq!(Variable::Boolean(false), Variable::from_json(&Json::Boolean(false)));
        assert_eq!(Variable::Number(1.0), Variable::from_json(&Json::F64(1.0)));
        let array = Variable::from_json(&Json::from_str("[1, [true]]").unwrap());
        assert!(array.is_array());
        assert_eq!(Some(Rc::new(Variable::Number(1.0))), array.get_index(0));
        let map = Variable::from_json(&Json::from_str("{\"a\": {\"b\": 1}}").unwrap());
        assert!(map.is_object());
        assert_eq!(Some(Rc::new(Variable::Number(1.0))), map.get_value("a").unwrap().get_value("b"));
    }

    #[test]
    fn creates_variable_from_str() {
        assert_eq!(Ok(Variable::Boolean(true)), Variable::from_str("true"));
        assert_eq!(Err("SyntaxError(\"invalid syntax\", 1, 1)".to_string()),
                   Variable::from_str("abc"));
    }

    #[test]
    fn test_determines_types() {
         assert_eq!("object", Variable::from_str(&"{\"foo\": \"bar\"}").unwrap().get_type());
         assert_eq!("array", Variable::from_str(&"[\"foo\"]").unwrap().get_type());
         assert_eq!("null", Variable::Null.get_type());
         assert_eq!("boolean", Variable::Boolean(true).get_type());
         assert_eq!("string", Variable::String("foo".to_string()).get_type());
         assert_eq!("number", Variable::Number(10.0).get_type());
    }

    #[test]
    fn test_is_truthy() {
        assert_eq!(true, Variable::from_str(&"{\"foo\": \"bar\"}").unwrap().is_truthy());
        assert_eq!(false, Variable::from_str(&"{}").unwrap().is_truthy());
        assert_eq!(true, Variable::from_str(&"[\"foo\"]").unwrap().is_truthy());
        assert_eq!(false, Variable::from_str(&"[]").unwrap().is_truthy());
        assert_eq!(false, Variable::Null.is_truthy());
        assert_eq!(true, Variable::Boolean(true).is_truthy());
        assert_eq!(false, Variable::Boolean(false).is_truthy());
        assert_eq!(true, Variable::String("foo".to_string()).is_truthy());
        assert_eq!(false, Variable::String("".to_string()).is_truthy());
        assert_eq!(true, Variable::Number(10.0).is_truthy());
        assert_eq!(true, Variable::Number(0.0).is_truthy());
    }

    #[test]
    fn test_eq_ne_compare() {
        let l = Variable::String("foo".to_string());
        let r = Variable::String("foo".to_string());
        assert_eq!(Some(true), l.compare(&Comparator::Eq, &r));
        assert_eq!(Some(false), l.compare(&Comparator::Ne, &r));
    }

    #[test]
    fn test_compare() {
        let invalid = Variable::String("foo".to_string());
        let l = Variable::Number(10.0);
        let r = Variable::Number(20.0);
        assert_eq!(None, invalid.compare(&Comparator::Gt, &r));
        assert_eq!(Some(false), l.compare(&Comparator::Gt, &r));
        assert_eq!(Some(false), l.compare(&Comparator::Gte, &r));
        assert_eq!(Some(true), r.compare(&Comparator::Gt, &l));
        assert_eq!(Some(true), r.compare(&Comparator::Gte, &l));
        assert_eq!(Some(true), l.compare(&Comparator::Lt, &r));
        assert_eq!(Some(true), l.compare(&Comparator::Lte, &r));
        assert_eq!(Some(false), r.compare(&Comparator::Lt, &l));
        assert_eq!(Some(false), r.compare(&Comparator::Lte, &l));
    }

    #[test]
    fn gets_value_from_object() {
        let var = Variable::from_str("{\"foo\":1}").unwrap();
        assert_eq!(Some(Rc::new(Variable::Number(1.0))), var.get_value("foo"));
    }

    #[test]
    fn getting_value_from_non_object_is_none() {
        assert_eq!(None, Variable::Boolean(false).get_value("foo"));
    }

    #[test]
    fn getting_index_from_non_array_is_none() {
        assert_eq!(None, Variable::Boolean(false).get_index(2));
    }

    #[test]
    fn gets_index_from_array() {
        let var = Variable::from_str("[1, 2, 3]").unwrap();
        assert_eq!(Some(Rc::new(Variable::Number(1.0))), var.get_index(0));
        assert_eq!(Some(Rc::new(Variable::Number(2.0))), var.get_index(1));
        assert_eq!(Some(Rc::new(Variable::Number(3.0))), var.get_index(2));
        assert_eq!(None, var.get_index(3));
    }

    #[test]
    fn getting_negative_index_from_non_array_is_none() {
        assert_eq!(None, Variable::Boolean(false).get_negative_index(2));
    }

    #[test]
    fn gets_negative_index_from_array() {
        let var = Variable::from_str("[1, 2, 3]").unwrap();
        assert_eq!(Some(Rc::new(Variable::Number(3.0))), var.get_negative_index(0));
        assert_eq!(Some(Rc::new(Variable::Number(3.0))), var.get_negative_index(1));
        assert_eq!(Some(Rc::new(Variable::Number(2.0))), var.get_negative_index(2));
        assert_eq!(Some(Rc::new(Variable::Number(1.0))), var.get_negative_index(3));
        assert_eq!(None, var.get_negative_index(4));
    }

    #[test]
    fn determines_if_null() {
        assert_eq!(false, Variable::Boolean(true).is_null());
        assert_eq!(true, Variable::Null.is_null());
    }

    #[test]
    fn option_of_null() {
        assert_eq!(Some(()), Variable::Null.as_null());
        assert_eq!(None, Variable::Boolean(true).as_null());
    }

    #[test]
    fn determines_if_boolean() {
        assert_eq!(true, Variable::Boolean(true).is_boolean());
        assert_eq!(false, Variable::Null.is_boolean());
    }

    #[test]
    fn option_of_boolean() {
        assert_eq!(Some(true), Variable::Boolean(true).as_boolean());
        assert_eq!(None, Variable::Null.as_boolean());
    }

    #[test]
    fn determines_if_string() {
        assert_eq!(false, Variable::Boolean(true).is_string());
        assert_eq!(true, Variable::String("foo".to_string()).is_string());
    }

    #[test]
    fn option_of_string() {
        assert_eq!(Some(&"foo".to_string()), Variable::String("foo".to_string()).as_string());
        assert_eq!(None, Variable::Null.as_string());
    }

    #[test]
    fn test_is_number() {
        let value = Variable::from_str("12").unwrap();
        assert!(value.is_number());
    }

    #[test]
    fn test_as_number() {
        let value = Variable::from_str("12.0").unwrap();
        assert_eq!(value.as_number(), Some(12f64));
    }

    #[test]
    fn test_is_object() {
        let value = Variable::from_str("{}").unwrap();
        assert!(value.is_object());
    }

    #[test]
    fn test_as_object() {
        let value = Variable::from_str("{}").unwrap();
        assert!(value.as_object().is_some());
    }

    #[test]
    fn test_is_array() {
        let value = Variable::from_str("[1, 2, 3]").unwrap();
        assert!(value.is_array());
    }

    #[test]
    fn test_as_array() {
        let value = Variable::from_str("[1, 2, 3]").unwrap();
        let array = value.as_array();
        let expected_length = 3;
        assert!(array.is_some() && array.unwrap().len() == expected_length);
    }

    #[test]
    fn test_converts_to_json() {
        let test_cases = vec![
            "true", "false", "{}", "[]", "0.0", "null",
            "[1.0,2.0]", "{\"foo\":[true,false,-5.0],\"bar\":null}"];
        for case in test_cases {
            let var = Variable::from_str(case).unwrap();
            assert_eq!(Json::from_str(case).unwrap(), var.to_json().unwrap());
        }
    }

    #[test]
    fn test_converting_to_json_with_expref_returns_none() {
        let var = Variable::Expref(Ast::CurrentNode);
        assert!(var.to_json().is_none());
    }

    #[test]
    fn test_converts_to_string() {
        assert_eq!("true", Variable::Boolean(true).to_string().unwrap());
    }

    #[test]
    fn test_is_expref() {
        assert_eq!(true, Variable::Expref(Ast::CurrentNode).is_expref());
        assert_eq!(&Ast::CurrentNode, Variable::Expref(Ast::CurrentNode).as_expref().unwrap());
    }
}
