use std::{collections::HashSet, fmt::Display, sync::atomic::AtomicUsize};

use crate::{
    parser,
    semantic_analysis::{SemanticError, get_expression_constant},
};

static LOOP_COUNTER: AtomicUsize = AtomicUsize::new(0);
static SWITCH_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn get_unique_loop_label_name(loop_type: impl Display) -> String {
    let id = LOOP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("loop_{}.{}", loop_type, id)
}

fn get_unique_switch_label_name() -> String {
    let id = SWITCH_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("switch.{}", id)
}

pub fn resolve_loops(program: parser::Program) -> Result<parser::Program, SemanticError> {
    program.resolve_loops(None, None, None, &mut HashSet::new(), &mut false)
}

trait ResolveLoops
where
    Self: Sized,
{
    fn resolve_loops(
        self,
        current_loop: Option<String>,
        current_switch: Option<String>,
        current_break_target: Option<String>,
        used_cases: &mut HashSet<u32>,
        default_used: &mut bool,
    ) -> Result<Self, SemanticError>;
}

impl ResolveLoops for parser::Program {
    fn resolve_loops(
        self,
        current_loop: Option<String>,
        current_switch: Option<String>,
        current_break_target: Option<String>,
        used_cases: &mut HashSet<u32>,
        default_used: &mut bool,
    ) -> Result<Self, SemanticError> {
        Ok(parser::Program(self.0.resolve_loops(
            current_loop,
            current_switch,
            current_break_target,
            used_cases,
            default_used,
        )?))
    }
}

impl ResolveLoops for parser::FuncDef {
    fn resolve_loops(
        self,
        current_loop: Option<String>,
        current_switch: Option<String>,
        current_break_target: Option<String>,
        used_cases: &mut HashSet<u32>,
        default_used: &mut bool,
    ) -> Result<Self, SemanticError> {
        Ok(parser::FuncDef {
            name: self.name,
            body: self.body.resolve_loops(
                current_loop,
                current_switch,
                current_break_target,
                used_cases,
                default_used,
            )?,
        })
    }
}

impl ResolveLoops for parser::Block {
    fn resolve_loops(
        self,
        current_loop: Option<String>,
        current_switch: Option<String>,
        current_break_target: Option<String>,
        used_cases: &mut HashSet<u32>,
        default_used: &mut bool,
    ) -> Result<Self, SemanticError> {
        Ok(parser::Block(
            self.0
                .into_iter()
                .map(|item| {
                    item.resolve_loops(
                        current_loop.clone(),
                        current_switch.clone(),
                        current_break_target.clone(),
                        used_cases,
                        default_used,
                    )
                })
                .collect::<Result<_, _>>()?,
        ))
    }
}

impl ResolveLoops for parser::BlockItem {
    fn resolve_loops(
        self,
        current_loop: Option<String>,
        current_switch: Option<String>,
        current_break_target: Option<String>,
        used_cases: &mut HashSet<u32>,
        default_used: &mut bool,
    ) -> Result<Self, SemanticError> {
        match self {
            Self::Statement(statement) => Ok(Self::Statement(statement.resolve_loops(
                current_loop,
                current_switch,
                current_break_target,
                used_cases,
                default_used,
            )?)),
            Self::Declaration(declaration) => Ok(Self::Declaration(declaration)),
        }
    }
}

