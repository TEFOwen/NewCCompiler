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

pub fn resolve_identifiers(program: parser::Program) -> Result<parser::Program, SemanticError> {
    program.resolve_identifiers(&mut IdentifierResolver::default())
}

#[derive(Default)]
struct IdentifierResolver {
    variables: HashMap<String, IdentifierInfo>,
}

impl IdentifierResolver {
    fn new_scope(&self) -> Self {
        Self {
            variables: self
                .variables
                .clone()
                .into_iter()
                .map(|(name, identifier)| (name, identifier.new_scope()))
                .collect(),
        }
    }

    fn resolve_identifier(&self, identifier: impl Into<String>) -> Result<String, SemanticError> {
        let identifier = identifier.into();
        if let Some(info) = self.variables.get(&identifier) {
            Ok(info.unique_name.clone())
        } else {
            Err(SemanticError::VariableNotDeclared(identifier))
        }
    }

    fn declare_identifier(
        &mut self,
        name: impl Into<String>,
        external: bool,
    ) -> Result<String, SemanticError> {
        let name = name.into();
        if !external {
            if let Some(prev_info) = self.variables.get(&name) {
                if prev_info.from_current_scope {
                    return Err(SemanticError::VariableAlreadyDeclared(name));
                }
            }
            let unique_name = get_unique_variable_name(name.clone());
            self.variables.insert(
                name,
                IdentifierInfo {
                    unique_name: unique_name.clone(),
                    from_current_scope: true,
                    has_linkage: false,
                },
            );
            Ok(unique_name)
        } else {
            if let Some(prev_info) = self.variables.get(&name) {
                if prev_info.from_current_scope && !prev_info.has_linkage {
                    return Err(SemanticError::VariableAlreadyDeclared(name));
                }
            }
            self.variables.insert(
                name.clone(),
                IdentifierInfo {
                    unique_name: name.clone(),
                    from_current_scope: true,
                    has_linkage: true,
                },
            );
            Ok(name)
        }
    }
}

#[derive(Clone)]
struct IdentifierInfo {
    unique_name: String,
    from_current_scope: bool,
    has_linkage: bool,
}

impl IdentifierInfo {
    fn new_scope(self) -> Self {
        Self {
            unique_name: self.unique_name,
            from_current_scope: false,
            has_linkage: self.has_linkage,
        }
    }
}

trait ResolveIdentifiers
where
    Self: Sized,
{
    fn resolve_identifiers(self, resolver: &mut IdentifierResolver) -> Result<Self, SemanticError>;
}

impl ResolveIdentifiers for parser::Program {
    fn resolve_identifiers(self, resolver: &mut IdentifierResolver) -> Result<Self, SemanticError> {
        let mut func_defs = Vec::new();

        for func_def in self.0 {
            func_defs.push(func_def.resolve_identifiers(resolver)?);
        }

        Ok(Self(func_defs))
    }
}

impl ResolveIdentifiers for parser::FuncDeclaration {
    fn resolve_identifiers(self, resolver: &mut IdentifierResolver) -> Result<Self, SemanticError> {
        let identifier = resolver.declare_identifier(self.identifier, true)?;
        let mut new_resolver = resolver.new_scope();
        let parameters = self.parameters.resolve_identifiers(&mut new_resolver)?;

        Ok(Self {
            identifier,
            parameters,
            body: self
                .body
                .map(|body| body.resolve_identifiers(&mut new_resolver))
                .transpose()?,
        })
    }
}

impl ResolveIdentifiers for parser::ParamList {
    fn resolve_identifiers(self, resolver: &mut IdentifierResolver) -> Result<Self, SemanticError> {
        Ok(Self(
            self.0
                .into_iter()
                .map(|param| resolver.declare_identifier(param, false))
                .collect::<Result<_, _>>()?,
        ))
    }
}

impl ResolveIdentifiers for parser::Block {
    fn resolve_identifiers(self, resolver: &mut IdentifierResolver) -> Result<Self, SemanticError> {
        Ok(parser::Block(
            self.0
                .into_iter()
                .map(|item| item.resolve_identifiers(resolver))
                .collect::<Result<_, _>>()?,
        ))
    }
}

