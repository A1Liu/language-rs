use crate::util::*;

#[derive(Debug)]
pub enum Expr<'a> {
    Int {
        value: u64,
        view: CRange,
    },
    Float {
        value: f64,
        view: CRange,
    },
    None(CRange),
    True(CRange),
    False(CRange),
    Ident {
        id: u32,
        view: CRange,
    },
    Call {
        callee: u32,
        callee_view: CRange,
        arguments: &'a mut [Expr<'a>],
        arguments_view: CRange,
    },
    DotAccess {
        parent: &'a mut Expr<'a>,
        member_id: u32,
        member_view: CRange,
    },
    Tup {
        values: &'a mut [Expr<'a>],
        view: CRange,
    },
    Minus {
        left: &'a mut Expr<'a>,
        right: &'a mut Expr<'a>,
        view: CRange,
    },
    Add {
        left: &'a mut Expr<'a>,
        right: &'a mut Expr<'a>,
        view: CRange,
    },
}

impl<'a> Expr<'a> {
    pub fn view(&self) -> CRange {
        use Expr::*;
        return match self {
            Int { view, .. } => *view,
            Float { view, .. } => *view,
            Ident { id, view } => *view,
            None(view) => *view,
            True(view) => *view,
            False(view) => *view,
            Call {
                callee,
                callee_view,
                arguments,
                arguments_view,
            } => joinr(*callee_view, *arguments_view),
            DotAccess {
                parent,
                member_view,
                ..
            } => joinr(parent.view(), *member_view),
            Add { view, .. } => *view,
            Minus { view, .. } => *view,
            Tup { view, .. } => *view,
        };
    }
}

#[derive(Debug)]
pub struct FuncParam {
    pub name: u32,
    pub type_name: u32,
    pub view: CRange,
}

#[derive(Debug)]
pub struct IfBranch<'a> {
    pub condition: Expr<'a>,
    pub block: &'a mut [Stmt<'a>],
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
        name_view: CRange,
        return_type_view: CRange,
        arguments: &'a mut [FuncParam],
        return_type: Option<u32>,
        stmts: &'a mut [Stmt<'a>],
    },
    Assign {
        to: u32,
        to_view: CRange,
        value: &'a mut Expr<'a>,
    },
    AssignMember {
        to: &'a mut Expr<'a>,
        to_member: u32,
        value: &'a mut Expr<'a>,
    },
    If {
        conditioned_blocks: &'a mut [IfBranch<'a>],
        else_branch: &'a mut [Stmt<'a>],
    },
    While {
        condition: &'a mut Expr<'a>,
        block: &'a mut [Stmt<'a>],
        else_branch: &'a mut [Stmt<'a>],
    },
    Break,
    Return {
        ret_val: &'a mut Expr<'a>,
    },
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Type<'a> {
    None,
    Any,
    Int,
    Float,
    Bool,
    Function {
        return_type: &'a Type<'a>,
        arguments: &'a [Type<'a>],
    },
}

impl<'a> Type<'a> {
    pub fn is_primitive(&self) -> bool {
        return match self {
            Type::Int | Type::Float => true,
            _ => false,
        };
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TExpr<'a> {
    Ident {
        uid: u32,
        type_: Type<'a>,
    },
    None,
    Int(i64),
    Float(f64),
    Bool(bool),
    Minus {
        left: &'a TExpr<'a>,
        right: &'a TExpr<'a>,
        type_: Type<'a>,
    },
    Add {
        left: &'a TExpr<'a>,
        right: &'a TExpr<'a>,
        type_: Type<'a>,
    },
    Call {
        callee_uid: u32,
        arguments: &'a [TExpr<'a>],
        type_: Type<'a>,
    },
    ECall {
        arguments: &'a [TExpr<'a>],
    },
}

impl<'a> TExpr<'a> {
    pub fn type_(&self) -> Type<'a> {
        use TExpr::*;
        return match self {
            Ident { type_, .. } => *type_,
            Int(_) => Type::Int,
            Float(_) => Type::Float,
            Bool(_) => Type::Bool,
            None => Type::None,
            Minus { type_, .. } => *type_,
            Add { type_, .. } => *type_,
            Call { type_, .. } => *type_,
            ECall { .. } => Type::None,
        };
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Declaration {
    pub uid: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum TStmt<'a> {
    Expr(&'a TExpr<'a>),
    Assign {
        uid: u32,
        value: &'a TExpr<'a>,
    },
    Function {
        uid: u32,
        argument_uids: &'a [u32],
        declarations: &'a [Declaration],
        stmts: &'a [TStmt<'a>],
    },
    If {
        condition: &'a TExpr<'a>,
        if_true: &'a [TStmt<'a>],
        if_false: &'a [TStmt<'a>],
    },
    While {
        condition: &'a TExpr<'a>,
        block: &'a [TStmt<'a>],
        else_block: &'a [TStmt<'a>],
    },
    Break,
    Return {
        ret_val: &'a TExpr<'a>,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct TProgram<'a> {
    pub declarations: &'a [Declaration],
    pub stmts: &'a [TStmt<'a>],
}
