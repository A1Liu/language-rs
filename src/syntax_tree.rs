use crate::util::CRange;

#[derive(Debug)]
pub enum ExprTag<'a> {
    Int(u64),
    Float(f64),
    Ident {
        id: u32,
    },
    Tup(&'a mut [Expr<'a>]),
    Call {
        callee: u32,
        arguments: &'a mut [Expr<'a>],
    },
    DotAccess {
        parent: &'a mut Expr<'a>,
        member_id: u32,
    },
    Add(&'a mut Expr<'a>, &'a mut Expr<'a>),
}

#[derive(Debug)]
pub struct Expr<'a> {
    pub tag: ExprTag<'a>,
    pub view: CRange,
}

#[derive(Debug)]
pub struct FuncParam {
    pub name: u32,
    pub type_name: u32,
}

#[derive(Debug)]
pub enum Stmt<'a> {
    Pass,
    Expr(&'a mut Expr<'a>),
    Declare {
        name: u32,
        name_view: CRange,
        type_name: u32,
        type_view: CRange,
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
