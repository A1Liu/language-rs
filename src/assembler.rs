use crate::runtime::*;
use crate::syntax_tree::*;
use std::collections::HashMap;

pub fn convert_program_to_ops(stmts: &[TStmt]) -> Vec<Opcode> {
    let mut functions = convert_stmts_to_ops(0, stmts, 0);
    let mut program = functions.remove(&0).unwrap();
    let mut function_translations = HashMap::new();

    for (id, stmts) in functions.drain() {
        let function_offset = program.len() as u32;
        function_translations.insert(id, function_offset);

        for (idx, stmt) in stmts.into_iter().enumerate() {
            program.push(match stmt {
                Opcode::JumpIf(loc) => Opcode::JumpIf(idx as i32 + loc + function_offset as i32),
                x => x,
            });
        }
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
            callee, arguments, ..
        } => {
            ops.push(Opcode::PushNone);
            for arg in arguments.iter().rev() {
                convert_expression_to_ops(ops, arg);
            }
            ops.push(Opcode::Call(*callee));
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
