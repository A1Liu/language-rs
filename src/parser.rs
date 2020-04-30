use crate::lexer::*;
use crate::syntax_tree::*;
use crate::util::*;
use std::ops::Range;

#[derive(Debug)]
pub struct Error<'a> {
    location: Range<u32>,
    message: &'a str,
}

pub struct Parser<'a> {
    buckets: &'a mut Buckets<'a>,
    pub data: &'a [u8],
    pub id_list: Vec<&'a str>,
    tokens: Vec<Token>,
    index: usize,
}

impl<'a> Parser<'a> {
    pub fn new(buckets: &'a mut Buckets<'a>, mut lexer: Lexer<'a>) -> Self {
        let mut tok = lexer.next();
        let mut tokens = Vec::new();

        while match tok {
            Token::End(_) => false,
            _ => true,
        } {
            tokens.push(tok);
            tok = lexer.next();
        }
        tokens.push(lexer.next()); // add the end token

        return Parser {
            buckets,
            data: lexer.data,
            id_list: lexer.id_list,
            tokens,
            index: 0,
        };
    }

    fn peek(&self) -> Token {
        return self.tokens[self.index];
    }

    fn pop(&mut self) -> Token {
        let prev_index = self.index;
        self.index += 1;
        return self.tokens[prev_index];
    }

    pub fn try_parse_program(&mut self) -> Result<Vec<Stmt<'a>>, Error<'a>> {
        let mut stmts = Vec::new();
        while match self.peek() {
            Token::End(_) => false,
            _ => true,
        } {
            stmts.push(self.try_parse_stmt()?);
        }
        return Ok(stmts);
    }

    pub fn try_parse_stmt(&mut self) -> Result<Stmt<'a>, Error<'a>> {
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

        let stmt = Stmt::Expr(self.try_parse_expr_atom()?);
        match self.pop() {
            Newline(loc2) => return Ok(stmt),
            _ => {
                return Err(Error {
                    location: tok.get_begin()..self.peek().get_end(),
                    message: "statement needs to end in a newline",
                })
            }
        }
    }

    pub fn try_parse_expr_add(&mut self) -> Result<Expr<'a>, Error<'a>> {
        use Token::*;
        let atom = self.try_parse_expr_atom()?;
        loop {
            match self.peek() {
                Plus(loc) => {
                    self.pop();
                    let atom = self.buckets.add(atom);
                    let atom2 = self.try_parse_expr_atom()?;
                    let atom2 = self.buckets.add(atom2);
                    return Ok(Expr {
                        tag: ExprTag::Add(atom, atom2),
                        inferred_type: InferredType::Unknown,
                        view: atom.view.start..atom2.view.end,
                    });
                }
                Minus(loc) => {
                    self.pop();
                    let atom = self.buckets.add(atom);
                    let atom2 = self.try_parse_expr_atom()?;
                    let atom2 = self.buckets.add(atom2);
                    return Ok(Expr {
                        tag: ExprTag::Sub(atom, atom2),
                        inferred_type: InferredType::Unknown,
                        view: atom.view.start..atom2.view.end,
                    });
                }
                _ => return Ok(atom),
            }
        }
    }

    pub fn try_parse_expr_atom(&mut self) -> Result<Expr<'a>, Error<'a>> {
        use Token::*;
        match self.pop() {
            Ident { id, location } => {
                return Ok(Expr {
                    tag: ExprTag::Ident(id as usize),
                    inferred_type: InferredType::Unknown,
                    view: location..(location + self.id_list[id as usize].len() as u32),
                })
            }
            FloatingPoint { value, begin, end } => {
                return Ok(Expr {
                    tag: ExprTag::Float(value),
                    inferred_type: InferredType::Float,
                    view: begin..end.get(),
                })
            }
            Integer { value, begin, end } => {
                return Ok(Expr {
                    tag: ExprTag::Int(value),
                    inferred_type: InferredType::Int,
                    view: begin..end.get(),
                })
            }
            LParen(tup_begin) => {
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

                let expr = self.try_parse_expr_add()?;
                match self.peek() {
                    RParen(tup_end) => {
                        self.pop();
                        return Ok(expr);
                    }
                    _ => {}
                }

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

                let end = match tok {
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
                    view: tup_begin..end,
                });
            }
            _ => panic!(),
        }
    }
}
