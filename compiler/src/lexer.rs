use std::{fmt::Display, io::BufRead};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LexerError {
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("Unknown token at line {line}, column {column}: {character}")]
    UnknownToken {
        line: usize,
        column: usize,
        character: char,
    },

    #[error("Failed to parse integer at {1}: {0}")]
    ParseIntError(std::num::ParseIntError, Location),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType {
    Identifier(String),
    Constant(u32),
    Keyword(Keyword),
    Symbol(Symbol),
    EndOfFile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Keyword {
    Int,
    Void,
    Return,
}

impl TryFrom<&String> for Keyword {
    type Error = ();

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "int" => Ok(Keyword::Int),
            "void" => Ok(Keyword::Void),
            "return" => Ok(Keyword::Return),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Symbol {
    OpenParen,  // (
    CloseParen, // )
    OpenBrace,  // {
    CloseBrace, // }
    Semicolon,  // ;

    Exclamation,  // !
    Tilde,        // ~
    Hyphen,       // -
    DoubleHyphen, // --
    Plus,         // +
    Asterisk,     // *
    Slash,        // /
    Percent,      // %
    Ampersand,    // &
    Bar,          // |
    Hat,          // ^
    DoubleLt,     // <<
    DoubleGt,     // >>

    DoubleAmp,    // &&
    DoubleBar,    // ||
    Equal,        // =
    DoubleEqual,  // ==
    NotEqual,     // !=
    LessThan,     // <
    GreaterThan,  // >
    LessEqual,    // <=
    GreaterEqual, // >=

    PlusEqual,      // +=
    MinusEqual,     // -=
    AsteriskEqual,  // *=
    SlashEqual,     // /=
    PercentEqual,   // %=
    DoubleLtEqual,  // <<=
    DoubleGtEqual,  // >>=
    AmpersandEqual, // &=
    BarEqual,       // |=
    HatEqual,       // ^=
}

#[derive(Debug, Clone)]
/// Location of a token in the source code, used for error reporting
/// line and column numbers are 1-based
/// (start, end] column range of the token in the line
pub struct Location {
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}, columns {}-{}", self.line, self.start, self.end)
    }
}

#[derive(Debug, Clone)]
pub struct Token(pub TokenType, pub Location);

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} at {}", self.0, self.1)
    }
}

