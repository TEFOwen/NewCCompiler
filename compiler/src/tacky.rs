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
        let mut body = self.body.to_tacky();
        body.push(Instruction::Return(Value::Constant(0)));
        FuncDef {
            name: self.name,
            body,
        }
    }
}

impl ToTacky for parser::Block {
    type Output = Vec<Instruction>;

    fn to_tacky(self) -> Self::Output {
        self.0
            .into_iter()
            .flat_map(|item| item.to_tacky())
            .collect()
    }
}

impl ToTacky for parser::BlockItem {
    type Output = Vec<Instruction>;

    fn to_tacky(self) -> Self::Output {
        match self {
            parser::BlockItem::Statement(statement) => statement.to_tacky(),
            parser::BlockItem::Declaration(declaration) => declaration.to_tacky(),
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
            parser::Statement::Expression(expression) => expression.to_tacky().0,
            parser::Statement::Null => vec![],
            parser::Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let else_label = next_jump_label("if_else");
                let end_label = next_jump_label("if_end");
                let (mut instructions, cond) = condition.to_tacky();
                instructions.push(Instruction::JumpIfZero {
                    val: cond,
                    target: else_label.clone(),
                });
                instructions.extend(then_branch.to_tacky());
                instructions.push(Instruction::Jump(end_label.clone()));
                instructions.push(Instruction::Label(else_label));
                if let Some(else_branch) = else_branch {
                    instructions.extend(else_branch.to_tacky());
                }
                instructions.push(Instruction::Label(end_label));

                instructions
            }
            parser::Statement::Block(block) => block.to_tacky(),
            parser::Statement::Labeled(label, statement) => {
                let mut instructions = vec![Instruction::Label(label)];
                instructions.extend(statement.to_tacky());
                instructions
            }
            parser::Statement::Goto(label) => {
                vec![Instruction::Jump(label)]
            }
            parser::Statement::Break(label) => {
                vec![Instruction::Jump(format!(
                    "{}.break",
                    label.expect("Loops have not been resolved")
                ))]
            }
            parser::Statement::Continue(label) => {
                vec![Instruction::Jump(format!(
                    "{}.continue",
                    label.expect("Loops have not been resolved")
                ))]
            }
            parser::Statement::While {
                condition,
                body,
                label,
            } => {
                let label = label.clone().expect("Loops have not been resolved");
                let continue_label = format!("{}.continue", label);
                let break_label = format!("{}.break", label);

                let mut instructions = vec![Instruction::Label(continue_label.clone())];
                let (cond_instructions, cond) = condition.to_tacky();
                instructions.extend(cond_instructions);
                instructions.push(Instruction::JumpIfZero {
                    val: cond,
                    target: break_label.clone(),
                });
                instructions.extend(body.to_tacky());
                instructions.push(Instruction::Jump(continue_label));
                instructions.push(Instruction::Label(break_label));
                instructions
            }
            parser::Statement::DoWhile {
                body,
                condition,
                label,
            } => {
                let label = label.clone().expect("Loops have not been resolved");
                let start_label = format!("{}.start", label);
                let continue_label = format!("{}.continue", label);
                let break_label = format!("{}.break", label);

                let mut instructions = vec![Instruction::Label(start_label.clone())];
                instructions.extend(body.to_tacky());
                instructions.push(Instruction::Label(continue_label));
                let (cond_instructions, cond) = condition.to_tacky();
                instructions.extend(cond_instructions);
                instructions.push(Instruction::JumpIfNotZero {
                    val: cond,
                    target: start_label,
                });
                instructions.push(Instruction::Label(break_label));
                instructions
            }
            parser::Statement::For {
                init,
                condition,
                post,
                body,
                label,
            } => {
                let label = label.clone().expect("Loops have not been resolved");
                let start_label = format!("{}.start", label);
                let continue_label = format!("{}.continue", label);
                let break_label = format!("{}.break", label);

                let mut instructions = init.to_tacky();
                instructions.push(Instruction::Label(start_label.clone()));
                if let Some(condition) = condition {
                    let (cond_instructions, cond) = condition.to_tacky();
                    instructions.extend(cond_instructions);
                    instructions.push(Instruction::JumpIfZero {
                        val: cond,
                        target: break_label.clone(),
                    });
                }
                instructions.extend(body.to_tacky());
                instructions.push(Instruction::Label(continue_label.clone()));
                if let Some(post) = post {
                    instructions.extend(post.to_tacky().0);
                }
                instructions.push(Instruction::Jump(start_label));
                instructions.push(Instruction::Label(break_label));
                instructions
            }
        }
    }
}

