use crate::lexer;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TypeError {
    #[error("Invalid type tokens: {0:?}")]
    InvalidTypeTokens(Vec<lexer::Token>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Long,
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
}

impl Type {
    pub fn from_tokens(tokens: &[lexer::Token]) -> Result<Self, TypeError> {
        if matches!(
            tokens,
            [lexer::Token(
                lexer::TokenType::Keyword(lexer::Keyword::Int),
                _
            )]
        ) {
            return Ok(Type::Int);
        } else if matches!(
            tokens,
            [lexer::Token(
                lexer::TokenType::Keyword(lexer::Keyword::Long),
                _
            )] | [
                lexer::Token(lexer::TokenType::Keyword(lexer::Keyword::Int), _),
                lexer::Token(lexer::TokenType::Keyword(lexer::Keyword::Long), _),
            ] | [
                lexer::Token(lexer::TokenType::Keyword(lexer::Keyword::Long), _),
                lexer::Token(lexer::TokenType::Keyword(lexer::Keyword::Int), _),
            ]
        ) {
            return Ok(Type::Long);
        } else {
            return Err(TypeError::InvalidTypeTokens(tokens.to_vec()));
        }
    }

    pub fn get_common_type(&self, other: &Type) -> Type {
        if self == other {
            self.clone()
        } else {
            Type::Long
        }
    }

    pub fn get_function_params(&self) -> Option<&Vec<Type>> {
        match self {
            Type::Function { params, .. } => Some(params),
            _ => None,
        }
    }
}

pub trait IsType {
    fn is_type(&self) -> bool;
}

impl IsType for lexer::Keyword {
    fn is_type(&self) -> bool {
        matches!(self, lexer::Keyword::Int | lexer::Keyword::Long)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Constant {
    Int(i32),
    Long(i64),
}

impl Constant {
    pub fn to_integer(&self) -> Option<i64> {
        match self {
            Constant::Int(value) => Some(*value as i64),
            Constant::Long(value) => Some(*value),
        }
    }

    pub fn to_type(&self) -> Type {
        match self {
            Constant::Int(_) => Type::Int,
            Constant::Long(_) => Type::Long,
        }
    }

    pub fn zero_with_type(ty: &Type) -> Option<Constant> {
        match ty {
            Type::Int => Some(Constant::Int(0)),
            Type::Long => Some(Constant::Long(0)),
            _ => None,
        }
    }

    pub fn cast_to_type(&self, ty: Type) -> Option<Constant> {
        self.to_integer().and_then(|value| match ty {
            Type::Int => Some(Constant::Int(value as i32)),
            Type::Long => Some(Constant::Long(value)),
            _ => None,
        })
    }
}
