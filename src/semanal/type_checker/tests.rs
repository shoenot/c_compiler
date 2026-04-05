//! Expression type-checking tests.
//!
//! These tests exercise `TypeChecker::type_expression` in isolation. Each test
//! follows a build / run / inspect pattern:
//!   1. Build an `Expression` (and any needed `SymbolTable` entries) using the
//!      builder methods at the bottom of this file.
//!   2. Run the type checker on it via `check(&mut expr, &mut symbols)`.
//!   3. Inspect the now-mutated expression and assert on its annotated type
//!      and structural shape (e.g. inserted Cast nodes).
//!
//! Tests are organized by language feature, not by type-checker function.

use super::*;
use crate::parser::ast::*;
use crate::lexer::Span;
use crate::types::*;
use ordered_float::OrderedFloat;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------

/// Run the type checker on a single expression with the given symbol table.
/// Mutates `expr` in place.
fn check(expr: &mut Expression, symbols: &mut SymbolTable) -> Result<(), SemanticError> {
    let mut checker = TypeChecker {
        symbols,
        scope_depth: 1,
        current_function_type: None,
    };
    checker.type_expression(expr)?;
    Ok(())
}

/// Convenience: run the type checker with an empty symbol table.
fn check_empty(expr: &mut Expression) -> Result<(), SemanticError> {
    let mut symbols = SymbolTable::new();
    check(expr, &mut symbols)
}

/// Build a SymbolTable pre-populated with local variables of the given types.
fn symbols_with(vars: &[(&str, Type)]) -> SymbolTable {
    let mut t = SymbolTable::new();
    for (name, ty) in vars {
        t.insert(
            name.to_string(),
            Symbol {
                ident: name.to_string(),
                datatype: ty.clone(),
                attrs: IdentAttrs::LocalAttr,
            },
        );
    }
    t
}

/// Build a SymbolTable with a function symbol.
fn symbols_with_func(name: &str, params: Vec<Type>, ret: Type) -> SymbolTable {
    let mut t = SymbolTable::new();
    let func_type = Type::FuncType {
        params: params.into_iter().map(Box::new).collect(),
        ret: Box::new(ret),
    };
    t.insert(
        name.to_string(),
        Symbol {
            ident: name.to_string(),
            datatype: func_type,
            attrs: IdentAttrs::FuncAttr {
                defined: true,
                global: true,
            },
        },
    );
    t
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

#[test]
fn constant_int_annotates_as_int() {
    let mut e = Expression::int_const(5);
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Int));
}

#[test]
fn constant_long_annotates_as_long() {
    let mut e = Expression::long_const(5);
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Long));
}

#[test]
fn constant_uint_annotates_as_uint() {
    let mut e = Expression::uint_const(5);
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::UInt));
}

#[test]
fn constant_ulong_annotates_as_ulong() {
    let mut e = Expression::ulong_const(5);
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::ULong));
}

#[test]
fn constant_double_annotates_as_double() {
    let mut e = Expression::double_const(3.14);
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Double));
}

// ---------------------------------------------------------------------------
// Variables
// ---------------------------------------------------------------------------

#[test]
fn var_lookup_annotates_with_symbol_type() {
    let mut symbols = symbols_with(&[("x", Type::Long)]);
    let mut e = Expression::var("x");
    check(&mut e, &mut symbols).unwrap();
    assert_eq!(e.expression_type, Some(Type::Long));
}

#[test]
fn var_referring_to_function_is_error() {
    let mut symbols = symbols_with_func("f", vec![Type::Int], Type::Int);
    let mut e = Expression::var("f");
    let err = check(&mut e, &mut symbols).unwrap_err();
    assert!(matches!(err, SemanticError::FuncUsedAsVar(name, _) if name == "f"));
}

// ---------------------------------------------------------------------------
// Cast
// ---------------------------------------------------------------------------

