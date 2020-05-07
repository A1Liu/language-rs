use crate::builtins::*;
use crate::syntax_tree::*;
use crate::util::*;
use std::collections::HashMap;
use std::ptr::NonNull;
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

struct SymbolTable<'a> {
    symbols: HashMap<u32, SymbolInfo<'a>>,
    parent: Option<NonNull<SymbolTable<'a>>>,
}

impl<'a> SymbolTable<'a> {
    pub fn new_global(symbols: HashMap<u32, SymbolInfo<'a>>) -> Self {
        return Self {
            symbols,
            parent: None,
        };
    }
    pub fn new<'b>(parent: &mut SymbolTable<'b>) -> Self
    where
        'b: 'a,
    {
        return Self {
            symbols: HashMap::new(),
            parent: Some(NonNull::from(parent)),
        };
    }

    pub fn search_current(&self, symbol: u32) -> Option<SymbolInfo<'a>> {
        return self.symbols.get(&symbol).map(|r| *r);
    }

    pub fn declare(
        &mut self,
        symbol: u32,
        info: SymbolInfo<'a>,
        view: CRange,
    ) -> Result<(), Error<'static>> {
        if self.symbols.contains_key(&symbol) {
            return err(view, "name already exists in scope");
        }
        self.symbols.insert(symbol, info);
        return Ok(());
    }

    pub fn search(&self, symbol: u32) -> Option<SymbolInfo<'a>> {
        return unsafe { self.search_unsafe(symbol) };
    }

    unsafe fn search_unsafe(&self, symbol: u32) -> Option<SymbolInfo<'a>> {
        let mut current = NonNull::from(self);
        let mut symbols = NonNull::from(&current.as_ref().symbols);

        loop {
            if let Some(info) = symbols.as_ref().get(&symbol) {
                return Some(*info);
            } else if let Some(parent) = current.as_ref().parent {
                current = parent;
                symbols = NonNull::from(&current.as_ref().symbols);
            } else {
                return None;
            }
        }
    }
}

