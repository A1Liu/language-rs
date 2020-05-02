use crate::builtins::*;
use crate::syntax_tree::*;
use std::collections::HashMap;

pub struct RuntimeScope<'a> {
    pub values: HashMap<u32, u64>,
    pub parent: Option<&'a RuntimeScope<'a>>,
}

impl<'a> RuntimeScope<'a> {
    pub fn new() -> Self {
        return Self {
            values: HashMap::new(),
            parent: None,
        };
    }

    fn search_values_table(&self, id: u32) -> &u64 {
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
            Assign { to, to_loc, value } => {
                let value = self.eval_expr(value);
                self.values.insert(*to, value);
            }
            Declare {
                name,
                name_loc,
                type_name,
                value,
            } => {
                let value = self.eval_expr(value);
                self.values.insert(*name, value);
            }
            _ => panic!(),
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> u64 {
        use ExprTag::*;
        return match &expr.tag {
            Ident(id) => self.values[&id].clone(),
            Int(value) => *value as u64,
            Float(value) => value.to_bits(),
            Call { callee, arguments } => {
                if let ExprTag::Ident(PRINT_IDX) = callee.tag {
                    let bytes = self.eval_expr(&arguments[0]);
                    if let InferredType::Int = arguments[0].inferred_type {
                        println!("{}", bytes as i64);
                    } else if let InferredType::Float = arguments[0].inferred_type {
                        println!("{}", f64::from_bits(bytes));
                    }
                    0
                } else {
                    panic!();
                }
            }
            Add(l, r) => {
                let lexpr = self.eval_expr(l);
                let rexpr = self.eval_expr(r);
                if let InferredType::Int = l.inferred_type {
                    if let InferredType::Int = r.inferred_type {
                        (lexpr as i64 + rexpr as i64) as u64
                    } else if let InferredType::Float = r.inferred_type {
                        ((lexpr as i64) as f64 + f64::from_bits(rexpr)).to_bits()
                    } else {
                        panic!();
                    }
                } else if let InferredType::Float = l.inferred_type {
                    if let InferredType::Int = r.inferred_type {
                        (f64::from_bits(lexpr) + rexpr as i64 as f64).to_bits()
                    } else if let InferredType::Float = r.inferred_type {
                        (f64::from_bits(lexpr) + f64::from_bits(rexpr)).to_bits()
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
