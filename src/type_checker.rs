use crate::builtins::*;
use crate::syntax_tree::*;
use crate::util::*;
use std::collections::HashMap;
use std::marker::PhantomData;

pub struct SymbolTable<'a, 'b>
where
    'b: 'a,
{
    pub scope_id: u32,
    pub symbols: HashMap<u32, &'b InferredType<'b>>,
    phantom: PhantomData<&'a u8>,
}

pub struct TypeChecker<'a, 'b>
where
    'b: 'a,
{
    buckets: &'a mut Buckets<'b>,
    type_tables: Vec<SymbolTable<'a, 'b>>,
    symbol_tables: Vec<SymbolTable<'a, 'b>>,
}

impl<'a, 'b> TypeChecker<'a, 'b>
where
    'b: 'a,
{
    pub fn new(buckets: &'a mut Buckets<'b>) -> Self {
        let type_table = builtin_types(buckets);
        let symbol_table = builtin_symbols(buckets);
        return Self {
            buckets,
            type_tables: vec![SymbolTable {
                scope_id: 0,
                symbols: type_table,
                phantom: PhantomData,
            }],
            symbol_tables: vec![SymbolTable {
                scope_id: 0,
                symbols: symbol_table,
                phantom: PhantomData,
            }],
        };
    }

    pub fn check_program<'c>(&mut self, program: &'c mut [Stmt<'b>]) -> Result<(), Error<'b>> {
        // TODO add functions to symbol table
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
                let end = value.view.end;
                if self.symbol_scope_contains(*name) {
                    return Err(Error {
                        location: *name_loc..end,
                        message: "can't shadow variables in current scope",
                    });
                }

                let declared_type;
                if let Some(decl_type) = self.search_type_table(*type_name) {
                    declared_type = decl_type;
                } else {
                    return Err(Error {
                        location: *name_loc..end,
                        message: "type doesn't exist",
                    });
                }

                if Self::is_assignment_compatible(declared_type, self.check_expr(value)?) {
                    self.symbol_scope_add(*name, declared_type);
                    return Ok(());
                } else {
                    return Err(Error {
                        location: *name_loc..end,
                        message: "type doesn't match value type",
                    });
                }
            }
            Stmt::Assign {
                to,
                to_scope,
                to_loc,
                value,
            } => {
                let var_type = self.search_symbol_table(*to);
                let var_type = match var_type {
                    Some(t) => t,
                    None => {
                        return Err(Error {
                            location: *to_loc..value.view.start,
                            message: "variable not found",
                        })
                    }
                };

                if Self::is_assignment_compatible(var_type.1, self.check_expr(value)?) {
                    *to_scope = var_type.0;
                    return Ok(());
                }

                return Err(Error {
                    location: *to_loc..value.view.end,
                    message: "not assignment compatible",
                });
            }
            Stmt::AssignMember {
                to,
                to_member,
                value,
            } => {
                return Err(Error {
                    location: to.view.start..value.view.end,
                    message: "no member assignments yet",
                })
            }
            Stmt::Function {
                name,
                name_loc,
                scope_id,
                arguments,
                stmts,
            } => {
                if self.symbol_scope_contains(*name) {
                    return Err(Error {
                        location: *name_loc..(*name_loc + 1),
                        message: "can't shadow variables in current scope",
                    });
                }

                let mut symbols = HashMap::new();
                let mut arg_types = Vec::new();
                for arg in arguments.iter() {
                    if let Some(t) = self.search_type_table(arg.type_name) {
                        arg_types.push(*t);
                        symbols.insert(arg.name, t);
                    }
                }

                let function_type = InferredType::Function {
                    return_type: self.search_symbol_table(NONE_IDX).unwrap().1,
                    arguments: self.buckets.add_array(arg_types),
                };
                let function_type = self.buckets.add(function_type);
                self.symbol_scope_add(*name, function_type);
                self.symbol_tables.push(SymbolTable {
                    scope_id: *scope_id,
                    symbols,
                    phantom: PhantomData,
                });
                self.check_program(stmts)?;
                self.symbol_tables.pop();

                return Ok(());
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
            Ident { id, scope_origin } => {
                let sym_entry = self.search_symbol_table(*id);
                if let Some(sym_type) = sym_entry {
                    expr.inferred_type = *sym_type.1;
                    *scope_origin = sym_type.0;
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
                    arguments: args_formal,
                } = callee_type
                {
                    for (formal, arg) in args_formal.iter().zip(arguments.iter_mut()) {
                        let (start, end) = (arg.view.start, arg.view.end);
                        let arg_type = self.check_expr(arg)?;
                        if !Self::is_assignment_compatible(formal, arg_type) {
                            return Err(Error {
                                location: start..end,
                                message: "argument is wrong type",
                            });
                        }
                    }

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

    fn is_assignment_compatible(to: &InferredType<'b>, value: &InferredType<'b>) -> bool {
        return match to {
            InferredType::Unknown => panic!(),
            InferredType::Float => *value == InferredType::Float || *value == InferredType::Int,
            InferredType::Any => true,
            x => x == value,
        };
    }

    fn symbol_scope_add(&mut self, id: u32, symbol_type: &'b InferredType<'b>) {
        let sym = self.symbol_tables.last_mut().unwrap();
        sym.symbols.insert(id, symbol_type);
    }

    fn symbol_scope_contains(&self, id: u32) -> bool {
        let sym = self.symbol_tables.last().unwrap();
        return sym.symbols.contains_key(&id);
    }

    fn search_symbol_scope(&self, id: u32) -> Option<&'b InferredType<'b>> {
        let sym = self.symbol_tables.last().unwrap();
        if sym.symbols.contains_key(&id) {
            return Some(sym.symbols[&id]);
        } else {
            return None;
        }
    }

    fn search_type_table(&self, id: u32) -> Option<&'b InferredType<'b>> {
        for type_table in self.type_tables.iter().rev() {
            if type_table.symbols.contains_key(&id) {
                return Some(type_table.symbols[&id]);
            }
        }
        return None;
    }

    fn search_symbol_table(&self, id: u32) -> Option<(u32, &'b InferredType<'b>)> {
        for symbol_table in self.symbol_tables.iter().rev() {
            if symbol_table.symbols.contains_key(&id) {
                return Some((symbol_table.scope_id, symbol_table.symbols[&id]));
            }
        }
        return None;
    }
}
