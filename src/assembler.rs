use crate::runtime::*;
use crate::type_checker::*;
use std::collections::HashMap;

pub fn convert_program_to_ops(stmts: &[TStmt]) -> Vec<Opcode> {
    let mut functions = convert_stmts_to_ops(0, stmts);
    let mut program = functions.remove(&0).unwrap();
    let mut function_translations = HashMap::new();

    for (id, mut stmts) in functions.drain() {
        function_translations.insert(id, program.len() as u32);
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

pub fn convert_stmts_to_ops(function_index: u32, stmts: &[TStmt]) -> HashMap<u32, Vec<Opcode>> {
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
            TStmt::Function {
                uid,
                return_type,
                arguments,
                stmts,
            } => {
                for (f, stmts) in convert_stmts_to_ops(*uid, stmts).drain() {
                    functions.insert(f, stmts);
                }
            }
            _ => panic!(),
        }
    }
    ops.push(Opcode::Return);
    functions.insert(function_index, ops);
    return functions;
}

pub fn convert_expression_to_ops(ops: &mut Vec<Opcode>, expr: &TExpr) {
    match &expr.tag {
        TExprTag::Ident { stack_offset } => {
            ops.push(Opcode::GetLocal {
                stack_offset: *stack_offset,
            });
        }
        TExprTag::Add(l, r) => {
            convert_expression_to_ops(ops, l);
            convert_expression_to_ops(ops, r);
            if l.type_ == Type::Float {
                ops.push(Opcode::AddFloat);
            } else {
                ops.push(Opcode::AddInt);
            }
        }
        TExprTag::Int(value) => {
            ops.push(Opcode::MakeInt(*value as i64));
        }
        TExprTag::Float(value) => {
            ops.push(Opcode::MakeFloat(*value));
        }
        TExprTag::Call { callee, arguments } => {
            ops.push(Opcode::PushNone);
            for arg in arguments.iter().rev() {
                convert_expression_to_ops(ops, arg);
            }
            ops.push(Opcode::Call(*callee));
        }
        TExprTag::ECall { arguments } => {
            for arg in arguments.iter().rev() {
                convert_expression_to_ops(ops, arg);
            }
            ops.push(Opcode::ECall);
        }
    }
}
