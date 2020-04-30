use std::collections::HashMap;
use std::str::from_utf8_unchecked;

#[derive(Debug, PartialEq)]
pub enum Token<'a> {
    Pass(usize),
    Ident { id: usize, location: usize },
    LParen(usize),
    RParen(usize),
    Plus(usize),
    Minus(usize),
    Star(usize),
    Div(usize),
    Comma(usize),
    Newline(usize),
    Indent { begin: usize, end: usize },
    Dedent,
    UnknownDedent,
    Unknown(&'a str),
    End,
    Integer(i64),
    FloatingPoint(f64),
}

#[derive(Eq, PartialEq)]
enum LexerState {
    Normal,
    Indentation,
    Dedent,
    End,
}

pub struct Lexer<'a> {
    data: &'a [u8],
    id_map: HashMap<&'a str, usize>,
    id_list: Vec<&'a str>,
    indent_stack: Vec<u16>,
    index: usize,
    indent_level: u16,
    paren_count: u8,
    state: LexerState,
}

impl<'a> Lexer<'a> {
    pub fn new(data: &'a str) -> Self {
        return Lexer {
            data: data.as_bytes(),
            id_map: HashMap::new(),
            id_list: Vec::new(),
            indent_stack: vec![0],
            index: 0,
            indent_level: 0,
            paren_count: 0,
            state: LexerState::Indentation,
        };
    }

    pub fn next(&mut self) -> Token<'a> {
        return match self.state {
            LexerState::Dedent => self.next_dedent(),
            LexerState::Normal => self.next_normal(),
            LexerState::Indentation => self.next_indent(),
            LexerState::End => {
                if self.indent_stack.len() > 1 {
                    Token::Dedent
                } else {
                    Token::End
                }
            }
        };
    }

    fn substr<'b>(&'b self, start: usize, end: usize) -> &'a str {
        return unsafe { from_utf8_unchecked(&self.data[start..end]) };
    }

    #[inline]
    fn cur(&self) -> u8 {
        return self.data[self.index];
    }

    fn next_indent(&mut self) -> Token<'a> {
        let mut indent_level: u16 = 0;
        let mut begin = self.index;
        while self.index < self.data.len() {
            match self.cur() {
                b'\n' => {
                    indent_level = 0;
                    begin = self.index;
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
        if self.index == self.data.len() {
            self.state = LexerState::End;
            return Token::End;
        } else if indent_level < prev_indent {
            self.state = LexerState::Dedent;
            self.indent_level = indent_level;
            return self.next_dedent();
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

    fn next_dedent(&mut self) -> Token<'a> {
        let prev_indent = *self.indent_stack.last().unwrap();
        if self.indent_level < prev_indent {
            self.indent_stack.pop();
            if self.indent_level > *self.indent_stack.last().unwrap() {
                self.state = LexerState::Normal;
                self.indent_stack.push(self.indent_level);
                return Token::UnknownDedent;
            }
        } else if self.indent_level == prev_indent {
            self.state = LexerState::Normal;
            return self.next();
        }
        self.state = LexerState::Normal;
        self.indent_stack.push(self.indent_level);
        return Token::UnknownDedent;
    }

    fn next_normal(&mut self) -> Token<'a> {
        loop {
            while self.index < self.data.len() && (self.cur() == b' ' || self.cur() == b'\t') {
                self.index += 1;
            }

            if self.index == self.data.len() {
                self.state = LexerState::End;
                return Token::End;
            }

            let ret_val = match self.data[self.index] {
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
                    self.index += 1;
                    Token::Minus(self.index - 1)
                }
                b'/' => {
                    self.index += 1;
                    Token::Div(self.index - 1)
                }
                b'*' => {
                    self.index += 1;
                    Token::Star(self.index - 1)
                }
                b',' => {
                    self.index += 1;
                    Token::Comma(self.index - 1)
                }
                b'\n' => {
                    self.index += 1;
                    if self.paren_count == 0 {
                        return self.next_indent();
                    }
                    continue;
                }
                b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9' => {
                    let begin = self.index;
                    while self.index < self.data.len() && self.cur() <= b'9' && self.cur() >= b'0' {
                        self.index += 1;
                    }

                    if self.cur() == b'.' {
                        self.index += 1;
                        while self.index < self.data.len()
                            && self.cur() <= b'9'
                            && self.cur() >= b'0'
                        {
                            self.index += 1;
                        }
                        Token::FloatingPoint(self.substr(begin, self.index).parse().unwrap())
                    } else {
                        Token::Integer(self.substr(begin, self.index).parse().unwrap())
                    }
                }
                c => {
                    if (c as char).is_alphabetic() {
                        break;
                    } else {
                        self.index += 1;
                        Token::Unknown(self.substr(self.index - 1, self.index))
                    }
                }
            };

            if self.index == self.data.len() {
                self.state = LexerState::End;
            }
            return ret_val;
        }

        let begin = self.index;
        while self.index < self.data.len() && (self.data[self.index] as char).is_alphanumeric() {
            self.index += 1;
        }

        let id_string = self.substr(begin, self.index);

        return match id_string {
            "pass" => Token::Pass(begin),
            x => {
                let id = if self.id_map.contains_key(x) {
                    self.id_map[x]
                } else {
                    let id = self.id_list.len();
                    self.id_map.insert(x, id);
                    id
                };

                Token::Ident {
                    id,
                    location: begin,
                }
            }
        };
    }
}
