use crate::parser;
use crate::semantic_analysis::SemanticError;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::atomic::AtomicUsize;

static VARIABLE_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn get_unique_variable_name(user_defined_name: impl Display) -> String {
    let id = VARIABLE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("var_{}.{}", user_defined_name, id)
}

pub fn resolve_variables(program: parser::Program) -> Result<parser::Program, SemanticError> {
    let mut variables = HashMap::new();
    program.resolve_variables(&mut variables)
}

trait ResolveVariables
where
    Self: Sized,
{
    fn resolve_variables(
        self,
        variables: &mut HashMap<String, String>,
    ) -> Result<Self, SemanticError>;
}

impl ResolveVariables for parser::Program {
    fn resolve_variables(
        self,
        variables: &mut HashMap<String, String>,
    ) -> Result<Self, SemanticError> {
        Ok(parser::Program(self.0.resolve_variables(variables)?))
    }
}

impl ResolveVariables for parser::FuncDef {
    fn resolve_variables(
        self,
        variables: &mut HashMap<String, String>,
    ) -> Result<Self, SemanticError> {
        Ok(parser::FuncDef {
            name: self.name,
            body: self
                .body
                .into_iter()
                .map(|stmt| stmt.resolve_variables(variables))
                .collect::<Result<_, _>>()?,
        })
    }
}

impl ResolveVariables for parser::BlockItem {
    fn resolve_variables(
        self,
        variables: &mut HashMap<String, String>,
    ) -> Result<Self, SemanticError> {
        match self {
            parser::BlockItem::Statement(statement) => {
                Ok(Self::Statement(statement.resolve_variables(variables)?))
            }
            parser::BlockItem::Declaration(declaration) => {
                Ok(Self::Declaration(declaration.resolve_variables(variables)?))
            }
        }
    }
}

impl ResolveVariables for parser::Declaration {
    fn resolve_variables(
        self,
        variables: &mut HashMap<String, String>,
    ) -> Result<Self, SemanticError> {
        if variables.contains_key(&self.identifier) {
            return Err(SemanticError::VariableAlreadyDeclared(self.identifier));
        }
        let unique_name = get_unique_variable_name(&self.identifier);
        variables.insert(self.identifier.clone(), unique_name.clone());
        Ok(Self {
            identifier: unique_name,
            initialiser: self
                .initialiser
                .map(|exp| exp.resolve_variables(variables))
                .transpose()?,
        })
    }
}

impl ResolveVariables for parser::Statement {
    fn resolve_variables(
        self,
        variables: &mut HashMap<String, String>,
    ) -> Result<Self, SemanticError> {
        match self {
            parser::Statement::Return(exp) => Ok(Self::Return(exp.resolve_variables(variables)?)),
            parser::Statement::Expression(exp) => {
                Ok(Self::Expression(exp.resolve_variables(variables)?))
            }
            parser::Statement::Null => Ok(Self::Null),
            parser::Statement::If {
                condition,
                then_branch,
                else_branch,
            } => Ok(Self::If {
                condition: condition.resolve_variables(variables)?,
                then_branch: Box::new(then_branch.resolve_variables(variables)?),
                else_branch: else_branch
                    .map(|else_branch| else_branch.resolve_variables(variables))
                    .transpose()?
                    .map(Box::new),
            }),
            parser::Statement::Labeled(label, statement) => Ok(Self::Labeled(
                label,
                Box::new(statement.resolve_variables(variables)?),
            )),
            parser::Statement::Goto(label) => Ok(Self::Goto(label)),
        }
    }
}

impl ResolveVariables for parser::Expression {
    fn resolve_variables(
        self,
        variables: &mut HashMap<String, String>,
    ) -> Result<Self, SemanticError> {
        match self {
            parser::Expression::Factor(factor) => {
                Ok(Self::Factor(factor.resolve_variables(variables)?))
            }
            parser::Expression::BinaryOp { op, left, right } => Ok(Self::BinaryOp {
                op,
                left: Box::new(left.resolve_variables(variables)?),
                right: Box::new(right.resolve_variables(variables)?),
            }),
            parser::Expression::Assignment { left, right } => Ok(Self::Assignment {
                left: Box::new(left.resolve_variables(variables)?),
                right: Box::new(right.resolve_variables(variables)?),
            }),
            parser::Expression::Conditional {
                condition,
                then_branch,
                else_branch,
            } => Ok(Self::Conditional {
                condition: Box::new(condition.resolve_variables(variables)?),
                then_branch: Box::new(then_branch.resolve_variables(variables)?),
                else_branch: Box::new(else_branch.resolve_variables(variables)?),
            }),
        }
    }
}

impl ResolveVariables for parser::Factor {
    fn resolve_variables(
        self,
        variables: &mut HashMap<String, String>,
    ) -> Result<Self, SemanticError> {
        match self {
            parser::Factor::UnaryOp { op, fac }
                if matches!(
                    op,
                    parser::UnaryOperator::PrefixIncrement | parser::UnaryOperator::PrefixDecrement
                ) =>
            {
                Ok(Self::UnaryOp {
                    op,
                    fac: Box::new(fac.resolve_variables(variables)?),
                })
            }
            parser::Factor::UnaryOp { op, fac } => Ok(Self::UnaryOp {
                op,
                fac: Box::new(fac.resolve_variables(variables)?),
            }),
            parser::Factor::Postfix(postfix) => postfix
                .resolve_variables(variables)
                .map(parser::Factor::Postfix),
        }
    }
}

impl ResolveVariables for parser::Postfix {
    fn resolve_variables(
        self,
        variables: &mut HashMap<String, String>,
    ) -> Result<Self, SemanticError> {
        Ok(Self {
            primary: self.primary.resolve_variables(variables)?,
            postfix: self.postfix,
        })
    }
}

impl ResolveVariables for parser::Primary {
    fn resolve_variables(
        self,
        variables: &mut HashMap<String, String>,
    ) -> Result<Self, SemanticError> {
        match self {
            parser::Primary::Var(identifier) => {
                if let Some(unique_name) = variables.get(&identifier) {
                    Ok(Self::Var(unique_name.clone()))
                } else {
                    Err(SemanticError::VariableNotDeclared(identifier))
                }
            }
            parser::Primary::Paren(expression) => Ok(Self::Paren(Box::new(
                expression.resolve_variables(variables)?,
            ))),
            parser::Primary::Constant(val) => Ok(Self::Constant(val)),
        }
    }
}
