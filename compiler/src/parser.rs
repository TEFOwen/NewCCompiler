use crate::{
    lexer::{Keyword, Symbol, Token, TokenType},
    types::{self, IsType},
};
use itertools::{Itertools, MultiPeek};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("Unexpected token: expected {expected}, found {found}")]
    UnexpectedToken { expected: String, found: Token },
    #[error("End of token stream")]
    EndOfTokenStream,
    #[error("Invalid type or storage class: {0}")]
    InvalidTypeOrStorageClass(Token),
    #[error("Invalid type")]
    InvalidType,
    #[error("Invalid storage class")]
    InvalidStorageClass,
    #[error("Function declaration in for loop initializer is not allowed")]
    FunctionDeclarationInForInit,
    #[error("Type error: {0}")]
    TypeError(#[from] types::TypeError),
}

#[derive(Debug, Clone)]
pub struct Program(pub Vec<Declaration>);

#[derive(Debug, Clone)]
pub enum Declaration {
    Variable(VariableDeclaration),
    Function(FuncDeclaration),
}

#[derive(Debug, Clone)]
pub struct FuncDeclaration {
    pub identifier: String,
    pub parameters: ParamList,
    pub body: Option<Block>,
    pub ty: types::Type,
    pub storage_class: Option<StorageClass>,
}

#[derive(Debug, Clone)]
pub struct VariableDeclaration {
    pub identifier: String,
    pub initialiser: Option<TypedExpression>,
    pub ty: types::Type,
    pub storage_class: Option<StorageClass>,
}

