use std::collections::HashMap;

use crate::{
    parser,
    semantic_analysis::{SemanticError, get_expression_value},
    types::{self, Type},
};

#[derive(Debug, Clone)]
pub enum IdentifierAttribute {
    Function { defined: bool, global: bool },
    Static { init: InitialValue, global: bool },
    Local,
}

#[derive(Debug, Clone)]
pub enum InitialValue {
    Tentative,
    Initial(types::Constant),
    NoInitialiser,
}

#[derive(Default, Debug)]
pub struct SymbolTable {
    pub symbols: HashMap<String, (Type, IdentifierAttribute)>,
}

impl SymbolTable {
    pub fn has_symbol(&self, identifier: impl Into<String>) -> bool {
        let identifier = identifier.into();
        self.symbols.contains_key(&identifier)
    }

    pub fn is_global_function(&self, identifier: impl Into<String>) -> bool {
        let identifier = identifier.into();
        if let Some((_, IdentifierAttribute::Function { global, .. })) =
            self.symbols.get(&identifier)
        {
            *global
        } else {
            unreachable!("Symbol {} is not a function", identifier)
        }
    }

    pub fn is_static_var(&self, identifier: impl Into<String>) -> bool {
        let identifier = identifier.into();
        matches!(
            self.symbols.get(&identifier),
            Some((_, IdentifierAttribute::Static { .. }))
        )
    }

    fn block_scope_variable_declared(
        &mut self,
        identifier: impl Into<String>,
        ty: Type,
        init: Option<&parser::TypedExpression>,
        storage_class: Option<parser::StorageClass>,
    ) -> Result<String, SemanticError> {
        let identifier = identifier.into();
        match storage_class {
            Some(parser::StorageClass::Extern) => {
                if init.is_some() {
                    return Err(SemanticError::InitOnLocalExtern(identifier));
                }
                if let Some((prev_type, _prev_attr)) = self.symbols.get(&identifier) {
                    if prev_type != &ty {
                        return Err(SemanticError::TypeMismatch(format!(
                            "Variable {} redeclared as different type",
                            identifier
                        )));
                    }
                } else {
                    self.symbols.insert(
                        identifier.clone(),
                        (
                            ty,
                            IdentifierAttribute::Static {
                                init: InitialValue::NoInitialiser,
                                global: true,
                            },
                        ),
                    );
                }
            }
            Some(parser::StorageClass::Static) => {
                let initial_value = if let Some(Some(i)) =
                    init.map(|ty_exp| get_expression_value(&ty_exp.expression))
                {
                    InitialValue::Initial(i)
                } else if init.is_none() {
                    InitialValue::Initial(
                        types::Constant::zero_with_type(&ty)
                            .expect("Cannot create zero value for type"),
                    )
                } else {
                    return Err(SemanticError::NonConstantExpression);
                };
                self.symbols.insert(
                    identifier.clone(),
                    (
                        ty,
                        IdentifierAttribute::Static {
                            init: initial_value,
                            global: false,
                        },
                    ),
                );
            }
            _ => {
                self.symbols
                    .insert(identifier.clone(), (ty, IdentifierAttribute::Local));
            }
        }

        Ok(identifier)
    }

    fn file_scope_variable_declared(
        &mut self,
        identifier: impl Into<String>,
        ty: Type,
        init: Option<&parser::TypedExpression>,
        storage_class: Option<parser::StorageClass>,
    ) -> Result<String, SemanticError> {
        let identifier = identifier.into();

        let mut initial_value =
            if let Some(Some(i)) = init.map(|ty_exp| get_expression_value(&ty_exp.expression)) {
                InitialValue::Initial(i)
            } else if init.is_none() {
                if storage_class == Some(parser::StorageClass::Extern) {
                    InitialValue::NoInitialiser
                } else {
                    InitialValue::Tentative
                }
            } else {
                return Err(SemanticError::NonConstantExpression);
            };

        let mut global = storage_class != Some(parser::StorageClass::Static);

        if let Some((prev_type, prev_attr)) = self.symbols.get(&identifier) {
            if prev_type != &ty {
                return Err(SemanticError::TypeMismatch(format!(
                    "Variable {} redeclared as different type",
                    identifier
                )));
            }
            let (old_init, old_global) = match prev_attr {
                IdentifierAttribute::Static { init, global } => (Some(init), *global),
                IdentifierAttribute::Local => (None, false),
                IdentifierAttribute::Function { .. } => unreachable!(),
            };
            if storage_class == Some(parser::StorageClass::Extern) {
                global = old_global;
            } else if global != old_global {
                return Err(SemanticError::ConflictingVariableLinkage(identifier));
            }

            if matches!(old_init, Some(&InitialValue::Initial(_))) {
                if matches!(initial_value, InitialValue::Initial(_)) {
                    return Err(SemanticError::ConflictingFileScopeDeclarations(identifier));
                } else {
                    initial_value = old_init.unwrap().clone();
                }
            } else if !matches!(initial_value, InitialValue::Initial(_))
                && matches!(old_init, Some(&InitialValue::Tentative))
            {
                initial_value = InitialValue::Tentative;
            }
        }

        self.symbols.insert(
            identifier.clone(),
            (
                ty,
                IdentifierAttribute::Static {
                    init: initial_value,
                    global,
                },
            ),
        );

        Ok(identifier)
    }

