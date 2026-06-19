use std::{collections::HashMap, fmt::Display, sync::atomic::AtomicUsize};

use crate::{parser, semantic_analysis::SemanticError};

static LABEL_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn get_unique_label_name(user_defined_name: impl Display) -> String {
    let id = LABEL_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("label_{}.{}", user_defined_name, id)
}

pub fn resolve_labels(program: parser::Program) -> Result<parser::Program, SemanticError> {
    Ok(parser::Program(
        program
            .0
            .into_iter()
            .map(|declaration| match declaration {
                parser::Declaration::Variable(_) => Ok(declaration),
                parser::Declaration::Function(func_def) => {
                    let mut labels = HashMap::new();
                    let func_def = func_def.collect_labels(&mut labels)?;
                    func_def
                        .resolve_labels(&mut labels)
                        .map(parser::Declaration::Function)
                }
            })
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

trait CollectLabels
where
    Self: Sized,
{
    fn collect_labels(self, labels: &mut HashMap<String, String>) -> Result<Self, SemanticError>;
}

impl CollectLabels for parser::FuncDeclaration {
    fn collect_labels(self, labels: &mut HashMap<String, String>) -> Result<Self, SemanticError> {
        Ok(parser::FuncDeclaration {
            identifier: self.identifier,
            parameters: self.parameters,
            body: self
                .body
                .map(|body| body.collect_labels(labels))
                .transpose()?,
            storage_class: self.storage_class,
        })
    }
}

impl CollectLabels for parser::Block {
    fn collect_labels(self, labels: &mut HashMap<String, String>) -> Result<Self, SemanticError> {
        Ok(parser::Block(
            self.0
                .into_iter()
                .map(|item| item.collect_labels(labels))
                .collect::<Result<_, _>>()?,
        ))
    }
}

impl CollectLabels for parser::BlockItem {
    fn collect_labels(self, labels: &mut HashMap<String, String>) -> Result<Self, SemanticError> {
        match self {
            Self::Statement(statement) => Ok(Self::Statement(statement.collect_labels(labels)?)),
            Self::Declaration(declaration) => Ok(Self::Declaration(declaration)),
        }
    }
}

impl CollectLabels for parser::Statement {
    fn collect_labels(self, labels: &mut HashMap<String, String>) -> Result<Self, SemanticError> {
        match self {
            parser::Statement::Labeled(label, statement) => {
                if labels.contains_key(&label) {
                    return Err(SemanticError::DuplicateLabel(label));
                }
                let unique_label = get_unique_label_name(&label);
                labels.insert(label, unique_label.clone());
                statement
                    .collect_labels(labels)
                    .map(|stmt| parser::Statement::Labeled(unique_label, Box::new(stmt)))
            }
            parser::Statement::If {
                condition,
                then_branch,
                else_branch,
            } => Ok(parser::Statement::If {
                condition,
                then_branch: Box::new(then_branch.collect_labels(labels)?),
                else_branch: else_branch
                    .map(|else_branch| else_branch.collect_labels(labels))
                    .transpose()?
                    .map(Box::new),
            }),
            parser::Statement::Block(block) => {
                block.collect_labels(labels).map(parser::Statement::Block)
            }
            parser::Statement::While {
                condition,
                body,
                label,
            } => Ok(parser::Statement::While {
                condition,
                body: Box::new(body.collect_labels(labels)?),
                label,
            }),
            parser::Statement::DoWhile {
                body,
                condition,
                label,
            } => Ok(parser::Statement::DoWhile {
                body: Box::new(body.collect_labels(labels)?),
                condition,
                label,
            }),
            parser::Statement::For {
                init,
                condition,
                post,
                body,
                label,
            } => Ok(parser::Statement::For {
                init,
                condition,
                post,
                body: Box::new(body.collect_labels(labels)?),
                label,
            }),
            parser::Statement::Goto(_)
            | parser::Statement::Expression(_)
            | parser::Statement::Break(_)
            | parser::Statement::Continue(_)
            | parser::Statement::Return(_)
            | parser::Statement::Null => Ok(self),
            parser::Statement::Case(expression, statement, label) => Ok(parser::Statement::Case(
                expression,
                Box::new(statement.collect_labels(labels)?),
                label,
            )),
            parser::Statement::Default(statement, label) => Ok(parser::Statement::Default(
                Box::new(statement.collect_labels(labels)?),
                label,
            )),
            parser::Statement::Switch {
                condition,
                body,
                label,
                cases,
                default_exists,
            } => Ok(parser::Statement::Switch {
                condition,
                body: Box::new(body.collect_labels(labels)?),
                label,
                cases,
                default_exists,
            }),
        }
    }
}

trait ResolveLabels
where
    Self: Sized,
{
    fn resolve_labels(self, labels: &mut HashMap<String, String>) -> Result<Self, SemanticError>;
}

impl ResolveLabels for parser::FuncDeclaration {
    fn resolve_labels(self, labels: &mut HashMap<String, String>) -> Result<Self, SemanticError> {
        Ok(parser::FuncDeclaration {
            identifier: self.identifier,
            parameters: self.parameters,
            body: self
                .body
                .map(|body| body.resolve_labels(labels))
                .transpose()?,
            storage_class: self.storage_class,
        })
    }
}

impl ResolveLabels for parser::Block {
    fn resolve_labels(self, labels: &mut HashMap<String, String>) -> Result<Self, SemanticError> {
        Ok(parser::Block(
            self.0
                .into_iter()
                .map(|item| item.resolve_labels(labels))
                .collect::<Result<_, _>>()?,
        ))
    }
}

impl ResolveLabels for parser::BlockItem {
    fn resolve_labels(self, labels: &mut HashMap<String, String>) -> Result<Self, SemanticError> {
        match self {
            Self::Statement(statement) => Ok(Self::Statement(statement.resolve_labels(labels)?)),
            Self::Declaration(declaration) => Ok(Self::Declaration(declaration)),
        }
    }
}

impl ResolveLabels for parser::Statement {
    fn resolve_labels(self, labels: &mut HashMap<String, String>) -> Result<Self, SemanticError> {
        match self {
            parser::Statement::Goto(label) => match labels.get(&label) {
                Some(unique_label) => {
                    return Ok(Self::Goto(unique_label.clone()));
                }
                None => {
                    return Err(SemanticError::LabelNotDeclared(label));
                }
            },
            parser::Statement::Labeled(label, statement) => statement
                .resolve_labels(labels)
                .map(|stmt| Self::Labeled(label, Box::new(stmt))),
            parser::Statement::If {
                condition,
                then_branch,
                else_branch,
            } => Ok(Self::If {
                condition,
                then_branch: Box::new(then_branch.resolve_labels(labels)?),
                else_branch: else_branch
                    .map(|else_branch| else_branch.resolve_labels(labels))
                    .transpose()?
                    .map(Box::new),
            }),
            parser::Statement::Block(block) => block.resolve_labels(labels).map(Self::Block),
            parser::Statement::While {
                condition,
                body,
                label,
            } => Ok(Self::While {
                condition,
                body: Box::new(body.resolve_labels(labels)?),
                label,
            }),
            parser::Statement::DoWhile {
                body,
                condition,
                label,
            } => Ok(Self::DoWhile {
                body: Box::new(body.resolve_labels(labels)?),
                condition,
                label,
            }),
            parser::Statement::For {
                init,
                condition,
                post,
                body,
                label,
            } => Ok(Self::For {
                init,
                condition,
                post,
                body: Box::new(body.resolve_labels(labels)?),
                label,
            }),
            parser::Statement::Return(_)
            | parser::Statement::Break(_)
            | parser::Statement::Continue(_)
            | parser::Statement::Expression(_)
            | parser::Statement::Null => Ok(self),
            parser::Statement::Case(expression, statement, label) => Ok(Self::Case(
                expression,
                Box::new(statement.resolve_labels(labels)?),
                label,
            )),
            parser::Statement::Default(statement, label) => Ok(Self::Default(
                Box::new(statement.resolve_labels(labels)?),
                label,
            )),
            parser::Statement::Switch {
                condition,
                body,
                label,
                cases,
                default_exists,
            } => Ok(parser::Statement::Switch {
                condition,
                body: Box::new(body.resolve_labels(labels)?),
                label,
                cases,
                default_exists,
            }),
        }
    }
}
