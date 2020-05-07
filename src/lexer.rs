use crate::builtins::builtin_names;
use crate::util::{newr, CRange};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::str::from_utf8_unchecked;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Token {
    Pass(u32),
    Return(u32),
    If(u32),
    Else(u32),
    Elif(u32),
    Ident {
        id: u32,
        view: CRange,
    },
    LParen(u32),
    RParen(u32),
    Plus(u32),
    Comma(u32),
    Newline(u32),
    Colon(u32),
    Dash(u32),
    Dot(u32),
    Equal(u32),
    Def(u32),
    Arrow(u32),
    Indent {
        begin: u32,
        end: u32,
    },
    Dedent(u32),
    UnknownDedent(u32),
    Unknown {
        begin: u32,
        end: u32,
    },
    End(u32),
    Integer {
        value: u64,
        begin: u32,
        end: NonZeroU32,
    },
    FloatingPoint {
        value: f64,
        begin: u32,
        end: NonZeroU32,
    },
}

impl Token {
    pub fn view(self) -> CRange {
        use Token::*;
        return match self {
            Pass(x) => newr(x, x + 1),
            Return(x) => newr(x, x + 6),
            If(x) => newr(x, x + 2),
            Else(x) => newr(x, x + 4),
            Elif(x) => newr(x, x + 4),
            Ident { id, view } => view,
            LParen(x) => newr(x, x + 1),
            RParen(x) => newr(x, x + 1),
            Plus(x) => newr(x, x + 1),
            Dot(x) => newr(x, x + 1),
            Def(x) => newr(x, x + 3),
            Comma(x) => newr(x, x + 1),
            Dash(x) => newr(x, x + 1),
            Colon(x) => newr(x, x + 1),
            Equal(x) => newr(x, x + 1),
            Arrow(x) => newr(x, x + 2),
            Newline(x) => newr(x, x + 1),
            Indent { begin, end } => newr(begin, end),
            Integer { value, begin, end } => newr(begin, end.get()),
            FloatingPoint { value, begin, end } => newr(begin, end.get()),
            Dedent(x) => newr(x, x),
            UnknownDedent(x) => newr(x, x),
            Unknown { begin, end } => newr(begin, end),
            End(x) => newr(x, x),
        };
    }
}

#[derive(Eq, PartialEq)]
enum LexerState {
    Normal,
    Indentation,
    Dedent,
    End,
}

pub struct Lexer<'a> {
    pub data: &'a [u8],
    pub id_list: Vec<&'a str>,
    pub id_map: HashMap<&'a str, u32>,
    indent_stack: Vec<u16>,
    index: u32,
    indent_level: u16,
    paren_count: u8,
    state: LexerState,
}

impl<'a> Lexer<'a> {
    pub fn new(data: &'a str) -> Self {
        let (id_list, id_map) = builtin_names();
        return Lexer {
            data: data.as_bytes(),
            id_list,
            id_map,
            indent_stack: vec![0],
            index: 0,
            indent_level: 0,
            paren_count: 0,
            state: LexerState::Indentation,
        };
    }

    pub fn next(&mut self) -> Token {
        return match self.state {
            LexerState::Dedent => self.next_dedent(),
            LexerState::Normal => self.next_normal(),
            LexerState::Indentation => self.next_indent(),
            LexerState::End => {
                if self.indent_stack.len() > 1 {
                    self.indent_stack.pop();
                    Token::Dedent(self.index)
                } else {
                    Token::End(self.index)
                }
            }
        };
    }

