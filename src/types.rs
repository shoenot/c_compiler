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
}

impl Type {
    pub fn size(&self) -> usize {
        match self {
            Type::Int => 32,
            Type::UInt => 32,
            Type::Long => 64,
            Type::ULong => 64,
            Type::Pointer(_) => 64,
            Type::Double => unreachable!(),
            Type::FuncType { .. } => unreachable!(),
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
        }
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

