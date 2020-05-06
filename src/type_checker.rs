use crate::builtins::*;
use crate::syntax_tree::*;
use crate::util::*;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Type<'a> {
    Unknown,
    None,
    Any,
    Int,
    Float,
    Function {
        return_type: &'a Type<'a>,
        arguments: &'a [Type<'a>],
    },
}

impl<'a> Type<'a> {
    pub fn is_primitive(&self) -> bool {
        return match self {
            Type::Int | Type::Float => true,
            _ => false,
        };
    }
}

#[derive(Debug)]
pub enum TExprTag<'a> {
    Ident {
        stack_offset: i32,
    },
    Int(i64),
    Float(f64),
    Add(&'a TExpr<'a>, &'a TExpr<'a>),
    Call {
        callee: u32,
        arguments: &'a [TExpr<'a>],
    },
    ECall {
        arguments: &'a [TExpr<'a>],
    },
}

#[derive(Debug)]
pub struct TExpr<'a> {
    pub tag: TExprTag<'a>,
    pub type_: Type<'a>,
}

#[derive(Debug)]
pub enum TStmt<'a> {
    Expr(&'a TExpr<'a>),
    Declare {
        decl_type: &'a Type<'a>,
        value: &'a TExpr<'a>,
    },
    Assign {
        stack_offset: i32,
        value: &'a TExpr<'a>,
    },
    Function {
        uid: u32,
        return_type: &'a Type<'a>,
        arguments: &'a [Type<'a>],
        stmts: &'a [TStmt<'a>],
    },
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SymbolInfo<'a> {
    Function {
        uid: u32,
        return_type: &'a Type<'a>,
        arguments: &'a [Type<'a>],
    },
    Variable {
        offset: i32,
        type_: &'a Type<'a>,
    },
}

impl<'a> SymbolInfo<'a> {
    pub fn get_type(&self) -> Type<'a> {
        return match self {
            SymbolInfo::Function {
                uid,
                return_type,
                arguments,
            } => Type::Function {
                return_type,
                arguments,
            },
            SymbolInfo::Variable { offset, type_ } => **type_,
        };
    }
}

pub struct TypeChecker<'a, 'b>
where
    'b: 'a,
{
    next_uid: u32,
    buckets: &'a mut Buckets<'b>,
    symbol_tables: Vec<HashMap<u32, SymbolInfo<'b>>>,
    types: HashMap<u32, &'b Type<'b>>,
}

