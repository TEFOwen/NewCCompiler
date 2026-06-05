use std::sync::atomic::{AtomicUsize, Ordering};

use crate::parser;

static UNARY_OP_VAR_COUNTER: AtomicUsize = AtomicUsize::new(0);
static BINARY_OP_VAR_COUNTER: AtomicUsize = AtomicUsize::new(0);
static JUMP_LABEL_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn next_unary_op_var() -> Value {
    Value::Var(format!(
        "unary_op.{}",
        UNARY_OP_VAR_COUNTER.fetch_add(1, Ordering::SeqCst)
    ))
}

fn next_binary_op_var() -> Value {
    Value::Var(format!(
        "binary_op.{}",
        BINARY_OP_VAR_COUNTER.fetch_add(1, Ordering::SeqCst)
    ))
}

fn next_jump_label(name: &str) -> String {
    format!(
        "label_{}.{}",
        name,
        JUMP_LABEL_COUNTER.fetch_add(1, Ordering::SeqCst)
    )
}

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
        op: BinaryOperator,
        val1: Value,
        val2: Value,
        dst: Value,
    },
    Copy {
        src: Value,
        dst: Value,
    },
    Jump(String),
    JumpIfZero {
        val: Value,
        target: String,
    },
    JumpIfNotZero {
        val: Value,
        target: String,
    },
    Label(String),
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

    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessEqual,
    GreaterEqual,
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

impl TryFrom<parser::BinaryOperator> for BinaryOperator {
    type Error = ();

    fn try_from(op: parser::BinaryOperator) -> Result<BinaryOperator, Self::Error> {
        match op {
            parser::BinaryOperator::Add => Ok(BinaryOperator::Add),
            parser::BinaryOperator::Subtract => Ok(BinaryOperator::Subtract),
            parser::BinaryOperator::Multiply => Ok(BinaryOperator::Multiply),
            parser::BinaryOperator::Divide => Ok(BinaryOperator::Divide),
            parser::BinaryOperator::Remainder => Ok(BinaryOperator::Remainder),
            parser::BinaryOperator::BitwiseAnd => Ok(BinaryOperator::BitwiseAnd),
            parser::BinaryOperator::BitwiseOr => Ok(BinaryOperator::BitwiseOr),
            parser::BinaryOperator::BitwiseXor => Ok(BinaryOperator::BitwiseXor),
            parser::BinaryOperator::LeftShift => Ok(BinaryOperator::LeftShift),
            parser::BinaryOperator::RightShift => Ok(BinaryOperator::RightShift),
            parser::BinaryOperator::Equal => Ok(BinaryOperator::Equal),
            parser::BinaryOperator::NotEqual => Ok(BinaryOperator::NotEqual),
            parser::BinaryOperator::LessThan => Ok(BinaryOperator::LessThan),
            parser::BinaryOperator::GreaterThan => Ok(BinaryOperator::GreaterThan),
            parser::BinaryOperator::LessEqual => Ok(BinaryOperator::LessEqual),
            parser::BinaryOperator::GreaterEqual => Ok(BinaryOperator::GreaterEqual),
            _ => Err(()),
        }
    }
}

impl ToTacky for parser::Expression {
    type Output = (Vec<Instruction>, Value);

    fn to_tacky(self) -> Self::Output {
        match self {
            parser::Expression::Factor(factor) => factor.to_tacky(),
            parser::Expression::BinaryOp { op, left, right }
                if BinaryOperator::try_from(op).is_ok() =>
            {
                let (instructions1, var1) = left.to_tacky();
                let (instructions2, var2) = right.to_tacky();
                let dst = next_binary_op_var();
                let instruction = Instruction::BinaryOp {
                    op: BinaryOperator::try_from(op).unwrap(),
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
            parser::Expression::BinaryOp {
                op: parser::BinaryOperator::LogicalAnd,
                left,
                right,
            } => {
                let end_label = next_jump_label("binop_end");
                let false_label = next_jump_label("binop_false");
                let out_val = next_binary_op_var();

                // Get val1, if zero jump to false
                let (mut instructions, val1) = left.to_tacky();
                instructions.push(Instruction::JumpIfZero {
                    val: val1,
                    target: false_label.clone(),
                });

                // Get val2, if zero jump to false
                let (instructions2, val2) = right.to_tacky();
                instructions.extend(instructions2);
                instructions.push(Instruction::JumpIfZero {
                    val: val2,
                    target: false_label.clone(),
                });

                // Mark output as 1 (true) and jump to end
                instructions.push(Instruction::Copy {
                    src: Value::Constant(1),
                    dst: out_val.clone(),
                });
                instructions.push(Instruction::Jump(end_label.clone()));

                // False label, mark output as 0 (false)
                instructions.push(Instruction::Label(false_label));
                instructions.push(Instruction::Copy {
                    src: Value::Constant(0),
                    dst: out_val.clone(),
                });

                instructions.push(Instruction::Label(end_label));

                (instructions, out_val)
            }
            parser::Expression::BinaryOp {
                op: parser::BinaryOperator::LogicalOr,
                left,
                right,
            } => {
                let end_label = next_jump_label("binop_end");
                let true_label = next_jump_label("binop_true");
                let out_val = next_binary_op_var();

                // Get val1, if not zero jump to true
                let (mut instructions, val1) = left.to_tacky();
                instructions.push(Instruction::JumpIfNotZero {
                    val: val1,
                    target: true_label.clone(),
                });

                // Get val2, if not zero jump to true
                let (instructions2, val2) = right.to_tacky();
                instructions.extend(instructions2);
                instructions.push(Instruction::JumpIfNotZero {
                    val: val2,
                    target: true_label.clone(),
                });

                // Mark output as 0 (false) and jump to end
                instructions.push(Instruction::Copy {
                    src: Value::Constant(0),
                    dst: out_val.clone(),
                });
                instructions.push(Instruction::Jump(end_label.clone()));

                // False label, mark output as 1 (true)
                instructions.push(Instruction::Label(true_label));
                instructions.push(Instruction::Copy {
                    src: Value::Constant(1),
                    dst: out_val.clone(),
                });

                instructions.push(Instruction::Label(end_label));

                (instructions, out_val)
            }
            _ => unreachable!(),
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
                let dst = next_unary_op_var();
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
