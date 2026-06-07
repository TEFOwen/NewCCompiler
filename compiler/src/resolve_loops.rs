use std::{fmt::Display, sync::atomic::AtomicUsize};

use crate::{parser, semantic_analysis::SemanticError};

static LOOP_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn get_unique_loop_label_name(loop_type: impl Display) -> String {
    let id = LOOP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("loop_{}.{}", loop_type, id)
}

pub fn resolve_loops(program: parser::Program) -> Result<parser::Program, SemanticError> {
    program.resolve_loops(None)
}

trait ResolveLoops
where
    Self: Sized,
{
    fn resolve_loops(self, current_label: Option<String>) -> Result<Self, SemanticError>;
}

impl ResolveLoops for parser::Program {
    fn resolve_loops(self, current_label: Option<String>) -> Result<Self, SemanticError> {
        Ok(parser::Program(self.0.resolve_loops(current_label)?))
    }
}

impl ResolveLoops for parser::FuncDef {
    fn resolve_loops(self, current_label: Option<String>) -> Result<Self, SemanticError> {
        Ok(parser::FuncDef {
            name: self.name,
            body: self.body.resolve_loops(current_label)?,
        })
    }
}

impl ResolveLoops for parser::Block {
    fn resolve_loops(self, current_label: Option<String>) -> Result<Self, SemanticError> {
        Ok(parser::Block(
            self.0
                .into_iter()
                .map(|item| item.resolve_loops(current_label.clone()))
                .collect::<Result<_, _>>()?,
        ))
    }
}

impl ResolveLoops for parser::BlockItem {
    fn resolve_loops(self, current_label: Option<String>) -> Result<Self, SemanticError> {
        match self {
            Self::Statement(statement) => {
                Ok(Self::Statement(statement.resolve_loops(current_label)?))
            }
            Self::Declaration(declaration) => Ok(Self::Declaration(declaration)),
        }
    }
}

impl ResolveLoops for parser::Statement {
    fn resolve_loops(self, current_label: Option<String>) -> Result<Self, SemanticError> {
        match self {
            parser::Statement::Return(expression) => expression
                .resolve_loops(current_label)
                .map(parser::Statement::Return),
            parser::Statement::Labeled(label, statement) => statement
                .resolve_loops(None)
                .map(|stmt| parser::Statement::Labeled(label, Box::new(stmt))),
            parser::Statement::Expression(expression) => expression
                .resolve_loops(current_label)
                .map(parser::Statement::Expression),
            parser::Statement::If {
                condition,
                then_branch,
                else_branch,
            } => Ok(parser::Statement::If {
                condition: condition.resolve_loops(current_label.clone())?,
                then_branch: Box::new(then_branch.resolve_loops(current_label.clone())?),
                else_branch: else_branch
                    .map(|else_branch| else_branch.resolve_loops(current_label))
                    .transpose()?
                    .map(Box::new),
            }),
            parser::Statement::Break(_) => {
                if current_label.is_none() {
                    Err(SemanticError::BreakNotWithinLoopOrSwitch)
                } else {
                    Ok(parser::Statement::Break(current_label))
                }
            }
            parser::Statement::Continue(_) => {
                if current_label.is_none() {
                    Err(SemanticError::ContinueNotWithinLoop)
                } else {
                    Ok(parser::Statement::Continue(current_label))
                }
            }
            parser::Statement::While {
                condition, body, ..
            } => {
                let label = get_unique_loop_label_name("while");
                Ok(parser::Statement::While {
                    condition: condition.resolve_loops(Some(label.clone()))?,
                    body: Box::new(body.resolve_loops(Some(label.clone()))?),
                    label: Some(label),
                })
            }
            parser::Statement::DoWhile {
                body, condition, ..
            } => {
                let label = get_unique_loop_label_name("do_while");
                Ok(parser::Statement::DoWhile {
                    body: Box::new(body.resolve_loops(Some(label.clone()))?),
                    condition: condition.resolve_loops(Some(label.clone()))?,
                    label: Some(label),
                })
            }
            parser::Statement::For {
                init,
                condition,
                post,
                body,
                ..
            } => {
                let label = get_unique_loop_label_name("for");
                Ok(parser::Statement::For {
                    init: init.resolve_loops(Some(label.clone()))?,
                    condition: condition
                        .map(|cond| cond.resolve_loops(Some(label.clone())))
                        .transpose()?,
                    post: post
                        .map(|post| post.resolve_loops(Some(label.clone())))
                        .transpose()?,
                    body: Box::new(body.resolve_loops(Some(label.clone()))?),
                    label: Some(label),
                })
            }
            parser::Statement::Block(block) => block
                .resolve_loops(current_label)
                .map(parser::Statement::Block),
            parser::Statement::Null | parser::Statement::Goto(_) => Ok(self),
        }
    }
}

impl ResolveLoops for parser::InitExp {
    fn resolve_loops(self, _current_label: Option<String>) -> Result<Self, SemanticError> {
        Ok(self)
    }
}

impl ResolveLoops for parser::Expression {
    fn resolve_loops(self, _current_label: Option<String>) -> Result<Self, SemanticError> {
        Ok(self)
    }
}
