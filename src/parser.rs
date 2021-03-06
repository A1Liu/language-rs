use crate::lexer::*;
use crate::syntax_tree::*;
use crate::util::*;
use std::mem;

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
        match self.peek() {
            Break(_) => {
                self.pop();
                self.expect_newline()?;
                return Ok(Stmt::Break);
            }
            Pass(_) => {
                self.pop();
                self.expect_newline()?;
                return Ok(Stmt::Pass);
            }
            Def(_) => return self.try_parse_func(),
            Return(_) => {
                self.pop();
                let expr = self.try_parse_expr()?;
                self.expect_newline()?;
                let expr = self.buckets.add(expr);
                return Ok(Stmt::Return { ret_val: expr });
            }
            While(_) => {
                self.pop();
                let condition = self.try_parse_expr()?;
                self.expect_colon()?;
                self.expect_newline()?;
                let block = self.try_parse_block()?;
                let else_branch = match self.peek() {
                    Else(_) => {
                        self.pop();
                        self.expect_colon()?;
                        self.expect_newline()?;
                        self.try_parse_block()?
                    }
                    _ => self.buckets.add_array(vec![]),
                };

                let condition = self.buckets.add(condition);
                return Ok(Stmt::While {
                    condition,
                    block,
                    else_branch,
                });
            }
            If(_) => {
                self.pop();
                let mut conditioned_blocks = Vec::new();
                let else_branch;
                loop {
                    let condition = self.try_parse_expr()?;
                    self.expect_colon()?;
                    self.expect_newline()?;
                    let block = self.try_parse_block()?;

                    conditioned_blocks.push(IfBranch { condition, block });

                    match self.peek() {
                        Else(_) => {
                            self.pop();
                            self.expect_colon()?;
                            self.expect_newline()?;
                            else_branch = self.try_parse_block()?;
                            break;
                        }
                        Elif(_) => {
                            self.pop();
                        }
                        _ => {
                            else_branch = self.buckets.add_array(vec![]);
                            break;
                        }
                    }
                }

                let conditioned_blocks = self.buckets.add_array(conditioned_blocks);
                return Ok(Stmt::If {
                    conditioned_blocks,
                    else_branch,
                });
            }
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
                self.expect_newline()?;
                let expr = self.buckets.add(expr);
                return Ok(Stmt::Declare {
                    name: id,
                    name_view: view,
                    type_name: type_ident,
                    type_view,
                    value: expr,
                });
            }
        }

        let expr = self.try_parse_expr()?;
        let expr = self.buckets.add(expr);
        match self.pop() {
            Newline(loc2) => return Ok(Stmt::Expr(expr)),
            Equal(_) => match expr {
                Expr::Ident { id, view } => {
                    let value = self.try_parse_expr()?;
                    let value = self.buckets.add(value);
                    match self.pop() {
                        Newline(_) => {}
                        x => {
                            return Err(Error {
                                location: joinr(*view, x.view()),
                                message: "statement needs to end in a newline",
                            })
                        }
                    }
                    return Ok(Stmt::Assign {
                        to: *id,
                        to_view: *view,
                        value,
                    });
                }
                Expr::DotAccess {
                    parent,
                    member_id,
                    member_view,
                } => {
                    let value = self.try_parse_expr()?;
                    let value = self.buckets.add(value);
                    return Ok(Stmt::AssignMember {
                        to: *parent,
                        to_member: 0,
                        value,
                    });
                }
                x => {
                    return err(
                        x.view(),
                        "assignment can only happen to member accessors or names",
                    );
                }
            },
            x => {
                return err(
                    joinr(expr.view(), x.view()),
                    "statement needs to end in a newline",
                );
            }
        }
    }

    fn try_parse_block(&mut self) -> Result<&'b mut [Stmt<'b>], Error<'b>> {
        match self.pop() {
            Token::Indent { begin, end } => {}
            x => {
                return err(x.view(), "unexpected token when parsing function signature");
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
                return err(x.view(), "unexpected token when parsing function dedent");
            }
        }

        return Ok(stmts);
    }

    fn try_parse_func(&mut self) -> Result<Stmt<'b>, Error<'b>> {
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
                return err(x.view(), "unexpected token when parsing function arguments");
            }
        }

        let mut args = Vec::new();
        loop {
            let start;
            let arg_name;
            match self.pop() {
                Token::RParen(_) => {
                    break;
                }
                Token::Ident { id, view } => {
                    arg_name = id;
                    start = view.start;
                }
                x => {
                    return err(x.view(), "unexpected token when parsing function arguments");
                }
            }

            match self.pop() {
                Token::Colon(_) => {}
                x => {
                    return err(x.view(), "unexpected token when parsing function arguments");
                }
            }

            let end;
            let type_name;
            match self.pop() {
                Token::Ident { id, view } => {
                    type_name = id;
                    end = view.end;
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
                view: newr(start, end),
            });

            match self.pop() {
                Token::RParen(_) => {
                    break;
                }
                Token::Comma(_) => {}
                x => {
                    return err(x.view(), "unexpected token when parsing function arguments");
                }
            }
        }

        let arguments = self.buckets.add_array(args);

        let (return_type, return_type_view) = match self.pop() {
            Token::Arrow(_) => {
                let tok = self.pop();
                if let Token::Ident { id, view } = tok {
                    let tok = self.pop();
                    if let Token::Colon(_) = tok {
                        (Some(id), tok.view())
                    } else {
                        return err(tok.view(), "expected ':' character");
                    }
                } else {
                    return err(tok.view(), "expected identifier");
                }
            }
            c @ Token::Colon(_) => (None, c.view()),
            tok => {
                return err(tok.view(), "expected '->' or ':'");
            }
        };

        match self.pop() {
            Token::Newline(_) => {}
            x => {
                return err(x.view(), "expected newline");
            }
        }

        let stmts = self.try_parse_block()?;

        let function = Stmt::Function {
            name: def_name,
            name_view: def_view,
            return_type_view,
            arguments,
            stmts,
            return_type,
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
                Dash(loc) => {
                    self.pop();
                    let left = self.buckets.add(expr);
                    let right = self.try_parse_unary_postfix()?;
                    let right = self.buckets.add(right);
                    let view = joinr(left.view(), right.view());
                    expr = Expr::Minus { left, right, view };
                }
                Plus(loc) => {
                    self.pop();
                    let left = self.buckets.add(expr);
                    let right = self.try_parse_unary_postfix()?;
                    let right = self.buckets.add(right);
                    let view = joinr(left.view(), right.view());
                    expr = Expr::Add { left, right, view };
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
                        let start = parent.view().start;

                        expr = Expr::DotAccess {
                            parent,
                            member_id: id,
                            member_view: view,
                        };
                    }
                    x => {
                        return err(x.view(), "expected identifier after dot");
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
                    if let Expr::Tup {
                        values,
                        view: eview,
                    } = arguments
                    {
                        expr = Expr::Call {
                            callee: id,
                            callee_view: view,
                            arguments: values,
                            arguments_view: eview,
                        };
                    } else {
                        let aview = arguments.view();
                        expr = Expr::Call {
                            callee: id,
                            callee_view: view,
                            arguments: self.buckets.add_array(vec![arguments]),
                            arguments_view: aview,
                        };
                    }
                    return Ok(expr);
                } else {
                    return Ok(Expr::Ident { id, view });
                }
            }
            FloatingPoint { value, begin, end } => {
                self.pop();
                return Ok(Expr::Float {
                    value,
                    view: newr(begin, end.get()),
                });
            }
            Integer { value, begin, end } => {
                self.pop();
                return Ok(Expr::Int {
                    value,
                    view: newr(begin, end.get()),
                });
            }
            n @ None(_) => {
                self.pop();
                return Ok(Expr::None(n.view()));
            }
            t @ False(_) => {
                self.pop();
                return Ok(Expr::False(t.view()));
            }
            t @ True(_) => {
                self.pop();
                return Ok(Expr::True(t.view()));
            }
            String { id, view } => {
                self.pop();
                panic!("not implemented yet");
                // let substr = self.lexer.substr(view.start, view.end);
                // let value = unwrap_err(unescape(substr), view, "invalid escape sequence")?;
                // let value = self.buckets.add_str(&value);
                // return Ok(Expr::StringLiteral { id, value, view });
            }
            LParen(tup_begin) => {
                let mut tup = self.try_parse_expr_tup()?;
                let slice = match &mut tup {
                    Expr::Tup { values, view } => values,
                    _ => panic!(),
                };

                if slice.len() == 1 {
                    let mut e = Expr::None(newr(0, 0));
                    mem::swap(&mut e, &mut slice[0]);
                    return Ok(e);
                } else {
                    return Ok(tup);
                }
            }
            x => {
                return err(x.view(), "unexpected token while parsing expression");
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
                return Ok(Expr::Tup {
                    values: self.buckets.add_array(Vec::new()),
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
            _ => return err(tok.view(), "expected ')' character"),
        };

        return Ok(Expr::Tup {
            values: self.buckets.add_array(exprs),
            view: newr(tup_begin, tup_end),
        });
    }

    pub fn expect_colon(&mut self) -> Result<(), Error<'b>> {
        return match self.pop() {
            Token::Colon(_) => Ok(()),
            x => err(x.view(), "expected ':' character"),
        };
    }

    pub fn expect_newline(&mut self) -> Result<(), Error<'b>> {
        return match self.pop() {
            Token::Newline(_) => Ok(()),
            x => err(x.view(), "expected newline character"),
        };
    }
}
