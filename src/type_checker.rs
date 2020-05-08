use crate::builtins::*;
use crate::syntax_tree::*;
use crate::util::*;
use std::collections::HashMap;

pub struct TypeChecker<'a, 'b>
where
    'b: 'a,
{
    next_uid_: u32,
    buckets: &'a mut Buckets<'b>,
    types: HashMap<u32, &'b Type<'b>>,
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
        };
    }

    pub fn next_uid(&mut self) -> u32 {
        let uid = self.next_uid_;
        self.next_uid_ += 1;
        return uid;
    }

    pub fn check_program(&mut self, program: &[Stmt]) -> Result<&[TStmt<'b>], Error<'b>> {
        let type_table = builtin_types(self.buckets);
        let symbol_table = builtin_symbols(self.buckets);
        self.types = type_table;
        self.next_uid_ = UID_BEGIN;

        let sym = SymbolTable::new_global(symbol_table);

        let (mut sym, mut tstmts) = self.check_stmts(program, sym, None)?;

        let mut program = Vec::new();
        for (symbol, info) in sym.symbols.drain() {
            println!("{:?}", info);
            program.push(TStmt::Declare {
                uid: info.uid(),
                decl_type: self.buckets.add(info.get_type()),
            });
        }

        program.append(&mut tstmts);
        program.append(&mut builtin_definitions(self.buckets));

        return Ok(self.buckets.add_array(program));
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
                    let expr = self.check_expr(&sym, expr)?;
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
                    let ret_val = self.check_expr(&sym, ret_val)?;

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

                    let expr = self.check_expr(&sym, value)?;
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

                    let expr = self.check_expr(&sym, value)?;
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

                    let mut fsym = SymbolTable::new(&sym);
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

                    let (mut fsym, mut fblock) =
                        self.check_stmts(stmts, fsym, Some(*return_type))?;
                    let mut fstmts = Vec::new();

                    for (uid, info) in fsym.symbols.drain() {
                        fstmts.push(TStmt::Declare {
                            uid,
                            decl_type: self.buckets.add(info.get_type()),
                        });
                    }

                    fstmts.append(&mut fblock);

                    let fstmts = self.buckets.add_array(fstmts);
                    let argument_uids = self.buckets.add_array(argument_uids);

                    tstmts.push(TStmt::Function {
                        uid,
                        argument_types: arg_types,
                        argument_uids,
                        return_type,
                        stmts: fstmts,
                    });
                }
                Stmt::If {
                    conditioned_blocks,
                    else_branch,
                } => {
                    let mut sym_tables = Vec::new();
                    let mut ifstmts = Vec::new();
                    for conditioned_block in conditioned_blocks.iter() {
                        let condition = self.check_expr(&sym, &conditioned_block.condition)?;

                        let (ifsym, tblock) = self.check_stmts(
                            conditioned_block.block,
                            SymbolTable::new(&sym),
                            return_type,
                        )?;
                        sym_tables.push(ifsym);
                        ifstmts.push((condition, tblock));
                    }

                    let (else_sym, else_block) =
                        self.check_stmts(else_branch, SymbolTable::new(&sym), return_type)?;

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

    fn check_expr(&mut self, sym: &SymbolTable<'b>, expr: &Expr) -> Result<TExpr<'b>, Error<'b>> {
        match expr {
            Expr::Int { value, view } => {
                return Ok(TExpr::Int(*value as i64));
            }
            Expr::Float { value, view } => {
                return Ok(TExpr::Float(*value));
            }
            Expr::Ident { id, view } => {
                let var_info = unwrap_err(sym.search(*id), *view, "referenced name doesn't exist")?;

                let (uid, type_) = match var_info {
                    SymbolInfo::Variable { uid, type_, .. } => (uid, type_),
                    SymbolInfo::Function { .. } => panic!("we don't have function objects yet"),
                };

                return Ok(TExpr::Ident { uid, type_: *type_ });
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
