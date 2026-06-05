use std::collections::HashMap;

use crate::{parser, tacky};

#[derive(Debug)]
pub struct Program(pub FuncDef);

#[derive(Debug)]
pub struct FuncDef {
    pub name: String,
    pub body: Vec<Instruction>,
    pub stack_size: Option<usize>,
}

#[derive(Debug)]
pub enum Instruction {
    Move {
        src: Operand,
        dst: Operand,
    },
    UnaryOp {
        operator: parser::UnaryOperator,
        operand: Operand,
    },
    BinaryOp {
        op: BinaryOperator,
        src: Operand,
        dst: Operand,
    },
    Shift {
        left: bool,
        val: Operand,
    },
    Idiv(Operand),
    Cdq,
    AllocateStack {
        size: usize,
    },
    Return,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mult,
    BitAnd,
    BitOr,
    BitXor,
}

#[derive(Debug)]
pub enum Operand {
    Immediate(u32),
    Register(Register),
    Pseudo(String),
    Stack(usize),
}

#[derive(Debug)]
pub enum Register {
    AX,
    DX,
    CX,
    R10,
    R11,
}

trait ToAssembly {
    type Output;

    fn to_assembly(self) -> Self::Output;
}

impl ToAssembly for tacky::Program {
    type Output = Program;

    fn to_assembly(self) -> Self::Output {
        Program(self.0.to_assembly())
    }
}

impl ToAssembly for tacky::FuncDef {
    type Output = FuncDef;

    fn to_assembly(self) -> Self::Output {
        FuncDef {
            name: self.name,
            body: self
                .body
                .into_iter()
                .flat_map(|instruction| instruction.to_assembly())
                .collect(),
            stack_size: None,
        }
    }
}

impl ToAssembly for tacky::Instruction {
    type Output = Vec<Instruction>;

    fn to_assembly(self) -> Self::Output {
        match self {
            tacky::Instruction::Return(value) => vec![
                Instruction::Move {
                    src: value.to_assembly(),
                    dst: Operand::Register(Register::AX),
                },
                Instruction::Return,
            ],
            tacky::Instruction::UnaryOp { op, src, dst } => vec![
                Instruction::Move {
                    src: src.to_assembly(),
                    dst: dst.clone().to_assembly(),
                },
                Instruction::UnaryOp {
                    operator: op,
                    operand: dst.to_assembly(),
                },
            ],
            tacky::Instruction::BinaryOp {
                op,
                val1,
                val2,
                dst,
            } => match op {
                parser::BinaryOperator::Add
                | parser::BinaryOperator::Subtract
                | parser::BinaryOperator::Multiply
                | parser::BinaryOperator::BitwiseAnd
                | parser::BinaryOperator::BitwiseOr
                | parser::BinaryOperator::BitwiseXor => vec![
                    Instruction::Move {
                        src: val1.to_assembly(),
                        dst: dst.clone().to_assembly(),
                    },
                    Instruction::BinaryOp {
                        op: op.to_assembly(),
                        src: val2.to_assembly(),
                        dst: dst.to_assembly(),
                    },
                ],
                parser::BinaryOperator::LeftShift | parser::BinaryOperator::RightShift => {
                    vec![
                        Instruction::Move {
                            src: val1.to_assembly(),
                            dst: dst.clone().to_assembly(),
                        },
                        Instruction::Move {
                            src: val2.to_assembly(),
                            dst: Operand::Register(Register::CX),
                        },
                        Instruction::Shift {
                            left: matches!(op, parser::BinaryOperator::LeftShift),
                            val: dst.to_assembly(),
                        },
                    ]
                }
                parser::BinaryOperator::Divide | parser::BinaryOperator::Remainder => {
                    vec![
                        Instruction::Move {
                            src: val1.to_assembly(),
                            dst: Operand::Register(Register::AX),
                        },
                        Instruction::Cdq,
                        Instruction::Idiv(val2.to_assembly()),
                        Instruction::Move {
                            src: if op == parser::BinaryOperator::Divide {
                                Operand::Register(Register::AX)
                            } else {
                                Operand::Register(Register::DX)
                            },
                            dst: dst.to_assembly(),
                        },
                    ]
                }
            },
        }
    }
}

impl ToAssembly for parser::BinaryOperator {
    type Output = BinaryOperator;

