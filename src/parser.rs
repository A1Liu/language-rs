use crate::lexer::*;
use crate::syntax_tree::*;
use crate::util::*;
use std::ptr;

pub struct Parser<'a, 'b>
where
    'b: 'a,
{
    buckets: &'a mut Buckets<'b>,
    pub lexer: Lexer<'a>,
    token: Token,
    token2: Token,
}

impl<'a, 'b> Parser<'a, 'b>
where
    'b: 'a,
{
    pub fn new(buckets: &'a mut Buckets<'b>, data: &'a str) -> Self {
        let mut lexer = Lexer::new(data);
        let token = lexer.next();
        let token2 = lexer.next();

        return Parser {
            buckets,
            lexer,
            token,
            token2,
        };
    }

    fn peek(&self) -> Token {
        return self.token;
    }

    fn peek2(&self) -> Token {
        return self.token2;
    }

    fn pop(&mut self) -> Token {
        let prev_token = self.token;
        self.token = self.token2;
        self.token2 = self.lexer.next();
        return prev_token;
    }

    pub fn try_parse_program(&mut self) -> Result<Vec<Stmt<'b>>, Error<'b>> {
        let mut stmts = Vec::new();
        while match self.peek() {
            Token::End(_) => false,
            _ => true,
        } {
            stmts.push(self.try_parse_stmt()?);
        }
        return Ok(stmts);
    }

    pub fn try_parse_stmt(&mut self) -> Result<Stmt<'b>, Error<'b>> {
        use Token::*;
        let tok = self.peek();
        match self.peek() {
            p @ Pass(_) => {
                self.pop();
                match self.pop() {
                    Newline(_) => return Ok(Stmt::Pass),
                    _ => {
                        return Err(Error {
                            location: p.view(),
                            message: "pass needs to end in a newline",
                        })
                    }
                }
            }
            Def(_) => return self.try_parse_func(),
            _ => {}
        }

        if let Ident { id, view } = self.peek() {
            if let Colon(cloc) = self.peek2() {
                self.pop();
                self.pop();
                let type_ident;
                let type_view;
                let tok = self.pop();
                if let Ident { id, view } = tok {
                    type_ident = id;
                    type_view = view;
                } else {
                    return Err(Error {
                        location: tok.view(),
                        message: "type needs to be identifier",
                    });
                }

                match self.pop() {
                    Equal(_) => {}
                    x => {
                        return Err(Error {
                            location: x.view(),
                            message: "expected equal sign after variable declaration",
                        })
                    }
                }

                let expr = self.try_parse_expr()?;
                match self.pop() {
                    Newline(_) => {
                        let expr = self.buckets.add(expr);
                        return Ok(Stmt::Declare {
                            name: id,
                            name_view: view,
                            type_name: type_ident,
                            type_view,
                            value: expr,
                        });
                    }
                    x => {
                        return Err(Error {
                            location: joinr(tok.view(), x.view()),
                            message: "statement needs to end in a newline",
                        })
                    }
                }
            }
        }

        let expr = self.try_parse_expr()?;
        let expr = self.buckets.add(expr);
        match self.pop() {
            Newline(loc2) => return Ok(Stmt::Expr(expr)),
            Equal(_) => match &mut expr.tag {
                ExprTag::Ident { id } => {
                    let value = self.try_parse_expr()?;
                    let value = self.buckets.add(value);
                    match self.pop() {
                        Newline(_) => {}
                        x => {
                            return Err(Error {
                                location: joinr(expr.view, x.view()),
                                message: "statement needs to end in a newline",
                            })
                        }
                    }
                    return Ok(Stmt::Assign {
                        to: *id,
                        to_loc: expr.view.start,
                        value,
                    });
                }
                ExprTag::DotAccess { parent, member_id } => {
                    let value = self.try_parse_expr()?;
                    let value = self.buckets.add(value);
                    return Ok(Stmt::AssignMember {
                        to: *parent,
                        to_member: 0,
                        value,
                    });
                }
                x => {
                    return Err(Error {
                        location: expr.view,
                        message: "assignment can only happen to member accessors or names",
                    })
                }
            },
            x => {
                return Err(Error {
                    location: joinr(expr.view, x.view()),
                    message: "statement needs to end in a newline",
                })
            }
        }
    }

    pub fn try_parse_func(&mut self) -> Result<Stmt<'b>, Error<'b>> {
        match self.pop() {
            Token::Def(_) => {}
            _ => panic!(),
        }

        let def_name;
        let def_view;
        match self.pop() {
            Token::Ident { id, view } => {
                def_name = id;
                def_view = view;
            }
            x => {
                return Err(Error {
                    location: x.view(),
                    message: "unexpected token when parsing function arguments",
                });
            }
        }

        match self.pop() {
            Token::LParen(_) => {}
            x => {
                return Err(Error {
                    location: x.view(),
                    message: "unexpected token when parsing function arguments",
                });
            }
        }

        let mut args = Vec::new();
        loop {
            let arg_name;
            match self.pop() {
                Token::RParen(_) => {
                    break;
                }
                Token::Ident { id, view } => {
                    arg_name = id;
                }
                x => {
                    return Err(Error {
                        location: x.view(),
                        message: "unexpected token when parsing function arguments",
                    });
                }
            }

            match self.pop() {
                Token::Colon(_) => {}
                x => {
                    return Err(Error {
                        location: x.view(),
                        message: "unexpected token when parsing function arguments",
                    });
                }
            }

            let type_name;
            match self.pop() {
                Token::Ident { id, view } => {
                    type_name = id;
                }
                x => {
                    return Err(Error {
                        location: x.view(),
                        message: "unexpected token when parsing function arguments",
                    })
                }
            }

            args.push(FuncParam {
                name: arg_name,
                type_name,
            });

            match self.pop() {
                Token::RParen(_) => {
                    break;
                }
                Token::Comma(_) => {}
                x => {
                    return Err(Error {
                        location: x.view(),
                        message: "unexpected token when parsing function arguments",
                    });
                }
            }
        }

        let arguments = self.buckets.add_array(args);

        match self.pop() {
            Token::Colon(_) => {}
            x => {
                return Err(Error {
                    location: x.view(),
                    message: "unexpected token when parsing function signature",
                });
            }
        }

        match self.pop() {
            Token::Newline(_) => {}
            x => {
                return Err(Error {
                    location: x.view(),
                    message: "unexpected token when parsing function signature",
                });
            }
        }

        match self.pop() {
            Token::Indent { begin, end } => {}
            x => {
                return Err(Error {
                    location: x.view(),
                    message: "unexpected token when parsing function signature",
                });
            }
        }

        let mut stmts = Vec::new();
        while match self.peek() {
            Token::Dedent(_) => false,
            _ => true,
        } {
            stmts.push(self.try_parse_stmt()?);
        }

        let stmts = self.buckets.add_array(stmts);

        match self.pop() {
            Token::Dedent(_) => {}
            x => {
                return Err(Error {
                    location: x.view(),
                    message: "unexpected token when parsing function dedent",
                });
            }
        }

        let function = Stmt::Function {
            name: def_name,
            name_loc: def_view.start,
            arguments,
            stmts,
        };
        return Ok(function);
    }

    pub fn try_parse_expr(&mut self) -> Result<Expr<'b>, Error<'b>> {
        return self.try_parse_expr_add();
    }

    pub fn try_parse_expr_add(&mut self) -> Result<Expr<'b>, Error<'b>> {
        use Token::*;
        let mut expr = self.try_parse_unary_postfix()?;
        loop {
            match self.peek() {
                Plus(loc) => {
                    self.pop();
                    let op = self.buckets.add(expr);
                    let op2 = self.try_parse_unary_postfix()?;
                    let op2 = self.buckets.add(op2);
                    let view = joinr(op.view, op2.view);
                    expr = Expr {
                        tag: ExprTag::Add(op, op2),
                        view,
                    };
                }
                _ => return Ok(expr),
            }
        }
    }

    pub fn try_parse_unary_postfix(&mut self) -> Result<Expr<'b>, Error<'b>> {
        let mut expr = self.try_parse_expr_atom()?;

        loop {
            if let Token::Dot(begin) = self.peek() {
                self.pop();
                match self.pop() {
                    Token::Ident { id, view } => {
                        let parent = self.buckets.add(expr);
                        let start = parent.view.start;

                        expr = Expr {
                            tag: ExprTag::DotAccess {
                                parent,
                                member_id: id,
                            },
                            view,
                        }
                    }
                    x => {
                        return Err(Error {
                            location: x.view(),
                            message: "expected identifier after dot",
                        })
                    }
                }
            } else {
                return Ok(expr);
            }
        }
    }

    pub fn try_parse_expr_atom(&mut self) -> Result<Expr<'b>, Error<'b>> {
        use Token::*;
        match self.peek() {
            Ident { id, view } => {
                self.pop();
                if let LParen(tup_begin) = self.peek() {
                    let arguments = self.try_parse_expr_tup()?;
                    let expr;
                    if let Expr {
                        tag: ExprTag::Tup(slice),
                        view: eview,
                    } = arguments
                    {
                        expr = Expr {
                            tag: ExprTag::Call {
                                callee: id,
                                arguments: slice,
                            },
                            view: joinr(view, eview),
                        };
                    } else {
                        let aview = arguments.view;
                        expr = Expr {
                            tag: ExprTag::Call {
                                callee: id,
                                arguments: self.buckets.add_array(vec![arguments]),
                            },
                            view: joinr(view, aview),
                        };
                    }
                    return Ok(expr);
                } else {
                    return Ok(Expr {
                        tag: ExprTag::Ident { id },
                        view,
                    });
                }
            }
            FloatingPoint { value, begin, end } => {
                self.pop();
                return Ok(Expr {
                    tag: ExprTag::Float(value),
                    view: newr(begin, end.get()),
                });
            }
            Integer { value, begin, end } => {
                self.pop();
                return Ok(Expr {
                    tag: ExprTag::Int(value),
                    view: newr(begin, end.get()),
                });
            }
            LParen(tup_begin) => {
                let tup = self.try_parse_expr_tup()?;
                let slice = match &tup.tag {
                    ExprTag::Tup(slice) => slice,
                    _ => panic!(),
                };

                if slice.len() == 1 {
                    return Ok(unsafe { ptr::read(&slice[0]) });
                } else {
                    return Ok(tup);
                }
            }
            x => {
                return Err(Error {
                    location: x.view(),
                    message: "unexpected token while parsing expression",
                });
            }
        }
    }

    pub fn try_parse_expr_tup(&mut self) -> Result<Expr<'b>, Error<'b>> {
        use Token::*;
        let tup_begin = match self.pop() {
            LParen(tup_begin) => tup_begin,
            _ => panic!(),
        };

        match self.peek() {
            RParen(tup_end) => {
                self.pop();
                return Ok(Expr {
                    tag: ExprTag::Tup(self.buckets.add_array(Vec::new())),
                    view: newr(tup_begin, tup_end + 1),
                });
            }
            _ => {}
        }

        let expr = self.try_parse_expr()?;
        let mut exprs = vec![expr];
        let mut tok = self.pop();

        while match tok {
            Comma(_) => true,
            _ => false,
        } {
            if match self.peek() {
                RParen(_) => true,
                _ => false,
            } {
                tok = self.pop();
                break;
            }

            exprs.push(self.try_parse_expr_add()?);
            tok = self.pop();
        }

        let tup_end = match tok {
            RParen(end) => end + 1,
            _ => {
                return Err(Error {
                    location: tok.view(),
                    message: "expected ')' character",
                })
            }
        };

        return Ok(Expr {
            tag: ExprTag::Tup(self.buckets.add_array(exprs)),
            view: newr(tup_begin, tup_end),
        });
    }
}