#[test]
fn cast_annotates_with_target_type() {
    let mut e = Expression::cast(Type::Long, Expression::int_const(5));
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Long));
}

#[test]
fn cast_recursively_annotates_inner() {
    let mut e = Expression::cast(Type::Long, Expression::int_const(5));
    check_empty(&mut e).unwrap();
    let inner = e.expect_cast_inner();
    assert_eq!(inner.expression_type, Some(Type::Int));
}

// ---------------------------------------------------------------------------
// Unary operators
// ---------------------------------------------------------------------------

#[test]
fn unary_negate_preserves_int_type() {
    let mut e = Expression::unary(UnaryOp::Negate, Expression::int_const(5));
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Int));
}

#[test]
fn unary_negate_preserves_double_type() {
    let mut e = Expression::unary(UnaryOp::Negate, Expression::double_const(3.14));
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Double));
}

#[test]
fn unary_complement_preserves_int_type() {
    let mut e = Expression::unary(UnaryOp::Complement, Expression::int_const(5));
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Int));
}

#[test]
fn unary_complement_on_double_is_error() {
    let mut e = Expression::unary(UnaryOp::Complement, Expression::double_const(3.14));
    let err = check_empty(&mut e).unwrap_err();
    assert!(matches!(err, SemanticError::ComplementFloat(_)));
}

#[test]
fn unary_not_returns_int_for_int_operand() {
    let mut e = Expression::unary(UnaryOp::Not, Expression::int_const(5));
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Int));
}

#[test]
fn unary_not_returns_int_for_double_operand() {
    let mut e = Expression::unary(UnaryOp::Not, Expression::double_const(3.14));
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Int));
}

// ---------------------------------------------------------------------------
// Binary: same-type arithmetic (no conversion expected)
// ---------------------------------------------------------------------------

#[test]
fn binary_int_plus_int_is_int_no_casts() {
    let mut e = Expression::binop(
        BinaryOp::Add,
        Expression::int_const(1),
        Expression::int_const(2),
    );
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Int));
    let (lhs, rhs) = e.expect_binary_operands();
    assert!(!matches!(lhs.kind, ExpressionKind::Cast(..)));
    assert!(!matches!(rhs.kind, ExpressionKind::Cast(..)));
}

#[test]
fn binary_double_plus_double_is_double_no_casts() {
    let mut e = Expression::binop(
        BinaryOp::Add,
        Expression::double_const(1.0),
        Expression::double_const(2.0),
    );
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Double));
    let (lhs, rhs) = e.expect_binary_operands();
    assert!(!matches!(lhs.kind, ExpressionKind::Cast(..)));
    assert!(!matches!(rhs.kind, ExpressionKind::Cast(..)));
}

// ---------------------------------------------------------------------------
// Binary: arithmetic with promotion (cast nodes should be inserted)
// ---------------------------------------------------------------------------

#[test]
fn binary_int_plus_long_promotes_int_to_long() {
    let mut e = Expression::binop(
        BinaryOp::Add,
        Expression::int_const(1),
        Expression::long_const(2),
    );
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Long));
    let (lhs, rhs) = e.expect_binary_operands();
    // lhs should be wrapped in a Cast to Long
    let inner = lhs.expect_cast_to(&Type::Long);
    assert!(matches!(inner.kind, ExpressionKind::Constant(Const::Int(1))));
    // rhs should be untouched
    assert!(!matches!(rhs.kind, ExpressionKind::Cast(..)));
    assert_eq!(rhs.expression_type, Some(Type::Long));
}

#[test]
fn binary_int_plus_double_promotes_int_to_double() {
    let mut e = Expression::binop(
        BinaryOp::Add,
        Expression::int_const(1),
        Expression::double_const(2.0),
    );
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Double));
    let (lhs, rhs) = e.expect_binary_operands();
    lhs.expect_cast_to(&Type::Double);
    assert!(!matches!(rhs.kind, ExpressionKind::Cast(..)));
}

