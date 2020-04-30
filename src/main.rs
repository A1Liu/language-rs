#![allow(unused_variables)]
#![allow(dead_code)]

mod lexer;
mod parser;
mod syntax_tree;
mod util;

use lexer::*;
use parser::*;

fn main() {
    let mut buckets = util::Buckets::new();
    let mut l = Lexer::new("hey\n12\n20");
    let mut p = Parser::new(&mut buckets, l);

    println!("{:?}", p.try_parse_program());
}
