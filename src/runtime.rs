use crate::syntax_tree::*;
use crate::util::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    None,
}

pub struct RuntimeScope<'a> {
    pub values: HashMap<u32, Value>,
    pub parent: Option<&'a RuntimeScope<'a>>,
}

impl<'a> RuntimeScope<'a> {
    pub fn new() -> Self {
        return Self {
            values: HashMap::new(),
            parent: None,
        };
    }

    fn search_values_table(&self, id: u32) -> &Value {
        let mut values = &self.values;
        let mut parent = self.parent;
        loop {
            if values.contains_key(&id) {
                return &values[&id];
            }

            if let Some(p) = parent {
                values = &p.values;
                parent = p.parent;
            } else {
                break;
            }
        }

        panic!();
    }

    pub fn run_stmt(&mut self, stmt: &Stmt) {
        use Stmt::*;
        match stmt {
            Expr(expr) => {
                self.eval_expr(expr);
            }
            _ => panic!(),
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> Value {
        use ExprTag::*;
        return match &expr.tag {
            Ident(id) => self.values[&id].clone(),
            Int(value) => Value::Int(*value as i64),
            Float(value) => Value::Float(*value),
            Call { callee, arguments } => {
                if let ExprTag::Ident(PRINT_IDX) = callee.tag {
                    println!("{:?}", self.eval_expr(&arguments[0]));
                    Value::None
                } else {
                    panic!();
                }
            }
            Add(l, r) => {
                let lexpr = self.eval_expr(l);
                let rexpr = self.eval_expr(r);
                if let Value::Int(lval) = lexpr {
                    if let Value::Int(rval) = rexpr {
                        Value::Int(lval + rval)
                    } else if let Value::Float(rval) = rexpr {
                        Value::Float(lval as f64 + rval)
                    } else {
                        panic!();
                    }
                } else if let Value::Float(lval) = lexpr {
                    if let Value::Int(rval) = rexpr {
                        Value::Float(lval + rval as f64)
                    } else if let Value::Float(rval) = rexpr {
                        Value::Float(lval + rval)
                    } else {
                        panic!();
                    }
                } else {
                    panic!();
                }
            }
            _ => panic!(),
        };
    }
}
