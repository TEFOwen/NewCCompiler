use std::io::Write;

use crate::{codegen::*, parser};

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
                write!(out, "\tmovl ")?;
                src.emit_code(&mut out)?;
                write!(out, ", ")?;
                dst.emit_code(&mut out)?;
                writeln!(out)
            }
            Instruction::Return => {
                writeln!(out, "\tmovq %rbp, %rsp")?;
                writeln!(out, "\tpopq %rbp")?;
                writeln!(out, "\tret")
            }
            Instruction::UnaryOp { operator, operand } => {
                write!(out, "\t")?;
                operator.emit_code(&mut out)?;
                write!(out, " ")?;
                operand.emit_code(&mut out)?;
                writeln!(out)
            }
            Instruction::AllocateStack { size } => {
                writeln!(out, "\tsubq ${}, %rsp", size)
            }
            Instruction::BinaryOp { op, src, dst } => {
                write!(out, "\t")?;
                op.emit_code(&mut out)?;
                write!(out, " ")?;
                src.emit_code(&mut out)?;
                write!(out, ", ")?;
                dst.emit_code(&mut out)?;
                writeln!(out)
            }
            Instruction::Shift { left, val } => {
                write!(out, "\t{} %cl, ", if *left { "sall" } else { "sarl" })?;
                val.emit_code(&mut out)?;
                writeln!(out)
            }
            Instruction::Idiv(operand) => {
                write!(out, "\tidivl ")?;
                operand.emit_code(&mut out)?;
                writeln!(out)
            }
            Instruction::Cdq => writeln!(out, "\tcdq"),
        }
    }
}

impl EmitCode for parser::UnaryOperator {
    fn emit_code(&self, mut out: impl Write) -> std::io::Result<()> {
        match self {
            parser::UnaryOperator::Complement => write!(out, "notl"),
            parser::UnaryOperator::Negate => write!(out, "negl"),
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

impl EmitCode for Operand {
    fn emit_code(&self, mut out: impl Write) -> std::io::Result<()> {
        match self {
            Operand::Immediate(i) => write!(out, "${}", i),
            Operand::Register(register) => register.emit_code(&mut out),
            Operand::Pseudo(_) => unreachable!(
                "Pseudo operands should have been replaced by stack offsets in codegen::Program::update_pseudo_operands()"
            ),
            Operand::Stack(offset) => write!(out, "-{}(%rbp)", offset),
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