#[test]
fn binary_int_plus_uint_same_size_uses_unsigned() {
    // get_common_type rule: same size, one signed and one unsigned -> unsigned wins.
    let mut e = Expression::binop(
        BinaryOp::Add,
        Expression::int_const(1),
        Expression::uint_const(2),
    );
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::UInt));
    let (lhs, _rhs) = e.expect_binary_operands();
    lhs.expect_cast_to(&Type::UInt);
}

// ---------------------------------------------------------------------------
// Binary: comparisons (always Int, but operands still promoted)
// ---------------------------------------------------------------------------

#[test]
fn comparison_returns_int_regardless_of_operand_types() {
    let mut e = Expression::binop(
        BinaryOp::LessThan,
        Expression::double_const(1.0),
        Expression::double_const(2.0),
    );
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Int));
}

#[test]
fn comparison_promotes_operands_to_common_type() {
    let mut e = Expression::binop(
        BinaryOp::LessThan,
        Expression::int_const(1),
        Expression::long_const(2),
    );
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Int));
    let (lhs, _rhs) = e.expect_binary_operands();
    lhs.expect_cast_to(&Type::Long);
}

// ---------------------------------------------------------------------------
// Binary: logical and/or (always Int, operands NOT promoted)
// ---------------------------------------------------------------------------

#[test]
fn logical_and_returns_int() {
    let mut e = Expression::binop(
        BinaryOp::LogicalAnd,
        Expression::int_const(1),
        Expression::long_const(2),
    );
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Int));
}

#[test]
fn logical_or_does_not_promote_operands() {
    let mut e = Expression::binop(
        BinaryOp::LogicalOr,
        Expression::int_const(1),
        Expression::long_const(2),
    );
    check_empty(&mut e).unwrap();
    let (lhs, rhs) = e.expect_binary_operands();
    // Neither operand should have been wrapped in a cast.
    assert!(!matches!(lhs.kind, ExpressionKind::Cast(..)));
    assert!(!matches!(rhs.kind, ExpressionKind::Cast(..)));
    assert_eq!(lhs.expression_type, Some(Type::Int));
    assert_eq!(rhs.expression_type, Some(Type::Long));
}

// ---------------------------------------------------------------------------
// Binary: shifts (result type = lhs type, no promotion)
// ---------------------------------------------------------------------------

#[test]
fn left_shift_result_type_is_lhs_type() {
    let mut e = Expression::binop(
        BinaryOp::LeftShift,
        Expression::long_const(1),
        Expression::int_const(2),
    );
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Long));
    let (lhs, rhs) = e.expect_binary_operands();
    assert!(!matches!(lhs.kind, ExpressionKind::Cast(..)));
    assert!(!matches!(rhs.kind, ExpressionKind::Cast(..)));
}

#[test]
fn right_shift_result_type_is_lhs_type() {
    let mut e = Expression::binop(
        BinaryOp::RightShift,
        Expression::int_const(1),
        Expression::long_const(2),
    );
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Int));
}

// ---------------------------------------------------------------------------
// Binary: remainder on doubles is an error
// ---------------------------------------------------------------------------

#[test]
fn remainder_on_doubles_is_error() {
    let mut e = Expression::binop(
        BinaryOp::Remainder,
        Expression::double_const(1.0),
        Expression::double_const(2.0),
    );
    let err = check_empty(&mut e).unwrap_err();
    assert!(matches!(err, SemanticError::RemainderFloat(_)));
}

#[test]
fn remainder_on_ints_is_ok() {
    let mut e = Expression::binop(
        BinaryOp::Remainder,
        Expression::int_const(5),
        Expression::int_const(3),
    );
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Int));
}

// ---------------------------------------------------------------------------
// Assignment
// ---------------------------------------------------------------------------

#[test]
fn assignment_result_type_is_lhs_type() {
    let mut symbols = symbols_with(&[("x", Type::Long)]);
    let mut e = Expression::assign(Expression::var("x"), Expression::int_const(5));
    check(&mut e, &mut symbols).unwrap();
    assert_eq!(e.expression_type, Some(Type::Long));
}

