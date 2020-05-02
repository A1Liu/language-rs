use std::collections::HashMap;
use std::ops::Range;

#[derive(Debug)]
pub enum ExprTag<'a> {
    Int(u64),
    Float(f64),
    Ident(u32),
    Tup(&'a mut [Expr<'a>]),
    Call {
        callee: &'a mut Expr<'a>,
        arguments: &'a mut [Expr<'a>],
    },
    DotAccess {
        parent: &'a mut Expr<'a>,
        member_id: u32,
    },
    Add(&'a mut Expr<'a>, &'a mut Expr<'a>),
    Sub(&'a mut Expr<'a>, &'a mut Expr<'a>),
    Mul(&'a mut Expr<'a>, &'a mut Expr<'a>),
    Div(&'a mut Expr<'a>, &'a mut Expr<'a>),
    Mod(&'a mut Expr<'a>, &'a mut Expr<'a>),
    Leq(&'a mut Expr<'a>, &'a mut Expr<'a>),
    Geq(&'a mut Expr<'a>, &'a mut Expr<'a>),
    Lt(&'a mut Expr<'a>, &'a mut Expr<'a>),
    Gt(&'a mut Expr<'a>, &'a mut Expr<'a>),
    Eq(&'a mut Expr<'a>, &'a mut Expr<'a>),
    Neq(&'a mut Expr<'a>, &'a mut Expr<'a>),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InferredType<'a> {
    Unknown,
    None,
    Any,
    Int,
    Float,
    Function {
        return_type: &'a InferredType<'a>,
        arguments: &'a [InferredType<'a>],
    },
    Class(&'a HashMap<u32, InferredType<'a>>), // This will leak the buffer that the hashmap stores its data in
}

impl<'a> InferredType<'a> {
    pub fn is_primitive(&self) -> bool {
        return match self {
            InferredType::Int | InferredType::Float => true,
            _ => false,
        };
    }
}

#[derive(Debug)]
pub struct Expr<'a> {
    pub tag: ExprTag<'a>,
    pub inferred_type: InferredType<'a>,
    pub view: Range<u32>,
}

#[derive(Debug)]
pub struct FuncParam {
    pub name: u32,
    pub type_name: u32,
}

#[derive(Debug)]
pub enum Stmt<'a> {
    End,
    Pass,
    Expr(&'a mut Expr<'a>),
    Declare {
        name: u32,
        name_loc: u32,
        type_name: u32,
        value: &'a mut Expr<'a>,
    },
    Function {
        name: u32,
        name_loc: u32,
        arguments: &'a mut [FuncParam],
        stmts: &'a mut [Stmt<'a>],
    },
    Assign {
        to: u32,
        to_loc: u32,
        value: &'a mut Expr<'a>,
    },
    AssignMember {
        to: &'a mut Expr<'a>,
        to_member: u32,
        value: &'a mut Expr<'a>,
    },
}
