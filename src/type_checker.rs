use crate::builtins::*;
use crate::syntax_tree::*;
use crate::util::*;
use std::collections::HashMap;
use std::marker::PhantomData;

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
        scope_offset: u32,
    },
    Int(i64),
    Float(f64),
    Add(&'a TExpr<'a>, &'a TExpr<'a>),
    Call {
        callee: u32,
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
}

pub struct SymbolTable<'a, 'b>
where
    'b: 'a,
{
    pub scope_id: u32,
    pub symbol_types: HashMap<u32, &'b Type<'b>>,
    pub symbol_offsets: HashMap<u32, u32>,
    phantom: PhantomData<&'a u8>,
}

pub struct TypeChecker<'a, 'b>
where
    'b: 'a,
{
    buckets: &'a mut Buckets<'b>,
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
            symbol_tables: vec![SymbolTable {
                scope_id: 0,
                symbol_types: symbol_table,
                symbol_offsets: HashMap::new(),
                phantom: PhantomData,
            }],
        };
    }

    pub fn check_stmts(&mut self, stmts: &[Stmt]) -> Result<&[TStmt<'b>], Error<'b>> {
        let mut tstmts = Vec::new();
        for stmt in stmts {
            match stmt {
                Stmt::Pass => {}
                Stmt::Expr(expr) => {
                    let expr = self.check_expr(expr)?;
                    let expr = self.buckets.add(expr);
                    tstmts.push(TStmt::Expr(expr));
                }
                _ => panic!(),
            }
        }

        return Ok(self.buckets.add_array(tstmts));
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
            ExprTag::Add(l, r) => {
                let le = self.check_expr(l)?;
                let re = self.check_expr(r)?;
                let mut le = self.buckets.add(le);
                let mut re = self.buckets.add(re);

                if le.type_ == Type::Float || re.type_ == Type::Float {
                    if re.type_ == Type::Int {
                        re = self.buckets.add(TExpr {
                            tag: TExprTag::Call {
                                callee: FLOAT_IDX,
                                arguments: ref_to_slice(re),
                            },
                            type_: Type::Float,
                        });
                    }
                    if le.type_ == Type::Int {
                        le = self.buckets.add(TExpr {
                            tag: TExprTag::Call {
                                callee: FLOAT_IDX,
                                arguments: ref_to_slice(le),
                            },
                            type_: Type::Float,
                        });
                    }
                    return Ok(TExpr {
                        tag: TExprTag::Add(le, re),
                        type_: Type::Float,
                    });
                }
                return Ok(TExpr {
                    tag: TExprTag::Add(le, re),
                    type_: Type::Int,
                });
            }
            ExprTag::Call { callee, arguments } => {
                if let Some((
                    scope_id,
                    Type::Function {
                        return_type,
                        arguments: args_formal,
                    },
                )) = self.search_symbol_table(*callee)
                {
                    let mut args = Vec::new();
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
                            callee: *callee,
                            arguments: self.buckets.add_array(args),
                        },
                        type_: **return_type,
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
                    location: newr(0, 0),
                    message: "not implemented yet",
                })
            }
        }
    }

    fn is_assignment_compatible(to: &Type<'b>, value: &Type<'b>) -> bool {
        return match to {
            Type::Unknown => panic!(),
            Type::Float => *value == Type::Float || *value == Type::Int,
            Type::Any => true,
            x => x == value,
        };
    }

    fn symbol_scope_add(&mut self, id: u32, symbol_type: &'b Type<'b>) {
        let sym = self.symbol_tables.last_mut().unwrap();
        sym.symbol_types.insert(id, symbol_type);
    }

    fn symbol_scope_contains(&self, id: u32) -> bool {
        let sym = self.symbol_tables.last().unwrap();
        return sym.symbol_types.contains_key(&id);
    }

    fn search_symbol_scope(&self, id: u32) -> Option<&'b Type<'b>> {
        let sym = self.symbol_tables.last().unwrap();
        if sym.symbol_types.contains_key(&id) {
            return Some(sym.symbol_types[&id]);
        } else {
            return None;
        }
    }

    fn search_symbol_table(&self, id: u32) -> Option<(u32, &'b Type<'b>)> {
        for symbol_table in self.symbol_tables.iter().rev() {
            if symbol_table.symbol_types.contains_key(&id) {
                return Some((symbol_table.scope_id, symbol_table.symbol_types[&id]));
            }
        }
        return None;
    }
}