pub struct TypeChecker<'a, 'b>
where
    'b: 'a,
{
    next_uid: u32,
    buckets: &'a mut Buckets<'b>,
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
            types: HashMap::new(),
        };
    }

    fn add_function_symbols(
        &mut self,
        sym: &mut SymbolTable<'b>,
        stmts: &[Stmt],
    ) -> Result<(), Error<'b>> {
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
                    sym.declare(
                        *name,
                        SymbolInfo::Function {
                            uid: self.next_uid,
                            return_type: decl_return_type,
                            arguments: arg_types,
                        },
                        *name_view,
                    )?;
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
        self.types = type_table;
        self.next_uid = symbol_table.len() as u32 + 1;

        let mut sym = SymbolTable::new_global(symbol_table);

        let mut tstmts = builtin_definitions(self.buckets);

        self.check_stmts(program, &mut sym, &mut tstmts, None)?;
        return Ok(self.buckets.add_array(tstmts));
    }

    fn check_stmts(
        &mut self,
        stmts: &[Stmt],
        sym: &mut SymbolTable<'b>,
        tstmts: &mut Vec<TStmt<'b>>,
        return_type: Option<Type<'b>>,
    ) -> Result<(), Error<'b>> {
        self.add_function_symbols(sym, stmts)?;
        let mut current_offset = 0;
        for stmt in stmts {
            match stmt {
                Stmt::Pass => {}
                Stmt::Expr(expr) => {
                    let expr = self.check_expr(sym, expr)?;
                    let expr = self.buckets.add(expr);
                    tstmts.push(TStmt::Expr(expr));
                }
                Stmt::Return { ret_val } => {
                    let return_type = unwrap_err(
                        return_type,
                        ret_val.view(),
                        "can't return a value from this context",
                    )?;

                    let ret_val_view = ret_val.view();
                    let ret_val = self.check_expr(sym, ret_val)?;

                    if !Self::is_assignment_compatible(&return_type, &ret_val.type_()) {
                        return err(ret_val_view, "returning the wrong type");
                    }

                    let ret_val = self.buckets.add(ret_val);
                    tstmts.push(TStmt::Return { ret_val });
                }
                Stmt::Declare {
                    name,
                    name_view,
                    type_name,
                    type_view,
                    value,
                } => {
                    let decl_type =
                        *unwrap_err(self.types.get(type_name), *type_view, "type doesn't exist")?;

                    sym.declare(
                        *name,
                        SymbolInfo::Variable {
                            type_: decl_type,
                            offset: current_offset,
                        },
                        *name_view,
                    )?;

                    let expr = self.check_expr(sym, value)?;
                    let expr = self.buckets.add(expr);

                    if !Self::is_assignment_compatible(decl_type, &expr.type_()) {
                        return err(value.view(), "value type doesn't match variable type");
                    }

                    tstmts.push(TStmt::Declare {
                        decl_type,
                        value: expr,
                    });
                    current_offset += 1;
                }
                Stmt::Assign { to, to_view, value } => {
                    let var_info = unwrap_err(sym.search(*to), *to_view, "name doesn't exist")?;

                    let to_type;
                    let to_offset;
                    if let SymbolInfo::Variable { offset, type_ } = var_info {
                        to_type = *type_;
                        to_offset = offset;
                    } else {
                        return err(*to_view, "name being assigned to is a function");
                    }

                    let expr = self.check_expr(sym, value)?;
                    let expr = self.buckets.add(expr);

                    if !Self::is_assignment_compatible(&to_type, &expr.type_()) {
                        return err(value.view(), "value type doesn't match variable type");
                    }

                    let expr = if to_type == Type::Float && expr.type_() == Type::Int {
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
                    } = sym.search(*name).unwrap()
                    {
                        uid = id;
                        return_type = rtype;
                        arg_types = arguments;
                    } else {
                        panic!();
                    }

                    let mut fsym = SymbolTable::new(sym);

                    let mut offset = -1;
                    for (arg, arg_type) in arguments.iter().zip(arg_types) {
                        fsym.declare(
                            arg.name,
                            SymbolInfo::Variable {
                                type_: arg_type,
                                offset,
                            },
                            arg.view,
                        )?;

                        offset -= 1;
                    }

                    let mut fstmts = Vec::new();
                    self.check_stmts(stmts, &mut fsym, &mut fstmts, Some(*return_type))?;

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

    pub fn check_expr(
        &mut self,
        sym: &SymbolTable<'b>,
        expr: &Expr,
    ) -> Result<TExpr<'b>, Error<'b>> {
        match expr {
            Expr::Int { value, view } => {
                return Ok(TExpr::Int(*value as i64));
            }
            Expr::Float { value, view } => {
                return Ok(TExpr::Float(*value));
            }
            Expr::Ident { id, view } => {
                let var_info = unwrap_err(sym.search(*id), *view, "referenced name doesn't exist")?;

                let (offset, type_) = match var_info {
                    SymbolInfo::Variable { offset, type_ } => (offset, type_),
                    SymbolInfo::Function { .. } => panic!("we don't have function objects yet"),
                };

                return Ok(TExpr::Ident {
                    stack_offset: offset,
                    type_: *type_,
                });
            }
            Expr::Add { left, right, view } => {
                let left = self.check_expr(sym, left)?;
                let right = self.check_expr(sym, right)?;
                let left = self.buckets.add(left);
                let right = self.buckets.add(right);

                if left.type_() == right.type_() {
                    return Ok(TExpr::Add {
                        left,
                        right,
                        type_: Type::Float,
                    });
                } else {
                    return err(*view, "incompatible types for addition");
                }
            }
            Expr::Call {
                callee,
                callee_view,
                arguments,
                arguments_view,
            } => {
                let var_info = unwrap_err(
                    sym.search(*callee),
                    *callee_view,
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
                        return err(*arguments_view, "wrong number of arguments");
                    }

                    for (formal, arg) in args_formal.iter().zip(arguments.iter()) {
                        let view = arg.view();
                        let arg = self.check_expr(sym, arg)?;
                        if !Self::is_assignment_compatible(formal, &arg.type_()) {
                            return err(view, "argument is wrong type");
                        }
                        args.push(arg);
                    }

                    return Ok(TExpr::Call {
                        callee_uid: uid,
                        arguments: self.buckets.add_array(args),
                        type_: *return_type,
                    });
                } else {
                    return err(expr.view(), "callee not a function");
                }
            }
            x => {
                return err(expr.view(), "not implemented yet");
            }
        }
    }

    fn is_assignment_compatible(to: &Type<'b>, value: &Type<'b>) -> bool {
        match value {
            Type::None => return true,
            _ => {}
        }

        return match to {
            Type::Any => true,
            Type::None => false,
            x => x == value,
        };
    }

    fn cast_to_float(&mut self, value: &'b TExpr<'b>) -> &'b mut TExpr<'b> {
        return self.buckets.add(TExpr::Call {
            callee_uid: FLOAT_IDX,
            arguments: ref_to_slice(value),
            type_: Type::Float,
        });
    }
}
