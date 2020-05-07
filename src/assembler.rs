use crate::runtime::*;
use crate::syntax_tree::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
struct OpLoc {
    pub function_index: u32,
    pub offset: u32,
}

pub struct Assembler {
    functions: HashMap<u32, Vec<Opcode>>,
    labels: Vec<OpLoc>,
}

impl Assembler {
    pub fn new() -> Self {
        return Self {
            functions: HashMap::new(),
            labels: Vec::new(),
        };
    }

    pub fn create_label(&mut self, function_index: u32) -> u32 {
        let idx = self.labels.len() as u32;
        self.labels.push(OpLoc {
            function_index,
            offset: !0,
        });
        return idx;
    }

    pub fn attach_label(&mut self, label_idx: u32, location: u32) {
        self.labels[label_idx as usize].offset = location;
    }

    pub fn assemble_program(&mut self, stmts: &[TStmt]) -> Vec<Opcode> {
        let mut program = Vec::new();
        self.assemble_stmts(0, &mut program, stmts, 0);
        let mut function_translations = HashMap::new();

        for (id, mut stmts) in self.functions.drain() {
            let function_offset = program.len() as u32;
            function_translations.insert(id, function_offset);
            program.append(&mut stmts);
        }

        for op in &mut program {
            match op {
                Opcode::Call(func) => *func = function_translations[func],
                Opcode::JumpIf(label) => {
                    let op_loc = self.labels[*label as usize];
                    *label = function_translations[&op_loc.function_index] + op_loc.offset;
                }
                Opcode::Jump(label) => {
                    let op_loc = self.labels[*label as usize];
                    *label = function_translations[&op_loc.function_index] + op_loc.offset;
                }

                _ => {}
            }
        }

        return program;
    }

    fn assemble_stmts(
        &mut self,
        function_index: u32,
        current: &mut Vec<Opcode>,
        stmts: &[TStmt],
        return_index: i32,
    ) {
        for stmt in stmts {
            if let TStmt::Declare { decl_type, value } = stmt {
                current.push(Opcode::PushNone);
            }
        }

        let mut function_queue = Vec::new();
        let mut decl_index = 0;
        for stmt in stmts {
            match stmt {
                TStmt::Expr(expr) => {
                    convert_expression_to_ops(current, expr);
                    current.push(Opcode::Pop);
                }
                TStmt::Declare { decl_type, value } => {
                    convert_expression_to_ops(current, value);
                    current.push(Opcode::SetLocal {
                        stack_offset: decl_index,
                    });
                    decl_index += 1;
                }
                TStmt::Assign {
                    stack_offset,
                    value,
                } => {
                    convert_expression_to_ops(current, value);
                    current.push(Opcode::SetLocal {
                        stack_offset: *stack_offset,
                    });
                }
                TStmt::Return { ret_val } => {
                    convert_expression_to_ops(current, ret_val);
                    current.push(Opcode::SetLocal {
                        stack_offset: return_index,
                    });
                    current.push(Opcode::Return);
                }
                TStmt::If {
                    condition,
                    if_true,
                    if_false,
                } => {
                    convert_expression_to_ops(current, condition);

                    let true_label = self.create_label(function_index);
                    let end_label = self.create_label(function_index);

                    current.push(Opcode::JumpIf(true_label));
                    self.assemble_stmts(function_index, current, if_false, return_index);
                    current.push(Opcode::Jump(end_label));
                    self.attach_label(end_label, current.len() as u32);
                    self.assemble_stmts(function_index, current, if_true, return_index);
                    self.attach_label(end_label, current.len() as u32);
                }
                TStmt::Function {
                    uid,
                    return_type,
                    arguments,
                    stmts,
                } => {
                    function_queue.push((*uid, *stmts, -(arguments.len() as i32) - 1));
                }
            }
        }

        current.push(Opcode::Return);
        for (uid, stmts, return_idx) in function_queue {
            let mut current_function = Vec::new();
            self.assemble_stmts(uid, &mut current_function, stmts, return_idx);
            self.functions.insert(uid, current_function);
        }
    }
}

