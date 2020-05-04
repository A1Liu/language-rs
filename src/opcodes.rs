use crate::builtins::*;
use crate::runtime::*;
use crate::type_checker::*;

pub fn convert_program_to_ops(stmts: &[TStmt]) -> Vec<Vec<Opcode>> {
    let mut functions = Vec::new();
    convert_stmts_to_ops(0, &mut functions, stmts);
    return functions;
}

pub fn convert_stmts_to_ops(
    function_index: usize,
    functions: &mut Vec<Vec<Opcode>>,
    stmts: &[TStmt],
) {
    let mut ops = Vec::new();

    functions.push(Vec::new());

    for stmt in stmts {
        if let TStmt::Declare { decl_type, value } = stmt {
            ops.push(Opcode::PushNone);
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
        }
    }

    functions[function_index] = ops;
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
            for arg in arguments.iter() {
                convert_expression_to_ops(ops, arg);
            }
            match *callee {
                PRINT_IDX => ops.push(Opcode::Call(PRINT_FUNC)),
                FLOAT_IDX => ops.push(Opcode::Call(FLOAT_CONSTRUCTOR)),
                _ => panic!(),
            }
        }
    }
}
