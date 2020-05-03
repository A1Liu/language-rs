use crate::builtins::*;
use crate::runtime::*;
use crate::syntax_tree::*;

pub fn convert_stmts_to_ops(stmts: &[Stmt]) -> Vec<Opcode> {
    let mut functions = Vec::new();

    for stmt in stmts {
        match stmt {
            Stmt::Expr(expr) => {
                convert_expression_to_ops(&mut functions, expr);
            }
            _ => panic!(),
        }
    }

    return functions;
}

pub fn convert_expression_to_ops(ops: &mut Vec<Opcode>, expr: &Expr) {
    match &expr.tag {
        ExprTag::Add(l, r) => {
            convert_expression_to_ops(ops, l);
            convert_expression_to_ops(ops, r);
            if l.inferred_type == InferredType::Float {
                ops.push(Opcode::AddFloat);
            } else {
                ops.push(Opcode::AddInt);
            }
        }
        ExprTag::Int(value) => {
            ops.push(Opcode::MakeInt(*value as i64));
        }
        ExprTag::Float(value) => {
            ops.push(Opcode::MakeFloat(*value));
        }
        ExprTag::Call { callee, arguments } => {
            for arg in arguments.iter() {
                convert_expression_to_ops(ops, arg);
            }
            match *callee {
                PRINT_IDX => ops.push(Opcode::Call(PRINT_FUNC)),
                FLOAT_IDX => ops.push(Opcode::Call(FLOAT_CONSTRUCTOR)),
                _ => panic!(),
            }
        }
        _ => panic!(),
    }
}