    fn function_declared(
        &mut self,
        identifier: impl Into<String>,
        ty: Type,
        has_body: bool,
        storage_class: Option<parser::StorageClass>,
    ) -> Result<String, SemanticError> {
        let identifier = identifier.into();
        let mut alread_defined = false;
        let mut global = storage_class != Some(parser::StorageClass::Static);

        if let Some(prev_info) = self.symbols.get(&identifier) {
            if prev_info.0 != ty {
                return Err(SemanticError::IncompatibleFunctionDeclaration(identifier));
            }
            let (
                _,
                IdentifierAttribute::Function {
                    defined: prev_defined,
                    global: prev_global,
                },
            ) = prev_info
            else {
                unreachable!()
            };
            alread_defined = *prev_defined;
            if alread_defined && has_body {
                return Err(SemanticError::FunctionDefinedMoreThanOnce(identifier));
            }

            if *prev_global && storage_class == Some(parser::StorageClass::Static) {
                return Err(SemanticError::StaticAfterNonStatic(identifier));
            }
            global = *prev_global;
        }

        self.symbols.insert(
            identifier.clone(),
            (
                ty,
                IdentifierAttribute::Function {
                    defined: alread_defined || has_body,
                    global,
                },
            ),
        );

        Ok(identifier)
    }