#[test]
fn assignment_converts_rhs_to_lhs_type() {
    let mut symbols = symbols_with(&[("x", Type::Long)]);
    let mut e = Expression::assign(Expression::var("x"), Expression::int_const(5));
    check(&mut e, &mut symbols).unwrap();
    let (_lhs, rhs) = e.expect_assignment_operands();
    rhs.expect_cast_to(&Type::Long);
}

#[test]
fn assignment_to_function_call_is_error() {
    let mut symbols = symbols_with_func("f", vec![], Type::Int);
    let mut e = Expression::assign(
        Expression::call("f", vec![]),
        Expression::int_const(5),
    );
    let err = check(&mut e, &mut symbols).unwrap_err();
    assert!(matches!(err, SemanticError::FuncUsedAsVar(name, _) if name == "f"));
}

// ---------------------------------------------------------------------------
// Function calls
// ---------------------------------------------------------------------------

#[test]
fn function_call_result_type_is_return_type() {
    let mut symbols = symbols_with_func("f", vec![Type::Int], Type::Long);
    let mut e = Expression::call("f", vec![Expression::int_const(5)]);
    check(&mut e, &mut symbols).unwrap();
    assert_eq!(e.expression_type, Some(Type::Long));
}

#[test]
fn function_call_converts_args_to_param_types() {
    let mut symbols = symbols_with_func("f", vec![Type::Long], Type::Int);
    let mut e = Expression::call("f", vec![Expression::int_const(5)]);
    check(&mut e, &mut symbols).unwrap();
    let args = e.expect_call_args();
    args[0].expect_cast_to(&Type::Long);
}

#[test]
fn function_call_with_wrong_arg_count_is_error() {
    let mut symbols = symbols_with_func("f", vec![Type::Int, Type::Int], Type::Int);
    let mut e = Expression::call("f", vec![Expression::int_const(5)]);
    let err = check(&mut e, &mut symbols).unwrap_err();
    assert!(matches!(
        err,
        SemanticError::FuncCalledWithWrongNumArgs(name, _) if name == "f"
    ));
}

#[test]
fn calling_a_variable_is_error() {
    let mut symbols = symbols_with(&[("x", Type::Int)]);
    let mut e = Expression::call("x", vec![]);
    let err = check(&mut e, &mut symbols).unwrap_err();
    assert!(matches!(err, SemanticError::VarCalledAsFunc(name, _) if name == "x"));
}

// ---------------------------------------------------------------------------
// Conditional (ternary)
// ---------------------------------------------------------------------------

#[test]
fn conditional_branches_promoted_to_common_type() {
    let mut e = Expression::conditional(
        Expression::int_const(1),
        Expression::int_const(2),
        Expression::long_const(3),
    );
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Long));
    let (_cond, then_e, else_e) = e.expect_conditional_parts();
    then_e.expect_cast_to(&Type::Long);
    assert!(!matches!(else_e.kind, ExpressionKind::Cast(..)));
}

#[test]
fn conditional_condition_does_not_affect_result_type() {
    // condition is double, branches are both int -> result is int
    let mut e = Expression::conditional(
        Expression::double_const(1.0),
        Expression::int_const(2),
        Expression::int_const(3),
    );
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Int));
}

// ---------------------------------------------------------------------------
// Increment / decrement
// ---------------------------------------------------------------------------

#[test]
fn prefix_increment_preserves_operand_type() {
    let mut symbols = symbols_with(&[("x", Type::Long)]);
    let mut e = Expression::prefix_inc(Expression::var("x"));
    check(&mut e, &mut symbols).unwrap();
    assert_eq!(e.expression_type, Some(Type::Long));
}

