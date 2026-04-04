use std::collections::HashMap;

use crate::types::*;
use crate::codegen::AsmType;

#[derive(Debug, Clone, PartialEq)]
pub enum AsmSymbol {
    ObjEntry(AsmType, bool),
    FuncEntry(bool),
}

pub type AsmSymbolTable = HashMap<String, AsmSymbol>;

pub fn convert_type(dtype: &Type) -> AsmType {
    match dtype {
        Type::Int => AsmType::Longword,
        Type::Long => AsmType::Quadword,
        Type::UInt => AsmType::Longword,
        Type::ULong => AsmType::Quadword,
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
                AsmSymbol::ObjEntry(asmtype, true)
            } else {
                AsmSymbol::ObjEntry(asmtype, false)
            }
        },
    }
}

pub fn convert_symtable(symbols: &SymbolTable, asm_symbols: &mut AsmSymbolTable) {
    for (_, sym) in symbols {
        asm_symbols.insert(sym.ident.clone(), convert_symbol(sym));
    }
}