    fn loopup_identifier(
        &self,
        identifier: impl Into<String>,
    ) -> Result<(String, Type), SemanticError> {
        let identifier = identifier.into();
        if let Some((ty, _)) = self.symbols.get(&identifier) {
            Ok((identifier, ty.clone()))
        } else {
            Err(SemanticError::IdentifierNotDeclared(identifier))
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
                .map(|declaration| match declaration {
                    parser::Declaration::Variable(var_decl) => {
                        let identifier = symbol_table.file_scope_variable_declared(
                            var_decl.identifier,
                            var_decl.ty.clone(),
                            var_decl.initialiser.as_ref(),
                            var_decl.storage_class,
                        )?;
                        Ok(parser::Declaration::Variable(parser::VariableDeclaration {
                            identifier,
                            initialiser: var_decl.initialiser,
                            storage_class: var_decl.storage_class,
                            ty: var_decl.ty,
                        }))
                    }
                    parser::Declaration::Function(func_decl) => func_decl
                        .resolve_types(symbol_table)
                        .map(parser::Declaration::Function),
                })
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

impl ResolveTypes for parser::FuncDeclaration {
    fn resolve_types(self, symbol_table: &mut SymbolTable) -> Result<Self, SemanticError> {
        let identifier = symbol_table.function_declared(
            self.identifier,
            self.ty.clone(),
            self.body.is_some(),
            self.storage_class,
        )?;

        if let Some(body) = self.body {
            for (param, param_ty) in self
                .parameters
                .0
                .iter()
                .zip(self.ty.get_function_params().unwrap())
            {
                symbol_table.block_scope_variable_declared(
                    param.clone(),
                    param_ty.clone(),
                    None,
                    None,
                )?;
            }

            let mut body = body.resolve_types(symbol_table)?;
            for item in &mut body.0 {
                if let parser::BlockItem::Statement(parser::Statement::Return(exp)) = item {
                    let _exp_ty = exp.find_type(symbol_table)?;
                    let Type::Function { return_type, .. } = &self.ty else {
                        unreachable!();
                    };
                    *exp = convert_to(exp.clone(), *return_type.clone());
                }
            }

            Ok(parser::FuncDeclaration {
                identifier: identifier,
                parameters: self.parameters,
                body: Some(body.resolve_types(symbol_table)?),
                storage_class: self.storage_class,
                ty: self.ty,
            })
        } else {
            Ok(parser::FuncDeclaration {
                identifier: identifier,
                parameters: self.parameters,
                body: None,
                storage_class: self.storage_class,
                ty: self.ty,
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
            identifier: symbol_table.block_scope_variable_declared(
                self.identifier,
                self.ty.clone(),
                self.initialiser.as_ref(),
                self.storage_class,
            )?,
            initialiser: self
                .initialiser
                .map(|init| init.resolve_types(symbol_table))
                .transpose()?
                .map(|init| convert_to(init, self.ty.clone())),
            storage_class: self.storage_class,
            ty: self.ty,
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
            } => {
                if matches!(init, parser::ForInit::Declaration(parser::VariableDeclaration { storage_class, ..}) if storage_class.is_some())
                {
                    return Err(SemanticError::InvalidStorageClass(format!(
                        "Storage class in for loop initialiser"
                    )));
                }
                Ok(parser::Statement::For {
                    init: init.resolve_types(symbol_table)?,
                    condition: condition
                        .map(|condition| condition.resolve_types(symbol_table))
                        .transpose()?,
                    post: post
                        .map(|post| post.resolve_types(symbol_table))
                        .transpose()?,
                    body: Box::new(body.resolve_types(symbol_table)?),
                    label,
                })
            }
            parser::Statement::Switch {
                condition,
                body,
                label,
                cases,
                default_exists,
            } => {
                let mut condition = condition.resolve_types(symbol_table)?;
                let ty = condition.find_type(symbol_table)?;
                let cases = cases
                    .into_iter()
                    .map(|constant| {
                        constant.cast_to_type(ty.clone()).ok_or_else(|| {
                            SemanticError::TypeMismatch(format!(
                                "Case value {:?} cannot be converted to type {:?}",
                                constant, condition.ty
                            ))
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                let mut used_cases = std::collections::HashSet::new();
                for case in &cases {
                    if !used_cases.insert(case.clone()) {
                        return Err(SemanticError::DuplicateCaseValue(case.clone()));
                    }
                }

                Ok(parser::Statement::Switch {
                    condition,
                    body: Box::new(body.resolve_types(symbol_table)?),
                    label,
                    cases,
                    default_exists,
                })
            }
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

impl ResolveTypes for parser::TypedExpression {
    fn resolve_types(self, symbol_table: &mut SymbolTable) -> Result<Self, SemanticError> {
        let mut expression = self.expression.resolve_types(symbol_table)?;
        let ty = expression.find_type(symbol_table)?;
        Ok(parser::TypedExpression {
            expression,
            ty: Some(ty),
        })
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
            parser::Factor::Cast { ty, fac } => Ok(parser::Factor::Cast {
                ty,
                fac: Box::new(fac.resolve_types(symbol_table)?),
            }),
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
                .loopup_identifier(identifier)
                .and_then(|(identifier, ty)| {
                    if matches!(ty, Type::Function { .. }) {
                        Err(SemanticError::TypeMismatch(format!(
                            "Function {} used as variable",
                            identifier
                        )))
                    } else {
                        Ok(identifier)
                    }
                })
                .map(parser::Primary::Var),
            parser::Primary::FunctionCall(identifier, argument_list) => {
                let identifier =
                    symbol_table
                        .loopup_identifier(identifier)
                        .and_then(|(identifier, ty)| {
                            if let Type::Function { params, .. } = ty {
                                if params.len() != argument_list.0.len() {
                                    Err(SemanticError::TypeMismatch(format!(
                                        "Function {} expects {} arguments, but {} were provided",
                                        identifier,
                                        params.len(),
                                        argument_list.0.len()
                                    )))
                                } else {
                                    Ok(identifier)
                                }
                            } else {
                                Err(SemanticError::TypeMismatch(format!(
                                    "Identifier {} is not a function",
                                    identifier
                                )))
                            }
                        })?;
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

pub trait FindType {
    fn find_type(&mut self, symbol_table: &SymbolTable) -> Result<Type, SemanticError>;
}

impl FindType for parser::TypedExpression {
    fn find_type(&mut self, symbol_table: &SymbolTable) -> Result<Type, SemanticError> {
        match &self.ty {
            Some(ty) => Ok(ty.clone()),
            None => {
                let ty = self.expression.find_type(symbol_table)?;
                self.ty = Some(ty.clone());
                Ok(ty)
            }
        }
    }
}

fn convert_to(expression: parser::TypedExpression, target_type: Type) -> parser::TypedExpression {
    if expression.ty.as_ref() == Some(&target_type) {
        expression
    } else {
        parser::TypedExpression {
            expression: parser::Expression::Factor(parser::Factor::Cast {
                ty: target_type.clone(),
                fac: Box::new(parser::Factor::Postfix(parser::Postfix {
                    postfix: Vec::new(),
                    primary: parser::Primary::Paren(Box::new(expression)),
                })),
            }),
            ty: Some(target_type),
        }
    }
}

impl FindType for parser::Expression {
    fn find_type(&mut self, symbol_table: &SymbolTable) -> Result<Type, SemanticError> {
        match self {
            parser::Expression::Factor(factor) => factor.find_type(symbol_table),
            parser::Expression::BinaryOp { op, left, right } => {
                if matches!(
                    *op,
                    parser::BinaryOperator::LogicalAnd | parser::BinaryOperator::LogicalOr
                ) {
                    return Ok(Type::Int);
                }

                let left_type = left.find_type(symbol_table)?;
                let right_type = right.find_type(symbol_table)?;
                let common_type = Type::get_common_type(&left_type, &right_type);

                *left = Box::new(convert_to(*left.clone(), common_type.clone()));
                *right = Box::new(convert_to(*right.clone(), common_type.clone()));

                Ok(match *op {
                    parser::BinaryOperator::Add
                    | parser::BinaryOperator::Subtract
                    | parser::BinaryOperator::Multiply
                    | parser::BinaryOperator::Divide
                    | parser::BinaryOperator::Remainder
                    | parser::BinaryOperator::BitwiseAnd
                    | parser::BinaryOperator::BitwiseOr
                    | parser::BinaryOperator::BitwiseXor => common_type,
                    parser::BinaryOperator::LeftShift | parser::BinaryOperator::RightShift => {
                        left_type
                    }
                    _ => Type::Int,
                })
            }
            parser::Expression::Assignment { left, right } => {
                let left_type = left.find_type(symbol_table)?;
                let _right_type = right.find_type(symbol_table)?;
                *right = Box::new(convert_to(*right.clone(), left_type.clone()));
                Ok(left_type)
            }
            parser::Expression::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                let _condition_type = condition.find_type(symbol_table)?;
                let then_type = then_branch.find_type(symbol_table)?;
                let else_type = else_branch.find_type(symbol_table)?;
                let common_type = Type::get_common_type(&then_type, &else_type);

                *then_branch = Box::new(convert_to(*then_branch.clone(), common_type.clone()));
                *else_branch = Box::new(convert_to(*else_branch.clone(), common_type.clone()));

                Ok(common_type)
            }
        }
    }
}

impl FindType for parser::Factor {
    fn find_type(&mut self, symbol_table: &SymbolTable) -> Result<Type, SemanticError> {
        match self {
            parser::Factor::UnaryOp { op, fac } => {
                let inner = fac.find_type(symbol_table)?;
                Ok(match op {
                    parser::UnaryOperator::LogicalNot => Type::Int,
                    parser::UnaryOperator::Complement
                    | parser::UnaryOperator::Negate
                    | parser::UnaryOperator::PrefixIncrement
                    | parser::UnaryOperator::PrefixDecrement => inner,
                })
            }
            parser::Factor::Postfix(postfix) => postfix.find_type(symbol_table),
            parser::Factor::Cast { ty, fac } => {
                let _ = fac.find_type(symbol_table)?;
                Ok(ty.clone())
            }
        }
    }
}

impl FindType for parser::Postfix {
    fn find_type(&mut self, symbol_table: &SymbolTable) -> Result<Type, SemanticError> {
        self.primary.find_type(symbol_table)
    }
}

impl FindType for parser::Primary {
    fn find_type(&mut self, symbol_table: &SymbolTable) -> Result<Type, SemanticError> {
        match self {
            parser::Primary::Constant(constant) => Ok(Type::from(constant.to_type())),
            parser::Primary::Var(identifier) => {
                let ty = symbol_table.loopup_identifier(&*identifier)?.1;
                if matches!(ty, Type::Function { .. }) {
                    unreachable!()
                }
                Ok(ty)
            }
            parser::Primary::Paren(typed_expression) => typed_expression.find_type(symbol_table),
            parser::Primary::FunctionCall(identifier, args) => {
                let ty = symbol_table.loopup_identifier(&*identifier)?.1;
                if let Type::Function {
                    params,
                    return_type,
                } = ty
                {
                    args.0
                        .iter_mut()
                        .zip(params.iter())
                        .for_each(|(arg, param_type)| {
                            *arg = convert_to(arg.clone(), param_type.clone());
                        });

                    Ok(*return_type)
                } else {
                    unreachable!()
                }
            }
        }
    }
}
