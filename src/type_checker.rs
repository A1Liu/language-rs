use crate::builtins::*;
use crate::syntax_tree::Type;
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
        view: CRange,
    },
    Variable {
        uid: u32,
        type_: &'a Type<'a>,
        view: CRange,
    },
}

impl<'a> SymbolInfo<'a> {
    pub fn get_type(&self) -> Type<'a> {
        return match self {
            SymbolInfo::Function {
                return_type,
                arguments,
                ..
            } => Type::Function {
                return_type,
                arguments,
            },
            SymbolInfo::Variable { type_, .. } => **type_,
        };
    }

    pub fn view(&self) -> CRange {
        use SymbolInfo::*;
        return match self {
            Function { view, .. } => *view,
            Variable { view, .. } => *view,
        };
    }

    pub fn uid(&self) -> u32 {
        return match self {
            SymbolInfo::Function { uid, .. } => *uid,
            SymbolInfo::Variable { uid, .. } => *uid,
        };
    }
}

pub fn symbols_<'a>(parent: &SymbolTable<'a>) -> SymbolTable<'a> {
    return SymbolTable {
        symbols: HashMap::new(),
        parent: Some(NonNull::from(parent)),
    };
}

pub struct SymbolTable<'a> {
    pub symbols: HashMap<u32, SymbolInfo<'a>>,
    parent: Option<NonNull<SymbolTable<'a>>>,
}

impl<'a> SymbolTable<'a> {
    pub fn new_global(symbols: HashMap<u32, SymbolInfo<'a>>) -> Self {
        return Self {
            symbols,
            parent: None,
        };
    }

