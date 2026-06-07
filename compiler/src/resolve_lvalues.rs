use crate::parser;
use crate::semantic_analysis::SemanticError;

pub fn resolve_lvalues(program: parser::Program) -> Result<parser::Program, SemanticError> {
    program.resolve_lvalues()
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
            parser::Expression::Conditional { .. } => false,
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

trait ResolveLvalues
where
    Self: Sized,
{
    fn resolve_lvalues(self) -> Result<Self, SemanticError>;
}

impl ResolveLvalues for parser::Program {
    fn resolve_lvalues(self) -> Result<Self, SemanticError> {
        self.0.resolve_lvalues().map(parser::Program)
    }
}

impl ResolveLvalues for parser::FuncDef {
    fn resolve_lvalues(self) -> Result<Self, SemanticError> {
        self.body.resolve_lvalues().map(|body| parser::FuncDef {
            name: self.name,
            body,
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
        Ok(self)
    }
}

impl ResolveLvalues for parser::Statement {
    fn resolve_lvalues(self) -> Result<Self, SemanticError> {
        match self {
            Self::Return(expression) => expression.resolve_lvalues().map(Self::Return),
            Self::Labeled(label, statement) => statement
                .resolve_lvalues()
                .map(|stmt| Self::Labeled(label, Box::new(stmt))),
            Self::Goto(label) => Ok(Self::Goto(label)),
            Self::Expression(expression) => expression.resolve_lvalues().map(Self::Expression),
            Self::If {
                condition,
                then_branch,
                else_branch,
            } => Ok(Self::If {
                condition: condition.resolve_lvalues()?,
                then_branch: Box::new(then_branch.resolve_lvalues()?),
                else_branch: else_branch
                    .map(|else_branch| else_branch.resolve_lvalues())
                    .transpose()?
                    .map(Box::new),
            }),
            Self::Block(block) => block.resolve_lvalues().map(Self::Block),
            parser::Statement::While {
                condition,
                body,
                label,
            } => Ok(Self::While {
                condition: condition.resolve_lvalues()?,
                body: Box::new(body.resolve_lvalues()?),
                label,
            }),
            parser::Statement::DoWhile {
                body,
                condition,
                label,
            } => Ok(Self::DoWhile {
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
        }
    }
}

impl ResolveLvalues for parser::InitExp {
    fn resolve_lvalues(self) -> Result<Self, SemanticError> {
        match self {
            parser::InitExp::Declaration(declaration) => declaration
                .resolve_lvalues()
                .map(parser::InitExp::Declaration),
            parser::InitExp::Expression(expression) => expression
                .map(|expr| expr.resolve_lvalues())
                .transpose()
                .map(parser::InitExp::Expression),
        }
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
                    Err(SemanticError::InvalidLvalue(*left))
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
            Self::UnaryOp { op, fac } => {
                if matches!(
                    op,
                    parser::UnaryOperator::PrefixIncrement | parser::UnaryOperator::PrefixDecrement
                ) && !fac.is_lvalue()
                {
                    return Err(SemanticError::InvalidLvalue(parser::Expression::Factor(
                        Self::UnaryOp { op, fac },
                    )));
                }
                Ok(Self::UnaryOp {
                    op,
                    fac: Box::new(fac.resolve_lvalues()?),
                })
            }
            Self::Postfix(postfix) => {
                if postfix.postfix.len() > 1
                    || (postfix.postfix.len() > 0 && !postfix.primary.is_lvalue())
                {
                    return Err(SemanticError::InvalidLvalue(parser::Expression::Factor(
                        Self::Postfix(postfix),
                    )));
                }
                Ok(Self::Postfix(postfix.resolve_lvalues()?))
            }
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
            Self::Constant(val) => Ok(Self::Constant(val)),
            Self::Var(identifier) => Ok(Self::Var(identifier)),
            Self::Paren(expression) => expression
                .resolve_lvalues()
                .map(|expression| Self::Paren(Box::new(expression))),
        }
    }
}
