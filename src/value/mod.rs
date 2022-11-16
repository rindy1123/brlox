use std::fmt::Debug;

use self::object::Obj;

pub mod object;

#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    Nil,
    Number(f64),
    LString(String),
    Obj(Obj),
}

impl Value {
    pub fn as_number(&self) -> f64 {
        match self {
            Value::Number(number) => *number,
            _ => panic!("Not number"),
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            Value::LString(string) => string.to_string(),
            _ => panic!("Not number"),
        }
    }

    pub fn values_equal(&self, b: Self) -> bool {
        match (self, b) {
            (Value::Bool(boolean1), Value::Bool(boolean2)) => boolean1.to_owned() == boolean2,
            (Value::Nil, Value::Nil) => true,
            (Value::Number(num1), Value::Number(num2)) => num1.to_owned() == num2,
            (Value::LString(str1), Value::LString(str2)) => str1.to_owned() == str2,
            (_, _) => false,
        }
    }

    pub fn println(&self) {
        println!("{}", self.to_string());
    }

    fn to_string(&self) -> String {
        match self {
            Self::Bool(boolean) => boolean.to_string(),
            Self::Nil => "nil".to_string(),
            Self::Number(num) => num.to_string(),
            Self::LString(string) => string.to_string(),
            Self::Obj(obj) => obj.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_string() {
        let boolean = Value::Bool(false);
        assert_eq!(boolean.to_string(), "false");
        let nil = Value::Nil;
        assert_eq!(nil.to_string(), "nil");
        let num = Value::Number(1.0);
        assert_eq!(num.to_string(), "1");
        let num = Value::Number(1.5);
        assert_eq!(num.to_string(), "1.5");
        let string = Value::LString("ABC".to_string());
        assert_eq!(string.to_string(), "ABC");
    }

    #[test]
    fn test_values_equal_bool() {
        let bool1 = Value::Bool(false);
        let bool2 = Value::Bool(false);
        assert!(bool1.values_equal(bool2));
        let bool3 = Value::Bool(true);
        assert!(!bool1.values_equal(bool3));
    }

    #[test]
    fn test_values_equal_nil() {
        let nil1 = Value::Nil;
        let nil2 = Value::Nil;
        assert!(nil1.values_equal(nil2));
    }

    #[test]
    fn test_values_equal_number() {
        let num1 = Value::Number(1.0);
        let num2 = Value::Number(1.0);
        assert!(num1.values_equal(num2));
        let num3 = Value::Number(3.0);
        assert!(!num1.values_equal(num3));
    }

    #[test]
    fn test_values_equal_string() {
        let str1 = Value::LString("AAA".to_string());
        let str2 = Value::LString("AAA".to_string());
        assert!(str1.values_equal(str2));
        let str3 = Value::LString("BBB".to_string());
        assert!(!str1.values_equal(str3));
    }

    #[test]
    fn test_values_equal_others() {
        let num = Value::Number(1.0);
        let nil = Value::Nil;
        assert!(!num.values_equal(nil));
    }

    #[test]
    fn test_as_number() {
        let value = Value::Number(3.0);
        assert_eq!(value.as_number(), 3.0)
    }

    #[test]
    #[should_panic]
    fn test_as_number_failure() {
        let value = Value::Nil;
        value.as_number();
    }
}
