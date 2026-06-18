use crate::lexer::{Keyword, Symbol, Token, TokenType};
use itertools::{Itertools, MultiPeek};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("Unexpected token: expected {expected}, found {found}")]
    UnexpectedToken { expected: String, found: Token },

    #[error("End of token stream")]
    EndOfTokenStream,
}

#[derive(Debug, Clone)]
pub struct Program(pub Vec<FuncDeclaration>);

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
}

#[derive(Debug, Clone)]
pub struct ParamList(pub Vec<String>);

#[derive(Debug, Clone)]
pub struct Block(pub Vec<BlockItem>);

#[derive(Debug, Clone)]
pub enum BlockItem {
    Statement(Statement),
    Declaration(Declaration),
}

#[derive(Debug, Clone)]
pub struct VariableDeclaration {
    pub identifier: String,
    pub initialiser: Option<Expression>,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Return(Expression),
    Labeled(String, Box<Statement>),
    Goto(String),
    Expression(Expression),
    If {
        condition: Expression,
        then_branch: Box<Statement>,
        else_branch: Option<Box<Statement>>,
    },
    Break(Option<String>),
    Continue(Option<String>),
    Case(Expression, Box<Statement>, Option<String>),
    Default(Box<Statement>, Option<String>),
    While {
        condition: Expression,
        body: Box<Statement>,
        label: Option<String>,
    },
    DoWhile {
        body: Box<Statement>,
        condition: Expression,
        label: Option<String>,
    },
    For {
        init: ForInit,
        condition: Option<Expression>,
        post: Option<Expression>,
        body: Box<Statement>,
        label: Option<String>,
    },
    Switch {
        condition: Expression,
        body: Box<Statement>,
        label: Option<String>,
        cases: Vec<u32>,
        default_exists: bool,
    },
    Block(Block),
    Null,
}

#[derive(Debug, Clone)]
pub enum ForInit {
    Declaration(VariableDeclaration),
    Expression(Option<Expression>),
}

