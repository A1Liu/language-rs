use crate::syntax_tree::*;
use crate::util::*;
use std::collections::HashMap;

pub struct TypeChecker<'a, 'b>
where
    'b: 'a,
{
    buckets: &'a mut Buckets<'b>,
    type_table: HashMap<u32, &'b InferredType<'b>>,
    symbol_table: HashMap<u32, &'b InferredType<'b>>,
    parent: Option<&'a TypeChecker<'a, 'b>>,
}

impl<'a, 'b> TypeChecker<'a, 'b>
where
    'b: 'a,
{
    pub fn new(buckets: &'a mut Buckets<'b>) -> Self {
        let type_table = HashMap::new();
        let symbol_table = HashMap::new();
        return Self {
            buckets,
            type_table,
            symbol_table,
            parent: None,
        };
    }

    pub fn check_program<'c>(&mut self, program: &'c mut [Stmt<'b>]) -> Result<(), Error<'b>> {
        for stmt in program {
            self.check_stmt(stmt)?;
        }
        return Ok(());
    }

    pub fn check_stmt<'c>(&mut self, stmt: &'c mut Stmt<'b>) -> Result<(), Error<'b>> {
        match stmt {
            Stmt::Pass => return Ok(()),
            Stmt::End => return Ok(()),
            Stmt::Expr(expr) => {
                self.check_expr(expr)?;
                return Ok(());
            }
            Stmt::Declare {
                name,
                name_loc,
                type_name,
                value,
            } => {
                return Err(Error {
                    location: *name_loc..value.view.end,
                    message: "no declarations yet",
                })
            }
            Stmt::Assign { to, value } => {
                return Err(Error {
                    location: to.view.start..value.view.end,
                    message: "no assignments yet",
                })
            }
        }
    }

    pub fn check_expr<'c>(
        &mut self,
        expr: &'c mut Expr<'b>,
    ) -> Result<&'c InferredType<'b>, Error<'b>> {
        use ExprTag::*;
        if expr.inferred_type != InferredType::Unknown {
            return Ok(&expr.inferred_type);
        }

        match &mut expr.tag {
            Ident(x) => {
                let sym_entry = self.search_symbol_table(*x);
                if let Some(sym_type) = sym_entry {
                    expr.inferred_type = *sym_type;
                    return Ok(&expr.inferred_type);
                } else {
                    return Err(Error {
                        location: expr.view.start..expr.view.end,
                        message: "identifier not found",
                    });
                }
            }
            Tup(tup) => {
                if tup.len() > 10 {
                    return Err(Error {
                        location: expr.view.start..expr.view.end,
                        message: "we don't support tuples of size larget than 10",
                    });
                }

                let mut types = HashMap::new();
                types.reserve(tup.len());
                let mut idx = 0;
                for tup_expr in tup.iter_mut() {
                    types.insert(tuple_component(idx), *self.check_expr(tup_expr)?);
                    idx += 1;
                }
                let types = self.buckets.add(types);
                expr.inferred_type = InferredType::Class(types);
                return Ok(&expr.inferred_type);
            }
            Add(l, r) => {
                let ltype = self.check_expr(l)?;
                let rtype = self.check_expr(r)?;
                if (*ltype == InferredType::Float && *rtype == InferredType::Int)
                    || (*rtype == InferredType::Float && *ltype == InferredType::Int)
                {
                    expr.inferred_type = InferredType::Float;
                    return Ok(&expr.inferred_type);
                }

                if ltype.is_primitive() && rtype == ltype {
                    expr.inferred_type = *ltype;
                    return Ok(&expr.inferred_type);
                }

                return Err(Error {
                    location: expr.view.start..expr.view.end,
                    message: "added together incompatible types",
                });
            }
            Call { callee, arguments } => {
                let (start, end) = (callee.view.start, callee.view.end);
                let callee_type = self.check_expr(callee)?;
                if let &InferredType::Function {
                    return_type,
                    arguments,
                } = callee_type
                {
                    expr.inferred_type = *return_type;
                    return Ok(&expr.inferred_type);
                } else {
                    return Err(Error {
                        location: start..end,
                        message: "called expression is not a function",
                    });
                }
            }
            DotAccess { parent, member_id } => {
                let (start, end) = (parent.view.start, parent.view.end);
                let parent_type = self.check_expr(parent)?;
                if let InferredType::Class(map) = parent_type {
                    expr.inferred_type = map[&member_id];
                    return Ok(&expr.inferred_type);
                } else {
                    return Err(Error {
                        location: start..end,
                        message: "parent isn't dereference-able",
                    });
                }
            }
            _ => panic!(),
        }
    }

    fn search_symbol_table<'c>(&'c self, id: u32) -> Option<&'c InferredType<'b>> {
        let mut symbol_table = &self.symbol_table;
        let mut parent = self.parent;
        loop {
            if symbol_table.contains_key(&id) {
                return Some(symbol_table[&id]);
            }

            if let Some(p) = parent {
                symbol_table = &p.symbol_table;
                parent = p.parent;
            } else {
                break;
            }
        }

        return None;
    }
}