#[test]
fn postfix_decrement_preserves_operand_type() {
    let mut symbols = symbols_with(&[("x", Type::UInt)]);
    let mut e = Expression::postfix_dec(Expression::var("x"));
    check(&mut e, &mut symbols).unwrap();
    assert_eq!(e.expression_type, Some(Type::UInt));
}

// ---------------------------------------------------------------------------
// Builder methods on Expression (test-only)
// ---------------------------------------------------------------------------

fn dummy_span() -> Span {
    Span {
        line_number: 1,
        col: 1,
    }
}

impl Expression {
    fn from_kind(kind: ExpressionKind) -> Self {
        Expression {
            kind,
            expression_type: None,
            span: dummy_span(),
        }
    }

    fn int_const(v: i32) -> Self {
        Self::from_kind(ExpressionKind::Constant(Const::Int(v)))
    }

    fn long_const(v: i64) -> Self {
        Self::from_kind(ExpressionKind::Constant(Const::Long(v)))
    }

    fn uint_const(v: u32) -> Self {
        Self::from_kind(ExpressionKind::Constant(Const::UInt(v)))
    }

    fn ulong_const(v: u64) -> Self {
        Self::from_kind(ExpressionKind::Constant(Const::ULong(v)))
    }

    fn double_const(v: f64) -> Self {
        Self::from_kind(ExpressionKind::Constant(Const::Double(OrderedFloat(v))))
    }

    fn var(name: &str) -> Self {
        Self::from_kind(ExpressionKind::Var(name.to_string()))
    }

    fn cast(ty: Type, inner: Expression) -> Self {
        Self::from_kind(ExpressionKind::Cast(ty, Box::new(inner)))
    }

    fn unary(op: UnaryOp, inner: Expression) -> Self {
        Self::from_kind(ExpressionKind::Unary(op, Box::new(inner)))
    }

    fn binop(op: BinaryOp, lhs: Expression, rhs: Expression) -> Self {
        Self::from_kind(ExpressionKind::Binary(op, Box::new(lhs), Box::new(rhs)))
    }

    fn assign(lhs: Expression, rhs: Expression) -> Self {
        Self::from_kind(ExpressionKind::Assignment(Box::new(lhs), Box::new(rhs)))
    }

    fn call(name: &str, args: Vec<Expression>) -> Self {
        Self::from_kind(ExpressionKind::FunctionCall(name.to_string(), args))
    }

    fn conditional(cond: Expression, then_e: Expression, else_e: Expression) -> Self {
        Self::from_kind(ExpressionKind::Conditional(
            Box::new(cond),
            Box::new(then_e),
            Box::new(else_e),
        ))
    }

    fn prefix_inc(inner: Expression) -> Self {
        Self::from_kind(ExpressionKind::PrefixIncrement(Box::new(inner)))
    }

    fn postfix_dec(inner: Expression) -> Self {
        Self::from_kind(ExpressionKind::PostfixDecrement(Box::new(inner)))
    }

    // ---- Inspection helpers ----

    /// assert this expression is a cast to `expected_ty` and returns the inner expression.
    fn expect_cast_to(&self, expected_ty: &Type) -> &Expression {
        match &self.kind {
            ExpressionKind::Cast(ty, inner) => {
                assert_eq!(
                    ty, expected_ty,
                    "expected Cast to {:?}, got Cast to {:?}",
                    expected_ty, ty
                );
                assert_eq!(
                    self.expression_type.as_ref(),
                    Some(expected_ty),
                    "Cast node's expression_type should equal target type"
                );
                inner
            }
            other => panic!("expected Cast, got {:?}", other),
        }
    }

    /// assert this expression is a cast and returns the inner expression (any target type).
    fn expect_cast_inner(&self) -> &Expression {
        match &self.kind {
            ExpressionKind::Cast(_, inner) => inner,
            other => panic!("expected Cast, got {:?}", other),
        }
    }