impl<'a, 'b> TypeChecker<'a, 'b>
where
    'b: 'a,
{
    pub fn new(buckets: &'a mut Buckets<'b>) -> Self {
        return Self {
            next_uid: 0,
            buckets,
            symbol_tables: Vec::new(),
            types: HashMap::new(),
        };
    }

    fn add_function_symbols(&mut self, stmts: &[Stmt]) -> Result<(), Error<'b>> {
        for stmt in stmts {
            match stmt {
                Stmt::Function {
                    name,
                    name_view,
                    arguments,
                    return_type_view,
                    return_type,
                    stmts,
                } => {
                    if self.symbol_scope_contains(*name) {
                        return Err(Error {
                            location: *name_view,
                            message: "redeclaration of name in the same scope",
                        });
                    }

                    let decl_return_type;
                    if let Some(return_type) = return_type {
                        decl_return_type = **unwrap_err(
                            self.types.get(return_type),
                            *return_type_view,
                            "type_doesn't exist",
                        )?;
                    } else {
                        decl_return_type = Type::None;
                    }

                    let decl_return_type = self.buckets.add(decl_return_type);

                    let mut arg_types = Vec::new();
                    let mut offset = -1;
                    for arg in arguments.iter() {
                        let arg_type;
                        arg_type = **unwrap_err(
                            self.types.get(&arg.type_name),
                            arg.view,
                            "type doesn't exist",
                        )?;

                        arg_types.push(arg_type);
                    }

                    let arg_types = self.buckets.add_array(arg_types);
                    self.symbol_scope_add(
                        *name,
                        SymbolInfo::Function {
                            uid: self.next_uid,
                            return_type: decl_return_type,
                            arguments: arg_types,
                        },
                    );
                    self.next_uid += 1;
                }
                _ => {}
            }
        }
        return Ok(());
    }

    pub fn check_program(&mut self, program: &[Stmt]) -> Result<&[TStmt<'b>], Error<'b>> {
        let type_table = builtin_types(self.buckets);
        let symbol_table = builtin_symbols(self.buckets);
        self.symbol_tables = vec![symbol_table];
        self.types = type_table;
        self.next_uid = self.symbol_tables.len() as u32 + 1;

        let mut tstmts = builtin_definitions(self.buckets);

        self.check_stmts(program, &mut tstmts)?;
        return Ok(self.buckets.add_array(tstmts));
    }

    fn check_stmts(
        &mut self,
        stmts: &[Stmt],
        tstmts: &mut Vec<TStmt<'b>>,
    ) -> Result<(), Error<'b>> {
        self.add_function_symbols(stmts);
        let mut current_offset = 0;
        for stmt in stmts {
            match stmt {
                Stmt::Pass => {}
                Stmt::Expr(expr) => {
                    let expr = self.check_expr(expr)?;
                    let expr = self.buckets.add(expr);
                    tstmts.push(TStmt::Expr(expr));
                }
                Stmt::Declare {
                    name,
                    name_view,
                    type_name,
                    type_view,
                    value,
                } => {
                    if self.symbol_scope_contains(*name) {
                        return Err(Error {
                            location: *name_view,
                            message: "redeclaration of name in the same scope",
                        });
                    }

                    let decl_type =
                        *unwrap_err(self.types.get(type_name), *type_view, "type doesn't exist")?;

                    let expr = self.check_expr(value)?;
                    let expr = self.buckets.add(expr);

                    if !Self::is_assignment_compatible(decl_type, &expr.type_) {
                        return Err(Error {
                            location: value.view,
                            message: "value type doesn't match variable type",
                        });
                    }

                    self.symbol_scope_add(
                        *name,
                        SymbolInfo::Variable {
                            type_: decl_type,
                            offset: current_offset,
                        },
                    );

                    tstmts.push(TStmt::Declare {
                        decl_type,
                        value: expr,
                    });
                    current_offset += 1;
                }
                Stmt::Assign { to, to_view, value } => {
                    let var_info = unwrap_err(
                        self.search_symbol_table(*to),
                        *to_view,
                        "name being assigned to doesn't exist",
                    )?;

                    let to_type;
                    let to_offset;
                    if let SymbolInfo::Variable { offset, type_ } = var_info {
                        to_type = *type_;
                        to_offset = offset;
                    } else {
                        return Err(Error {
                            location: *to_view,
                            message: "name being assigned to is function",
                        });
                    }

                    let expr = self.check_expr(value)?;
                    let expr = self.buckets.add(expr);

                    if !Self::is_assignment_compatible(&to_type, &expr.type_) {
                        return Err(Error {
                            location: value.view,
                            message: "value type doesn't match variable type",
                        });
                    }

                    let expr = if to_type == Type::Float && expr.type_ == Type::Int {
                        self.cast_to_float(expr)
                    } else {
                        expr
                    };

                    tstmts.push(TStmt::Assign {
                        stack_offset: to_offset,
                        value: expr,
                    });
                }
                Stmt::Function {
                    name,
                    name_view,
                    return_type_view,
                    arguments,
                    return_type,
                    stmts,
                } => {
                    let uid;
                    let return_type;
                    let arg_types;
                    if let SymbolInfo::Function {
                        uid: id,
                        return_type: rtype,
                        arguments,
                    } = self.search_symbol_table(*name).unwrap()
                    {
                        uid = id;
                        return_type = rtype;
                        arg_types = arguments;
                    } else {
                        panic!();
                    }

                    let mut scope = HashMap::new();

                    let mut offset = -1;
                    for (arg, arg_type) in arguments.iter().zip(arg_types) {
                        if scope.contains_key(&arg.name) {
                            return Err(Error {
                                location: arg.view,
                                message: "argument name already defined",
                            });
                        }

                        scope.insert(
                            arg.name,
                            SymbolInfo::Variable {
                                type_: arg_type,
                                offset,
                            },
                        );
                        offset -= 1;
                    }

                    self.symbol_tables.push(scope);
                    let mut fstmts = Vec::new();
                    self.check_stmts(stmts, &mut fstmts);
                    self.symbol_tables.pop();

                    let fstmts = self.buckets.add(fstmts);
                    tstmts.push(TStmt::Function {
                        uid,
                        arguments: arg_types,
                        return_type,
                        stmts: fstmts,
                    });
                }
                _ => panic!(),
            }
        }

        return Ok(());
    }

    pub fn check_expr(&mut self, expr: &Expr) -> Result<TExpr<'b>, Error<'b>> {
        match &expr.tag {
            ExprTag::Int(val) => {
                return Ok(TExpr {
                    tag: TExprTag::Int(*val as i64),
                    type_: Type::Int,
                });
            }
            ExprTag::Float(val) => {
                return Ok(TExpr {
                    tag: TExprTag::Float(*val),
                    type_: Type::Float,
                });
            }
            ExprTag::Ident { id } => {
                let var_info = unwrap_err(
                    self.search_symbol_table(*id),
                    expr.view,
                    "referenced name doesn't exist",
                )?;

                let (offset, type_) = match var_info {
                    SymbolInfo::Variable { offset, type_ } => (offset, type_),
                    SymbolInfo::Function { .. } => panic!("we don't have function objects yet"),
                };

                return Ok(TExpr {
                    tag: TExprTag::Ident {
                        stack_offset: offset,
                    },
                    type_: *type_,
                });
            }
            ExprTag::Add(l, r) => {
                let le = self.check_expr(l)?;
                let re = self.check_expr(r)?;
                let le = self.buckets.add(le);
                let re = self.buckets.add(re);

                if le.type_ == re.type_ {
                    return Ok(TExpr {
                        tag: TExprTag::Add(le, re),
                        type_: Type::Float,
                    });
                } else {
                    return Err(Error {
                        location: expr.view,
                        message: "incompatible types for addition",
                    });
                }
            }
            ExprTag::Call { callee, arguments } => {
                let var_info = unwrap_err(
                    self.search_symbol_table(*callee),
                    expr.view,
                    "name being called doesn't exist",
                )?;

                if let SymbolInfo::Function {
                    uid,
                    return_type,
                    arguments: args_formal,
                } = var_info
                {
                    let mut args = Vec::new();
                    if args_formal.len() != arguments.len() {
                        return Err(Error {
                            location: expr.view,
                            message: "wrong number of arguments",
                        });
                    }

                    for (formal, arg) in args_formal.iter().zip(arguments.iter()) {
                        let (start, end) = (arg.view.start, arg.view.end);
                        let arg = self.check_expr(arg)?;
                        if !Self::is_assignment_compatible(formal, &arg.type_) {
                            return Err(Error {
                                location: newr(start, end),
                                message: "argument is wrong type",
                            });
                        }
                        args.push(arg);
                    }

                    return Ok(TExpr {
                        tag: TExprTag::Call {
                            callee: uid,
                            arguments: self.buckets.add_array(args),
                        },
                        type_: *return_type,
                    });
                } else {
                    return Err(Error {
                        location: expr.view,
                        message: "callee not a function",
                    });
                }
            }
            x => {
                return Err(Error {
                    location: expr.view,
                    message: "not implemented yet",
                })
            }
        }
    }

    fn is_assignment_compatible(to: &Type<'b>, value: &Type<'b>) -> bool {
        match value {
            Type::None => return true,
            _ => {}
        }

        return match to {
            Type::Unknown => panic!(),
            Type::Any => true,
            Type::None => false,
            x => x == value,
        };
    }

    fn cast_to_float(&mut self, value: &'b TExpr<'b>) -> &'b mut TExpr<'b> {
        return self.buckets.add(TExpr {
            tag: TExprTag::Call {
                callee: FLOAT_IDX,
                arguments: ref_to_slice(value),
            },
            type_: Type::Float,
        });
    }

    fn symbol_scope_add(&mut self, id: u32, info: SymbolInfo<'b>) {
        let sym = self.symbol_tables.last_mut().unwrap();
        sym.insert(id, info);
    }

    fn symbol_scope_contains(&self, id: u32) -> bool {
        let sym = self.symbol_tables.last().unwrap();
        return sym.contains_key(&id);
    }

    fn search_symbol_scope(&self, id: u32) -> Option<SymbolInfo<'b>> {
        let sym = self.symbol_tables.last().unwrap();
        if sym.contains_key(&id) {
            return Some(sym[&id]);
        } else {
            return None;
        }
    }

    fn search_symbol_table(&self, id: u32) -> Option<SymbolInfo<'b>> {
        for symbol_table in self.symbol_tables.iter().rev() {
            if symbol_table.contains_key(&id) {
                return Some(symbol_table[&id]);
            }
        }
        return None;
    }
}
