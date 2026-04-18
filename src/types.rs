use std::collections::hash_map::HashMap;
use ordered_float::OrderedFloat;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Long,
    UInt,
    ULong,
    Double,
    Pointer(Box<Type>),
    FuncType{params: Vec<Box<Type>>, ret: Box<Type>},
    Array(Box<Type>, i64),
}

pub static ARITHMETIC_TYPES: &[Type] = &[Type::Int, Type::Long,
                                     Type::UInt, Type::ULong,
                                     Type::Double];

pub static INTEGER_TYPES: &[Type] = &[Type::Int, Type::Long,
                                  Type::UInt, Type::ULong];

impl Type {
    pub fn size(&self) -> usize {
        match self {
            Type::Int => 32,
            Type::UInt => 32,
            Type::Long => 64,
            Type::ULong => 64,
            Type::Pointer(_) => 64,
            Type::Double => 64,
            Type::FuncType { .. } => unreachable!(),
            Type::Array(t, s) => t.size() * *s as usize,
        }
    }

    pub fn is_signed(&self) -> bool {
        match self {
            Type::Int => true,
            Type::UInt => false,
            Type::Long => true,
            Type::ULong => false,
            Type::Pointer(_) => false,
            Type::Double => unreachable!(),
            Type::FuncType { .. } => unreachable!(),
            Type::Array(_, _) => unreachable!(),
        }
    }

    pub fn is_pointer(&self) -> bool {
        matches!(self, Type::Pointer(_))
    }

    pub fn is_double(&self) -> bool {
        matches!(self, Type::Double)
    }

    pub fn is_array(&self) -> bool {
        matches!(self, Type::Array(_, _))
    }

    pub fn is_arithmetic(&self) -> bool {
        ARITHMETIC_TYPES.contains(&self)
    }

    pub fn is_integer(&self) -> bool {
        INTEGER_TYPES.contains(&self)
    }

    pub fn is_scalar(&self) -> bool {
        self.is_arithmetic() || self.is_pointer()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub ident: String,
    pub datatype: Type,
    pub attrs: IdentAttrs,
}

pub type SymbolTable = HashMap<String, Symbol>;

#[derive(Debug, Clone, PartialEq)]
pub enum IdentAttrs {
    FuncAttr{defined: bool, global: bool},
    StaticAttr{init: InitialValue, global: bool},
    LocalAttr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InitialValue {
    Tentative,
    Initial(StaticInit),
    NoInitializer,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StaticInit {
    IntInit(i32),
    LongInit(i64),
    UIntInit(u32),
    ULongInit(u64),
    DoubleInit(OrderedFloat<f64>),
}

impl IdentAttrs {
    pub fn is_global(&self) -> bool {
        match &self {
            IdentAttrs::StaticAttr { init:_ , global } => *global,
            IdentAttrs::FuncAttr { defined:_ , global } => *global,
            IdentAttrs::LocalAttr => false,
        }
    }
}

