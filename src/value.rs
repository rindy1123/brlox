use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    Nil,
    Number(f64),
    LString(String),
}

impl Value {
    fn as_bool(&self) -> bool {
        match self {
            Value::Bool(boolean) => *boolean,
            _ => panic!("Not boolean"),
        }
    }

    pub fn as_number(&self) -> f64 {
        match self {
            Value::Number(number) => *number,
            _ => panic!("Not number"),
        }
    }

    pub fn values_equal(self, b: Self) -> bool {
        match (self, b) {
            (Value::Bool(boolean1), Value::Bool(boolean2)) => boolean1 == boolean2,
            (Value::Nil, Value::Nil) => true,
            (Value::Number(num1), Value::Number(num2)) => num1 == num2,
            (_, _) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
