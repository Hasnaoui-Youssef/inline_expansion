use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Signature {
    pub name : String,
    pub return_type : String,
    pub args : Vec<Parameter>,
    pub is_variadic : bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Parameter {
    pub name : Option<String>,
    pub param_type : String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Definition {
    pub signature : Signature,
    pub body : String,
    pub source_file : PathBuf,
    pub is_static : bool

}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Call{
    pub function_name : String,
    pub args : Vec<String>,
    pub assigned_to : Option<String>,
    pub line_number : usize,
}

#[derive(Debug, Clone, Default)]
pub struct FunctionDatabase {
    functions : HashMap<String, Definition>,
}

impl FunctionDatabase {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_function(&mut self, def : Definition){
        self.functions.insert(def.signature.name.clone(), def);
    }

    pub fn get_function_definition(&self, name : &str) -> Option<&Definition> {
        self.functions.get(name)
    }
    pub fn iter_cloned(&self) -> impl Iterator<Item = Definition> {
        self.functions.values().cloned()
    }

    pub fn iter(&self) -> impl Iterator<Item = Definition> {
        self.functions.values()
    }

    pub fn merge(&mut self, other : FunctionDatabase, mut resolve : F)
        where F : FnMut(Definition, Definition) -> Definition,
    {
        use std::collections::hash_map::Entry;
        for (k, v) in other.functions {
            match self.functions.entry(k) {
                Entry::Vacant(e) => {
                    e.insert(v);
                }
                Entry::Occupied(mut e) => {
                    let old = e.insert(v);
                    let new = resolve(old, v.clone());
                    e.insert(new);
                }
            }
        }
    }

}