#[derive(Debug, Clone)]
pub enum Expression {
    Factor(Factor),
    BinaryOp {
        op: BinaryOperator,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Assignment {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Conditional {
        condition: Box<Expression>,
        then_branch: Box<Expression>,
        else_branch: Box<Expression>,
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
    Constant(u32),
    Var(String),
    Paren(Box<Expression>),
    FunctionCall(String, ArgumentList),
}

#[derive(Debug, Clone)]
pub struct ArgumentList(pub Vec<Expression>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Complement,
    Negate,
    LogicalNot,
    PrefixIncrement,
    PrefixDecrement,
}

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
    ($tokens:expr, $expected:pat, $expected_str:literal) => {
        match $tokens.next() {
            Some(token) if matches!(token.0, $expected) => token,
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
        let mut funcs = Vec::new();
        while !matches!(tokens.peek(), Some(Token(TokenType::EndOfFile, _))) {
            tokens.reset_peek();
            funcs.push(FuncDeclaration::to_ast(tokens)?);
        }
        expect_token!(tokens, TokenType::EndOfFile, "end of file");

        Ok(Program(funcs))
    }
}

impl ToAst for FuncDeclaration {
    fn to_ast(tokens: &mut MultiPeek<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        expect_token!(tokens, TokenType::Keyword(Keyword::Int), "int");
        let Token(TokenType::Identifier(identifier), _) =
            expect_token!(tokens, TokenType::Identifier(_), "function name")
        else {
            unreachable!()
        };

        expect_token!(tokens, TokenType::Symbol(Symbol::OpenParen), "'('");

        let parameters = ParamList::to_ast(tokens)?;

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

        Ok(FuncDeclaration {
            identifier,
            parameters,
            body,
        })
    }
}

impl ToAst for ParamList {
    fn to_ast(tokens: &mut MultiPeek<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        let Token(TokenType::Keyword(keyword), _) = expect_token!(
            tokens,
            TokenType::Keyword(Keyword::Void | Keyword::Int),
            "type specifier"
        ) else {
            unreachable!()
        };
        let mut params = Vec::new();
        if keyword == Keyword::Void {
            Ok(ParamList(params))
        } else {
            loop {
                let Token(TokenType::Identifier(identifier), _) =
                    expect_token!(tokens, TokenType::Identifier(_), "parameter name")
                else {
                    unreachable!()
                };
                params.push(identifier);
                if matches!(
                    tokens.peek(),
                    Some(Token(TokenType::Symbol(Symbol::Comma), _))
                ) {
                    tokens.next();
                    expect_token!(
                        tokens,
                        TokenType::Keyword(Keyword::Int),
                        "type specifier for parameter"
                    );
                } else {
                    break;
                }
            }
            Ok(ParamList(params))
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
            Some(Token(TokenType::Keyword(Keyword::Int), _))
        ) {
            tokens.reset_peek();
            Ok(BlockItem::Declaration(Declaration::to_ast(tokens)?))
        } else {
            tokens.reset_peek();
            Ok(BlockItem::Statement(Statement::to_ast(tokens)?))
        }
    }
}

impl ToAst for Declaration {
    fn to_ast(tokens: &mut MultiPeek<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        tokens.peek(); // int
        tokens.peek(); // identifier
        if matches!(
            tokens.peek(),
            Some(Token(TokenType::Symbol(Symbol::OpenParen), _))
        ) {
            tokens.reset_peek();
            Ok(Declaration::Function(FuncDeclaration::to_ast(tokens)?))
        } else {
            tokens.reset_peek();
            Ok(Declaration::Variable(VariableDeclaration::to_ast(tokens)?))
        }
    }
}

impl ToAst for VariableDeclaration {
    fn to_ast(tokens: &mut MultiPeek<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        expect_token!(tokens, TokenType::Keyword(Keyword::Int), "int");
        let Token(TokenType::Identifier(identifier), _) =
            expect_token!(tokens, TokenType::Identifier(_), "variable name")
        else {
            unreachable!()
        };

        let initialiser = if matches!(
            tokens.peek(),
            Some(Token(TokenType::Symbol(Symbol::Equal), _))
        ) {
            tokens.next();
            Some(Expression::to_ast(tokens)?)
        } else {
            None
        };

        expect_token!(tokens, TokenType::Symbol(Symbol::Semicolon), "';'");

        Ok(VariableDeclaration {
            identifier,
            initialiser,
        })
    }
}

impl ToAst for Statement {
    fn to_ast(tokens: &mut MultiPeek<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        match tokens.peek() {
            Some(Token(TokenType::Keyword(Keyword::Return), _)) => {
                tokens.next(); // Consume the 'return' token
                let expr = Expression::to_ast(tokens)?;
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
                    let expr = Expression::to_ast(tokens)?;
                    expect_token!(tokens, TokenType::Symbol(Symbol::Semicolon), "';'");
                    Ok(Statement::Expression(expr))
                }
            }
            Some(Token(TokenType::Keyword(Keyword::Case), _)) => {
                tokens.next(); // Consume the 'case' token
                let value = Expression::to_ast(tokens)?;
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
                let condition = Expression::to_ast(tokens)?;
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
                let condition = Expression::to_ast(tokens)?;
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
                let condition = Expression::to_ast(tokens)?;
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
                    Some(Token(TokenType::Keyword(Keyword::Int), _)) => {
                        tokens.reset_peek();
                        ForInit::Declaration(VariableDeclaration::to_ast(tokens)?)
                    }
                    Some(Token(TokenType::Symbol(Symbol::Semicolon), _)) => {
                        tokens.next(); // Consume the ';'
                        ForInit::Expression(None)
                    }
                    _ => {
                        tokens.reset_peek();
                        let exp = ForInit::Expression(Some(Expression::to_ast(tokens)?));
                        expect_token!(tokens, TokenType::Symbol(Symbol::Semicolon), "';'");
                        exp
                    }
                };
                let condition = if !matches!(
                    tokens.peek(),
                    Some(Token(TokenType::Symbol(Symbol::Semicolon), _))
                ) {
                    tokens.reset_peek();
                    Some(Expression::to_ast(tokens)?)
                } else {
                    None
                };
                expect_token!(tokens, TokenType::Symbol(Symbol::Semicolon), "';'");
                let post = if !matches!(
                    tokens.peek(),
                    Some(Token(TokenType::Symbol(Symbol::CloseParen), _))
                ) {
                    tokens.reset_peek();
                    Some(Expression::to_ast(tokens)?)
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
                let condition = Expression::to_ast(tokens)?;
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
                let expr = Expression::to_ast(tokens)?;
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
                        condition: Box::new(left),
                        then_branch: Box::new(middle),
                        else_branch: Box::new(right),
                    };
                } else {
                    let right = Self::parse_min_prec(tokens, Self::get_precedence(op).unwrap())?;
                    left = Expression::Assignment {
                        left: Box::new(left.clone()),
                        right: Box::new(Self::op_to_assignment(op, left, right)),
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
                    left: Box::new(left),
                    right: Box::new(right),
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
                left: Box::new(left),
                right: Box::new(right),
            },
            Symbol::MinusEqual => Expression::BinaryOp {
                op: BinaryOperator::Subtract,
                left: Box::new(left),
                right: Box::new(right),
            },
            Symbol::AsteriskEqual => Expression::BinaryOp {
                op: BinaryOperator::Multiply,
                left: Box::new(left),
                right: Box::new(right),
            },
            Symbol::SlashEqual => Expression::BinaryOp {
                op: BinaryOperator::Divide,
                left: Box::new(left),
                right: Box::new(right),
            },
            Symbol::PercentEqual => Expression::BinaryOp {
                op: BinaryOperator::Remainder,
                left: Box::new(left),
                right: Box::new(right),
            },
            Symbol::DoubleLtEqual => Expression::BinaryOp {
                op: BinaryOperator::LeftShift,
                left: Box::new(left),
                right: Box::new(right),
            },
            Symbol::DoubleGtEqual => Expression::BinaryOp {
                op: BinaryOperator::RightShift,
                left: Box::new(left),
                right: Box::new(right),
            },
            Symbol::AmpersandEqual => Expression::BinaryOp {
                op: BinaryOperator::BitwiseAnd,
                left: Box::new(left),
                right: Box::new(right),
            },
            Symbol::BarEqual => Expression::BinaryOp {
                op: BinaryOperator::BitwiseOr,
                left: Box::new(left),
                right: Box::new(right),
            },
            Symbol::HatEqual => Expression::BinaryOp {
                op: BinaryOperator::BitwiseXor,
                left: Box::new(left),
                right: Box::new(right),
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
        if matches!(
            tokens.peek(),
            Some(Token(
                TokenType::Symbol(
                    Symbol::Tilde
                        | Symbol::Hyphen
                        | Symbol::Exclamation
                        | Symbol::DoublePlus
                        | Symbol::DoubleHyphen
                ),
                _
            ))
        ) {
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
        } else {
            tokens.reset_peek();
            Postfix::to_ast(tokens).map(Factor::Postfix)
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
                let expr = Expression::to_ast(tokens)?;
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
            args.push(Expression::to_ast(tokens)?);
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
