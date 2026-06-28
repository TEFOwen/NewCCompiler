use crate::parser;
use crate::semantic_analysis::SemanticError;

pub fn resolve_lvalues(program: parser::Program) -> Result<parser::Program, SemanticError> {
    program.resolve_lvalues()
}

trait IsLvalue {
    fn is_lvalue(&self) -> bool;
}

impl IsLvalue for parser::TypedExpression {
    fn is_lvalue(&self) -> bool {
        match &self.expression {
            parser::Expression::Factor(factor) => factor.is_lvalue(),
            parser::Expression::BinaryOp { .. } => false,
            parser::Expression::Assignment { .. } => false,
            parser::Expression::Conditional { .. } => false,
        }
    }
}

impl IsLvalue for parser::Factor {
    fn is_lvalue(&self) -> bool {
        match self {
            parser::Factor::Postfix(postfix) => postfix.is_lvalue(),
            parser::Factor::UnaryOp { .. } | parser::Factor::Cast { .. } => false,
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
            parser::Primary::FunctionCall(_, _) => false,
        }
    }
}

trait ResolveLvalues
where
    Self: Sized,
{
    fn resolve_lvalues(self) -> Result<Self, SemanticError>;
}

impl ResolveLvalues for parser::Program {
    fn resolve_lvalues(self) -> Result<Self, SemanticError> {
        self.0
            .into_iter()
            .map(|item| item.resolve_lvalues())
            .collect::<Result<_, _>>()
            .map(parser::Program)
    }
}

impl ResolveLvalues for parser::FuncDeclaration {
    fn resolve_lvalues(self) -> Result<Self, SemanticError> {
        self.body
            .map(|body| body.resolve_lvalues())
            .transpose()
            .map(|body| parser::FuncDeclaration {
                identifier: self.identifier,
                parameters: self.parameters,
                body,
                storage_class: self.storage_class,
                ty: self.ty,
            })
    }
}

impl ResolveLvalues for parser::Block {
    fn resolve_lvalues(self) -> Result<Self, SemanticError> {
        Ok(parser::Block(
            self.0
                .into_iter()
                .map(|item| item.resolve_lvalues())
                .collect::<Result<_, _>>()?,
        ))
    }
}

impl ResolveLvalues for parser::BlockItem {
    fn resolve_lvalues(self) -> Result<Self, SemanticError> {
        match self {
            parser::BlockItem::Statement(statement) => {
                Ok(Self::Statement(statement.resolve_lvalues()?))
            }
            parser::BlockItem::Declaration(declaration) => declaration
                .resolve_lvalues()
                .map(parser::BlockItem::Declaration),
        }
    }
}

impl ResolveLvalues for parser::Declaration {
    fn resolve_lvalues(self) -> Result<Self, SemanticError> {
        match self {
            parser::Declaration::Variable(variable_declaration) => variable_declaration
                .resolve_lvalues()
                .map(parser::Declaration::Variable),
            parser::Declaration::Function(func_declaration) => func_declaration
                .resolve_lvalues()
                .map(parser::Declaration::Function),
        }
    }
}

impl ResolveLvalues for parser::VariableDeclaration {
    fn resolve_lvalues(self) -> Result<Self, SemanticError> {
        Ok(self)
    }
}

impl ResolveLvalues for parser::Statement {
    fn resolve_lvalues(self) -> Result<Self, SemanticError> {
        match self {
            parser::Statement::Return(expression) => expression.resolve_lvalues().map(Self::Return),
            parser::Statement::Labeled(label, statement) => statement
                .resolve_lvalues()
                .map(|stmt| parser::Statement::Labeled(label, Box::new(stmt))),
            parser::Statement::Goto(label) => Ok(parser::Statement::Goto(label)),
            parser::Statement::Expression(expression) => {
                expression.resolve_lvalues().map(Self::Expression)
            }
            parser::Statement::If {
                condition,
                then_branch,
                else_branch,
            } => Ok(parser::Statement::If {
                condition: condition.resolve_lvalues()?,
                then_branch: Box::new(then_branch.resolve_lvalues()?),
                else_branch: else_branch
                    .map(|else_branch| else_branch.resolve_lvalues())
                    .transpose()?
                    .map(Box::new),
            }),
            parser::Statement::Block(block) => {
                block.resolve_lvalues().map(parser::Statement::Block)
            }
            parser::Statement::While {
                condition,
                body,
                label,
            } => Ok(parser::Statement::While {
                condition: condition.resolve_lvalues()?,
                body: Box::new(body.resolve_lvalues()?),
                label,
            }),
            parser::Statement::DoWhile {
                body,
                condition,
                label,
            } => Ok(parser::Statement::DoWhile {
                body: Box::new(body.resolve_lvalues()?),
                condition: condition.resolve_lvalues()?,
                label,
            }),
            parser::Statement::For {
                init,
                condition,
                post,
                body,
                label,
            } => Ok(parser::Statement::For {
                init: init.resolve_lvalues()?,
                condition: condition.map(|cond| cond.resolve_lvalues()).transpose()?,
                post: post.map(|post| post.resolve_lvalues()).transpose()?,
                body: Box::new(body.resolve_lvalues()?),
                label,
            }),
            parser::Statement::Null
            | parser::Statement::Break(_)
            | parser::Statement::Continue(_) => Ok(self),
            parser::Statement::Case(expression, statement, label) => Ok(parser::Statement::Case(
                expression.resolve_lvalues()?,
                Box::new(statement.resolve_lvalues()?),
                label,
            )),
            parser::Statement::Default(statement, label) => statement
                .resolve_lvalues()
                .map(|stmt| parser::Statement::Default(Box::new(stmt), label)),
            parser::Statement::Switch {
                condition,
                body,
                label,
                cases,
                default_exists,
            } => Ok(parser::Statement::Switch {
                condition: condition.resolve_lvalues()?,
                body: Box::new(body.resolve_lvalues()?),
                label,
                cases,
                default_exists,
            }),
        }
    }
}

