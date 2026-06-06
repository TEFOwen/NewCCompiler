use std::{collections::HashMap, fmt::Display, sync::atomic::AtomicUsize};
use thiserror::Error;

use crate::parser;

static VARIABLE_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn get_unique_variable_name(user_defined_name: impl Display) -> String {
    let id = VARIABLE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("var_{}.{}", user_defined_name, id)
}

pub fn resolve_program(program: parser::Program) -> Result<parser::Program, SemanticError> {
    let mut variables = HashMap::new();
    program.resolve(&mut variables)
}

#[derive(Debug, Error)]
pub enum SemanticError {
    #[error("Variable already declared: {0}")]
    VariableAlreadyDeclared(String),
    #[error("Variable not declared: {0}")]
    VariableNotDeclared(String),
    #[error("Invalid lvalue: {0:?}")]
    InvalidLvalue(parser::Expression),
}

trait IsLvalue {
    fn is_lvalue(&self) -> bool;
}

impl IsLvalue for parser::Expression {
    fn is_lvalue(&self) -> bool {
        match self {
            parser::Expression::Factor(factor) => factor.is_lvalue(),
            parser::Expression::BinaryOp { .. } => false,
            parser::Expression::Assignment { .. } => false,
        }
    }
}

impl IsLvalue for parser::Factor {
    fn is_lvalue(&self) -> bool {
        match self {
            parser::Factor::UnaryOp { .. } => false,
            parser::Factor::Postfix(postfix) => postfix.is_lvalue(),
        }
    }
}

impl IsLvalue for parser::Postfix {
    fn is_lvalue(&self) -> bool {
        self.postfix.len() == 0 && self.primary.is_lvalue()
    }
}

impl IsLvalue for parser::Primary {
    fn is_lvalue(&self) -> bool {
        match self {
            parser::Primary::Constant(_) => false,
            parser::Primary::Var(_) => true,
            parser::Primary::Paren(expression) => expression.is_lvalue(),
        }
    }
}

trait Resolve
where
    Self: Sized,
{
    fn resolve(self, variables: &mut HashMap<String, String>) -> Result<Self, SemanticError>;
}

impl Resolve for parser::Program {
    fn resolve(self, variables: &mut HashMap<String, String>) -> Result<Self, SemanticError> {
        Ok(parser::Program(self.0.resolve(variables)?))
    }
}

impl Resolve for parser::FuncDef {
    fn resolve(self, variables: &mut HashMap<String, String>) -> Result<Self, SemanticError> {
        Ok(parser::FuncDef {
            name: self.name,
            body: self
                .body
                .into_iter()
                .map(|stmt| stmt.resolve(variables))
                .collect::<Result<_, _>>()?,
        })
    }
}

impl Resolve for parser::BlockItem {
    fn resolve(self, variables: &mut HashMap<String, String>) -> Result<Self, SemanticError> {
        match self {
            parser::BlockItem::Statement(statement) => {
                Ok(Self::Statement(statement.resolve(variables)?))
            }
            parser::BlockItem::Declaration(declaration) => {
                Ok(Self::Declaration(declaration.resolve(variables)?))
            }
        }
    }
}

impl Resolve for parser::Declaration {
    fn resolve(self, variables: &mut HashMap<String, String>) -> Result<Self, SemanticError> {
        if variables.contains_key(&self.identifier) {
            return Err(SemanticError::VariableAlreadyDeclared(self.identifier));
        }
        let unique_name = get_unique_variable_name(&self.identifier);
        variables.insert(self.identifier.clone(), unique_name.clone());
        Ok(Self {
            identifier: unique_name,
            initialiser: self
                .initialiser
                .map(|exp| exp.resolve(variables))
                .transpose()?,
        })
    }
}

impl Resolve for parser::Statement {
    fn resolve(self, variables: &mut HashMap<String, String>) -> Result<Self, SemanticError> {
        match self {
            parser::Statement::Return(exp) => Ok(Self::Return(exp.resolve(variables)?)),
            parser::Statement::Expression(exp) => Ok(Self::Expression(exp.resolve(variables)?)),
            parser::Statement::Null => Ok(Self::Null),
        }
    }
}

impl Resolve for parser::Expression {
    fn resolve(self, variables: &mut HashMap<String, String>) -> Result<Self, SemanticError> {
        match self {
            parser::Expression::Factor(factor) => Ok(Self::Factor(factor.resolve(variables)?)),
            parser::Expression::BinaryOp { op, left, right } => Ok(Self::BinaryOp {
                op,
                left: Box::new(left.resolve(variables)?),
                right: Box::new(right.resolve(variables)?),
            }),
            parser::Expression::Assignment { left, right } => {
                if left.is_lvalue() {
                    Ok(Self::Assignment {
                        left: Box::new(left.resolve(variables)?),
                        right: Box::new(right.resolve(variables)?),
                    })
                } else {
                    Err(SemanticError::InvalidLvalue(*left))
                }
            }
        }
    }
}

impl Resolve for parser::Factor {
    fn resolve(self, variables: &mut HashMap<String, String>) -> Result<Self, SemanticError> {
        match self {
            parser::Factor::UnaryOp { op, fac }
                if matches!(
                    op,
                    parser::UnaryOperator::PrefixIncrement | parser::UnaryOperator::PrefixDecrement
                ) =>
            {
                if !fac.is_lvalue() {
                    return Err(SemanticError::InvalidLvalue(parser::Expression::Factor(
                        *fac,
                    )));
                }
                Ok(Self::UnaryOp {
                    op,
                    fac: Box::new(fac.resolve(variables)?),
                })
            }
            parser::Factor::UnaryOp { op, fac } => Ok(Self::UnaryOp {
                op,
                fac: Box::new(fac.resolve(variables)?),
            }),
            parser::Factor::Postfix(postfix) => {
                postfix.resolve(variables).map(parser::Factor::Postfix)
            }
        }
    }
}

impl Resolve for parser::Postfix {
    fn resolve(self, variables: &mut HashMap<String, String>) -> Result<Self, SemanticError> {
        if self.postfix.len() > 1 || (self.postfix.len() > 0 && !self.primary.is_lvalue()) {
            return Err(SemanticError::InvalidLvalue(parser::Expression::Factor(
                parser::Factor::Postfix(self),
            )));
        }
        Ok(Self {
            primary: self.primary.resolve(variables)?,
            postfix: self.postfix,
        })
    }
}

impl Resolve for parser::Primary {
    fn resolve(self, variables: &mut HashMap<String, String>) -> Result<Self, SemanticError> {
        match self {
            parser::Primary::Var(identifier) => {
                if let Some(unique_name) = variables.get(&identifier) {
                    Ok(Self::Var(unique_name.clone()))
                } else {
                    Err(SemanticError::VariableNotDeclared(identifier))
                }
            }
            parser::Primary::Paren(expression) => {
                Ok(Self::Paren(Box::new(expression.resolve(variables)?)))
            }
            parser::Primary::Constant(val) => Ok(Self::Constant(val)),
        }
    }
}
