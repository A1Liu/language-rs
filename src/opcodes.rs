use crate::syntax_tree::*;

pub enum Opcode {
    Add(u32, u32),
    Call(u32),
}
