use crate::syntax_tree::InferredType;
use crate::util::Buckets;
use std::collections::HashMap;

pub const PRINT_IDX: u32 = 0;
pub const FLOAT_IDX: u32 = 1;
pub const INT_IDX: u32 = 2;
pub const NONE_IDX: u32 = 3;
pub const TUPLE_IDX: u32 = 4;

pub fn builtin_names<'a>() -> (Vec<&'a str>, HashMap<&'a str, u32>) {
    let names = vec![
        "print", "float", "int", "None", "tuple", "c0", "c1", "c2", "c3", "c4", "c5", "c6", "c7",
        "c8", "c9",
    ];
    let mut names_map = HashMap::new();
    for (idx, name) in names.iter().enumerate() {
        names_map.insert(*name, idx as u32);
    }

    return (names, names_map);
}

pub fn builtin_symbols<'a, 'b>(buckets: &'b mut Buckets<'a>) -> HashMap<u32, &'a InferredType<'a>> {
    let mut map = HashMap::new();
    let none_type = &*buckets.add(InferredType::None);
    let any_arg = &*buckets.add_array(vec![InferredType::Any]);
    let print_type = &*buckets.add(InferredType::Function {
        return_type: none_type,
        arguments: any_arg,
    });
    let float = &*buckets.add(InferredType::Float);
    let float_type = &*buckets.add(InferredType::Function {
        return_type: float,
        arguments: any_arg,
    });

    map.insert(PRINT_IDX, print_type);
    map.insert(NONE_IDX, none_type);
    map.insert(FLOAT_IDX, float_type);
    return map;
}

pub fn builtin_types<'a, 'b>(buckets: &'b mut Buckets<'a>) -> HashMap<u32, &'a InferredType<'a>> {
    let mut map = HashMap::new();
    map.insert(FLOAT_IDX, &*buckets.add(InferredType::Float));
    map.insert(INT_IDX, &*buckets.add(InferredType::Int));
    return map;
}

#[inline]
pub fn tuple_component(idx: u32) -> u32 {
    return idx + 5;
}
