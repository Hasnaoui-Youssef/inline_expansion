use std::path::PathBuf;

use crate::ast::core::Type;

#[derive(Debug, Clone)]
pub struct Signature {
    pub name : String,
    pub signature : Type
}

#[derive(Debug, Clone)]
pub struct Definition {
    pub signature : Signature,
    pub body : String,
    pub source_file : PathBuf,
    pub is_static : bool
}

#[derive(Debug, Clone)]
pub struct Call{
    pub function_name : String,
    pub args : Vec<String>,
    pub assigned_to : Option<String>,
    pub line_number : usize
}