impl ResolveLvalues for parser::ForInit {
    fn resolve_lvalues(self) -> Result<Self, SemanticError> {
        match self {
            parser::ForInit::Declaration(declaration) => declaration
                .resolve_lvalues()
                .map(parser::ForInit::Declaration),
            parser::ForInit::Expression(expression) => expression
                .map(|expr| expr.resolve_lvalues())
                .transpose()
                .map(parser::ForInit::Expression),
        }
    }
}

impl ResolveLvalues for parser::TypedExpression {
    fn resolve_lvalues(self) -> Result<Self, SemanticError> {
        self.expression
            .resolve_lvalues()
            .map(|expression| parser::TypedExpression {
                ty: self.ty,
                expression,
            })
    }
}

impl ResolveLvalues for parser::Expression {
    fn resolve_lvalues(self) -> Result<Self, SemanticError> {
        match self {
            Self::Factor(factor) => factor.resolve_lvalues().map(Self::Factor),
            Self::BinaryOp { op, left, right } => Ok(Self::BinaryOp {
                op,
                left: Box::new(left.resolve_lvalues()?),
                right: Box::new(right.resolve_lvalues()?),
            }),
            Self::Assignment { left, right } => {
                if left.is_lvalue() {
                    Ok(Self::Assignment {
                        left: Box::new(left.resolve_lvalues()?),
                        right: Box::new(right.resolve_lvalues()?),
                    })
                } else {
                    Err(SemanticError::InvalidLvalue(left.expression))
                }
            }
            Self::Conditional {
                condition,
                then_branch,
                else_branch,
            } => Ok(Self::Conditional {
                condition: Box::new(condition.resolve_lvalues()?),
                then_branch: Box::new(then_branch.resolve_lvalues()?),
                else_branch: Box::new(else_branch.resolve_lvalues()?),
            }),
        }
    }
}

impl ResolveLvalues for parser::Factor {
    fn resolve_lvalues(self) -> Result<Self, SemanticError> {
        match self {
            parser::Factor::UnaryOp { op, fac } => {
                if matches!(
                    op,
                    parser::UnaryOperator::PrefixIncrement | parser::UnaryOperator::PrefixDecrement
                ) && !fac.is_lvalue()
                {
                    return Err(SemanticError::InvalidLvalue(parser::Expression::Factor(
                        parser::Factor::UnaryOp { op, fac },
                    )));
                }
                Ok(parser::Factor::UnaryOp {
                    op,
                    fac: Box::new(fac.resolve_lvalues()?),
                })
            }
            parser::Factor::Postfix(postfix) => {
                if postfix.postfix.len() > 1
                    || (postfix.postfix.len() > 0 && !postfix.primary.is_lvalue())
                {
                    return Err(SemanticError::InvalidLvalue(parser::Expression::Factor(
                        parser::Factor::Postfix(postfix),
                    )));
                }
                Ok(parser::Factor::Postfix(postfix.resolve_lvalues()?))
            }
            parser::Factor::Cast { ty, fac } => Ok(parser::Factor::Cast {
                ty,
                fac: Box::new(fac.resolve_lvalues()?),
            }),
        }
    }
}

impl ResolveLvalues for parser::Postfix {
    fn resolve_lvalues(self) -> Result<Self, SemanticError> {
        self.primary.resolve_lvalues().map(|primary| Self {
            primary,
            postfix: self.postfix,
        })
    }
}

impl ResolveLvalues for parser::Primary {
    fn resolve_lvalues(self) -> Result<Self, SemanticError> {
        match self {
            parser::Primary::Constant(val) => Ok(parser::Primary::Constant(val)),
            parser::Primary::Var(identifier) => Ok(parser::Primary::Var(identifier)),
            parser::Primary::Paren(expression) => expression
                .resolve_lvalues()
                .map(|expression| parser::Primary::Paren(Box::new(expression))),
            parser::Primary::FunctionCall(_, _) => Ok(self),
        }
    }
}