    fn expect_binary_operands(&self) -> (&Expression, &Expression) {
        match &self.kind {
            ExpressionKind::Binary(_, lhs, rhs) => (lhs, rhs),
            other => panic!("expected Binary, got {:?}", other),
        }
    }

    fn expect_assignment_operands(&self) -> (&Expression, &Expression) {
        match &self.kind {
            ExpressionKind::Assignment(lhs, rhs) => (lhs, rhs),
            other => panic!("expected Assignment, got {:?}", other),
        }
    }

    fn expect_call_args(&self) -> &[Expression] {
        match &self.kind {
            ExpressionKind::FunctionCall(_, args) => args,
            other => panic!("expected FunctionCall, got {:?}", other),
        }
    }

    fn expect_conditional_parts(&self) -> (&Expression, &Expression, &Expression) {
        match &self.kind {
            ExpressionKind::Conditional(c, t, e) => (c, t, e),
            other => panic!("expected Conditional, got {:?}", other),
        }
    }
}

// ---------------------------------------------------------------------------
// Nested expressions
//
// These tests check that recursive type-checking threads types correctly
// through multiple levels and inserts casts at the right depths. Add these
// to your existing tests.rs file (they reuse the same builders and helpers).
// ---------------------------------------------------------------------------

#[test]
fn nested_binary_promotes_at_inner_level() {
    // (int + long) + int
    //   inner result is Long, outer Long + Int promotes the outer int.
    let mut e = Expression::binop(
        BinaryOp::Add,
        Expression::binop(
            BinaryOp::Add,
            Expression::int_const(1),
            Expression::long_const(2),
        ),
        Expression::int_const(3),
    );
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Long));

    let (outer_lhs, outer_rhs) = e.expect_binary_operands();

    // Outer rhs (the standalone int 3) should have been cast to Long.
    outer_rhs.expect_cast_to(&Type::Long);

    // Outer lhs is the inner binary; it should be annotated as Long, and
    // *not* wrapped in a cast (since its result is already Long).
    assert_eq!(outer_lhs.expression_type, Some(Type::Long));
    assert!(!matches!(outer_lhs.kind, ExpressionKind::Cast(..)));

    // Drill into the inner binary and verify its int operand was cast to Long.
    let (inner_lhs, inner_rhs) = outer_lhs.expect_binary_operands();
    inner_lhs.expect_cast_to(&Type::Long);
    assert_eq!(inner_rhs.expression_type, Some(Type::Long));
    assert!(!matches!(inner_rhs.kind, ExpressionKind::Cast(..)));
}

#[test]
fn nested_binary_promotes_through_two_levels() {
    // int + (int + double)
    //   inner result is Double, outer Int + Double promotes the outer int.
    let mut e = Expression::binop(
        BinaryOp::Add,
        Expression::int_const(1),
        Expression::binop(
            BinaryOp::Add,
            Expression::int_const(2),
            Expression::double_const(3.0),
        ),
    );
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Double));

    let (outer_lhs, outer_rhs) = e.expect_binary_operands();
    outer_lhs.expect_cast_to(&Type::Double);
    assert_eq!(outer_rhs.expression_type, Some(Type::Double));

    let (inner_lhs, inner_rhs) = outer_rhs.expect_binary_operands();
    inner_lhs.expect_cast_to(&Type::Double);
    assert!(!matches!(inner_rhs.kind, ExpressionKind::Cast(..)));
}

