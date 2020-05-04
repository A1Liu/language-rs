use crate::type_checker::Type;
use crate::util::Buckets;
use std::collections::HashMap;

pub const ECALL_IDX: u32 = 0;
pub const FLOAT_IDX: u32 = 1;
pub const INT_IDX: u32 = 2;

pub fn builtin_names<'a>() -> (Vec<&'a str>, HashMap<&'a str, u32>) {
    let names = vec!["ecall", "float", "int"];
    let mut names_map = HashMap::new();
    for (idx, name) in names.iter().enumerate() {
        names_map.insert(*name, idx as u32);
    }

    return (names, names_map);
}

pub fn builtin_symbols<'a, 'b>(buckets: &'b mut Buckets<'a>) -> HashMap<u32, &'a Type<'a>> {
    let mut map = HashMap::new();
    let int_arg = &*buckets.add_array(vec![Type::Int, Type::Any]);
    let none = &*buckets.add(Type::None);
    let ecall_type = &*buckets.add(Type::Function {
        return_type: none,
        arguments: int_arg,
    });

    map.insert(ECALL_IDX, ecall_type);
    return map;
}

pub fn builtin_types<'a, 'b>(buckets: &'b mut Buckets<'a>) -> HashMap<u32, &'a Type<'a>> {
    let mut map = HashMap::new();
    map.insert(FLOAT_IDX, &*buckets.add(Type::Float));
    map.insert(INT_IDX, &*buckets.add(Type::Int));
    return map;
}
