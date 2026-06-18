use std::collections::HashMap;

use crate::{parser, tacky};

static ARG_REGISTERS: &[Register] = &[
    Register::DI,
    Register::SI,
    Register::DX,
    Register::CX,
    Register::R8,
    Register::R9,
];

#[derive(Debug)]
pub struct Program(pub Vec<FuncDef>);

#[derive(Debug)]
pub struct FuncDef {
    pub identifier: String,
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
    DeallocateStack {
        size: usize,
    },
    Push(Operand),
    Call(String),
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
    Stack(i32),
}

#[derive(Debug, Clone, Copy)]
pub enum Register {
    AX,
    CX,
    DX,
    DI,
    SI,
    R8,
    R9,
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
        Program(
            self.0
                .into_iter()
                .map(|func_def| func_def.to_assembly())
                .collect(),
        )
    }
}

impl ToAssembly for tacky::FuncDef {
    type Output = FuncDef;

    fn to_assembly(self) -> Self::Output {
        let reg_params = self.params.iter().take(6).cloned().collect::<Vec<_>>();
        let stack_params = self.params.iter().skip(6).cloned().collect::<Vec<_>>();

        let param_copies = reg_params
            .into_iter()
            .zip(ARG_REGISTERS.iter())
            .map(|(param, register)| Instruction::Move {
                src: Operand::Register(*register),
                dst: Operand::Pseudo(param),
            })
            .chain(
                stack_params
                    .into_iter()
                    .enumerate()
                    .map(|(i, param)| Instruction::Move {
                        src: Operand::Stack(16 + i as i32 * 8),
                        dst: Operand::Pseudo(param),
                    }),
            );

        FuncDef {
            identifier: self.identifier,
            body: param_copies
                .chain(
                    self.body
                        .into_iter()
                        .flat_map(|instruction| instruction.to_assembly()),
                )
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
            tacky::Instruction::Label(identifier) => vec![Instruction::Label(identifier)],
            tacky::Instruction::FunCall { name, args, dst } => {
                let mut instructions = vec![];
                let register_args = args
                    .iter()
                    .take(ARG_REGISTERS.len())
                    .cloned()
                    .collect::<Vec<_>>();
                let stack_args = args
                    .iter()
                    .skip(ARG_REGISTERS.len())
                    .cloned()
                    .collect::<Vec<_>>();
                let stack_padding = if stack_args.len() % 2 == 1 { 8 } else { 0 };
                if stack_padding > 0 {
                    instructions.push(Instruction::AllocateStack {
                        size: stack_padding,
                    });
                }

                for (register, tacky_arg) in ARG_REGISTERS.iter().zip(register_args.into_iter()) {
                    instructions.push(Instruction::Move {
                        src: tacky_arg.to_assembly(),
                        dst: Operand::Register(*register),
                    });
                }

                let bytes_to_remove = 8 * stack_args.len() + stack_padding;
                for tacky_arg in stack_args.into_iter().rev() {
                    let asm_arg = tacky_arg.to_assembly();
                    if matches!(asm_arg, Operand::Immediate(_) | Operand::Register(_)) {
                        instructions.push(Instruction::Push(asm_arg));
                    } else {
                        instructions.push(Instruction::Move {
                            src: asm_arg,
                            dst: Operand::Register(Register::AX),
                        });
                        instructions.push(Instruction::Push(Operand::Register(Register::AX)));
                    }
                }

                instructions.push(Instruction::Call(name));

                if bytes_to_remove > 0 {
                    instructions.push(Instruction::DeallocateStack {
                        size: bytes_to_remove,
                    });
                }

                let asm_dst = dst.to_assembly();
                instructions.push(Instruction::Move {
                    src: Operand::Register(Register::AX),
                    dst: asm_dst,
                });

                instructions
            }
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
        for func in self.0.iter_mut() {
            let mut var_map = HashMap::new();
            let mut stack_size = 0i32;

            let mut update_operand = |operand: &mut Operand, size: i32| {
                if let Operand::Pseudo(name) = operand {
                    let stack_offset = *var_map.entry(name.clone()).or_insert_with(|| {
                        stack_size += size;
                        stack_size
                    });
                    *operand = Operand::Stack(-stack_offset);
                }
            };

            for instruction in &mut func.body {
                match instruction {
                    Instruction::Move { src, dst } => {
                        update_operand(dst, 4);
                        update_operand(src, 4);
                    }
                    Instruction::UnaryOp { operand, .. } => update_operand(operand, 4),
                    Instruction::BinaryOp { src: val2, dst, .. } => {
                        update_operand(val2, 4);
                        update_operand(dst, 4);
                    }
                    Instruction::Shift { val, shift, .. } => {
                        update_operand(val, 4);
                        update_operand(shift, 4);
                    }
                    Instruction::Idiv(operand) => update_operand(operand, 4),
                    Instruction::Cmp { val1, val2 } => {
                        update_operand(val1, 4);
                        update_operand(val2, 4);
                    }
                    Instruction::SetCC(_, operand) => update_operand(operand, 4),
                    Instruction::Push(operand) => update_operand(operand, 8),
                    Instruction::Cdq
                    | Instruction::Return
                    | Instruction::AllocateStack { .. }
                    | Instruction::Jmp(_)
                    | Instruction::JmpCC(..)
                    | Instruction::Label(_)
                    | Instruction::DeallocateStack { .. }
                    | Instruction::Call(_) => {}
                }
            }

            func.stack_size = Some(stack_size as usize);
        }
    }

    fn final_pass(&mut self) {
        for func_def in self.0.iter_mut() {
            let instructions = std::mem::take(&mut func_def.body);

            let mut stack_size = func_def
                .stack_size
                .expect("No stack size found for function");
            if stack_size % 16 != 0 {
                let padding = 16 - (stack_size % 16);
                stack_size += padding;
            }
            func_def
                .body
                .push(Instruction::AllocateStack { size: stack_size });

            for instruction in instructions {
                match instruction {
                    Instruction::Move {
                        src: Operand::Stack(src),
                        dst: Operand::Stack(dst),
                    } => {
                        func_def.body.push(Instruction::Move {
                            src: Operand::Stack(src),
                            dst: Operand::Register(Register::R10),
                        });
                        func_def.body.push(Instruction::Move {
                            src: Operand::Register(Register::R10),
                            dst: Operand::Stack(dst),
                        });
                    }
                    Instruction::Idiv(Operand::Immediate(val)) => {
                        func_def.body.push(Instruction::Move {
                            src: Operand::Immediate(val),
                            dst: Operand::Register(Register::R10),
                        });
                        func_def
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
                        func_def.body.push(Instruction::Move {
                            src: Operand::Stack(src),
                            dst: Operand::Register(Register::R10),
                        });
                        func_def.body.push(Instruction::BinaryOp {
                            op,
                            src: Operand::Register(Register::R10),
                            dst: Operand::Stack(dst),
                        });
                    }
                    Instruction::Cmp {
                        val1: Operand::Stack(val1),
                        val2: Operand::Stack(val2),
                    } => {
                        func_def.body.push(Instruction::Move {
                            src: Operand::Stack(val1),
                            dst: Operand::Register(Register::R10),
                        });
                        func_def.body.push(Instruction::Cmp {
                            val1: Operand::Register(Register::R10),
                            val2: Operand::Stack(val2),
                        });
                    }
                    Instruction::Cmp {
                        val1,
                        val2: Operand::Immediate(i),
                    } => {
                        func_def.body.push(Instruction::Move {
                            src: Operand::Immediate(i),
                            dst: Operand::Register(Register::R11),
                        });
                        func_def.body.push(Instruction::Cmp {
                            val1,
                            val2: Operand::Register(Register::R11),
                        });
                    }
                    Instruction::BinaryOp {
                        op: BinaryOperator::Mult,
                        src,
                        dst: Operand::Stack(dst),
                    } => {
                        func_def.body.push(Instruction::Move {
                            src: Operand::Stack(dst),
                            dst: Operand::Register(Register::R11),
                        });
                        func_def.body.push(Instruction::BinaryOp {
                            op: BinaryOperator::Mult,
                            src,
                            dst: Operand::Register(Register::R11),
                        });
                        func_def.body.push(Instruction::Move {
                            src: Operand::Register(Register::R11),
                            dst: Operand::Stack(dst),
                        });
                    }
                    Instruction::Shift { left, shift, val }
                        if !matches!(shift, Operand::Immediate(_)) =>
                    {
                        func_def.body.push(Instruction::Move {
                            src: shift,
                            dst: Operand::Register(Register::CX),
                        });
                        func_def.body.push(Instruction::Shift {
                            left,
                            shift: Operand::Register(Register::CX),
                            val,
                        });
                    }
                    instruction => func_def.body.push(instruction),
                }
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