#[derive(Debug, Clone)]
pub struct ParamList(pub Vec<String>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageClass {
    Static,
    Extern,
}

#[derive(Debug, Clone)]
pub struct Block(pub Vec<BlockItem>);

#[derive(Debug, Clone)]
pub enum BlockItem {
    Statement(Statement),
    Declaration(Declaration),
}

#[derive(Debug, Clone)]
pub enum Statement {
    Return(TypedExpression),
    Labeled(String, Box<Statement>),
    Goto(String),
    Expression(TypedExpression),
    If {
        condition: TypedExpression,
        then_branch: Box<Statement>,
        else_branch: Option<Box<Statement>>,
    },
    Break(Option<String>),
    Continue(Option<String>),
    Case(TypedExpression, Box<Statement>, Option<String>),
    Default(Box<Statement>, Option<String>),
    While {
        condition: TypedExpression,
        body: Box<Statement>,
        label: Option<String>,
    },
    DoWhile {
        body: Box<Statement>,
        condition: TypedExpression,
        label: Option<String>,
    },
    For {
        init: ForInit,
        condition: Option<TypedExpression>,
        post: Option<TypedExpression>,
        body: Box<Statement>,
        label: Option<String>,
    },
    Switch {
        condition: TypedExpression,
        body: Box<Statement>,
        label: Option<String>,
        cases: Vec<types::Constant>,
        default_exists: bool,
    },
    Block(Block),
    Null,
}

#[derive(Debug, Clone)]
pub enum ForInit {
    Declaration(VariableDeclaration),
    Expression(Option<TypedExpression>),
}

#[derive(Debug, Clone)]
pub struct TypedExpression {
    pub expression: Expression,
    pub ty: Option<types::Type>,
}

impl From<Expression> for TypedExpression {
    fn from(expression: Expression) -> Self {
        TypedExpression {
            expression,
            ty: None,
        }
    }
}

impl ToAst for TypedExpression {
    fn to_ast(tokens: &mut MultiPeek<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        return Expression::to_ast(tokens).map(|exp| exp.into());
    }
}

#[derive(Debug, Clone)]
pub enum Expression {
    Factor(Factor),
    BinaryOp {
        op: BinaryOperator,
        left: Box<TypedExpression>,
        right: Box<TypedExpression>,
    },
    Assignment {
        left: Box<TypedExpression>,
        right: Box<TypedExpression>,
    },
    Conditional {
        condition: Box<TypedExpression>,
        then_branch: Box<TypedExpression>,
        else_branch: Box<TypedExpression>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    RightShift,

    LogicalAnd,
    LogicalOr,
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessEqual,
    GreaterEqual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Associativity {
    LeftToRight,
    RightToLeft,
}

#[derive(Debug, Clone)]
pub enum Factor {
    UnaryOp { op: UnaryOperator, fac: Box<Factor> },
    Postfix(Postfix),
    Cast { ty: types::Type, fac: Box<Factor> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Complement,
    Negate,
    LogicalNot,
    PrefixIncrement,
    PrefixDecrement,
}

#[derive(Debug, Clone)]
pub struct Postfix {
    pub primary: Primary,
    pub postfix: Vec<PostfixOp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostfixOp {
    PostfixIncrement,
    PostfixDecrement,
}

#[derive(Debug, Clone)]
pub enum Primary {
    Constant(types::Constant),
    Var(String),
    Paren(Box<TypedExpression>),
    FunctionCall(String, ArgumentList),
}

#[derive(Debug, Clone)]
pub struct ArgumentList(pub Vec<TypedExpression>);

trait ToAst
where
    Self: Sized,
{
    fn to_ast(tokens: &mut MultiPeek<impl Iterator<Item = Token>>) -> Result<Self, ParserError>;
}

pub fn parse(tokens: Vec<Token>) -> Result<Program, ParserError> {
    let mut tokens = tokens.into_iter().multipeek();
    Program::to_ast(&mut tokens)
}

macro_rules! expect_token {
    ($tokens:expr, $expected:pat $(if $guard:expr)?, $expected_str:literal) => {
        match $tokens.next() {
            Some(token) if matches!(token.0, $expected $(if $guard)?) => token,
            Some(token) => {
                return Err(ParserError::UnexpectedToken {
                    expected: $expected_str.into(),
                    found: token,
                });
            }
            None => {
                return Err(ParserError::EndOfTokenStream);
            }
        }
    };
}

impl ToAst for Program {
    fn to_ast(tokens: &mut MultiPeek<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        let mut declaration = Vec::new();
        while !matches!(tokens.peek(), Some(Token(TokenType::EndOfFile, _))) {
            tokens.reset_peek();
            declaration.push(Declaration::to_ast(tokens)?);
        }
        expect_token!(tokens, TokenType::EndOfFile, "end of file");

        Ok(Program(declaration))
    }
}

fn parse_types_and_storage_class(
    tokens: impl IntoIterator<Item = Token>,
) -> Result<(types::Type, Option<StorageClass>), ParserError> {
    let mut types = Vec::new();
    let mut storage_classes = Vec::new();

    for token in tokens {
        match token.0 {
            TokenType::Keyword(kwd) if kwd.is_type() => types.push(token),
            TokenType::Keyword(kwd) if kwd.is_storage_class_specifier() => {
                storage_classes.push(token.0)
            }
            _ => return Err(ParserError::InvalidTypeOrStorageClass(token)),
        }
    }

    let ty = types::Type::from_tokens(&types)?;

    if storage_classes.len() > 1 {
        return Err(ParserError::InvalidStorageClass);
    }

    Ok((
        ty,
        match storage_classes.first() {
            Some(TokenType::Keyword(Keyword::Static)) => Some(StorageClass::Static),
            Some(TokenType::Keyword(Keyword::Extern)) => Some(StorageClass::Extern),
            None => None,
            _ => unreachable!(),
        },
    ))
}

trait IsStorageClassSpecifier {
    fn is_storage_class_specifier(&self) -> bool;
}

impl IsStorageClassSpecifier for Keyword {
    fn is_storage_class_specifier(&self) -> bool {
        match self {
            Keyword::Static | Keyword::Extern => true,
            _ => false,
        }
    }
}

trait IsSpecifier {
    fn is_specifier(&self) -> bool;
}

impl IsSpecifier for Keyword {
    fn is_specifier(&self) -> bool {
        self.is_type() || self.is_storage_class_specifier()
    }
}

fn parse_params(
    tokens: &mut MultiPeek<impl Iterator<Item = Token>>,
) -> Result<Vec<(types::Type, String)>, ParserError> {
    match tokens.peek() {
        Some(Token(TokenType::Keyword(Keyword::Void), _)) => {
            tokens.next();
            return Ok(Vec::new());
        }
        Some(Token(TokenType::Keyword(kwd), _)) if kwd.is_type() => {}
        None => return Err(ParserError::EndOfTokenStream),
        _ => {
            return Err(ParserError::UnexpectedToken {
                expected: "type specifier".into(),
                found: tokens.next().unwrap(),
            });
        }
    }

    let mut params = Vec::new();
    loop {
        tokens.reset_peek();
        let types = tokens.peeking_take_while(
            |token| matches!(token.0, TokenType::Keyword(kwd) if kwd.is_type()),
        );
        let ty = types::Type::from_tokens(&types.collect_vec())?;

        let Token(TokenType::Identifier(identifier), _) =
            expect_token!(tokens, TokenType::Identifier(_), "parameter name")
        else {
            unreachable!()
        };
        params.push((ty, identifier));
        if !matches!(
            tokens.peek(),
            Some(Token(TokenType::Symbol(Symbol::Comma), _))
        ) {
            break;
        }
        tokens.next(); // Consume the ',' token
    }

    Ok(params)
}

impl ToAst for Declaration {
    fn to_ast(tokens: &mut MultiPeek<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        let types_and_storage_class_tokens = tokens.peeking_take_while(
            |token| matches!(token.0, TokenType::Keyword(kwd) if kwd.is_specifier()),
        );
        let (ty, storage_class) = parse_types_and_storage_class(types_and_storage_class_tokens)?;
        let Token(TokenType::Identifier(identifier), _) =
            expect_token!(tokens, TokenType::Identifier(_), "identifier")
        else {
            unreachable!()
        };

        if matches!(
            tokens.peek(),
            Some(Token(TokenType::Symbol(Symbol::OpenParen), _))
        ) {
            tokens.next(); // Consume the '(' token
            let parameters = parse_params(tokens)?;
            let (param_types, identifiers): (Vec<types::Type>, Vec<String>) =
                parameters.into_iter().unzip();

            expect_token!(tokens, TokenType::Symbol(Symbol::CloseParen), "')'");

            let body = if matches!(
                tokens.peek(),
                Some(Token(TokenType::Symbol(Symbol::Semicolon), _))
            ) {
                tokens.next();
                None
            } else {
                tokens.reset_peek();
                Some(Block::to_ast(tokens)?)
            };

            Ok(Declaration::Function(FuncDeclaration {
                identifier,
                parameters: ParamList(identifiers),
                body,
                storage_class,
                ty: types::Type::Function {
                    params: param_types,
                    return_type: Box::new(ty),
                },
            }))
        } else {
            match expect_token!(
                tokens,
                TokenType::Symbol(Symbol::Equal | Symbol::Semicolon),
                "'=' or ';'"
            ) {
                Token(TokenType::Symbol(Symbol::Equal), _) => {
                    let initialiser = Some(TypedExpression::to_ast(tokens)?);
                    expect_token!(tokens, TokenType::Symbol(Symbol::Semicolon), "';'");
                    Ok(Declaration::Variable(VariableDeclaration {
                        identifier,
                        initialiser,
                        storage_class,
                        ty,
                    }))
                }
                Token(TokenType::Symbol(Symbol::Semicolon), _) => {
                    Ok(Declaration::Variable(VariableDeclaration {
                        identifier,
                        initialiser: None,
                        storage_class,
                        ty,
                    }))
                }
                _ => unreachable!(),
            }
        }
    }
}

impl ToAst for Block {
    fn to_ast(tokens: &mut MultiPeek<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        expect_token!(tokens, TokenType::Symbol(Symbol::OpenBrace), "'{'");
        let mut items = Vec::new();
        while !matches!(
            tokens.peek(),
            Some(Token(TokenType::Symbol(Symbol::CloseBrace), _))
        ) {
            tokens.reset_peek();
            items.push(BlockItem::to_ast(tokens)?);
        }
        expect_token!(tokens, TokenType::Symbol(Symbol::CloseBrace), "'}'");
        Ok(Block(items))
    }
}

impl ToAst for BlockItem {
    fn to_ast(tokens: &mut MultiPeek<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        if matches!(
            tokens.peek(),
            Some(Token(
                TokenType::Keyword(kwd),
                _
            )) if kwd.is_specifier()
        ) {
            tokens.reset_peek();
            Ok(BlockItem::Declaration(Declaration::to_ast(tokens)?))
        } else {
            tokens.reset_peek();
            Ok(BlockItem::Statement(Statement::to_ast(tokens)?))
        }
    }
}

impl ToAst for Statement {
    fn to_ast(tokens: &mut MultiPeek<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        match tokens.peek() {
            Some(Token(TokenType::Keyword(Keyword::Return), _)) => {
                tokens.next(); // Consume the 'return' token
                let expr = TypedExpression::to_ast(tokens)?;
                expect_token!(tokens, TokenType::Symbol(Symbol::Semicolon), "';'");
                Ok(Statement::Return(expr))
            }
            Some(Token(TokenType::Keyword(Keyword::Break), _)) => {
                tokens.next(); // Consume the 'break' token
                expect_token!(tokens, TokenType::Symbol(Symbol::Semicolon), "';'");
                Ok(Statement::Break(None))
            }
            Some(Token(TokenType::Keyword(Keyword::Continue), _)) => {
                tokens.next(); // Consume the 'continue' token
                expect_token!(tokens, TokenType::Symbol(Symbol::Semicolon), "';'");
                Ok(Statement::Continue(None))
            }
            Some(Token(TokenType::Identifier(_), _)) => {
                if matches!(
                    tokens.peek(),
                    Some(Token(TokenType::Symbol(Symbol::Colon), _))
                ) {
                    let Token(TokenType::Identifier(identifier), _) = tokens.next().unwrap() else {
                        unreachable!()
                    };
                    expect_token!(tokens, TokenType::Symbol(Symbol::Colon), "':'");
                    let stmt = Statement::to_ast(tokens)?;
                    Ok(Statement::Labeled(identifier.clone(), Box::new(stmt)))
                } else {
                    tokens.reset_peek();
                    let expr = TypedExpression::to_ast(tokens)?;
                    expect_token!(tokens, TokenType::Symbol(Symbol::Semicolon), "';'");
                    Ok(Statement::Expression(expr))
                }
            }
            Some(Token(TokenType::Keyword(Keyword::Case), _)) => {
                tokens.next(); // Consume the 'case' token
                let value = TypedExpression::to_ast(tokens)?;
                expect_token!(tokens, TokenType::Symbol(Symbol::Colon), "':'");
                let stmt = Statement::to_ast(tokens)?;
                Ok(Statement::Case(value, Box::new(stmt), None))
            }
            Some(Token(TokenType::Keyword(Keyword::Default), _)) => {
                tokens.next(); // Consume the 'default' token
                expect_token!(tokens, TokenType::Symbol(Symbol::Colon), "':'");
                let stmt = Statement::to_ast(tokens)?;
                Ok(Statement::Default(Box::new(stmt), None))
            }
            Some(Token(TokenType::Keyword(Keyword::Goto), _)) => {
                tokens.next(); // Consume the 'goto' token
                let Token(TokenType::Identifier(label), _) =
                    expect_token!(tokens, TokenType::Identifier(_), "label name")
                else {
                    unreachable!()
                };
                expect_token!(tokens, TokenType::Symbol(Symbol::Semicolon), "';'");
                Ok(Statement::Goto(label))
            }
            Some(Token(TokenType::Keyword(Keyword::If), _)) => {
                tokens.next(); // Consume the 'if' token
                expect_token!(tokens, TokenType::Symbol(Symbol::OpenParen), "'('");
                let condition = TypedExpression::to_ast(tokens)?;
                expect_token!(tokens, TokenType::Symbol(Symbol::CloseParen), "')'");
                let then_branch = Box::new(Statement::to_ast(tokens)?);
                let else_branch = if matches!(
                    tokens.peek(),
                    Some(Token(TokenType::Keyword(Keyword::Else), _))
                ) {
                    tokens.next(); // Consume the 'else' token
                    Some(Box::new(Statement::to_ast(tokens)?))
                } else {
                    None
                };

                tokens.reset_peek();

                Ok(Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                })
            }
            Some(Token(TokenType::Keyword(Keyword::While), _)) => {
                tokens.next(); // Consume the 'while' token
                expect_token!(tokens, TokenType::Symbol(Symbol::OpenParen), "'('");
                let condition = TypedExpression::to_ast(tokens)?;
                expect_token!(tokens, TokenType::Symbol(Symbol::CloseParen), "')'");
                let body = Box::new(Statement::to_ast(tokens)?);
                Ok(Statement::While {
                    condition,
                    body,
                    label: None,
                })
            }
            Some(Token(TokenType::Keyword(Keyword::Do), _)) => {
                tokens.next(); // Consume the 'do' token
                let body = Box::new(Statement::to_ast(tokens)?);
                expect_token!(tokens, TokenType::Keyword(Keyword::While), "while");
                expect_token!(tokens, TokenType::Symbol(Symbol::OpenParen), "'('");
                let condition = TypedExpression::to_ast(tokens)?;
                expect_token!(tokens, TokenType::Symbol(Symbol::CloseParen), "')'");
                expect_token!(tokens, TokenType::Symbol(Symbol::Semicolon), "';'");
                Ok(Statement::DoWhile {
                    body,
                    condition,
                    label: None,
                })
            }
            Some(Token(TokenType::Keyword(Keyword::For), _)) => {
                tokens.next(); // Consume the 'for' token
                expect_token!(tokens, TokenType::Symbol(Symbol::OpenParen), "'('");
                let init = match tokens.peek() {
                    Some(Token(TokenType::Keyword(kwd), _)) if kwd.is_specifier() => {
                        tokens.reset_peek();
                        let Declaration::Variable(var_decl) = Declaration::to_ast(tokens)? else {
                            return Err(ParserError::FunctionDeclarationInForInit);
                        };
                        ForInit::Declaration(var_decl)
                    }
                    Some(Token(TokenType::Symbol(Symbol::Semicolon), _)) => {
                        tokens.next(); // Consume the ';'
                        ForInit::Expression(None)
                    }
                    _ => {
                        tokens.reset_peek();
                        let exp = ForInit::Expression(Some(TypedExpression::to_ast(tokens)?));
                        expect_token!(tokens, TokenType::Symbol(Symbol::Semicolon), "';'");
                        exp
                    }
                };
                let condition = if !matches!(
                    tokens.peek(),
                    Some(Token(TokenType::Symbol(Symbol::Semicolon), _))
                ) {
                    tokens.reset_peek();
                    Some(TypedExpression::to_ast(tokens)?)
                } else {
                    None
                };
                expect_token!(tokens, TokenType::Symbol(Symbol::Semicolon), "';'");
                let post = if !matches!(
                    tokens.peek(),
                    Some(Token(TokenType::Symbol(Symbol::CloseParen), _))
                ) {
                    tokens.reset_peek();
                    Some(TypedExpression::to_ast(tokens)?)
                } else {
                    None
                };
                expect_token!(tokens, TokenType::Symbol(Symbol::CloseParen), "')'");
                let body = Box::new(Statement::to_ast(tokens)?);
                Ok(Statement::For {
                    init,
                    condition,
                    post,
                    body,
                    label: None,
                })
            }
            Some(Token(TokenType::Keyword(Keyword::Switch), _)) => {
                tokens.next(); // Consume the 'switch' token
                expect_token!(tokens, TokenType::Symbol(Symbol::OpenParen), "'('");
                let condition = TypedExpression::to_ast(tokens)?;
                expect_token!(tokens, TokenType::Symbol(Symbol::CloseParen), "')'");
                let body = Box::new(Statement::to_ast(tokens)?);
                Ok(Statement::Switch {
                    condition,
                    body,
                    label: None,
                    cases: Vec::new(),
                    default_exists: false,
                })
            }
            Some(Token(TokenType::Symbol(Symbol::OpenBrace), _)) => {
                tokens.reset_peek();
                Ok(Statement::Block(Block::to_ast(tokens)?))
            }
            Some(Token(TokenType::Symbol(Symbol::Semicolon), _)) => {
                tokens.next(); // Consume the ';'
                Ok(Statement::Null)
            }
            Some(_) => {
                tokens.reset_peek();
                let expr = TypedExpression::to_ast(tokens)?;
                expect_token!(tokens, TokenType::Symbol(Symbol::Semicolon), "';'");
                Ok(Statement::Expression(expr))
            }
            None => Err(ParserError::EndOfTokenStream),
        }
    }
}

impl Expression {
    fn parse_min_prec(
        tokens: &mut MultiPeek<impl Iterator<Item = Token>>,
        min_prec: i32,
    ) -> Result<Self, ParserError> {
        let mut left = Expression::Factor(Factor::to_ast(tokens)?);
        while matches!(
            tokens.peek(),
            Some(Token(TokenType::Symbol(s), _)) if Self::get_precedence(*s).map_or(false, |prec| prec >= min_prec)
        ) {
            let Some(Token(TokenType::Symbol(op), _)) = tokens.next() else {
                unreachable!()
            };

            if Self::associativity(op) == Associativity::RightToLeft {
                if op == Symbol::Question {
                    let middle = Self::parse_min_prec(tokens, 0)?;
                    expect_token!(tokens, TokenType::Symbol(Symbol::Colon), "':'");
                    let right = Self::parse_min_prec(tokens, Self::get_precedence(op).unwrap())?;
                    left = Expression::Conditional {
                        condition: Box::new(left.into()),
                        then_branch: Box::new(middle.into()),
                        else_branch: Box::new(right.into()),
                    };
                } else {
                    let right = Self::parse_min_prec(tokens, Self::get_precedence(op).unwrap())?;
                    left = Expression::Assignment {
                        left: Box::new(left.clone().into()),
                        right: Box::new(Self::op_to_assignment(op, left, right).into()),
                    };
                }
            } else {
                let right = Self::parse_min_prec(tokens, Self::get_precedence(op).unwrap() + 1)?;
                left = Expression::BinaryOp {
                    op: match op {
                        Symbol::Plus => BinaryOperator::Add,
                        Symbol::Hyphen => BinaryOperator::Subtract,
                        Symbol::Asterisk => BinaryOperator::Multiply,
                        Symbol::Slash => BinaryOperator::Divide,
                        Symbol::Percent => BinaryOperator::Remainder,
                        Symbol::Ampersand => BinaryOperator::BitwiseAnd,
                        Symbol::Bar => BinaryOperator::BitwiseOr,
                        Symbol::Hat => BinaryOperator::BitwiseXor,
                        Symbol::DoubleLt => BinaryOperator::LeftShift,
                        Symbol::DoubleGt => BinaryOperator::RightShift,
                        Symbol::DoubleAmp => BinaryOperator::LogicalAnd,
                        Symbol::DoubleBar => BinaryOperator::LogicalOr,
                        Symbol::DoubleEqual => BinaryOperator::Equal,
                        Symbol::NotEqual => BinaryOperator::NotEqual,
                        Symbol::LessThan => BinaryOperator::LessThan,
                        Symbol::GreaterThan => BinaryOperator::GreaterThan,
                        Symbol::LessEqual => BinaryOperator::LessEqual,
                        Symbol::GreaterEqual => BinaryOperator::GreaterEqual,
                        _ => unreachable!(),
                    },
                    left: Box::new(left.into()),
                    right: Box::new(right.into()),
                };
            }
        }

        tokens.reset_peek();

        Ok(left)
    }

    fn get_precedence(op: Symbol) -> Option<i32> {
        match op {
            Symbol::Asterisk | Symbol::Slash | Symbol::Percent => Some(50),
            Symbol::Plus | Symbol::Hyphen => Some(45),
            Symbol::DoubleLt | Symbol::DoubleGt => Some(40),
            Symbol::LessThan | Symbol::GreaterThan | Symbol::LessEqual | Symbol::GreaterEqual => {
                Some(35)
            }
            Symbol::DoubleEqual | Symbol::NotEqual => Some(30),
            Symbol::Ampersand => Some(25),
            Symbol::Hat => Some(20),
            Symbol::Bar => Some(15),
            Symbol::DoubleAmp => Some(10),
            Symbol::DoubleBar => Some(5),
            Symbol::Question => Some(3),
            Symbol::Equal
            | Symbol::PlusEqual
            | Symbol::MinusEqual
            | Symbol::AsteriskEqual
            | Symbol::SlashEqual
            | Symbol::PercentEqual
            | Symbol::AmpersandEqual
            | Symbol::BarEqual
            | Symbol::HatEqual
            | Symbol::DoubleLtEqual
            | Symbol::DoubleGtEqual => Some(1),
            _ => None,
        }
    }

    fn associativity(op: Symbol) -> Associativity {
        match op {
            Symbol::Equal
            | Symbol::PlusEqual
            | Symbol::MinusEqual
            | Symbol::AsteriskEqual
            | Symbol::SlashEqual
            | Symbol::PercentEqual
            | Symbol::AmpersandEqual
            | Symbol::BarEqual
            | Symbol::HatEqual
            | Symbol::DoubleLtEqual
            | Symbol::DoubleGtEqual
            | Symbol::Question => Associativity::RightToLeft,
            _ => Associativity::LeftToRight,
        }
    }

    fn op_to_assignment(op: Symbol, left: Expression, right: Expression) -> Expression {
        match op {
            Symbol::Equal => right,
            Symbol::PlusEqual => Expression::BinaryOp {
                op: BinaryOperator::Add,
                left: Box::new(left.into()),
                right: Box::new(right.into()),
            },
            Symbol::MinusEqual => Expression::BinaryOp {
                op: BinaryOperator::Subtract,
                left: Box::new(left.into()),
                right: Box::new(right.into()),
            },
            Symbol::AsteriskEqual => Expression::BinaryOp {
                op: BinaryOperator::Multiply,
                left: Box::new(left.into()),
                right: Box::new(right.into()),
            },
            Symbol::SlashEqual => Expression::BinaryOp {
                op: BinaryOperator::Divide,
                left: Box::new(left.into()),
                right: Box::new(right.into()),
            },
            Symbol::PercentEqual => Expression::BinaryOp {
                op: BinaryOperator::Remainder,
                left: Box::new(left.into()),
                right: Box::new(right.into()),
            },
            Symbol::DoubleLtEqual => Expression::BinaryOp {
                op: BinaryOperator::LeftShift,
                left: Box::new(left.into()),
                right: Box::new(right.into()),
            },
            Symbol::DoubleGtEqual => Expression::BinaryOp {
                op: BinaryOperator::RightShift,
                left: Box::new(left.into()),
                right: Box::new(right.into()),
            },
            Symbol::AmpersandEqual => Expression::BinaryOp {
                op: BinaryOperator::BitwiseAnd,
                left: Box::new(left.into()),
                right: Box::new(right.into()),
            },
            Symbol::BarEqual => Expression::BinaryOp {
                op: BinaryOperator::BitwiseOr,
                left: Box::new(left.into()),
                right: Box::new(right.into()),
            },
            Symbol::HatEqual => Expression::BinaryOp {
                op: BinaryOperator::BitwiseXor,
                left: Box::new(left.into()),
                right: Box::new(right.into()),
            },
            _ => unreachable!(),
        }
    }
}

impl ToAst for Expression {
    fn to_ast(tokens: &mut MultiPeek<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        Self::parse_min_prec(tokens, 0)
    }
}

impl ToAst for Factor {
    fn to_ast(tokens: &mut MultiPeek<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        match tokens.peek() {
            Some(Token(
                TokenType::Symbol(
                    Symbol::Tilde
                    | Symbol::Hyphen
                    | Symbol::Exclamation
                    | Symbol::DoublePlus
                    | Symbol::DoubleHyphen,
                ),
                _,
            )) => {
                let Some(Token(TokenType::Symbol(s), _)) = tokens.next() else {
                    unreachable!()
                };

                let inner = Factor::to_ast(tokens)?;
                Ok(Factor::UnaryOp {
                    op: match s {
                        Symbol::Tilde => UnaryOperator::Complement,
                        Symbol::Hyphen => UnaryOperator::Negate,
                        Symbol::Exclamation => UnaryOperator::LogicalNot,
                        Symbol::DoublePlus => UnaryOperator::PrefixIncrement,
                        Symbol::DoubleHyphen => UnaryOperator::PrefixDecrement,
                        _ => unreachable!(),
                    },
                    fac: Box::new(inner),
                })
            }
            Some(Token(TokenType::Symbol(Symbol::OpenParen), _)) => {
                if !matches!(tokens.peek(), Some(Token(TokenType::Keyword(kwd), _)) if kwd.is_type())
                {
                    tokens.reset_peek();
                    Postfix::to_ast(tokens).map(Factor::Postfix)
                } else {
                    tokens.next(); // Consume the '(' token
                    let ty_tokens = tokens.peeking_take_while(
                        |token| matches!(token.0, TokenType::Keyword(kwd) if kwd.is_type()),
                    );
                    let ty = types::Type::from_tokens(&ty_tokens.collect_vec())?;
                    expect_token!(tokens, TokenType::Symbol(Symbol::CloseParen), "')'");
                    let inner = Factor::to_ast(tokens)?;
                    Ok(Factor::Cast {
                        ty,
                        fac: Box::new(inner),
                    })
                }
            }
            _ => {
                tokens.reset_peek();
                Postfix::to_ast(tokens).map(Factor::Postfix)
            }
        }
    }
}

impl ToAst for Postfix {
    fn to_ast(tokens: &mut MultiPeek<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        let primary = Primary::to_ast(tokens)?;
        let mut postfix = Vec::new();
        while matches!(
            tokens.peek(),
            Some(Token(
                TokenType::Symbol(Symbol::DoublePlus | Symbol::DoubleHyphen),
                _
            ))
        ) {
            let Some(Token(TokenType::Symbol(s), _)) = tokens.next() else {
                unreachable!()
            };

            postfix.push(match s {
                Symbol::DoublePlus => PostfixOp::PostfixIncrement,
                Symbol::DoubleHyphen => PostfixOp::PostfixDecrement,
                _ => unreachable!(),
            });
        }
        tokens.reset_peek();
        Ok(Postfix { primary, postfix })
    }
}

impl ToAst for Primary {
    fn to_ast(tokens: &mut MultiPeek<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        let Token(token_type, _) = expect_token!(
            tokens,
            TokenType::Constant(_)
                | TokenType::Identifier(_)
                | TokenType::Symbol(Symbol::OpenParen),
            "constant, identifier, or '('"
        );

        match token_type {
            TokenType::Constant(i) => Ok(Primary::Constant(i)),
            TokenType::Identifier(identifier) => {
                if matches!(
                    tokens.peek(),
                    Some(Token(TokenType::Symbol(Symbol::OpenParen), _))
                ) {
                    tokens.next(); // Consume the '('
                    let args = if !matches!(
                        tokens.peek(),
                        Some(Token(TokenType::Symbol(Symbol::CloseParen), _)),
                    ) {
                        tokens.reset_peek();
                        ArgumentList::to_ast(tokens)?
                    } else {
                        ArgumentList(Vec::new())
                    };
                    expect_token!(tokens, TokenType::Symbol(Symbol::CloseParen), "')'");
                    Ok(Primary::FunctionCall(identifier, args))
                } else {
                    tokens.reset_peek();
                    Ok(Primary::Var(identifier))
                }
            }
            TokenType::Symbol(Symbol::OpenParen) => {
                let expr = TypedExpression::to_ast(tokens)?;
                expect_token!(tokens, TokenType::Symbol(Symbol::CloseParen), "')'");
                Ok(Primary::Paren(Box::new(expr)))
            }
            _ => unreachable!(),
        }
    }
}

impl ToAst for ArgumentList {
    fn to_ast(tokens: &mut MultiPeek<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        let mut args = Vec::new();
        loop {
            tokens.reset_peek();
            args.push(TypedExpression::to_ast(tokens)?);
            if matches!(
                tokens.peek(),
                Some(Token(TokenType::Symbol(Symbol::Comma), _))
            ) {
                tokens.next(); // Consume the ','
            } else {
                break;
            }
        }
        Ok(ArgumentList(args))
    }
}