impl ResolveIdentifiers for parser::BlockItem {
    fn resolve_identifiers(self, resolver: &mut IdentifierResolver) -> Result<Self, SemanticError> {
        match self {
            parser::BlockItem::Statement(statement) => {
                Ok(Self::Statement(statement.resolve_identifiers(resolver)?))
            }
            parser::BlockItem::Declaration(declaration) => Ok(Self::Declaration(
                declaration.resolve_identifiers(resolver)?,
            )),
        }
    }
}

impl ResolveIdentifiers for parser::Declaration {
    fn resolve_identifiers(self, resolver: &mut IdentifierResolver) -> Result<Self, SemanticError> {
        match self {
            parser::Declaration::Variable(variable_declaration) => variable_declaration
                .resolve_identifiers(resolver)
                .map(Self::Variable),
            parser::Declaration::Function(func_declaration) => {
                // Cannot define function body inside a declaration, only in a top-level function definition
                if func_declaration.body.is_some() {
                    Err(SemanticError::FunctionBodyInDeclaration)
                } else {
                    func_declaration
                        .resolve_identifiers(resolver)
                        .map(Self::Function)
                }
            }
        }
    }
}

impl ResolveIdentifiers for parser::VariableDeclaration {
    fn resolve_identifiers(self, resolver: &mut IdentifierResolver) -> Result<Self, SemanticError> {
        let identifier = resolver.declare_identifier(self.identifier, false)?;
        Ok(Self {
            identifier,
            initialiser: self
                .initialiser
                .map(|init| init.resolve_identifiers(resolver))
                .transpose()?,
        })
    }
}

impl ResolveIdentifiers for parser::Statement {
    fn resolve_identifiers(self, resolver: &mut IdentifierResolver) -> Result<Self, SemanticError> {
        match self {
            parser::Statement::Return(exp) => Ok(parser::Statement::Return(
                exp.resolve_identifiers(resolver)?,
            )),
            parser::Statement::Expression(exp) => Ok(parser::Statement::Expression(
                exp.resolve_identifiers(resolver)?,
            )),
            parser::Statement::If {
                condition,
                then_branch,
                else_branch,
            } => Ok(parser::Statement::If {
                condition: condition.resolve_identifiers(resolver)?,
                then_branch: Box::new(then_branch.resolve_identifiers(resolver)?),
                else_branch: else_branch
                    .map(|else_branch| else_branch.resolve_identifiers(resolver))
                    .transpose()?
                    .map(Box::new),
            }),
            parser::Statement::Block(block) => {
                let mut new_resolver = resolver.new_scope();
                block
                    .resolve_identifiers(&mut new_resolver)
                    .map(parser::Statement::Block)
            }
            parser::Statement::Labeled(label, statement) => Ok(parser::Statement::Labeled(
                label,
                Box::new(statement.resolve_identifiers(resolver)?),
            )),
            parser::Statement::While {
                condition,
                body,
                label,
            } => Ok(parser::Statement::While {
                condition: condition.resolve_identifiers(resolver)?,
                body: Box::new(body.resolve_identifiers(resolver)?),
                label,
            }),
            parser::Statement::DoWhile {
                body,
                condition,
                label,
            } => Ok(parser::Statement::DoWhile {
                body: Box::new(body.resolve_identifiers(resolver)?),
                condition: condition.resolve_identifiers(resolver)?,
                label,
            }),
            parser::Statement::For {
                init,
                condition,
                post,
                body,
                label,
            } => {
                let mut new_resolver = resolver.new_scope();
                Ok(parser::Statement::For {
                    init: init.resolve_identifiers(&mut new_resolver)?,
                    condition: condition
                        .map(|cond| cond.resolve_identifiers(&mut new_resolver))
                        .transpose()?,
                    post: post
                        .map(|post| post.resolve_identifiers(&mut new_resolver))
                        .transpose()?,
                    body: Box::new(body.resolve_identifiers(&mut new_resolver)?),
                    label,
                })
            }
            parser::Statement::Null
            | parser::Statement::Goto(_)
            | parser::Statement::Break(_)
            | parser::Statement::Continue(_) => Ok(self),
            parser::Statement::Case(expression, statement, label) => Ok(parser::Statement::Case(
                expression.resolve_identifiers(resolver)?,
                Box::new(statement.resolve_identifiers(resolver)?),
                label,
            )),
            parser::Statement::Default(statement, label) => Ok(parser::Statement::Default(
                Box::new(statement.resolve_identifiers(resolver)?),
                label,
            )),
            parser::Statement::Switch {
                condition,
                body,
                label,
                cases,
                default_exists,
            } => Ok(parser::Statement::Switch {
                condition: condition.resolve_identifiers(resolver)?,
                body: Box::new(body.resolve_identifiers(resolver)?),
                label,
                cases,
                default_exists,
            }),
        }
    }
}

