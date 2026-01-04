use std::{collections::HashMap, path::PathBuf};
use std::sync::Arc;

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
    pub is_static : bool,
    pub calls : Vec<CallInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum CallContext {
    #[default]
    Sequential,
    /// Inside an if/else-if condition or body, with branch index
    Conditional { branch_id: u32 },
    Loop,
    Switch { case_id: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct CallInfo {
    pub function_name : String,
    pub line : u32,
    pub column : u32,
    pub order: u32,
    pub context: CallContext,
    pub context_depth: u32,
}

#[derive(Debug, Clone, Default)]
pub struct FunctionDatabase{
    functions : HashMap<String, Arc<Definition>>,
}

impl FunctionDatabase {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_function(&mut self, def : Arc<Definition>){
        self.functions.insert(def.signature.name.clone(), def);
    }
    pub fn add_function_ref(&mut self, def : &Definition){
        self.add_function(Arc::new(def.clone()));
    }

    pub fn get_function_definition(&self, name : & str) -> Option<Arc<Definition>> {
        self.functions.get(name).cloned()
    }

    pub fn clear(&mut self) {
        self.functions.clear();
        self.functions.shrink_to(0);
    }

    pub fn iter(&self) -> impl Iterator<Item = Arc<Definition>> + '_ {
        self.functions.values().cloned()
    }

}


