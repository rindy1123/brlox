use crate::chunk::Chunk;

#[derive(Debug, Clone)]
pub enum Obj {
    Function(ObjFunction),
}

impl Obj {
    pub fn to_string(&self) -> String {
        match self {
            Self::Function(function) => format!("<fn {}>", function.name),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjFunction {
    name: String,
    pub chunk: Chunk,
    arity: usize,
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