pub fn lex<R>(input: R) -> Result<Vec<Token>, LexerError>
where
    R: BufRead,
{
    let is_identifier_start = |c: char| c.is_ascii_alphabetic() || c == '_';
    let is_identifier_part = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let is_digit = |c: char| c.is_ascii_digit();

    let mut tokens = vec![];

    let mut line_number = 1;
    for line in input.lines() {
        let mut column_number = 1;
        let line = line?;
        let mut chars = line.chars().peekable();

        while let Some(c) = chars.next() {
            if c.is_whitespace() {
                column_number += 1;
                continue;
            }

            if is_identifier_start(c) {
                let start = column_number;
                column_number += 1;
                let mut identifier = c.to_string();

                while matches!(chars.peek(), Some(&next) if is_identifier_part(next)) {
                    identifier.push(chars.next().unwrap());
                    column_number += 1;
                }

                if let Ok(keyword) = Keyword::try_from(&identifier) {
                    tokens.push(Token(
                        TokenType::Keyword(keyword),
                        Location {
                            line: line_number,
                            start,
                            end: column_number,
                        },
                    ));
                } else {
                    tokens.push(Token(
                        TokenType::Identifier(identifier),
                        Location {
                            line: line_number,
                            start,
                            end: column_number,
                        },
                    ));
                }
            } else if is_digit(c) {
                let start = column_number;
                column_number += 1;
                let mut number = c.to_string();

                while matches!(chars.peek(), Some(&next) if is_identifier_part(next)) {
                    number.push(chars.next().unwrap());
                    column_number += 1;
                }

                match number.parse::<u32>() {
                    Ok(value) => tokens.push(Token(
                        TokenType::Constant(value),
                        Location {
                            line: line_number,
                            start,
                            end: column_number,
                        },
                    )),
                    Err(e) => {
                        return Err(LexerError::ParseIntError(
                            e,
                            Location {
                                line: line_number,
                                start,
                                end: column_number,
                            },
                        ));
                    }
                }
            } else {
                let symbol = match c {
                    '(' => Symbol::OpenParen,
                    ')' => Symbol::CloseParen,
                    '{' => Symbol::OpenBrace,
                    '}' => Symbol::CloseBrace,
                    ';' => Symbol::Semicolon,
                    '~' => Symbol::Tilde,
                    '-' => {
                        if matches!(chars.peek(), Some(&'-')) {
                            chars.next();
                            column_number += 1;
                            Symbol::DoubleHyphen
                        } else if matches!(chars.peek(), Some(&'=')) {
                            chars.next();
                            column_number += 1;
                            Symbol::MinusEqual
                        } else {
                            Symbol::Hyphen
                        }
                    }
                    '+' => {
                        if matches!(chars.peek(), Some(&'=')) {
                            chars.next();
                            column_number += 1;
                            Symbol::PlusEqual
                        } else {
                            Symbol::Plus
                        }
                    }
                    '*' => {
                        if matches!(chars.peek(), Some(&'=')) {
                            chars.next();
                            column_number += 1;
                            Symbol::AsteriskEqual
                        } else {
                            Symbol::Asterisk
                        }
                    }
                    '/' => {
                        if matches!(chars.peek(), Some(&'=')) {
                            chars.next();
                            column_number += 1;
                            Symbol::SlashEqual
                        } else {
                            Symbol::Slash
                        }
                    }
                    '%' => {
                        if matches!(chars.peek(), Some(&'=')) {
                            chars.next();
                            column_number += 1;
                            Symbol::PercentEqual
                        } else {
                            Symbol::Percent
                        }
                    }
                    '=' => {
                        if matches!(chars.peek(), Some(&'=')) {
                            chars.next();
                            column_number += 1;
                            Symbol::DoubleEqual
                        } else {
                            Symbol::Equal
                        }
                    }
                    '!' => {
                        if matches!(chars.peek(), Some(&'=')) {
                            chars.next();
                            column_number += 1;
                            Symbol::NotEqual
                        } else {
                            Symbol::Exclamation
                        }
                    }
                    '&' => {
                        if matches!(chars.peek(), Some(&'&')) {
                            chars.next();
                            column_number += 1;
                            Symbol::DoubleAmp
                        } else if matches!(chars.peek(), Some(&'=')) {
                            chars.next();
                            column_number += 1;
                            Symbol::AmpersandEqual
                        } else {
                            Symbol::Ampersand
                        }
                    }
                    '|' => {
                        if matches!(chars.peek(), Some(&'|')) {
                            chars.next();
                            column_number += 1;
                            Symbol::DoubleBar
                        } else if matches!(chars.peek(), Some(&'=')) {
                            chars.next();
                            column_number += 1;
                            Symbol::BarEqual
                        } else {
                            Symbol::Bar
                        }
                    }
                    '^' => {
                        if matches!(chars.peek(), Some(&'=')) {
                            chars.next();
                            column_number += 1;
                            Symbol::HatEqual
                        } else {
                            Symbol::Hat
                        }
                    }
                    '<' => {
                        if matches!(chars.peek(), Some(&'<')) {
                            chars.next();
                            column_number += 1;
                            if matches!(chars.peek(), Some(&'=')) {
                                chars.next();
                                column_number += 1;
                                Symbol::DoubleLtEqual
                            } else {
                                Symbol::DoubleLt
                            }
                        } else if matches!(chars.peek(), Some(&'=')) {
                            chars.next();
                            column_number += 1;
                            Symbol::LessEqual
                        } else {
                            Symbol::LessThan
                        }
                    }
                    '>' => {
                        if matches!(chars.peek(), Some(&'>')) {
                            chars.next();
                            column_number += 1;
                            if matches!(chars.peek(), Some(&'=')) {
                                chars.next();
                                column_number += 1;
                                Symbol::DoubleGtEqual
                            } else {
                                Symbol::DoubleGt
                            }
                        } else if matches!(chars.peek(), Some(&'=')) {
                            chars.next();
                            column_number += 1;
                            Symbol::GreaterEqual
                        } else {
                            Symbol::GreaterThan
                        }
                    }
                    _ => {
                        return Err(LexerError::UnknownToken {
                            line: line_number,
                            column: column_number,
                            character: c,
                        });
                    }
                };

                tokens.push(Token(
                    TokenType::Symbol(symbol),
                    Location {
                        line: line_number,
                        start: column_number,
                        end: column_number + 1,
                    },
                ));
                column_number += 1;
            }
        }

        line_number += 1;
    }

    tokens.push(Token(
        TokenType::EndOfFile,
        Location {
            line: line_number,
            start: 0,
            end: 0,
        },
    ));

    Ok(tokens)
}
