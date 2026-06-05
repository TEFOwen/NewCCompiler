use std::io::Write;

use crate::codegen::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterSize {
    Byte,
    Dword,
}

fn operand_to_string(operand: &Operand, size: RegisterSize) -> String {
    match operand {
        Operand::Immediate(i) => format!("${}", i),
        Operand::Register(register) => match register {
            Register::AX => format!(
                "{}",
                match size {
                    RegisterSize::Byte => "%al",
                    RegisterSize::Dword => "%eax",
                }
            ),
            Register::DX => format!(
                "{}",
                match size {
                    RegisterSize::Byte => "%dl",
                    RegisterSize::Dword => "%edx",
                }
            ),
            Register::CX => format!(
                "{}",
                match size {
                    RegisterSize::Byte => "%cl",
                    RegisterSize::Dword => "%ecx",
                }
            ),
            Register::R10 => format!(
                "{}",
                match size {
                    RegisterSize::Byte => "%r10b",
                    RegisterSize::Dword => "%r10d",
                }
            ),
            Register::R11 => format!(
                "{}",
                match size {
                    RegisterSize::Byte => "%r11b",
                    RegisterSize::Dword => "%r11d",
                }
            ),
        },
        Operand::Stack(offset) => format!("-{}(%rbp)", offset),
        Operand::Pseudo(_) => unreachable!(
            "Pseudo operands should have been replaced by stack offsets in codegen::Program::update_pseudo_operands()"
        ),
    }
}

pub trait EmitCode {
    fn emit_code(&self, out: impl Write) -> std::io::Result<()>;
}

impl EmitCode for Program {
    fn emit_code(&self, mut out: impl Write) -> std::io::Result<()> {
        self.0.emit_code(&mut out)?;
        writeln!(out, "\t.section .note.GNU-stack,\"\",@progbits")
    }
}

impl EmitCode for FuncDef {
    fn emit_code(&self, mut out: impl Write) -> std::io::Result<()> {
        writeln!(out, "\t.globl {}", self.name)?;
        writeln!(out, "{}:", self.name)?;
        writeln!(out, "\tpushq %rbp")?;
        writeln!(out, "\tmovq %rsp, %rbp")?;
        for instr in &self.body {
            instr.emit_code(&mut out)?;
        }
        Ok(())
    }
}

impl EmitCode for Instruction {
    fn emit_code(&self, mut out: impl Write) -> std::io::Result<()> {
        match self {
            Instruction::Move { src, dst } => {
                writeln!(
                    out,
                    "\tmovl {}, {}",
                    operand_to_string(src, RegisterSize::Dword),
                    operand_to_string(dst, RegisterSize::Dword)
                )
            }
            Instruction::Return => {
                writeln!(out, "\tmovq %rbp, %rsp")?;
                writeln!(out, "\tpopq %rbp")?;
                writeln!(out, "\tret")
            }
            Instruction::UnaryOp { operator, operand } => {
                write!(out, "\t")?;
                operator.emit_code(&mut out)?;
                writeln!(out, " {}", operand_to_string(operand, RegisterSize::Dword))
            }
            Instruction::AllocateStack { size } => {
                writeln!(out, "\tsubq ${}, %rsp", size)
            }
            Instruction::BinaryOp { op, src, dst } => {
                write!(out, "\t")?;
                op.emit_code(&mut out)?;
                writeln!(
                    out,
                    " {}, {}",
                    operand_to_string(src, RegisterSize::Dword),
                    operand_to_string(dst, RegisterSize::Dword)
                )
            }
            Instruction::Shift { left, val, shift } => {
                writeln!(
                    out,
                    "\t{} {}, {}",
                    if *left { "sall" } else { "sarl" },
                    operand_to_string(shift, RegisterSize::Byte),
                    operand_to_string(val, RegisterSize::Dword)
                )
            }
            Instruction::Idiv(operand) => {
                writeln!(
                    out,
                    "\tidivl {}",
                    operand_to_string(operand, RegisterSize::Dword)
                )
            }
            Instruction::Cdq => writeln!(out, "\tcdq"),
            Instruction::Cmp { val1, val2 } => {
                writeln!(
                    out,
                    "\tcmpl {}, {}",
                    operand_to_string(val1, RegisterSize::Dword),
                    operand_to_string(val2, RegisterSize::Dword)
                )
            }
            Instruction::Jmp(label) => writeln!(out, "\tjmp .L{}", label),
            Instruction::JmpCC(condition_code, label) => {
                write!(out, "\tj")?;
                condition_code.emit_code(&mut out)?;
                writeln!(out, " .L{}", label)
            }
            Instruction::SetCC(condition_code, operand) => {
                write!(out, "\tset")?;
                condition_code.emit_code(&mut out)?;
                writeln!(out, " {}", operand_to_string(operand, RegisterSize::Byte))
            }
            Instruction::Label(identifier) => writeln!(out, ".L{}:", identifier),
        }
    }
}

impl EmitCode for UnaryOperator {
    fn emit_code(&self, mut out: impl Write) -> std::io::Result<()> {
        match self {
            UnaryOperator::Complement => write!(out, "notl"),
            UnaryOperator::Negate => write!(out, "negl"),
        }
    }
}

impl EmitCode for BinaryOperator {
    fn emit_code(&self, mut out: impl Write) -> std::io::Result<()> {
        match self {
            BinaryOperator::Add => write!(out, "addl"),
            BinaryOperator::Sub => write!(out, "subl"),
            BinaryOperator::Mult => write!(out, "imull"),
            BinaryOperator::BitAnd => write!(out, "andl"),
            BinaryOperator::BitOr => write!(out, "orl"),
            BinaryOperator::BitXor => write!(out, "xorl"),
        }
    }
}

impl EmitCode for Register {
    fn emit_code(&self, mut out: impl Write) -> std::io::Result<()> {
        match self {
            Register::AX => write!(out, "%eax"),
            Register::DX => write!(out, "%edx"),
            Register::CX => write!(out, "%ecx"),
            Register::R10 => write!(out, "%r10d"),
            Register::R11 => write!(out, "%r11d"),
        }
    }
}

impl EmitCode for ConditionCode {
    fn emit_code(&self, mut out: impl Write) -> std::io::Result<()> {
        write!(
            out,
            "{}",
            match self {
                ConditionCode::Equal => "e",
                ConditionCode::NotEqual => "ne",
                ConditionCode::Less => "l",
                ConditionCode::LessEqual => "le",
                ConditionCode::Greater => "g",
                ConditionCode::GreaterEqual => "ge",
            }
        )
    }
}
