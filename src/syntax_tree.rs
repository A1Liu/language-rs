use std::ops::Range;

#[derive(Debug)]
pub enum ExprTag<'a> {
    None,
    Int(u64),
    Float(f64),
    Ident(u32),
    Tup(&'a [Expr<'a>]),
    Call {
        callee: &'a Expr<'a>,
        arguments: &'a [Expr<'a>],
    },
    DotAccess {
        parent: &'a Expr<'a>,
        member_id: u32,
    },
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

#[derive(Debug, PartialEq, Eq)]
pub enum InferredType<'a> {
    Unknown,
    Int,
    Float,
    Tup(&'a [InferredType<'a>]),
}

#[derive(Debug)]
pub struct Expr<'a> {
    pub tag: ExprTag<'a>,
    pub inferred_type: InferredType<'a>,
    pub view: Range<u32>,
}

#[derive(Debug)]
pub enum Stmt<'a> {
    End,
    Pass,
    Expr(Expr<'a>),
    Assign { to: Expr<'a>, value: Expr<'a> },
}