    fn substr<'b>(&'b self, start: u32, end: u32) -> &'a str {
        return unsafe { from_utf8_unchecked(&self.data[(start as usize)..(end as usize)]) };
    }

    #[inline]
    fn cur(&self) -> u8 {
        return self.data[self.index as usize];
    }

    fn next_indent(&mut self) -> Token {
        let mut indent_level: u16 = 0;
        let mut begin = self.index;
        while self.index < self.data.len() as u32 {
            match self.cur() {
                b'\n' => {
                    indent_level = 0;
                    begin = self.index;
                    self.index += 1;
                }
                b' ' => {
                    indent_level += 1;
                    self.index += 1;
                }
                b'\t' => {
                    indent_level += 8 - indent_level % 8;
                    self.index += 1;
                }
                _ => {
                    break;
                }
            }
        }

        let prev_indent = *self.indent_stack.last().unwrap();
        if indent_level < prev_indent {
            self.state = LexerState::Dedent;
            self.indent_level = indent_level;
            return self.next_dedent();
        } else if self.index == self.data.len() as u32 {
            self.state = LexerState::End;
            return Token::End(self.index);
        } else if indent_level == prev_indent {
            self.state = LexerState::Normal;
            return self.next_normal();
        } else {
            self.state = LexerState::Normal;
            self.indent_stack.push(indent_level);
            return Token::Indent {
                begin,
                end: self.index,
            };
        }
    }

    fn next_dedent(&mut self) -> Token {
        let prev_indent = *self.indent_stack.last().unwrap();
        if self.indent_level < prev_indent {
            self.indent_stack.pop();
            if self.indent_level > *self.indent_stack.last().unwrap() {
                self.state = LexerState::Normal;
                self.indent_stack.push(self.indent_level);
                return Token::UnknownDedent(self.index - 1);
            }

            return Token::Dedent(self.index - 1);
        } else if self.indent_level == prev_indent {
            self.state = LexerState::Normal;
            return self.next();
        }
        self.state = LexerState::Normal;
        self.indent_stack.push(self.indent_level);
        return Token::UnknownDedent(self.index - 1);
    }

    fn next_normal(&mut self) -> Token {
        loop {
            while self.index < self.data.len() as u32 && (self.cur() == b' ' || self.cur() == b'\t')
            {
                self.index += 1;
            }

            if self.index == self.data.len() as u32 {
                self.state = LexerState::End;
                return Token::End(self.index);
            }

            let ret_val = match self.cur() {
                b'(' => {
                    self.index += 1;
                    self.paren_count += 1;
                    Token::LParen(self.index - 1)
                }
                b')' => {
                    self.index += 1;
                    if self.paren_count != 0 {
                        self.paren_count -= 1;
                    }
                    Token::RParen(self.index - 1)
                }
                b'+' => {
                    self.index += 1;
                    Token::Plus(self.index - 1)
                }
                b'-' => {
                    let begin = self.index;
                    self.index += 1;
                    if self.cur() == b'>' {
                        self.index += 1;
                        Token::Arrow(begin)
                    } else {
                        Token::Dash(begin)
                    }
                }
                b',' => {
                    self.index += 1;
                    Token::Comma(self.index - 1)
                }
                b'=' => {
                    self.index += 1;
                    Token::Equal(self.index - 1)
                }
                b':' => {
                    self.index += 1;
                    Token::Colon(self.index - 1)
                }
                b'.' => {
                    self.index += 1;
                    Token::Dot(self.index - 1)
                }
                b'\n' => {
                    self.index += 1;
                    if self.paren_count == 0 {
                        self.state = LexerState::Indentation;
                        return Token::Newline(self.index - 1);
                    }
                    continue;
                }
                b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9' => {
                    let begin = self.index;
                    while self.index < self.data.len() as u32
                        && self.cur() <= b'9'
                        && self.cur() >= b'0'
                    {
                        self.index += 1;
                    }

                    if self.index < self.data.len() as u32 && self.cur() == b'.' {
                        self.index += 1;
                        while self.index < self.data.len() as u32
                            && self.cur() <= b'9'
                            && self.cur() >= b'0'
                        {
                            self.index += 1;
                        }
                        Token::FloatingPoint {
                            value: self.substr(begin, self.index).parse().unwrap(),
                            begin,
                            end: NonZeroU32::new(self.index).unwrap(),
                        }
                    } else {
                        Token::Integer {
                            value: self.substr(begin, self.index).parse().unwrap(),
                            begin,
                            end: NonZeroU32::new(self.index).unwrap(),
                        }
                    }
                }
                c => {
                    if (c as char).is_alphabetic() {
                        break;
                    } else {
                        println!(
                            "Warning: unknown character '{}' at location: {}",
                            c as char, self.index
                        );
                        self.index += 1;
                        Token::Unknown {
                            begin: self.index - 1,
                            end: self.index,
                        }
                    }
                }
            };

            if self.index == self.data.len() as u32 {
                self.state = LexerState::End;
            }
            return ret_val;
        }

        let begin = self.index;
        while self.index < self.data.len() as u32 && (self.cur() as char).is_alphanumeric() {
            self.index += 1;
        }

        return match self.substr(begin, self.index) {
            "pass" => Token::Pass(begin),
            "def" => Token::Def(begin),
            "return" => Token::Return(begin),
            "if" => Token::If(begin),
            "else" => Token::Else(begin),
            "elif" => Token::Elif(begin),
            x => {
                let id = if self.id_map.contains_key(x) {
                    self.id_map[x]
                } else {
                    let id = self.id_list.len() as u32;
                    self.id_map.insert(x, id);
                    self.id_list.push(x);
                    id
                };

                Token::Ident {
                    id,
                    view: newr(begin, self.index),
                }
            }
        };
    }
}