#[test]
fn comparison_inside_logical_does_not_promote_logical_operands() {
    // (int < long) && (double < double)
    //   Each comparison returns Int and promotes its own operands.
    //   The outer && returns Int and does NOT promote either side
    //   (both are already Int from the comparisons).
    let mut e = Expression::binop(
        BinaryOp::LogicalAnd,
        Expression::binop(
            BinaryOp::LessThan,
            Expression::int_const(1),
            Expression::long_const(2),
        ),
        Expression::binop(
            BinaryOp::LessThan,
            Expression::double_const(1.0),
            Expression::double_const(2.0),
        ),
    );
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Int));

    let (lhs, rhs) = e.expect_binary_operands();
    // Neither side of the && should be cast.
    assert!(!matches!(lhs.kind, ExpressionKind::Cast(..)));
    assert!(!matches!(rhs.kind, ExpressionKind::Cast(..)));
    assert_eq!(lhs.expression_type, Some(Type::Int));
    assert_eq!(rhs.expression_type, Some(Type::Int));

    // Inside the left comparison, the int operand was cast to Long.
    let (cmp_lhs, _) = lhs.expect_binary_operands();
    cmp_lhs.expect_cast_to(&Type::Long);
}

#[test]
fn function_call_arg_is_nested_binary() {
    // f(int + long), where f takes a Long.
    //   The inner binary's result is Long, so no cast should be inserted
    //   at the argument boundary — but the inner int operand should still
    //   be cast to Long.
    let mut symbols = symbols_with_func("f", vec![Type::Long], Type::Int);
    let mut e = Expression::call(
        "f",
        vec![Expression::binop(
            BinaryOp::Add,
            Expression::int_const(1),
            Expression::long_const(2),
        )],
    );
    check(&mut e, &mut symbols).unwrap();

    let args = e.expect_call_args();
    assert_eq!(args[0].expression_type, Some(Type::Long));
    // No cast at the arg boundary — already Long.
    assert!(!matches!(args[0].kind, ExpressionKind::Cast(..)));

    // The inner binary's int operand was promoted.
    let (inner_lhs, _) = args[0].expect_binary_operands();
    inner_lhs.expect_cast_to(&Type::Long);
}

#[test]
fn function_call_arg_needs_conversion_after_inner_promotion() {
    // f(int + int), where f takes a Long.
    //   The inner binary's result is Int, then the call site converts the
    //   whole arg to Long. So the arg should be a Cast(Long, Binary(...)).
    let mut symbols = symbols_with_func("f", vec![Type::Long], Type::Int);
    let mut e = Expression::call(
        "f",
        vec![Expression::binop(
            BinaryOp::Add,
            Expression::int_const(1),
            Expression::int_const(2),
        )],
    );
    check(&mut e, &mut symbols).unwrap();

    let args = e.expect_call_args();
    let inner = args[0].expect_cast_to(&Type::Long);
    // The thing inside the cast is the original binary, still annotated as Int.
    assert!(matches!(inner.kind, ExpressionKind::Binary(..)));
    assert_eq!(inner.expression_type, Some(Type::Int));
}

#[test]
fn assignment_with_nested_binary_rhs() {
    // x = (int + double), where x is Long.
    //   Inner binary promotes int to double, result is Double.
    //   Then the assignment converts the whole rhs from Double to Long.
    let mut symbols = symbols_with(&[("x", Type::Long)]);
    let mut e = Expression::assign(
        Expression::var("x"),
        Expression::binop(
            BinaryOp::Add,
            Expression::int_const(1),
            Expression::double_const(2.0),
        ),
    );
    check(&mut e, &mut symbols).unwrap();
    assert_eq!(e.expression_type, Some(Type::Long));

    let (_lhs, rhs) = e.expect_assignment_operands();
    let inner = rhs.expect_cast_to(&Type::Long);
    assert_eq!(inner.expression_type, Some(Type::Double));
    assert!(matches!(inner.kind, ExpressionKind::Binary(..)));
}

#[test]
fn conditional_with_nested_promotions() {
    // (cond) ? (int + int) : long
    //   Then-branch is Int, else-branch is Long.
    //   Common type is Long, so the then-branch (whole binary) is cast to Long.
    let mut e = Expression::conditional(
        Expression::int_const(1),
        Expression::binop(
            BinaryOp::Add,
            Expression::int_const(2),
            Expression::int_const(3),
        ),
        Expression::long_const(4),
    );
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Long));

    let (_cond, then_e, else_e) = e.expect_conditional_parts();
    let then_inner = then_e.expect_cast_to(&Type::Long);
    assert!(matches!(then_inner.kind, ExpressionKind::Binary(..)));
    assert_eq!(then_inner.expression_type, Some(Type::Int));
    assert!(!matches!(else_e.kind, ExpressionKind::Cast(..)));
}

