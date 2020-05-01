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
}

impl<'a, 'b> Parser<'a, 'b>
where
    'b: 'a,
{
    pub fn new(buckets: &'a mut Buckets<'b>, data: &'a str) -> Self {
        let mut lexer = Lexer::new(data);
        let token = lexer.next();

        return Parser {
            buckets,
            lexer,
            token,
        };
    }

    fn peek(&self) -> Token {
        return self.token;
    }

    fn pop(&mut self) -> Token {
        let prev_token = self.token;
        self.token = self.lexer.next();
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
            Pass(loc) => {
                self.pop();
                match self.pop() {
                    Newline(loc2) => return Ok(Stmt::Pass),
                    _ => {
                        return Err(Error {
                            location: loc..(loc + 1),
                            message: "pass needs to end in a newline",
                        })
                    }
                }
            }
            _ => {}
        }

        let expr = self.try_parse_expr()?;
        let expr = self.buckets.add(expr);
        match self.pop() {
            Newline(loc2) => return Ok(Stmt::Expr(expr)),
            Colon(loc2) => {}
            _ => {
                return Err(Error {
                    location: expr.view.start..expr.view.end,
                    message: "statement needs to end in a newline",
                })
            }
        }

        let (ident, ident_loc);
        if let ExprTag::Ident(id) = expr.tag {
            ident = id;
            ident_loc = expr.view.start;
        } else {
            return Err(Error {
                location: expr.view.start..expr.view.end,
                message: "left hand of declaration must be identifier",
            });
        }

        let type_ident;
        let tok = self.pop();
        if let Ident { id, location } = tok {
            type_ident = id;
        } else {
            return Err(Error {
                location: tok.get_begin()..tok.get_end(),
                message: "type needs to be identifier",
            });
        }

        match self.pop() {
            Equal(_) => {}
            x => {
                return Err(Error {
                    location: x.get_begin()..x.get_end(),
                    message: "expected equal sign after variable declaration",
                })
            }
        }

        let expr = self.try_parse_expr()?;
        match self.pop() {
            Newline(_) => {
                let expr = self.buckets.add(expr);
                return Ok(Stmt::Declare {
                    name: ident,
                    name_loc: ident_loc,
                    type_name: type_ident,
                    value: expr,
                });
            }
            _ => {
                return Err(Error {
                    location: tok.get_begin()..self.peek().get_end(),
                    message: "statement needs to end in a newline",
                })
            }
        }
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
                    let (start, end) = (op.view.start, op2.view.end);
                    expr = Expr {
                        tag: ExprTag::Add(op, op2),
                        inferred_type: InferredType::Unknown,
                        view: start..end,
                    };
                }
                Minus(loc) => {
                    self.pop();
                    let op = self.buckets.add(expr);
                    let op2 = self.try_parse_unary_postfix()?;
                    let op2 = self.buckets.add(op2);
                    let (start, end) = (op.view.start, op2.view.end);
                    expr = Expr {
                        tag: ExprTag::Sub(op, op2),
                        inferred_type: InferredType::Unknown,
                        view: start..end,
                    };
                }
                _ => return Ok(expr),
            }
        }
    }

    pub fn try_parse_unary_postfix(&mut self) -> Result<Expr<'b>, Error<'b>> {
        let mut expr = self.try_parse_expr_atom()?;

        loop {
            if let Token::LParen(begin) = self.peek() {
                let callee = self.buckets.add(expr);
                let arguments = self.try_parse_expr_tup()?;
                if let Expr {
                    tag: ExprTag::Tup(slice),
                    inferred_type,
                    view,
                } = arguments
                {
                    let (start, end) = (callee.view.start, callee.view.end);
                    expr = Expr {
                        tag: ExprTag::Call {
                            callee,
                            arguments: slice,
                        },
                        inferred_type: InferredType::Unknown,
                        view: start..end,
                    };
                } else {
                    let (start, end) = (callee.view.start, arguments.view.end);
                    expr = Expr {
                        tag: ExprTag::Call {
                            callee,
                            arguments: self.buckets.add_array(vec![arguments]),
                        },
                        inferred_type: InferredType::Unknown,
                        view: start..end,
                    };
                }
            } else if let Token::Dot(begin) = self.peek() {
                self.pop();
                match self.pop() {
                    Token::Ident { id, location } => {
                        let parent = self.buckets.add(expr);
                        let start = parent.view.start;

                        expr = Expr {
                            tag: ExprTag::DotAccess {
                                parent,
                                member_id: id,
                            },
                            inferred_type: InferredType::Unknown,
                            view: start..(location + self.lexer.id_list[id as usize].len() as u32),
                        }
                    }
                    x => {
                        return Err(Error {
                            location: x.get_begin()..x.get_end(),
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
            Ident { id, location } => {
                self.pop();
                return Ok(Expr {
                    tag: ExprTag::Ident(id),
                    inferred_type: InferredType::Unknown,
                    view: location..(location + self.lexer.id_list[id as usize].len() as u32),
                });
            }
            FloatingPoint { value, begin, end } => {
                self.pop();
                return Ok(Expr {
                    tag: ExprTag::Float(value),
                    inferred_type: InferredType::Float,
                    view: begin..end.get(),
                });
            }
            Integer { value, begin, end } => {
                self.pop();
                return Ok(Expr {
                    tag: ExprTag::Int(value),
                    inferred_type: InferredType::Int,
                    view: begin..end.get(),
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
            _ => panic!(),
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
                    inferred_type: InferredType::Unknown,
                    view: tup_begin..(tup_end + 1),
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
                    location: tok.get_begin()..tok.get_end(),
                    message: "expected ')' character",
                })
            }
        };

        return Ok(Expr {
            tag: ExprTag::Tup(self.buckets.add_array(exprs)),
            inferred_type: InferredType::Unknown,
            view: tup_begin..tup_end,
        });
    }
}
