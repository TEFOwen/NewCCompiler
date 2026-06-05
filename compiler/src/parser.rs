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

#[derive(Debug, Clone)]
pub struct Program(pub FuncDef);

#[derive(Debug, Clone)]
pub struct FuncDef {
    pub name: String,
    pub body: Vec<BlockItem>,
}

#[derive(Debug, Clone)]
pub enum BlockItem {
    Statement(Statement),
    Declaration(Declaration),
}

#[derive(Debug, Clone)]
pub struct Declaration {
    pub identifier: String,
    pub initialiser: Option<Expression>,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Return(Expression),
    Expression(Expression),
    Null,
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
    Constant(u32),
    Var(String),
    UnaryOp { op: UnaryOperator, fac: Box<Factor> },
    Paren(Box<Expression>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

        let mut body = Vec::new();
        while !matches!(
            tokens.peek(),
            Some(Token(TokenType::Symbol(Symbol::CloseBrace), _))
        ) {
            let block_item = BlockItem::to_ast(tokens)?;
            body.push(block_item);
        }

        expect_token!(tokens, TokenType::Symbol(Symbol::CloseBrace), "'}'");

        Ok(FuncDef { name, body })
    }
}

impl ToAst for BlockItem {
    fn to_ast(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        if matches!(
            tokens.peek(),
            Some(Token(TokenType::Keyword(Keyword::Int), _))
        ) {
            Ok(BlockItem::Declaration(Declaration::to_ast(tokens)?))
        } else {
            Ok(BlockItem::Statement(Statement::to_ast(tokens)?))
        }
    }
}

impl ToAst for Declaration {
    fn to_ast(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
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

        Ok(Declaration {
            identifier,
            initialiser,
        })
    }
}

impl ToAst for Statement {
    fn to_ast(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        match tokens.peek() {
            Some(Token(TokenType::Keyword(Keyword::Return), _)) => {
                tokens.next(); // Consume the 'return' token
                let expr = Expression::to_ast(tokens)?;
                expect_token!(tokens, TokenType::Symbol(Symbol::Semicolon), "';'");
                Ok(Statement::Return(expr))
            }
            Some(Token(TokenType::Symbol(Symbol::Semicolon), _)) => {
                tokens.next(); // Consume the ';'
                Ok(Statement::Null)
            }
            Some(_) => {
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

            if Self::associativity(op) == Associativity::RightToLeft {
                let right = Self::parse_min_prec(tokens, Self::get_precedence(op).unwrap())?;
                left = Expression::Assignment {
                    left: Box::new(left.clone()),
                    right: Box::new(Self::op_to_assignment(op, left, right)),
                };
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
            | Symbol::DoubleGtEqual => Associativity::RightToLeft,
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
    fn to_ast(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        Self::parse_min_prec(tokens, 0)
    }
}

impl ToAst for Factor {
    fn to_ast(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<Self, ParserError> {
        let Token(token_type, _) = expect_token!(
            tokens,
            TokenType::Constant(_)
                | TokenType::Identifier(_)
                | TokenType::Symbol(
                    Symbol::OpenParen | Symbol::Tilde | Symbol::Hyphen | Symbol::Exclamation
                ),
            "constant, identifier, unary operator, or '('"
        );

        match token_type {
            TokenType::Constant(i) => return Ok(Factor::Constant(i)),
            TokenType::Identifier(identifier) => return Ok(Factor::Var(identifier)),
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
