use std::collections::HashMap;

use crate::types::*;
use crate::codegen::{AsmTopLevel, AsmType};

#[derive(Debug, Clone, PartialEq)]
pub enum AsmSymbol {
    ObjEntry(AsmType, bool, bool),
    FuncEntry(bool),
}

pub type AsmSymbolTable = HashMap<String, AsmSymbol>;

pub fn convert_type(dtype: &Type) -> AsmType {
    match dtype {
        Type::Int => AsmType::Longword,
        Type::Long => AsmType::Quadword,
        Type::UInt => AsmType::Longword,
        Type::ULong => AsmType::Quadword,
        Type::Double => AsmType::Double,
        _ => unreachable!(),
    }
}

fn convert_symbol(symbol: &Symbol) -> AsmSymbol {
    match &symbol.datatype {
        Type::FuncType { .. } => {
            let IdentAttrs::FuncAttr { defined, global:_ } = symbol.attrs else { unreachable!() };
            AsmSymbol::FuncEntry(defined)
        },
        other => {
            let asmtype = convert_type(&other);
            if let IdentAttrs::StaticAttr { .. } = symbol.attrs {
                AsmSymbol::ObjEntry(asmtype, true, false)
            } else {
                AsmSymbol::ObjEntry(asmtype, false, false)
            }
        },
    }
}

pub fn convert_symtable(symbols: &SymbolTable, asm_symbols: &mut AsmSymbolTable, constants: &Vec<AsmTopLevel>) {
    for (_, sym) in symbols {
        asm_symbols.insert(sym.ident.clone(), convert_symbol(sym));
    }
    for item in constants {
        let AsmTopLevel::C(constant) = item else { unreachable!() };
        let asmsym = AsmSymbol::ObjEntry(AsmType::Double, true, true);
        asm_symbols.insert(constant.identifier.clone(), asmsym);
    }
}
