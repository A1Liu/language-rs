use std::ops::Range;

#[derive(Debug)]
pub enum ExprTag<'a> {
    None,
    Int(u64),
    Float(f64),
    Ident(usize),
    Tup(Option<&'a [Expr<'a>]>),
    Add(&'a Expr<'a>, &'a Expr<'a>),
    Sub(&'a Expr<'a>, &'a Expr<'a>),
    Mul(&'a Expr<'a>, &'a Expr<'a>),
    Div(&'a Expr<'a>, &'a Expr<'a>),
    Mod(&'a Expr<'a>, &'a Expr<'a>),
    Leq(&'a Expr<'a>, &'a Expr<'a>),
    Geq(&'a Expr<'a>, &'a Expr<'a>),
    Lt(&'a Expr<'a>, &'a Expr<'a>),
    Gt(&'a Expr<'a>, &'a Expr<'a>),
    Eq(&'a Expr<'a>, &'a Expr<'a>),
    Neq(&'a Expr<'a>, &'a Expr<'a>),
}

pub enum InferredType {
    Unknown,
}

#[derive(Debug)]
pub struct Expr<'a> {
    pub tag: ExprTag<'a>,
    pub view: Range<u32>,
}

#[derive(Debug)]
pub enum Stmt<'a> {
    End,
    Pass,
    Expr(Expr<'a>),
    Assign { to: Expr<'a>, value: Expr<'a> },
}
