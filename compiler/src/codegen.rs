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
        operator: UnaryOperator,
        operand: Operand,
    },
    BinaryOp {
        op: BinaryOperator,
        src: Operand,
        dst: Operand,
    },
    Cmp {
        val1: Operand,
        val2: Operand,
    },
    Shift {
        left: bool,
        shift: Operand,
        val: Operand,
    },
    Idiv(Operand),
    Cdq,
    Jmp(String),
    JmpCC(ConditionCode, String),
    SetCC(ConditionCode, Operand),
    Label(String),
    AllocateStack {
        size: usize,
    },
    Return,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionCode {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Complement,
    Negate,
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
            tacky::Instruction::UnaryOp {
                op: parser::UnaryOperator::LogicalNot,
                src,
                dst,
            } => vec![
                Instruction::Cmp {
                    val1: Operand::Immediate(0),
                    val2: src.to_assembly(),
                },
                Instruction::Move {
                    src: Operand::Immediate(0),
                    dst: dst.clone().to_assembly(),
                },
                Instruction::SetCC(ConditionCode::Equal, dst.to_assembly()),
            ],
            tacky::Instruction::UnaryOp { op, src, dst } => vec![
                Instruction::Move {
                    src: src.to_assembly(),
                    dst: dst.clone().to_assembly(),
                },
                Instruction::UnaryOp {
                    operator: UnaryOperator::try_from(op).unwrap(),
                    operand: dst.to_assembly(),
                },
            ],
            tacky::Instruction::BinaryOp {
                op,
                val1,
                val2,
                dst,
            } => match op {
                tacky::BinaryOperator::Add
                | tacky::BinaryOperator::Subtract
                | tacky::BinaryOperator::Multiply
                | tacky::BinaryOperator::BitwiseAnd
                | tacky::BinaryOperator::BitwiseOr
                | tacky::BinaryOperator::BitwiseXor => vec![
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
                tacky::BinaryOperator::LeftShift | tacky::BinaryOperator::RightShift => {
                    vec![
                        Instruction::Move {
                            src: val1.to_assembly(),
                            dst: dst.clone().to_assembly(),
                        },
                        Instruction::Shift {
                            left: matches!(op, tacky::BinaryOperator::LeftShift),
                            val: dst.to_assembly(),
                            shift: val2.to_assembly(),
                        },
                    ]
                }
                tacky::BinaryOperator::Divide | tacky::BinaryOperator::Remainder => {
                    vec![
                        Instruction::Move {
                            src: val1.to_assembly(),
                            dst: Operand::Register(Register::AX),
                        },
                        Instruction::Cdq,
                        Instruction::Idiv(val2.to_assembly()),
                        Instruction::Move {
                            src: if op == tacky::BinaryOperator::Divide {
                                Operand::Register(Register::AX)
                            } else {
                                Operand::Register(Register::DX)
                            },
                            dst: dst.to_assembly(),
                        },
                    ]
                }
                op => {
                    vec![
                        Instruction::Cmp {
                            val1: val2.to_assembly(),
                            val2: val1.to_assembly(),
                        },
                        Instruction::Move {
                            src: Operand::Immediate(0),
                            dst: dst.clone().to_assembly(),
                        },
                        Instruction::SetCC(
                            ConditionCode::try_from(op).expect("Invalid condition code"),
                            dst.to_assembly(),
                        ),
                    ]
                }
            },
            tacky::Instruction::Copy { src, dst } => vec![Instruction::Move {
                src: src.to_assembly(),
                dst: dst.to_assembly(),
            }],
            tacky::Instruction::Jump(label) => vec![Instruction::Jmp(label)],
            tacky::Instruction::JumpIfZero { val, target } => {
                vec![
                    Instruction::Cmp {
                        val1: Operand::Immediate(0),
                        val2: val.to_assembly(),
                    },
                    Instruction::JmpCC(ConditionCode::Equal, target),
                ]
            }
            tacky::Instruction::JumpIfNotZero { val, target } => {
                vec![
                    Instruction::Cmp {
                        val1: Operand::Immediate(0),
                        val2: val.to_assembly(),
                    },
                    Instruction::JmpCC(ConditionCode::NotEqual, target),
                ]
            }
            tacky::Instruction::Label(idenifier) => vec![Instruction::Label(idenifier)],
        }
    }
}