impl ToTacky for parser::InitExp {
    type Output = Vec<Instruction>;

    fn to_tacky(self) -> Self::Output {
        match self {
            parser::InitExp::Declaration(declaration) => declaration.to_tacky(),
            parser::InitExp::Expression(expression) => expression
                .map(|expr| expr.to_tacky().0)
                .unwrap_or_else(Vec::new),
        }
    }
}

impl ToTacky for parser::Declaration {
    type Output = Vec<Instruction>;

    fn to_tacky(self) -> Self::Output {
        let mut instructions = vec![];
        if let Some(initialiser) = self.initialiser {
            let (init_instructions, init_val) = initialiser.to_tacky();
            instructions.extend(init_instructions);
            instructions.push(Instruction::Copy {
                src: init_val,
                dst: Value::Var(self.identifier),
            });
        }
        instructions
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
            parser::Expression::Assignment { left, right } => {
                let (mut instructions, right_val) = right.to_tacky();
                let left_val = left.to_tacky().1;
                instructions.push(Instruction::Copy {
                    src: right_val,
                    dst: left_val.clone(),
                });
                (instructions, left_val)
            }
            parser::Expression::BinaryOp { .. } => unreachable!(),
            parser::Expression::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                let else_label = next_jump_label("cond_else");
                let end_label = next_jump_label("cond_end");
                let out_val = next_binary_op_var();
                let (mut instructions, cond) = condition.to_tacky();
                instructions.push(Instruction::JumpIfZero {
                    val: cond,
                    target: else_label.clone(),
                });
                let (then_instructions, then_val) = then_branch.to_tacky();
                instructions.extend(then_instructions);
                instructions.push(Instruction::Copy {
                    src: then_val,
                    dst: out_val.clone(),
                });
                instructions.push(Instruction::Jump(end_label.clone()));
                instructions.push(Instruction::Label(else_label));
                let (else_instructions, else_val) = else_branch.to_tacky();
                instructions.extend(else_instructions);
                instructions.push(Instruction::Copy {
                    src: else_val,
                    dst: out_val.clone(),
                });
                instructions.push(Instruction::Label(end_label));
                (instructions, out_val)
            }
        }
    }
}

impl ToTacky for parser::Factor {
    type Output = (Vec<Instruction>, Value);

    fn to_tacky(self) -> Self::Output {
        match self {
            parser::Factor::UnaryOp { op, fac }
                if matches!(
                    op,
                    parser::UnaryOperator::PrefixIncrement | parser::UnaryOperator::PrefixDecrement
                ) =>
            {
                let (mut instructions, var) = fac.to_tacky();
                let dst = next_unary_op_var();
                instructions.push(Instruction::BinaryOp {
                    op: if op == parser::UnaryOperator::PrefixIncrement {
                        BinaryOperator::Add
                    } else {
                        BinaryOperator::Subtract
                    },
                    val1: var.clone(),
                    val2: Value::Constant(1),
                    dst: dst.clone(),
                });
                instructions.push(Instruction::Copy {
                    src: dst.clone(),
                    dst: var,
                });
                (instructions, dst)
            }
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
            parser::Factor::Postfix(postfix) => postfix.to_tacky(),
        }
    }
}

impl ToTacky for parser::Postfix {
    type Output = (Vec<Instruction>, Value);

    fn to_tacky(self) -> Self::Output {
        debug_assert!(
            self.postfix.len() <= 1,
            "Only one postfix operator is supported"
        );
        let (mut instructions, var) = self.primary.to_tacky();
        if let Some(op) = self.postfix.into_iter().next() {
            let dst = next_unary_op_var();
            instructions.push(Instruction::Copy {
                src: var.clone(),
                dst: dst.clone(),
            });
            instructions.push(Instruction::BinaryOp {
                op: if op == parser::PostfixOp::PostfixIncrement {
                    BinaryOperator::Add
                } else {
                    BinaryOperator::Subtract
                },
                val1: dst.clone(),
                val2: Value::Constant(1),
                dst: var,
            });
            (instructions, dst)
        } else {
            (instructions, var)
        }
    }
}

impl ToTacky for parser::Primary {
    type Output = (Vec<Instruction>, Value);

    fn to_tacky(self) -> Self::Output {
        match self {
            parser::Primary::Constant(val) => (vec![], Value::Constant(val)),
            parser::Primary::Paren(expression) => expression.to_tacky(),
            parser::Primary::Var(identifier) => (vec![], Value::Var(identifier)),
        }
    }
}
