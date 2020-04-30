#![allow(unused_variables)]
#![allow(dead_code)]

use std::env;
use std::fs::read_to_string;

mod lexer;
mod parser;
mod syntax_tree;
mod util;

use lexer::*;
use parser::*;

fn main() {
    let args: Vec<String> = env::args().collect();
    let input = read_to_string(&args[1]).unwrap();

    let mut buckets = util::Buckets::new();
    let l = Lexer::new(&input);
    let mut l2 = Lexer::new(&input);
    let mut tokens = Vec::new();
    let mut tok = l2.next();
    while match tok {
        Token::End(_) => false,
        _ => true,
    } {
        tokens.push(tok);
        tok = l2.next();
    }
    tokens.push(l2.next()); // add the end token
    println!("{:?}", tokens);

    let mut p = Parser::new(&mut buckets, l);

    println!("{:?}", p.try_parse_program());
}