    pub fn fold_into_parent(mut self) -> Result<(), Error<'static>> {
        for (symbol, info) in self.symbols.drain() {
            unsafe { self.parent.unwrap().as_mut() }.declare(symbol, info)?;
        }
        return Ok(());
    }

    pub fn merge_parallel_tables<'b>(
        mut left: SymbolTable<'b>,
        mut right: SymbolTable<'b>,
    ) -> Result<SymbolTable<'b>, Error<'static>> {
        assert!(left.parent == right.parent && left.parent != None);
        let mut result = symbols_(unsafe { left.parent.unwrap().as_mut() });
        for (id, info) in left.symbols.drain() {
            if let Some(rinfo) = right.symbols.remove(&id) {
                if info.get_type() != rinfo.get_type() {
                    return err(info.view(), "variable type differs from other variable type in parallel scope with same name");
                }
            }
            result.declare(id, info)?;
        }

        for (id, info) in right.symbols.drain() {
            result.declare(id, info)?;
        }

        return Ok(result);
    }

    pub fn declare(&mut self, symbol: u32, info: SymbolInfo<'a>) -> Result<(), Error<'static>> {
        if self.symbols.contains_key(&symbol) {
            return err(info.view(), "name already exists in scope");
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
    next_uid_: u32,
    buckets: &'a mut Buckets<'b>,
    types: HashMap<u32, &'b Type<'b>>,
    warnings: Vec<Error<'b>>,
}

impl<'a, 'b> TypeChecker<'a, 'b>
where
    'b: 'a,
{
    pub fn new(buckets: &'a mut Buckets<'b>) -> Self {
        return Self {
            next_uid_: 0,
            buckets,
            types: HashMap::new(),
            warnings: Vec::new(),
        };
    }

    pub fn next_uid(&mut self) -> u32 {
        let uid = self.next_uid_;
        self.next_uid_ += 1;
        return uid;
    }

    pub fn check_program(&mut self, program: &[Stmt]) -> Result<TProgram<'b>, Error<'b>> {
        let type_table = builtin_types(self.buckets);
        let symbol_table = builtin_symbols(self.buckets);
        self.types = type_table;
        self.next_uid_ = UID_BEGIN;

        let sym = SymbolTable::new_global(symbol_table);

        let (mut sym, mut tstmts) = self.check_stmts(false, program, sym, None)?;

        let declarations = sym
            .symbols
            .drain()
            .map(|info| Declaration { uid: info.1.uid() })
            .collect();
        let declarations = self.buckets.add_array(declarations);

        tstmts.append(&mut builtin_definitions(self.buckets));
        let tstmts = self.buckets.add_array(tstmts);
        return Ok(TProgram {
            declarations,
            stmts: tstmts,
        });
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
                            "type doesn't exist",
                        )?;
                    } else {
                        decl_return_type = Type::None;
                    }

                    let decl_return_type = self.buckets.add(decl_return_type);

                    let mut arg_types = Vec::new();
                    for arg in arguments.iter() {
                        let arg_type = **unwrap_err(
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
                            uid: self.next_uid(),
                            return_type: decl_return_type,
                            arguments: arg_types,
                            view: *name_view,
                        },
                    )?;
                }
                _ => {}
            }
        }
        return Ok(());
    }

    fn check_stmts(
        &mut self,
        in_loop: bool,
        stmts: &[Stmt],
        mut sym: SymbolTable<'b>,
        return_type: Option<Type<'b>>,
    ) -> Result<(SymbolTable<'b>, Vec<TStmt<'b>>), Error<'b>> {
        self.add_function_symbols(&mut sym, stmts)?;
        let mut tstmts = Vec::new();
        for stmt in stmts {
            match stmt {
                Stmt::Pass => {}
                Stmt::Expr(expr) => {
                    let expr = self.check_expr(&mut sym, expr)?;
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
                    let ret_val = self.check_expr(&mut sym, ret_val)?;

                    if !Self::is_assignment_compatible(return_type, ret_val.type_()) {
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

                    let uid = self.next_uid();

                    sym.declare(
                        *name,
                        SymbolInfo::Variable {
                            type_: decl_type,
                            view: *name_view,
                            uid,
                        },
                    )?;

                    let expr = self.check_expr(&mut sym, value)?;
                    if !Self::is_assignment_compatible(*decl_type, expr.type_()) {
                        return err(value.view(), "value type doesn't match variable type");
                    }

                    let expr = self.buckets.add(expr);

                    tstmts.push(TStmt::Assign { uid, value: expr });
                }
                Stmt::Assign { to, to_view, value } => {
                    let var_info = unwrap_err(sym.search(*to), *to_view, "name doesn't exist")?;

                    let to_type;
                    let to_uid;
                    if let SymbolInfo::Variable { uid, type_, .. } = var_info {
                        to_type = *type_;
                        to_uid = uid;
                    } else {
                        return err(*to_view, "name being assigned to is a function");
                    }

                    let expr = self.check_expr(&mut sym, value)?;
                    if !Self::is_assignment_compatible(to_type, expr.type_()) {
                        return err(value.view(), "value type doesn't match variable type");
                    }

                    let expr = self.buckets.add(expr);

                    tstmts.push(TStmt::Assign {
                        uid: to_uid,
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
                        view,
                    } = sym.search(*name).unwrap()
                    {
                        uid = id;
                        return_type = rtype;
                        arg_types = arguments;
                    } else {
                        panic!();
                    }

                    let mut fsym = symbols_(&sym);
                    let mut argument_uids = Vec::new();

                    for (arg, arg_type) in arguments.iter().zip(arg_types) {
                        let uid = self.next_uid();

                        argument_uids.push(uid);
                        fsym.declare(
                            arg.name,
                            SymbolInfo::Variable {
                                type_: arg_type,
                                uid,
                                view: arg.view,
                            },
                        )?;
                    }

                    let (mut fsym, fblock) =
                        self.check_stmts(in_loop, stmts, symbols_(&fsym), Some(*return_type))?;
                    let fdecls = fsym
                        .symbols
                        .drain()
                        .map(|(_, info)| Declaration { uid: info.uid() })
                        .collect();

                    let fdecls = self.buckets.add_array(fdecls);

                    let fblock = self.buckets.add_array(fblock);
                    let argument_uids = self.buckets.add_array(argument_uids);

                    tstmts.push(TStmt::Function {
                        uid,
                        argument_uids,
                        declarations: fdecls,
                        stmts: fblock,
                    });
                }
                Stmt::While {
                    condition,
                    block,
                    else_branch,
                } => {
                    let condition = self.check_expr(&mut sym, condition)?;
                    let (while_sym, block) =
                        self.check_stmts(true, block, symbols_(&sym), return_type)?;
                    while_sym.fold_into_parent()?;
                    let (else_sym, else_block) =
                        self.check_stmts(in_loop, else_branch, symbols_(&sym), return_type)?;
                    else_sym.fold_into_parent()?;

                    let condition = self.buckets.add(condition);
                    let block = self.buckets.add_array(block);
                    let else_block = self.buckets.add_array(else_block);

                    tstmts.push(TStmt::While {
                        condition,
                        block,
                        else_block,
                    });
                }
                Stmt::Break => {
                    tstmts.push(TStmt::Break);
                }
                Stmt::If {
                    conditioned_blocks,
                    else_branch,
                } => {
                    let mut sym_tables = Vec::new();
                    let mut ifstmts = Vec::new();
                    for conditioned_block in conditioned_blocks.iter() {
                        let condition = self.check_expr(&mut sym, &conditioned_block.condition)?;
                        let block = &conditioned_block.block;

                        let (ifsym, tblock) =
                            self.check_stmts(in_loop, block, symbols_(&sym), return_type)?;
                        sym_tables.push(ifsym);
                        ifstmts.push((condition, tblock));
                    }

                    let (else_sym, else_block) =
                        self.check_stmts(in_loop, else_branch, symbols_(&sym), return_type)?;

                    sym_tables.push(else_sym);

                    // @Performance this could be faster, right now it's quadratic
                    while sym_tables.len() > 1 {
                        let left = sym_tables.pop().unwrap();
                        let right = sym_tables.pop().unwrap();
                        sym_tables.push(SymbolTable::merge_parallel_tables(left, right)?);
                    }

                    sym_tables.pop().unwrap().fold_into_parent()?;

                    let mut if_false = self.buckets.add_array(else_block);
                    for (condition, block) in ifstmts.into_iter().rev() {
                        let condition = self.buckets.add(condition);
                        let if_true = self.buckets.add_array(block);
                        if_false = self.buckets.add_array(vec![TStmt::If {
                            condition,
                            if_true,
                            if_false,
                        }]);
                    }
                    tstmts.push(if_false[0]);
                }
                _ => panic!(),
            }
        }

        return Ok((sym, tstmts));
    }

    fn check_expr(
        &mut self,
        sym: &mut SymbolTable<'b>,
        expr: &Expr,
    ) -> Result<TExpr<'b>, Error<'b>> {
        match expr {
            Expr::Int { value, view } => {
                return Ok(TExpr::Int(*value as i64));
            }
            Expr::Float { value, view } => {
                return Ok(TExpr::Float(*value));
            }
            Expr::None(view) => {
                return Ok(TExpr::None);
            }
            Expr::False(view) => {
                return Ok(TExpr::Bool(false));
            }
            Expr::True(view) => {
                return Ok(TExpr::Bool(true));
            }
            Expr::Ident { id, view } => {
                let var_info = unwrap_err(sym.search(*id), *view, "referenced name doesn't exist")?;

                let (uid, type_) = match var_info {
                    SymbolInfo::Variable { uid, type_, .. } => (uid, *type_),
                    SymbolInfo::Function { .. } => panic!("we don't have function objects yet"),
                };

                return Ok(TExpr::Ident { uid, type_ });
            }
            Expr::Minus { left, right, view } => {
                let left = self.check_expr(sym, left)?;
                let right = self.check_expr(sym, right)?;
                let left = self.buckets.add(left);
                let right = self.buckets.add(right);

                if left.type_() == right.type_() {
                    return Ok(TExpr::Minus {
                        left,
                        right,
                        type_: left.type_(),
                    });
                } else {
                    return err(*view, "incompatible types for addition");
                }
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
                        type_: left.type_(),
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
                    ..
                } = var_info
                {
                    let mut args = Vec::new();
                    if args_formal.len() != arguments.len() {
                        return err(*arguments_view, "wrong number of arguments");
                    }

                    for (formal, arg) in args_formal.iter().zip(arguments.iter()) {
                        let view = arg.view();
                        let arg = self.check_expr(sym, arg)?;
                        if !Self::is_assignment_compatible(*formal, arg.type_()) {
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

    fn is_assignment_compatible(to: Type<'b>, value: Type<'b>) -> bool {
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
}
