use crate::runtime::*;
use crate::syntax_tree::*;
use crate::type_checker::*;
use crate::util::*;
use std::collections::HashMap;

pub const PRINT_IDX: u32 = 0;
pub const FLOAT_IDX: u32 = 1;
pub const INT_IDX: u32 = 2;
pub const BOOL_IDX: u32 = 3;
pub const STR_IDX: u32 = 4;
pub const UID_BEGIN: u32 = 10;

pub fn builtin_names<'a>() -> (Vec<&'a str>, HashMap<&'a str, u32>) {
    let names = vec!["print", "float", "int", "bool", "str"];
    let mut names_map = HashMap::new();
    for (idx, name) in names.iter().enumerate() {
        names_map.insert(*name, idx as u32);
    }

    return (names, names_map);
}

pub fn builtin_symbols<'a, 'b>(buckets: &'b mut Buckets<'a>) -> HashMap<u32, SymbolInfo<'a>> {
    let mut map = HashMap::new();
    let none_type = &*buckets.add(Type::None);
    let any_arg = &*buckets.add_array(vec![Type::Any]);
    let print_type = &*buckets.add(Type::Function {
        return_type: none_type,
        arguments: any_arg,
    });
    map.insert(
        PRINT_IDX,
        SymbolInfo::Function {
            uid: 1,
            return_type: none_type,
            arguments: any_arg,
            view: newr(0, 0),
        },
    );
    return map;
}

pub fn builtin_definitions<'a, 'b>(buckets: &'b mut Buckets<'a>) -> Vec<TStmt<'a>> {
    let mut defns = Vec::new();

    let none_type = buckets.add(Type::None);
    let int_type = buckets.add(Type::Int);
    let any_arg = buckets.add_array(vec![Type::Any]);

    let ecall_args = buckets.add_array(vec![
        TExpr::Int(PRINT_PRIMITIVE as i64),
        TExpr::Ident {
            uid: 2,
            type_: Type::Any,
        },
    ]);

    let ecall_expr = buckets.add(TExpr::ECall {
        arguments: ecall_args,
    });

    let uids = buckets.add_array(vec![2]);

    let stmts = buckets.add_array(vec![TStmt::Expr(ecall_expr)]);

    defns.push(TStmt::Function {
        uid: 1,
        argument_uids: uids,
        declarations: buckets.add_array(vec![]),
        stmts,
    });
    return defns;
}

pub fn builtin_types<'a, 'b>(buckets: &'b mut Buckets<'a>) -> HashMap<u32, &'a Type<'a>> {
    let mut map = HashMap::new();
    map.insert(FLOAT_IDX, &*buckets.add(Type::Float));
    map.insert(INT_IDX, &*buckets.add(Type::Int));
    map.insert(BOOL_IDX, &*buckets.add(Type::Bool));
    return map;
}
