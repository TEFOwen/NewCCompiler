use std::collections::HashMap;

use crate::{parser, semantic_analysis::SemanticError};

#[derive(Debug)]
enum Type {
    Int,
    Function { defined: bool, parameters: u32 },
}
impl Type {
    fn is_defined(&self) -> bool {
        match self {
            Type::Int => true,
            Type::Function { defined, .. } => *defined,
        }
    }
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Function {
                    parameters: l_parameters,
                    ..
                },
                Self::Function {
                    parameters: r_parameters,
                    ..
                },
            ) => l_parameters == r_parameters,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

#[derive(Default, Debug)]
pub struct SymbolTable {
    symbols: HashMap<String, Type>,
}

impl SymbolTable {
    pub fn has_symbol(&self, identifier: impl Into<String>) -> bool {
        let identifier = identifier.into();
        self.symbols.contains_key(&identifier)
    }

    fn variable_declared(
        &mut self,
        identifier: impl Into<String>,
    ) -> Result<String, SemanticError> {
        let identifier = identifier.into();
        self.symbols.insert(identifier.clone(), Type::Int);
        Ok(identifier)
    }

    fn function_declared(
        &mut self,
        identifier: impl Into<String>,
        parameters: u32,
        has_body: bool,
    ) -> Result<String, SemanticError> {
        let identifier = identifier.into();
        let mut defined = has_body;

        if let Some(existing_type) = self.symbols.get(&identifier) {
            if existing_type
                != &(Type::Function {
                    defined: false,
                    parameters,
                })
            {
                return Err(SemanticError::IncompatibleFunctionDeclaration(identifier));
            }
            if existing_type.is_defined() && has_body {
                return Err(SemanticError::FunctionDefinedMoreThanOnce(identifier));
            }
            defined = existing_type.is_defined() || has_body;
        }

        self.symbols.insert(
            identifier.clone(),
            Type::Function {
                defined,
                parameters,
            },
        );

        Ok(identifier)
    }

    fn lookup_variable(&self, identifier: impl Into<String>) -> Result<String, SemanticError> {
        let identifier = identifier.into();
        if self
            .symbols
            .get(&identifier)
            .expect(format!("Variable not declared: {}", identifier).as_str())
            == &Type::Int
        {
            Ok(identifier)
        } else {
            Err(SemanticError::TypeMismatch(format!(
                "Expected variable, found function: {}",
                identifier
            )))
        }
    }

    fn lookup_function(
        &self,
        identifier: impl Into<String>,
        arguments: u32,
    ) -> Result<String, SemanticError> {
        let identifier = identifier.into();
        match self
            .symbols
            .get(&identifier)
            .expect(format!("Function not declared: {}", identifier).as_str())
        {
            Type::Function {
                defined: _,
                parameters,
            } => {
                if *parameters != arguments {
                    Err(SemanticError::TypeMismatch(format!(
                        "Function {} expects {} arguments, but {} were provided",
                        identifier, parameters, arguments
                    )))
                } else {
                    Ok(identifier)
                }
            }
            _ => Err(SemanticError::TypeMismatch(format!(
                "Expected function, found variable: {}",
                identifier
            ))),
        }
    }
}

trait ResolveTypes
where
    Self: Sized,
{
    fn resolve_types(self, symbol_table: &mut SymbolTable) -> Result<Self, SemanticError>;
}

pub fn resolve_types(
    program: parser::Program,
) -> Result<(parser::Program, SymbolTable), SemanticError> {
    let mut symbol_table = SymbolTable::default();
    let program = program.resolve_types(&mut symbol_table)?;
    Ok((program, symbol_table))
}

