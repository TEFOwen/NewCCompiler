use std::sync::atomic::{AtomicUsize, Ordering};

use crate::parser;

static UNARY_OP_VAR_COUNTER: AtomicUsize = AtomicUsize::new(0);
static BINARY_OP_VAR_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
pub struct Program(pub FuncDef);

#[derive(Debug)]
pub struct FuncDef {
    pub name: String,
    pub body: Vec<Instruction>,
}

#[derive(Debug)]
pub enum Instruction {
    Return(Value),
    UnaryOp {
        op: parser::UnaryOperator,
        src: Value,
        dst: Value,
    },
    BinaryOp {
        op: parser::BinaryOperator,
        val1: Value,
        val2: Value,
        dst: Value,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Constant(u32),
    Var(String),
}

pub trait ToTacky {
    type Output;

    fn to_tacky(self) -> Self::Output;
}

impl ToTacky for parser::Program {
    type Output = Program;

    fn to_tacky(self) -> Self::Output {
        Program(self.0.to_tacky())
    }
}

impl ToTacky for parser::FuncDef {
    type Output = FuncDef;

    fn to_tacky(self) -> Self::Output {
        FuncDef {
            name: self.name,
            body: self.body.to_tacky(),
        }
    }
}

impl ToTacky for parser::Statement {
    type Output = Vec<Instruction>;

    fn to_tacky(self) -> Self::Output {
        match self {
            parser::Statement::Return(exp) => {
                let (mut instructions, var) = exp.to_tacky();
                instructions.push(Instruction::Return(var));
                instructions
            }
        }
    }
}

impl ToTacky for parser::Expression {
    type Output = (Vec<Instruction>, Value);

    fn to_tacky(self) -> Self::Output {
        match self {
            parser::Expression::Factor(factor) => factor.to_tacky(),
            parser::Expression::BinaryOp { op, left, right } => {
                let (instructions1, var1) = left.to_tacky();
                let (instructions2, var2) = right.to_tacky();
                let dst = Value::Var(format!(
                    "binary_op.{}",
                    BINARY_OP_VAR_COUNTER.fetch_add(1, Ordering::SeqCst)
                ));
                let instruction = Instruction::BinaryOp {
                    op,
                    val1: var1,
                    val2: var2,
                    dst: dst.clone(),
                };
                (
                    instructions1
                        .into_iter()
                        .chain(instructions2.into_iter())
                        .chain(std::iter::once(instruction))
                        .collect(),
                    dst,
                )
            }
        }
    }
}

impl ToTacky for parser::Factor {
    type Output = (Vec<Instruction>, Value);

    fn to_tacky(self) -> Self::Output {
        match self {
            parser::Factor::Constant(val) => (vec![], Value::Constant(val)),
            parser::Factor::UnaryOp { op, fac } => {
                let (mut instructions, src) = fac.to_tacky();
                let dst = Value::Var(format!(
                    "unary_op.{}",
                    UNARY_OP_VAR_COUNTER.fetch_add(1, Ordering::SeqCst)
                ));
                instructions.push(Instruction::UnaryOp {
                    op,
                    src,
                    dst: dst.clone(),
                });
                (instructions, dst)
            }
            parser::Factor::Paren(expression) => expression.to_tacky(),
        }
    }
}
