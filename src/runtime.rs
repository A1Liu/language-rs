use crate::builtins::*;
use crate::syntax_tree::*;

pub struct RuntimeScope {
    pub stack: Vec<u64>,
    pub heap: Vec<u64>,
    pub fp: usize,
    pub sp: usize,
}

impl RuntimeScope {
    pub fn new() -> Self {
        return Self {
            stack: Vec::new(),
            heap: Vec::new(),
            fp: 0,
            sp: 0,
        };
    }

    // fn search_values_table(&self, id: u32) -> &u64 {
    //     let mut values = &self.values;
    //     let mut parent = self.parent;
    //     loop {
    //         if values.contains_key(&id) {
    //             return &values[&id];
    //         }

    //         if let Some(p) = parent {
    //             values = &p.values;
    //             parent = p.parent;
    //         } else {
    //             break;
    //         }
    //     }

    //     panic!();
    // }

    pub fn run_stmt(&mut self, stmt: &Stmt) {
        use Stmt::*;
        match stmt {
            Expr(expr) => {
                self.eval_expr(expr);
            }
            // Assign { to, to_loc, value } => {
            //     let value = self.eval_expr(value);
            //     self.values.insert(*to, value);
            // }
            Declare {
                name,
                name_loc,
                type_name,
                value,
            } => {}

            _ => panic!(),
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> usize {
        use ExprTag::*;
        return match &expr.tag {
            Int(value) => {
                let ret_val = self.heap.len();
                self.heap.push(*value);
                return ret_val;
            }
            Float(value) => {
                let ret_val = self.heap.len();
                self.heap.push(value.to_bits());
                return ret_val;
            }
            // Call { callee, arguments } => {
            //     if let ExprTag::Ident(PRINT_IDX) = callee.tag {
            //         self.eval_print_function(arguments)
            //     } else {
            //         panic!();
            //     }
            // }
            Add(l, r) => {
                let lexpr = self.eval_expr(l);
                let rexpr = self.eval_expr(r);
                let ret_val = self.heap.len();
                if let InferredType::Int = l.inferred_type {
                    if let InferredType::Int = r.inferred_type {
                        self.heap
                            .push((self.heap[lexpr] as i64 + self.heap[rexpr] as i64) as u64);
                        ret_val
                    } else if let InferredType::Float = r.inferred_type {
                        self.heap.push(
                            ((self.heap[lexpr] as i64) as f64 + f64::from_bits(self.heap[rexpr]))
                                .to_bits(),
                        );
                        ret_val
                    } else {
                        panic!();
                    }
                } else if let InferredType::Float = l.inferred_type {
                    if let InferredType::Int = r.inferred_type {
                        self.heap.push(
                            (f64::from_bits(self.heap[lexpr]) + self.heap[rexpr] as i64 as f64)
                                .to_bits(),
                        );
                        ret_val
                    } else if let InferredType::Float = r.inferred_type {
                        self.heap.push(
                            (f64::from_bits(self.heap[lexpr]) + f64::from_bits(self.heap[rexpr]))
                                .to_bits(),
                        );
                        ret_val
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

    fn eval_print_function(&mut self, args: &[Expr]) -> usize {
        let bytes = self.eval_expr(&args[0]);
        if let InferredType::Int = args[0].inferred_type {
            println!("{}", self.heap[bytes] as i64);
        } else if let InferredType::Float = args[0].inferred_type {
            println!("{}", f64::from_bits(self.heap[bytes]));
        }
        return 0;
    }
}