pub fn convert_program_to_ops(stmts: &[TStmt]) -> Vec<Opcode> {
    let mut functions = convert_stmts_to_ops(0, stmts, 0);
    let mut program = functions.remove(&0).unwrap();
    let mut function_translations = HashMap::new();

    for (id, mut stmts) in functions.drain() {
        let function_offset = program.len() as u32;
        function_translations.insert(id, function_offset);
        program.append(&mut stmts);
    }

    for op in &mut program {
        match op {
            Opcode::Call(func) => *func = function_translations[func],
            _ => {}
        }
    }

    return program;
}

pub fn convert_stmts_to_funcs(
    function_index: u32,
    stmts: &[TStmt],
    return_index: i32,
) -> HashMap<u32, Vec<Opcode>> {
    return HashMap::new();
}

pub fn convert_stmts_to_ops(
    function_index: u32,
    stmts: &[TStmt],
    return_index: i32,
) -> HashMap<u32, Vec<Opcode>> {
    let mut functions = HashMap::new();
    let mut ops = Vec::new();
    for stmt in stmts {
        if let TStmt::Declare { decl_type, value } = stmt {
            ops.push(Opcode::PushNone);
        } else if let TStmt::Function {
            uid,
            return_type,
            arguments,
            stmts,
        } = stmt
        {
            functions.insert(*uid, Vec::new());
        }
    }

    let mut decl_index = 0;
    for stmt in stmts {
        match stmt {
            TStmt::Expr(expr) => {
                convert_expression_to_ops(&mut ops, expr);
                ops.push(Opcode::Pop);
            }
            TStmt::Declare { decl_type, value } => {
                convert_expression_to_ops(&mut ops, value);
                ops.push(Opcode::SetLocal {
                    stack_offset: decl_index,
                });
                decl_index += 1;
            }
            TStmt::Assign {
                stack_offset,
                value,
            } => {
                convert_expression_to_ops(&mut ops, value);
                ops.push(Opcode::SetLocal {
                    stack_offset: *stack_offset,
                });
            }
            TStmt::Return { ret_val } => {
                convert_expression_to_ops(&mut ops, ret_val);
                ops.push(Opcode::SetLocal {
                    stack_offset: return_index,
                });
                ops.push(Opcode::Return);
            }
            TStmt::If {
                condition,
                if_true,
                if_false,
            } => {}
            TStmt::Function {
                uid,
                return_type,
                arguments,
                stmts,
            } => {
                let return_idx = -(arguments.len() as i32) - 1;
                for (f, stmts) in convert_stmts_to_ops(*uid, stmts, return_idx).drain() {
                    functions.insert(f, stmts);
                }
            }
        }
    }
    ops.push(Opcode::Return);
    functions.insert(function_index, ops);
    return functions;
}

pub fn convert_expression_to_ops(ops: &mut Vec<Opcode>, expr: &TExpr) {
    match expr {
        TExpr::Ident { stack_offset, .. } => {
            ops.push(Opcode::GetLocal {
                stack_offset: *stack_offset,
            });
        }
        TExpr::Add { left, right, type_ } => {
            convert_expression_to_ops(ops, left);
            convert_expression_to_ops(ops, right);
            if *type_ == Type::Float {
                ops.push(Opcode::AddFloat);
            } else {
                ops.push(Opcode::AddInt);
            }
        }
        TExpr::Int(value) => {
            ops.push(Opcode::MakeInt(*value as i64));
        }
        TExpr::Float(value) => {
            ops.push(Opcode::MakeFloat(*value));
        }
        TExpr::Call {
            callee_uid,
            arguments,
            ..
        } => {
            ops.push(Opcode::PushNone);
            for arg in arguments.iter().rev() {
                convert_expression_to_ops(ops, arg);
            }
            ops.push(Opcode::Call(*callee_uid));
            for _ in 0..arguments.len() {
                ops.push(Opcode::Pop);
            }
        }
        TExpr::ECall { arguments } => {
            for arg in arguments.iter().rev() {
                convert_expression_to_ops(ops, arg);
            }
            ops.push(Opcode::ECall);
        }
    }
}
