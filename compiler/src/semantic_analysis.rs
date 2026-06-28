use thiserror::Error;

use crate::{parser, resolve_types::SymbolTable, types};

pub fn get_expression_value(expression: &parser::Expression) -> Option<types::Constant> {
    if let parser::Expression::Factor(parser::Factor::Postfix(parser::Postfix {
        primary,
        postfix,
    })) = expression
    {
        if postfix.len() == 0 {
            get_primary_value(primary)
        } else {
            None
        }
    } else {
        None
    }
}

pub fn get_primary_value(primary: &parser::Primary) -> Option<types::Constant> {
    match primary {
        parser::Primary::Constant(value) => Some(value.clone()),
        parser::Primary::Paren(expression) => get_expression_value(&expression.expression),
        _ => None,
    }
}

pub fn resolve_program(
    program: parser::Program,
) -> Result<(parser::Program, SymbolTable), SemanticError> {
    let program = crate::resolve_identifiers::resolve_identifiers(program)?;
    let program = crate::resolve_lvalues::resolve_lvalues(program)?;
    let program = crate::resolve_labels::resolve_labels(program)?;
    let program = crate::resolve_loops::resolve_loops(program)?;
    crate::resolve_types::resolve_types(program)
}

#[derive(Debug, Error)]
pub enum SemanticError {
    #[error("Identifier already declared: {0}")]
    IdentifierAlreadyDeclared(String),
    #[error("Identifier not declared: {0}")]
    IdentifierNotDeclared(String),
    #[error("Invalid lvalue: {0:?}")]
    InvalidLvalue(parser::Expression),
    #[error("Label not declared: {0}")]
    LabelNotDeclared(String),
    #[error("Label already declared: {0}")]
    DuplicateLabel(String),
    #[error("Break statement not within a loop or switch")]
    BreakNotWithinLoopOrSwitch,
    #[error("Continue statement not within a loop")]
    ContinueNotWithinLoop,
    #[error("Default statement not within a switch")]
    DefaultNotWithinSwitch,
    #[error("Case statement not within a switch")]
    CaseNotWithinSwitch,
    #[error("Non-constant expression found")]
    NonConstantExpression,
    #[error("Duplicate case value: {0:?}")]
    DuplicateCaseValue(types::Constant),
    #[error("Duplicate default label")]
    DuplicateDefaultLabel,
    #[error("Function body found in a declaration")]
    FunctionBodyInDeclaration,
    #[error("Incompatible function declaration for {0}")]
    IncompatibleFunctionDeclaration(String),
    #[error("Function defined more than once: {0}")]
    FunctionDefinedMoreThanOnce(String),
    #[error("Incorrect type: {0}")]
    TypeMismatch(String),
    #[error("Conflicting local declarations for {0}")]
    ConflictingLocalDeclarations(String),
    #[error("Invalid storage class for {0}")]
    InvalidStorageClass(String),
    #[error("Static function {0} cannot be declared after a non-static declaration")]
    StaticAfterNonStatic(String),
    #[error("Conflicting variable linkage for {0}")]
    ConflictingVariableLinkage(String),
    #[error("Conflicting file scope declarations for {0}")]
    ConflictingFileScopeDeclarations(String),
    #[error("Initialiser on local extern variable {0}")]
    InitOnLocalExtern(String),
}
