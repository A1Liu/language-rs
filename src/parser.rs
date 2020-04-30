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
    tokens: Vec<Token<'a>>,
    index: usize,
}

impl<'a> Parser<'a> {
    pub fn new(buckets: &'a mut Buckets<'a>, mut lexer: Lexer<'a>) -> Self {
        let mut tok = lexer.next();

        let mut tokens = Vec::new();
        while tok != Token::End {
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

    fn peek(&self) -> Token<'a> {
        return self.tokens[self.index];
    }

    fn pop(&mut self) -> Token<'a> {
        let prev_index = self.index;
        self.index += 1;
        return self.tokens[prev_index];
    }

    pub fn try_parse_program(&mut self) -> Result<Vec<Stmt<'a>>, Error<'a>> {
        let mut stmts = Vec::new();
        while self.peek() != Token::End {
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
                    Newline(loc2) => {
                        return Err(Error {
                            location: loc..loc2,
                            message: "pass ends a line",
                        })
                    }
                    _ => return Ok(Stmt::Pass),
                }
            }
            _ => {}
        }

        return Ok(Stmt::Expr(self.try_parse_expr_atom()?));
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
                    view: location..(location + self.id_list[id as usize].len() as u32),
                })
            }
            FloatingPoint { value, begin, end } => {
                return Ok(Expr {
                    tag: ExprTag::Float(value),
                    view: begin..end.get(),
                })
            }
            Integer { value, begin, end } => {
                return Ok(Expr {
                    tag: ExprTag::Int(value),
                    view: begin..end.get(),
                })
            }
            _ => panic!(),
        }
    }
}