    fn to_assembly(self) -> Self::Output {
        match self {
            parser::BinaryOperator::Add => BinaryOperator::Add,
            parser::BinaryOperator::Subtract => BinaryOperator::Sub,
            parser::BinaryOperator::Multiply => BinaryOperator::Mult,
            parser::BinaryOperator::BitwiseAnd => BinaryOperator::BitAnd,
            parser::BinaryOperator::BitwiseOr => BinaryOperator::BitOr,
            parser::BinaryOperator::BitwiseXor => BinaryOperator::BitXor,
            parser::BinaryOperator::Divide
            | parser::BinaryOperator::Remainder
            | parser::BinaryOperator::LeftShift
            | parser::BinaryOperator::RightShift => unreachable!(),
        }
    }
}

impl ToAssembly for tacky::Value {
    type Output = Operand;

    fn to_assembly(self) -> Self::Output {
        match self {
            tacky::Value::Constant(i) => Operand::Immediate(i),
            tacky::Value::Var(name) => Operand::Pseudo(name),
        }
    }
}

impl Program {
    /// Replace pseudo operands with stack offsets and calculate stack size
    fn update_pseudo_operands(&mut self) {
        let mut var_map = HashMap::new();
        let mut stack_size = 0usize;

        let mut update_operand = |operand: &mut Operand| {
            if let Operand::Pseudo(name) = operand {
                let stack_offset = *var_map.entry(name.clone()).or_insert_with(|| {
                    stack_size += 4;
                    stack_size
                });
                *operand = Operand::Stack(stack_offset);
            }
        };

        for instruction in &mut self.0.body {
            match instruction {
                Instruction::Move { src, dst } => {
                    update_operand(dst);
                    update_operand(src);
                }
                Instruction::UnaryOp { operand, .. } => update_operand(operand),
                Instruction::BinaryOp { src: val2, dst, .. } => {
                    update_operand(val2);
                    update_operand(dst);
                }
                Instruction::Shift { val, .. } => {
                    update_operand(val);
                }
                Instruction::Idiv(operand) => update_operand(operand),
                Instruction::Cdq | Instruction::Return | Instruction::AllocateStack { .. } => {}
            }
        }

        self.0.stack_size = Some(stack_size);
    }

    fn final_pass(&mut self) {
        let instructions = std::mem::take(&mut self.0.body);

        self.0.body.push(Instruction::AllocateStack {
            size: self.0.stack_size.expect("No stack size found for function"),
        });

        for instruction in instructions {
            match instruction {
                Instruction::Move {
                    src: Operand::Stack(src),
                    dst: Operand::Stack(dst),
                } => {
                    self.0.body.push(Instruction::Move {
                        src: Operand::Stack(src),
                        dst: Operand::Register(Register::R10),
                    });
                    self.0.body.push(Instruction::Move {
                        src: Operand::Register(Register::R10),
                        dst: Operand::Stack(dst),
                    });
                }
                Instruction::Idiv(Operand::Immediate(val)) => {
                    self.0.body.push(Instruction::Move {
                        src: Operand::Immediate(val),
                        dst: Operand::Register(Register::R10),
                    });
                    self.0
                        .body
                        .push(Instruction::Idiv(Operand::Register(Register::R10)));
                }
                Instruction::BinaryOp {
                    op,
                    src: Operand::Stack(src),
                    dst: Operand::Stack(dst),
                } if matches!(
                    op,
                    BinaryOperator::Add
                        | BinaryOperator::Sub
                        | BinaryOperator::BitAnd
                        | BinaryOperator::BitOr
                        | BinaryOperator::BitXor
                ) =>
                {
                    self.0.body.push(Instruction::Move {
                        src: Operand::Stack(src),
                        dst: Operand::Register(Register::R10),
                    });
                    self.0.body.push(Instruction::BinaryOp {
                        op,
                        src: Operand::Register(Register::R10),
                        dst: Operand::Stack(dst),
                    });
                }
                Instruction::BinaryOp {
                    op: BinaryOperator::Mult,
                    src,
                    dst: Operand::Stack(dst),
                } => {
                    self.0.body.push(Instruction::Move {
                        src: Operand::Stack(dst),
                        dst: Operand::Register(Register::R11),
                    });
                    self.0.body.push(Instruction::BinaryOp {
                        op: BinaryOperator::Mult,
                        src,
                        dst: Operand::Register(Register::R11),
                    });
                    self.0.body.push(Instruction::Move {
                        src: Operand::Register(Register::R11),
                        dst: Operand::Stack(dst),
                    });
                }
                instruction => self.0.body.push(instruction),
            }
        }
    }
}

pub fn to_assembly(program: tacky::Program) -> Program {
    let mut program = program.to_assembly();
    program.update_pseudo_operands();
    program.final_pass();

    program
}
