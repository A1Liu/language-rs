#![allow(unused_variables)]
#![allow(dead_code)]

mod lexer;
mod parser;
mod syntax_tree;
mod util;

use lexer::*;

fn main() {
    let mut l = Lexer::new("(hey, 12.2 + 12)");

    let mut tok = l.next();

    let mut tokens = Vec::new();
    while tok != Token::End {
        tokens.push(tok);
        tok = l.next();
    }

    println!("{:?}", tokens);
}