impl ResolveLoops for parser::Statement {
    fn resolve_loops(
        self,
        current_loop: Option<String>,
        current_switch: Option<String>,
        current_break_target: Option<String>,
        used_cases: &mut HashSet<u32>,
        default_used: &mut bool,
    ) -> Result<Self, SemanticError> {
        match self {
            parser::Statement::Return(expression) => expression
                .resolve_loops(
                    current_loop,
                    current_switch,
                    current_break_target,
                    used_cases,
                    default_used,
                )
                .map(parser::Statement::Return),
            parser::Statement::Labeled(label, statement) => statement
                .resolve_loops(
                    current_loop,
                    current_switch,
                    current_break_target,
                    used_cases,
                    default_used,
                )
                .map(|stmt| parser::Statement::Labeled(label, Box::new(stmt))),
            parser::Statement::Expression(expression) => expression
                .resolve_loops(
                    current_loop,
                    current_switch,
                    current_break_target,
                    used_cases,
                    default_used,
                )
                .map(parser::Statement::Expression),
            parser::Statement::If {
                condition,
                then_branch,
                else_branch,
            } => Ok(parser::Statement::If {
                condition: condition.resolve_loops(
                    current_loop.clone(),
                    current_switch.clone(),
                    current_break_target.clone(),
                    used_cases,
                    default_used,
                )?,
                then_branch: Box::new(then_branch.resolve_loops(
                    current_loop.clone(),
                    current_switch.clone(),
                    current_break_target.clone(),
                    used_cases,
                    default_used,
                )?),
                else_branch: else_branch
                    .map(|else_branch| {
                        else_branch.resolve_loops(
                            current_loop,
                            current_switch,
                            current_break_target,
                            used_cases,
                            default_used,
                        )
                    })
                    .transpose()?
                    .map(Box::new),
            }),
            parser::Statement::Break(_) => {
                if current_break_target.is_some() {
                    Ok(parser::Statement::Break(current_break_target))
                } else {
                    Err(SemanticError::BreakNotWithinLoopOrSwitch)
                }
            }
            parser::Statement::Continue(_) => {
                if current_loop.is_none() {
                    Err(SemanticError::ContinueNotWithinLoop)
                } else {
                    Ok(parser::Statement::Continue(current_loop))
                }
            }
            parser::Statement::Default(stmt, _) => {
                if *default_used {
                    Err(SemanticError::DuplicateDefaultLabel)
                } else if current_switch.is_none() {
                    Err(SemanticError::DefaultNotWithinSwitch)
                } else {
                    *default_used = true;
                    stmt.resolve_loops(
                        current_loop.clone(),
                        current_switch.clone(),
                        current_break_target,
                        used_cases,
                        default_used,
                    )
                    .map(Box::new)
                    .map(|stmt| parser::Statement::Default(stmt, current_switch))
                }
            }
            parser::Statement::Case(expression, statement, _) => {
                if current_switch.is_none() {
                    return Err(SemanticError::CaseNotWithinSwitch);
                }
                match get_expression_constant(&expression) {
                    Some(value) => {
                        if used_cases.contains(&value) {
                            return Err(SemanticError::DuplicateCaseValue(value.to_string()));
                        }
                        used_cases.insert(value);
                    }
                    None => {
                        return Err(SemanticError::NonConstantExpression);
                    }
                }

                Ok(parser::Statement::Case(
                    expression.resolve_loops(
                        current_loop.clone(),
                        current_switch.clone(),
                        current_break_target.clone(),
                        used_cases,
                        default_used,
                    )?,
                    Box::new(statement.resolve_loops(
                        current_loop,
                        current_switch.clone(),
                        current_break_target.clone(),
                        used_cases,
                        default_used,
                    )?),
                    current_switch,
                ))
            }
            parser::Statement::While {
                condition, body, ..
            } => {
                let label = get_unique_loop_label_name("while");
                Ok(parser::Statement::While {
                    condition: condition.resolve_loops(
                        Some(label.clone()),
                        current_switch.clone(),
                        Some(label.clone()),
                        used_cases,
                        default_used,
                    )?,
                    body: Box::new(body.resolve_loops(
                        Some(label.clone()),
                        current_switch,
                        Some(label.clone()),
                        used_cases,
                        default_used,
                    )?),
                    label: Some(label),
                })
            }
            parser::Statement::DoWhile {
                body, condition, ..
            } => {
                let label = get_unique_loop_label_name("do_while");
                Ok(parser::Statement::DoWhile {
                    body: Box::new(body.resolve_loops(
                        Some(label.clone()),
                        current_switch.clone(),
                        Some(label.clone()),
                        used_cases,
                        default_used,
                    )?),
                    condition: condition.resolve_loops(
                        Some(label.clone()),
                        current_switch,
                        Some(label.clone()),
                        used_cases,
                        default_used,
                    )?,
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
                    init: init.resolve_loops(
                        Some(label.clone()),
                        current_switch.clone(),
                        Some(label.clone()),
                        used_cases,
                        default_used,
                    )?,
                    condition: condition
                        .map(|cond| {
                            cond.resolve_loops(
                                Some(label.clone()),
                                current_switch.clone(),
                                Some(label.clone()),
                                used_cases,
                                default_used,
                            )
                        })
                        .transpose()?,
                    post: post
                        .map(|post| {
                            post.resolve_loops(
                                Some(label.clone()),
                                current_switch.clone(),
                                Some(label.clone()),
                                used_cases,
                                default_used,
                            )
                        })
                        .transpose()?,
                    body: Box::new(body.resolve_loops(
                        Some(label.clone()),
                        current_switch,
                        Some(label.clone()),
                        used_cases,
                        default_used,
                    )?),
                    label: Some(label),
                })
            }
            parser::Statement::Switch(expression, statement, _, _, _) => {
                let label = get_unique_switch_label_name();
                let mut used_cases = HashSet::new();
                let mut default_used = false;
                let expression = expression.resolve_loops(
                    current_loop.clone(),
                    Some(label.clone()),
                    Some(label.clone()),
                    &mut used_cases,
                    &mut default_used,
                )?;
                let statement = Box::new(statement.resolve_loops(
                    current_loop,
                    Some(label.clone()),
                    Some(label.clone()),
                    &mut used_cases,
                    &mut default_used,
                )?);
                Ok(parser::Statement::Switch(
                    expression,
                    statement,
                    Some(label),
                    used_cases.into_iter().collect(),
                    default_used,
                ))
            }
            parser::Statement::Block(block) => block
                .resolve_loops(
                    current_loop,
                    current_switch,
                    current_break_target,
                    used_cases,
                    default_used,
                )
                .map(parser::Statement::Block),
            parser::Statement::Null | parser::Statement::Goto(_) => Ok(self),
        }
    }
}

impl ResolveLoops for parser::InitExp {
    fn resolve_loops(
        self,
        _current_loop: Option<String>,
        _current_switch: Option<String>,
        _current_break_target: Option<String>,
        _used_cases: &mut HashSet<u32>,
        _default_used: &mut bool,
    ) -> Result<Self, SemanticError> {
        Ok(self)
    }
}

impl ResolveLoops for parser::Expression {
    fn resolve_loops(
        self,
        _current_loop: Option<String>,
        _current_switch: Option<String>,
        _current_break_target: Option<String>,
        _used_cases: &mut HashSet<u32>,
        _default_used: &mut bool,
    ) -> Result<Self, SemanticError> {
        Ok(self)
    }
}
