use thiserror::Error;

use crate::parser;

pub fn get_expression_constant(expression: &parser::Expression) -> Option<u32> {
    if let parser::Expression::Factor(parser::Factor::Postfix(parser::Postfix {
        primary,
        postfix,
    })) = expression
    {
        if postfix.len() == 0 {
            get_primary_constant(primary)
        } else {
            None
        }
    } else {
        None
    }
}

pub fn get_primary_constant(primary: &parser::Primary) -> Option<u32> {
    match primary {
        parser::Primary::Constant(value) => Some(*value),
        parser::Primary::Paren(expression) => get_expression_constant(expression),
        _ => None,
    }
}

pub fn resolve_program(program: parser::Program) -> Result<parser::Program, SemanticError> {
    let program = crate::resolve_variables::resolve_variables(program)?;
    let program = crate::resolve_lvalues::resolve_lvalues(program)?;
    let program = crate::resolve_labels::resolve_labels(program)?;
    crate::resolve_loops::resolve_loops(program)
}

#[derive(Debug, Error)]
pub enum SemanticError {
    #[error("Variable already declared: {0}")]
    VariableAlreadyDeclared(String),
    #[error("Variable not declared: {0}")]
    VariableNotDeclared(String),
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
    #[error("Duplicate case value: {0}")]
    DuplicateCaseValue(String),
    #[error("Duplicate default label")]
    DuplicateDefaultLabel,
}
