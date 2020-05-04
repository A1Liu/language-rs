use crate::builtins::*;
use crate::runtime::*;
use crate::type_checker::*;

pub fn convert_stmts_to_ops(stmts: &[TStmt]) -> Vec<Opcode> {
    let mut functions = Vec::new();

    for stmt in stmts {
        if let TStmt::Declare { decl_type, value } = stmt {
            convert_expression_to_ops(&mut functions, value);
        }
    }

    for stmt in stmts {
        match stmt {
            TStmt::Expr(expr) => {
                convert_expression_to_ops(&mut functions, expr);
            }
            TStmt::Declare { decl_type, value } => {}
        }
    }

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
