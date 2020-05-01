use crate::syntax_tree::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
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

    fn search_type_table(&self, id: u32) -> &Value {
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

    pub fn run_stmt(&mut self, stmt: &mut Stmt) {
        use Stmt::*;
        match stmt {
            Expr(expr) => {
                self.eval_expr(expr);
            }
            _ => panic!(),
        }
    }

    fn eval_expr(&mut self, expr: &mut Expr) -> Value {
        use ExprTag::*;
        return match &expr.tag {
            Ident(id) => self.values[&id].clone(),
            Int(value) => Value::Int(*value as i64),
            Float(value) => Value::Float(*value),
            Add(left, right) => panic!(),
            _ => panic!(),
        };
    }
}