impl ResolveTypes for parser::Program {
    fn resolve_types(self, symbol_table: &mut SymbolTable) -> Result<Self, SemanticError> {
        Ok(parser::Program(
            self.0
                .into_iter()
                .map(|func_dec| func_dec.resolve_types(symbol_table))
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

impl ResolveTypes for parser::FuncDeclaration {
    fn resolve_types(self, symbol_table: &mut SymbolTable) -> Result<Self, SemanticError> {
        let identifier = symbol_table.function_declared(
            self.identifier,
            self.parameters.0.len() as u32,
            self.body.is_some(),
        )?;

        if let Some(body) = self.body {
            for param in &self.parameters.0 {
                symbol_table.variable_declared(param.clone())?;
            }
            Ok(parser::FuncDeclaration {
                identifier: identifier,
                parameters: self.parameters,
                body: Some(body.resolve_types(symbol_table)?),
            })
        } else {
            Ok(parser::FuncDeclaration {
                identifier: identifier,
                parameters: self.parameters,
                body: None,
            })
        }
    }
}

impl ResolveTypes for parser::Block {
    fn resolve_types(self, symbol_table: &mut SymbolTable) -> Result<Self, SemanticError> {
        Ok(parser::Block(
            self.0
                .into_iter()
                .map(|item| item.resolve_types(symbol_table))
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

impl ResolveTypes for parser::BlockItem {
    fn resolve_types(self, symbol_table: &mut SymbolTable) -> Result<Self, SemanticError> {
        match self {
            parser::BlockItem::Statement(statement) => statement
                .resolve_types(symbol_table)
                .map(parser::BlockItem::Statement),
            parser::BlockItem::Declaration(declaration) => declaration
                .resolve_types(symbol_table)
                .map(parser::BlockItem::Declaration),
        }
    }
}

impl ResolveTypes for parser::Declaration {
    fn resolve_types(self, symbol_table: &mut SymbolTable) -> Result<Self, SemanticError> {
        match self {
            parser::Declaration::Variable(variable_declaration) => variable_declaration
                .resolve_types(symbol_table)
                .map(parser::Declaration::Variable),
            parser::Declaration::Function(func_declaration) => func_declaration
                .resolve_types(symbol_table)
                .map(parser::Declaration::Function),
        }
    }
}

impl ResolveTypes for parser::VariableDeclaration {
    fn resolve_types(self, symbol_table: &mut SymbolTable) -> Result<Self, SemanticError> {
        Ok(parser::VariableDeclaration {
            identifier: symbol_table.variable_declared(self.identifier)?,
            initialiser: self
                .initialiser
                .map(|init| init.resolve_types(symbol_table))
                .transpose()?,
        })
    }
}

impl ResolveTypes for parser::Statement {
    fn resolve_types(self, symbol_table: &mut SymbolTable) -> Result<Self, SemanticError> {
        match self {
            parser::Statement::Return(expression) => expression
                .resolve_types(symbol_table)
                .map(parser::Statement::Return),
            parser::Statement::Labeled(label, statement) => statement
                .resolve_types(symbol_table)
                .map(|stmt| parser::Statement::Labeled(label, Box::new(stmt))),
            parser::Statement::Goto(label) => Ok(parser::Statement::Goto(label)),
            parser::Statement::Expression(expression) => expression
                .resolve_types(symbol_table)
                .map(parser::Statement::Expression),
            parser::Statement::If {
                condition,
                then_branch,
                else_branch,
            } => Ok(parser::Statement::If {
                condition: condition.resolve_types(symbol_table)?,
                then_branch: Box::new(then_branch.resolve_types(symbol_table)?),
                else_branch: else_branch
                    .map(|else_branch| else_branch.resolve_types(symbol_table))
                    .transpose()?
                    .map(Box::new),
            }),
            parser::Statement::Break(label) => Ok(parser::Statement::Break(label)),
            parser::Statement::Continue(label) => Ok(parser::Statement::Continue(label)),
            parser::Statement::Case(expression, statement, label) => Ok(parser::Statement::Case(
                expression.resolve_types(symbol_table)?,
                Box::new(statement.resolve_types(symbol_table)?),
                label,
            )),
            parser::Statement::Default(statement, label) => Ok(parser::Statement::Default(
                Box::new(statement.resolve_types(symbol_table)?),
                label,
            )),
            parser::Statement::While {
                condition,
                body,
                label,
            } => Ok(parser::Statement::While {
                condition: condition.resolve_types(symbol_table)?,
                body: Box::new(body.resolve_types(symbol_table)?),
                label,
            }),
            parser::Statement::DoWhile {
                body,
                condition,
                label,
            } => Ok(parser::Statement::DoWhile {
                body: Box::new(body.resolve_types(symbol_table)?),
                condition: condition.resolve_types(symbol_table)?,
                label,
            }),
            parser::Statement::For {
                init,
                condition,
                post,
                body,
                label,
            } => Ok(parser::Statement::For {
                init: init.resolve_types(symbol_table)?,
                condition: condition
                    .map(|condition| condition.resolve_types(symbol_table))
                    .transpose()?,
                post: post
                    .map(|post| post.resolve_types(symbol_table))
                    .transpose()?,
                body: Box::new(body.resolve_types(symbol_table)?),
                label,
            }),
            parser::Statement::Switch {
                condition,
                body,
                label,
                cases,
                default_exists,
            } => Ok(parser::Statement::Switch {
                condition: condition.resolve_types(symbol_table)?,
                body: Box::new(body.resolve_types(symbol_table)?),
                label,
                cases,
                default_exists,
            }),
            parser::Statement::Block(block) => block
                .resolve_types(symbol_table)
                .map(parser::Statement::Block),
            parser::Statement::Null => Ok(parser::Statement::Null),
        }
    }
}

impl ResolveTypes for parser::ForInit {
    fn resolve_types(self, symbol_table: &mut SymbolTable) -> Result<Self, SemanticError> {
        match self {
            parser::ForInit::Declaration(variable_declaration) => variable_declaration
                .resolve_types(symbol_table)
                .map(parser::ForInit::Declaration),
            parser::ForInit::Expression(expression) => expression
                .map(|expr| expr.resolve_types(symbol_table))
                .transpose()
                .map(parser::ForInit::Expression),
        }
    }
}

impl ResolveTypes for parser::Expression {
    fn resolve_types(self, symbol_table: &mut SymbolTable) -> Result<Self, SemanticError> {
        match self {
            parser::Expression::Factor(factor) => factor
                .resolve_types(symbol_table)
                .map(parser::Expression::Factor),
            parser::Expression::BinaryOp { op, left, right } => Ok(parser::Expression::BinaryOp {
                op,
                left: Box::new(left.resolve_types(symbol_table)?),
                right: Box::new(right.resolve_types(symbol_table)?),
            }),
            parser::Expression::Assignment { left, right } => Ok(parser::Expression::Assignment {
                left: Box::new(left.resolve_types(symbol_table)?),
                right: Box::new(right.resolve_types(symbol_table)?),
            }),
            parser::Expression::Conditional {
                condition,
                then_branch,
                else_branch,
            } => Ok(parser::Expression::Conditional {
                condition: Box::new(condition.resolve_types(symbol_table)?),
                then_branch: Box::new(then_branch.resolve_types(symbol_table)?),
                else_branch: Box::new(else_branch.resolve_types(symbol_table)?),
            }),
        }
    }
}

impl ResolveTypes for parser::Factor {
    fn resolve_types(self, symbol_table: &mut SymbolTable) -> Result<Self, SemanticError> {
        match self {
            parser::Factor::UnaryOp { op, fac } => Ok(parser::Factor::UnaryOp {
                op,
                fac: Box::new(fac.resolve_types(symbol_table)?),
            }),
            parser::Factor::Postfix(postfix) => postfix
                .resolve_types(symbol_table)
                .map(parser::Factor::Postfix),
        }
    }
}

impl ResolveTypes for parser::Postfix {
    fn resolve_types(self, symbol_table: &mut SymbolTable) -> Result<Self, SemanticError> {
        Ok(parser::Postfix {
            primary: self.primary.resolve_types(symbol_table)?,
            postfix: self.postfix,
        })
    }
}

impl ResolveTypes for parser::Primary {
    fn resolve_types(self, symbol_table: &mut SymbolTable) -> Result<Self, SemanticError> {
        match self {
            parser::Primary::Constant(_) => Ok(self),
            parser::Primary::Paren(expression) => expression
                .resolve_types(symbol_table)
                .map(|expr| parser::Primary::Paren(Box::new(expr))),
            parser::Primary::Var(identifier) => symbol_table
                .lookup_variable(identifier)
                .map(parser::Primary::Var),
            parser::Primary::FunctionCall(identifier, argument_list) => {
                let identifier =
                    symbol_table.lookup_function(identifier, argument_list.0.len() as u32)?;
                Ok(parser::Primary::FunctionCall(
                    identifier,
                    argument_list.resolve_types(symbol_table)?,
                ))
            }
        }
    }
}

impl ResolveTypes for parser::ArgumentList {
    fn resolve_types(self, symbol_table: &mut SymbolTable) -> Result<Self, SemanticError> {
        Ok(parser::ArgumentList(
            self.0
                .into_iter()
                .map(|arg| arg.resolve_types(symbol_table))
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}
