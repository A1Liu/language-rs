use std::collections::HashMap;
use std::str::from_utf8_unchecked;

#[derive(Debug)]
enum Token<'a> {
    Pass(&'a str),
    Else(&'a str),
    Ident(usize),
    Lparen(&'a str),
    RParen(&'a str),
    LBracket(&'a str),
    RBracket(&'a str),
    NotEqual(&'a str),
    Equals(&'a str),
    EqualsEquals(&'a str),
    LessThan(&'a str),
    GreaterThan(&'a str),
    LessEq(&'a str),
    GreaterEq(&'a str),
    Arrow(&'a str),
    Dot(&'a str),
    Plus(&'a str),
    Minus(&'a str),
    Star(&'a str),
    StarStar(&'a str),
    Div(&'a str),
    DivDiv(&'a str),
    Percent(&'a str),
    Colon(&'a str),
    Comma(&'a str),
    None(&'a str),
    Newline(&'a str),
    Indent(&'a str),
    Dedent,
    UnknownDedent,
    Unknown(&'a str),
    End,
    Integer(i64),
    FloatingPoint(f64),
    String(&'a str),
    TripleDash(&'a str),
    IntType(&'a str),
    FloatType(&'a str),
    StrType(&'a str),
    BoolType(&'a str),
}

#[derive(Eq, PartialEq)]
enum LexerState {
    Normal,
    Indentation,
    Dedent,
    End,
}

struct Lexer<'a> {
    data: &'a [u8],
    id_map: HashMap<&'a str, usize>,
    id_list: Vec<&'a str>,
    indent_stack: Vec<u16>,
    index: usize,
    indent_level: u16,
    parentheses_count: u8,
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
            parentheses_count: 0,
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

    fn next_indent(&mut self) -> Token<'a> {
        let mut indent_level: u16 = 0;
        let mut begin = self.index;
        while self.index < self.data.len() {
            match self.data[self.index] {
                b'\\' => {
                    self.index += 1;
                    if self.data[self.index] != b'\n' {
                        return Token::Unknown(self.substr(self.index - 2, self.index));
                    }
                }
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
        let end = self.index;
        if self.index == self.data.len() {
            self.state = LexerState::End;
            return self.next();
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
            return Token::Indent(self.substr(begin, end));
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
        return Token::End;
    }
}
