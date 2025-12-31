use std::collections::BTreeMap;
use std::rc::Rc;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sign{
    Signed,
    Unsigned
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumericType {
    Short(Sign),
    Int(Sign),
    Long(Sign),
    LongLong(Sign),
    Float,
    Double,
    LongDouble
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageClass {
    Auto,
    Register,
    Static,
    Extern,
    Typedef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TypeQualifiers {
    pub is_const : bool,
    pub is_volatile : bool,
    pub is_restrict : bool
}

impl TypeQualifiers {
    pub fn none() -> Self {
        Self::default()
    }
    pub fn const_qualified() -> Self {
        Self { is_const : true, ..Default::default()}
    }
    pub fn cv_qualified() -> Self {
        Self { is_const : true, is_volatile : true, is_restrict : false}
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Type {
    #[default]
    Void,
    Char(Sign),
    Number(NumericType),
    Pointer {
        pointee : Rc<Type>,
        qualifiers : TypeQualifiers,
    },
    Array{
        element_type : Rc<Type>,
        count : Option<usize>
    },
    Function {
        return_type : Rc<Type>,
        params : Vec<Parameter>,
        is_variadic : bool
    },

    Struct {
        name : Option<String>,
        fields : Option<Vec<StructField>>
    },

    Union {
        name : Option<String>,
        fields : Option<Vec<StructField>>
    },

    Enum {
        name : Option<String>,
        variants : Option<BTreeMap<String, i64>>
    },

    Typedef {
        name : String,
        resolved : Rc<Type>
    },

    Qualified {
        base : Rc<Type>,
        qualifiers : TypeQualifiers
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Parameter {
    pub name : Option<String>,
    pub param_type : Rc<Type>
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct StructField {
    pub name : String,
    pub field_type : Rc<Type>,
    pub bit_field : Option<usize>,
}

impl Type {
    pub fn is_pointer(&self) -> bool {
        matches!(self, Type::Pointer { .. })
    }

    pub fn is_function(&self) -> bool {
        matches!(self, Type::Function { .. })
    }

    pub fn is_void(&self) -> bool {
        matches!(self, Type::Void)
    }

    pub fn strip_qualifiers(&self) -> &Type {
        match self{
            Type::Qualified { base, .. } => base.strip_qualifiers(),
            _ => self,
        }
    }
    pub fn get_qualifiers(&self) -> TypeQualifiers {
        match self {
            Type::Qualified { qualifiers, .. } => qualifiers.clone(),
            Type::Pointer { qualifiers, .. } => qualifiers.clone(),
            _ => TypeQualifiers::none()
        }
    }

    pub fn resolve_typedef(&self) -> &Type {
        match self{
            Type::Typedef { resolved, .. } => resolved.resolve_typedef(),
            Type::Qualified { base, .. } => base.resolve_typedef(),
            _ => self
        }
    }
}

impl Type {

    pub fn int() -> Rc<Type> {
        Rc::new(Type::Number(NumericType::Int(Sign::Signed)))
    }

    pub fn unsigned_int() -> Rc<Type> {
        Rc::new(Type::Number(NumericType::Int(Sign::Unsigned)))
    }

    pub fn void() -> Rc<Type> {
        Rc::new(Type::Void)
    }

    pub fn pointer_to(pointee : Rc<Type>, qualifiers : &TypeQualifiers) -> Rc<Type> {
        Rc::new(Type::Pointer { pointee, qualifiers : qualifiers.clone() })
    }

    pub fn const_pointer_to(pointee : Rc<Type>) -> Rc<Type> {
        Self::pointer_to(pointee, &TypeQualifiers{ is_const : true, ..Default::default() })
    }

}