impl TryFrom<tacky::BinaryOperator> for ConditionCode {
    type Error = ();

    fn try_from(value: tacky::BinaryOperator) -> Result<Self, Self::Error> {
        match value {
            tacky::BinaryOperator::Equal => Ok(ConditionCode::Equal),
            tacky::BinaryOperator::NotEqual => Ok(ConditionCode::NotEqual),
            tacky::BinaryOperator::LessThan => Ok(ConditionCode::Less),
            tacky::BinaryOperator::GreaterThan => Ok(ConditionCode::Greater),
            tacky::BinaryOperator::LessEqual => Ok(ConditionCode::LessEqual),
            tacky::BinaryOperator::GreaterEqual => Ok(ConditionCode::GreaterEqual),
            _ => Err(()),
        }
    }
}

impl TryFrom<parser::UnaryOperator> for UnaryOperator {
    type Error = ();

    fn try_from(value: parser::UnaryOperator) -> Result<Self, Self::Error> {
        match value {
            parser::UnaryOperator::Complement => Ok(UnaryOperator::Complement),
            parser::UnaryOperator::Negate => Ok(UnaryOperator::Negate),
            _ => Err(()),
        }
    }
}

impl ToAssembly for tacky::BinaryOperator {
    type Output = BinaryOperator;

    fn to_assembly(self) -> Self::Output {
        match self {
            tacky::BinaryOperator::Add => BinaryOperator::Add,
            tacky::BinaryOperator::Subtract => BinaryOperator::Sub,
            tacky::BinaryOperator::Multiply => BinaryOperator::Mult,
            tacky::BinaryOperator::BitwiseAnd => BinaryOperator::BitAnd,
            tacky::BinaryOperator::BitwiseOr => BinaryOperator::BitOr,
            tacky::BinaryOperator::BitwiseXor => BinaryOperator::BitXor,
            tacky::BinaryOperator::Divide
            | tacky::BinaryOperator::Remainder
            | tacky::BinaryOperator::LeftShift
            | tacky::BinaryOperator::RightShift
            | tacky::BinaryOperator::Equal
            | tacky::BinaryOperator::NotEqual
            | tacky::BinaryOperator::LessThan
            | tacky::BinaryOperator::GreaterThan
            | tacky::BinaryOperator::LessEqual
            | tacky::BinaryOperator::GreaterEqual => unreachable!(),
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
                Instruction::Shift { val, shift, .. } => {
                    update_operand(val);
                    update_operand(shift);
                }
                Instruction::Idiv(operand) => update_operand(operand),
                Instruction::Cmp { val1, val2 } => {
                    update_operand(val1);
                    update_operand(val2);
                }
                Instruction::SetCC(_, operand) => update_operand(operand),
                Instruction::Cdq
                | Instruction::Return
                | Instruction::AllocateStack { .. }
                | Instruction::Jmp(_)
                | Instruction::JmpCC(..)
                | Instruction::Label(_) => {}
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
                Instruction::Cmp {
                    val1: Operand::Stack(val1),
                    val2: Operand::Stack(val2),
                } => {
                    self.0.body.push(Instruction::Move {
                        src: Operand::Stack(val1),
                        dst: Operand::Register(Register::R10),
                    });
                    self.0.body.push(Instruction::Cmp {
                        val1: Operand::Register(Register::R10),
                        val2: Operand::Stack(val2),
                    });
                }
                Instruction::Cmp {
                    val1,
                    val2: Operand::Immediate(i),
                } => {
                    self.0.body.push(Instruction::Move {
                        src: Operand::Immediate(i),
                        dst: Operand::Register(Register::R11),
                    });
                    self.0.body.push(Instruction::Cmp {
                        val1,
                        val2: Operand::Register(Register::R11),
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
                Instruction::Shift { left, shift, val }
                    if !matches!(shift, Operand::Immediate(_)) =>
                {
                    self.0.body.push(Instruction::Move {
                        src: shift,
                        dst: Operand::Register(Register::CX),
                    });
                    self.0.body.push(Instruction::Shift {
                        left,
                        shift: Operand::Register(Register::CX),
                        val,
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