#[test]
fn deeply_nested_unary_preserves_type_through_levels() {
    // -(-(-int))
    //   All three negations preserve Int.
    let mut e = Expression::unary(
        UnaryOp::Negate,
        Expression::unary(
            UnaryOp::Negate,
            Expression::unary(UnaryOp::Negate, Expression::int_const(5)),
        ),
    );
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Int));

    // Walk down and confirm each level is annotated.
    if let ExpressionKind::Unary(_, inner1) = &e.kind {
        assert_eq!(inner1.expression_type, Some(Type::Int));
        if let ExpressionKind::Unary(_, inner2) = &inner1.kind {
            assert_eq!(inner2.expression_type, Some(Type::Int));
            if let ExpressionKind::Unary(_, inner3) = &inner2.kind {
                assert_eq!(inner3.expression_type, Some(Type::Int));
            } else {
                panic!("expected innermost Unary");
            }
        } else {
            panic!("expected middle Unary");
        }
    } else {
        panic!("expected outer Unary");
    }
}

#[test]
fn not_of_comparison_returns_int() {
    // !(int < long)
    //   Comparison returns Int (with promotion inside), then ! returns Int.
    let mut e = Expression::unary(
        UnaryOp::Not,
        Expression::binop(
            BinaryOp::LessThan,
            Expression::int_const(1),
            Expression::long_const(2),
        ),
    );
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Int));

    if let ExpressionKind::Unary(_, inner) = &e.kind {
        assert_eq!(inner.expression_type, Some(Type::Int));
        // The promotion inside the comparison still happened.
        let (cmp_lhs, _) = inner.expect_binary_operands();
        cmp_lhs.expect_cast_to(&Type::Long);
    } else {
        panic!("expected Unary");
    }
}

#[test]
fn function_call_with_multiple_args_each_converted_independently() {
    // f(int, double, int), where f takes (Long, Long, Long).
    //   Each arg gets converted to Long independently.
    let mut symbols =
        symbols_with_func("f", vec![Type::Long, Type::Long, Type::Long], Type::Int);
    let mut e = Expression::call(
        "f",
        vec![
            Expression::int_const(1),
            Expression::double_const(2.0),
            Expression::int_const(3),
        ],
    );
    check(&mut e, &mut symbols).unwrap();

    let args = e.expect_call_args();
    args[0].expect_cast_to(&Type::Long);
    args[1].expect_cast_to(&Type::Long);
    args[2].expect_cast_to(&Type::Long);
}

#[test]
fn cast_inside_binary_is_respected() {
    // (long)int + long
    //   The explicit cast makes the lhs Long, so no further promotion needed
    //   on either side.
    let mut e = Expression::binop(
        BinaryOp::Add,
        Expression::cast(Type::Long, Expression::int_const(1)),
        Expression::long_const(2),
    );
    check_empty(&mut e).unwrap();
    assert_eq!(e.expression_type, Some(Type::Long));

    let (lhs, rhs) = e.expect_binary_operands();
    // lhs is the explicit Cast (annotated Long); should NOT be re-wrapped in
    // another cast by convert_type, since types already match.
    assert_eq!(lhs.expression_type, Some(Type::Long));
    // It should still be a Cast (the explicit one), not double-wrapped.
    if let ExpressionKind::Cast(_, inner) = &lhs.kind {
        // The inner is the int constant.
        assert!(matches!(inner.kind, ExpressionKind::Constant(Const::Int(1))));
    } else {
        panic!("expected explicit Cast to remain");
    }
    assert!(!matches!(rhs.kind, ExpressionKind::Cast(..)));
}
