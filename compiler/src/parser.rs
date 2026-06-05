use std::iter::Peekable;

use crate::lexer::{Keyword, Symbol, Token, TokenType};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("Unexpected token: expected {expected}, found {found}")]
    UnexpectedToken { expected: String, found: Token },

    #[error("End of token stream")]
    EndOfTokenStream,
}

#[derive(Debug)]
pub struct Program(pub FuncDef);

#[derive(Debug)]
pub struct FuncDef {
    pub name: String,
    pub body: Statement,
}

#[derive(Debug)]
pub enum Statement {
    Return(Expression),
}

#[derive(Debug)]
pub enum Expression {
    Factor(Factor),
    BinaryOp {
        op: BinaryOperator,
        left: Box<Expression>,
        right: Box<Expression>,
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

#[derive(Debug)]
pub enum Factor {
    Constant(u32),
    UnaryOp { op: UnaryOperator, fac: Box<Factor> },
    Paren(Box<Expression>),
}

#[derive(Debug)]
pub enum UnaryOperator {
    Complement,
    Negate,
    LogicalNot,
}

trait ToAst {
    fn to_ast(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<Self, ParserError>
    where
        Self: Sized;
}

pub fn parse(tokens: Vec<Token>) -> Result<Program, ParserError> {
    let mut tokens = tokens.into_iter().peekable();
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
    fn to_ast(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        let funcdef = FuncDef::to_ast(tokens)?;
        if let Some(token) = tokens.next() {
            if token.0 != TokenType::EndOfFile {
                return Err(ParserError::UnexpectedToken {
                    expected: "EOF".into(),
                    found: token,
                });
            }
        }

        Ok(Program(funcdef))
    }
}

impl ToAst for FuncDef {
    fn to_ast(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        expect_token!(tokens, TokenType::Keyword(Keyword::Int), "int");
        let Token(TokenType::Identifier(name), _) =
            expect_token!(tokens, TokenType::Identifier(_), "function name")
        else {
            unreachable!()
        };

        expect_token!(tokens, TokenType::Symbol(Symbol::OpenParen), "'('");
        expect_token!(tokens, TokenType::Keyword(Keyword::Void), "void");
        expect_token!(tokens, TokenType::Symbol(Symbol::CloseParen), "')'");
        expect_token!(tokens, TokenType::Symbol(Symbol::OpenBrace), "'{'");

        let body = Statement::to_ast(tokens)?;

        expect_token!(tokens, TokenType::Symbol(Symbol::CloseBrace), "'}'");

        Ok(FuncDef { name, body })
    }
}

impl ToAst for Statement {
    fn to_ast(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        expect_token!(tokens, TokenType::Keyword(Keyword::Return), "return");
        let expr = Expression::to_ast(tokens)?;
        expect_token!(tokens, TokenType::Symbol(Symbol::Semicolon), "';'");
        Ok(Statement::Return(expr))
    }
}

impl Expression {
    fn parse_min_prec(
        tokens: &mut Peekable<impl Iterator<Item = Token>>,
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
            _ => None,
        }
    }
}

impl ToAst for Expression {
    fn to_ast(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        Self::parse_min_prec(tokens, 0)
    }
}

impl ToAst for Factor {
    fn to_ast(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        let Token(token_type, _) = expect_token!(
            tokens,
            TokenType::Constant(_)
                | TokenType::Symbol(
                    Symbol::OpenParen | Symbol::Tilde | Symbol::Hyphen | Symbol::Exclamation
                ),
            "constant, unary operator, or '('"
        );

        match token_type {
            TokenType::Constant(i) => return Ok(Factor::Constant(i)),
            TokenType::Symbol(Symbol::OpenParen) => {
                let expr = Expression::to_ast(tokens)?;
                expect_token!(tokens, TokenType::Symbol(Symbol::CloseParen), "')'");
                return Ok(Factor::Paren(Box::new(expr)));
            }
            TokenType::Symbol(s) => {
                let inner = Factor::to_ast(tokens)?;
                return Ok(Factor::UnaryOp {
                    op: match s {
                        Symbol::Tilde => UnaryOperator::Complement,
                        Symbol::Hyphen => UnaryOperator::Negate,
                        Symbol::Exclamation => UnaryOperator::LogicalNot,
                        _ => unreachable!(),
                    },
                    fac: Box::new(inner),
                });
            }
            _ => unreachable!(),
        }
    }
}