impl ResolveIdentifiers for parser::ForInit {
    fn resolve_identifiers(self, resolver: &mut IdentifierResolver) -> Result<Self, SemanticError> {
        match self {
            parser::ForInit::Declaration(declaration) => declaration
                .resolve_identifiers(resolver)
                .map(parser::ForInit::Declaration),
            parser::ForInit::Expression(expression) => expression
                .map(|exp| exp.resolve_identifiers(resolver))
                .transpose()
                .map(parser::ForInit::Expression),
        }
    }
}

impl ResolveIdentifiers for parser::Expression {
    fn resolve_identifiers(self, resolver: &mut IdentifierResolver) -> Result<Self, SemanticError> {
        match self {
            parser::Expression::Factor(factor) => {
                Ok(Self::Factor(factor.resolve_identifiers(resolver)?))
            }
            parser::Expression::BinaryOp { op, left, right } => Ok(Self::BinaryOp {
                op,
                left: Box::new(left.resolve_identifiers(resolver)?),
                right: Box::new(right.resolve_identifiers(resolver)?),
            }),
            parser::Expression::Assignment { left, right } => Ok(Self::Assignment {
                left: Box::new(left.resolve_identifiers(resolver)?),
                right: Box::new(right.resolve_identifiers(resolver)?),
            }),
            parser::Expression::Conditional {
                condition,
                then_branch,
                else_branch,
            } => Ok(Self::Conditional {
                condition: Box::new(condition.resolve_identifiers(resolver)?),
                then_branch: Box::new(then_branch.resolve_identifiers(resolver)?),
                else_branch: Box::new(else_branch.resolve_identifiers(resolver)?),
            }),
        }
    }
}

impl ResolveIdentifiers for parser::Factor {
    fn resolve_identifiers(self, resolver: &mut IdentifierResolver) -> Result<Self, SemanticError> {
        match self {
            parser::Factor::UnaryOp { op, fac }
                if matches!(
                    op,
                    parser::UnaryOperator::PrefixIncrement | parser::UnaryOperator::PrefixDecrement
                ) =>
            {
                Ok(Self::UnaryOp {
                    op,
                    fac: Box::new(fac.resolve_identifiers(resolver)?),
                })
            }
            parser::Factor::UnaryOp { op, fac } => Ok(Self::UnaryOp {
                op,
                fac: Box::new(fac.resolve_identifiers(resolver)?),
            }),
            parser::Factor::Postfix(postfix) => postfix
                .resolve_identifiers(resolver)
                .map(parser::Factor::Postfix),
        }
    }
}

impl ResolveIdentifiers for parser::Postfix {
    fn resolve_identifiers(self, resolver: &mut IdentifierResolver) -> Result<Self, SemanticError> {
        Ok(Self {
            primary: self.primary.resolve_identifiers(resolver)?,
            postfix: self.postfix,
        })
    }
}

impl ResolveIdentifiers for parser::Primary {
    fn resolve_identifiers(self, resolver: &mut IdentifierResolver) -> Result<Self, SemanticError> {
        match self {
            parser::Primary::Var(identifier) => {
                Ok(Self::Var(resolver.resolve_identifier(identifier)?))
            }
            parser::Primary::Paren(expression) => Ok(Self::Paren(Box::new(
                expression.resolve_identifiers(resolver)?,
            ))),
            parser::Primary::Constant(val) => Ok(Self::Constant(val)),
            parser::Primary::FunctionCall(name, arguments) => {
                let resolved_name = resolver.resolve_identifier(name)?;
                let args = arguments.resolve_identifiers(resolver)?;
                Ok(Self::FunctionCall(resolved_name, args))
            }
        }
    }
}

impl ResolveIdentifiers for parser::ArgumentList {
    fn resolve_identifiers(self, resolver: &mut IdentifierResolver) -> Result<Self, SemanticError> {
        Ok(Self(
            self.0
                .into_iter()
                .map(|arg| arg.resolve_identifiers(resolver))
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}
