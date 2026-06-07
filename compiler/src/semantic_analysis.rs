use thiserror::Error;

use crate::parser;

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
}
