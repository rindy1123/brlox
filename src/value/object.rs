use crate::chunk::Chunk;

use super::Value;

#[derive(Debug, Clone)]
pub enum Obj {
    Function(ObjFunction),
    NativeFunction(ObjNative),
}

impl Obj {
    pub fn to_string(&self) -> String {
        match self {
            Self::Function(function) => format!("<fn {}>", function.name),
            Self::NativeFunction(_) => "<native fn>".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjNative {
    pub native_function: NativeFunction,
}

pub type NativeFunction = fn(arg_count: usize, ip: usize) -> Value;

impl ObjNative {
    pub fn new(native_function: NativeFunction) -> ObjNative {
        ObjNative { native_function }
    }
}

#[derive(Debug, Clone)]
pub struct ObjFunction {
    pub name: String,
    pub chunk: Chunk,
    pub arity: usize,
}

impl ObjFunction {
    pub fn new() -> ObjFunction {
        ObjFunction {
            name: String::new(),
            chunk: Chunk::new(),
            arity: 0,
        }
    }
}
