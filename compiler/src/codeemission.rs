use std::io::Write;

use crate::{codegen::*, resolve_types::SymbolTable};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterSize {
    Byte,
    Dword,
    Qword,
}

fn operand_to_string(operand: &Operand, size: RegisterSize) -> String {
    match operand {
        Operand::Immediate(i) => format!("${}", i),
        &Operand::Register(register) => match register {
            Register::AX => format!(
                "{}",
                match size {
                    RegisterSize::Byte => "%al",
                    RegisterSize::Dword => "%eax",
                    RegisterSize::Qword => "%rax",
                }
            ),
            Register::DX => format!(
                "{}",
                match size {
                    RegisterSize::Byte => "%dl",
                    RegisterSize::Dword => "%edx",
                    RegisterSize::Qword => "%rdx",
                }
            ),
            Register::CX => format!(
                "{}",
                match size {
                    RegisterSize::Byte => "%cl",
                    RegisterSize::Dword => "%ecx",
                    RegisterSize::Qword => "%rcx",
                }
            ),
            Register::DI => format!(
                "{}",
                match size {
                    RegisterSize::Byte => "%dil",
                    RegisterSize::Dword => "%edi",
                    RegisterSize::Qword => "%rdi",
                }
            ),
            Register::SI => format!(
                "{}",
                match size {
                    RegisterSize::Byte => "%sil",
                    RegisterSize::Dword => "%esi",
                    RegisterSize::Qword => "%rsi",
                }
            ),
            Register::R8 => format!(
                "{}",
                match size {
                    RegisterSize::Byte => "%r8b",
                    RegisterSize::Dword => "%r8d",
                    RegisterSize::Qword => "%r8",
                }
            ),
            Register::R9 => format!(
                "{}",
                match size {
                    RegisterSize::Byte => "%r9b",
                    RegisterSize::Dword => "%r9d",
                    RegisterSize::Qword => "%r9",
                }
            ),
            Register::R10 => format!(
                "{}",
                match size {
                    RegisterSize::Byte => "%r10b",
                    RegisterSize::Dword => "%r10d",
                    RegisterSize::Qword => "%r10",
                }
            ),
            Register::R11 => format!(
                "{}",
                match size {
                    RegisterSize::Byte => "%r11b",
                    RegisterSize::Dword => "%r11d",
                    RegisterSize::Qword => "%r11",
                }
            ),
        },
        Operand::Stack(offset) => format!("{}(%rbp)", offset),
        Operand::Pseudo(_) => unreachable!(
            "Pseudo operands should have been replaced by stack offsets in codegen::Program::update_pseudo_operands()"
        ),
    }
}

pub trait EmitCode {
    fn emit_code(&self, out: impl Write, symbol_table: &SymbolTable) -> std::io::Result<()>;
}

impl EmitCode for Program {
    fn emit_code(&self, mut out: impl Write, symbol_table: &SymbolTable) -> std::io::Result<()> {
        self.0
            .iter()
            .try_for_each(|func_def| func_def.emit_code(&mut out, symbol_table))?;
        writeln!(out, "\t.section .note.GNU-stack,\"\",@progbits")
    }
}

impl EmitCode for FuncDef {
    fn emit_code(&self, mut out: impl Write, symbol_table: &SymbolTable) -> std::io::Result<()> {
        writeln!(out, "\t.globl {}", self.identifier)?;
        writeln!(out, "{}:", self.identifier)?;
        writeln!(out, "\tpushq %rbp")?;
        writeln!(out, "\tmovq %rsp, %rbp")?;
        for instr in &self.body {
            instr.emit_code(&mut out, symbol_table)?;
        }
        Ok(())
    }
}

impl EmitCode for Instruction {
    fn emit_code(&self, mut out: impl Write, symbol_table: &SymbolTable) -> std::io::Result<()> {
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
                operator.emit_code(&mut out, symbol_table)?;
                writeln!(out, " {}", operand_to_string(operand, RegisterSize::Dword))
            }
            Instruction::AllocateStack { size } => {
                writeln!(out, "\tsubq ${}, %rsp", size)
            }
            Instruction::BinaryOp { op, src, dst } => {
                write!(out, "\t")?;
                op.emit_code(&mut out, symbol_table)?;
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
                condition_code.emit_code(&mut out, symbol_table)?;
                writeln!(out, " .L{}", label)
            }
            Instruction::SetCC(condition_code, operand) => {
                write!(out, "\tset")?;
                condition_code.emit_code(&mut out, symbol_table)?;
                writeln!(out, " {}", operand_to_string(operand, RegisterSize::Byte))
            }
            Instruction::Label(identifier) => writeln!(out, ".L{}:", identifier),
            Instruction::DeallocateStack { size } => {
                writeln!(out, "\taddq ${}, %rsp", size)
            }
            Instruction::Push(operand) => writeln!(
                out,
                "\tpushq {}",
                operand_to_string(operand, RegisterSize::Qword),
            ),
            Instruction::Call(func_name) => {
                let label = if symbol_table.has_symbol(func_name) {
                    func_name.clone()
                } else {
                    format!("{}@PLT", func_name)
                };
                writeln!(out, "\tcall {}", label)
            }
        }
    }
}

impl EmitCode for UnaryOperator {
    fn emit_code(&self, mut out: impl Write, _symbol_table: &SymbolTable) -> std::io::Result<()> {
        match self {
            UnaryOperator::Complement => write!(out, "notl"),
            UnaryOperator::Negate => write!(out, "negl"),
        }
    }
}

impl EmitCode for BinaryOperator {
    fn emit_code(&self, mut out: impl Write, _symbol_table: &SymbolTable) -> std::io::Result<()> {
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

impl EmitCode for ConditionCode {
    fn emit_code(&self, mut out: impl Write, _symbol_table: &SymbolTable) -> std::io::Result<()> {
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
