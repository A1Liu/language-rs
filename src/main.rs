#![allow(unused_variables)]
#![allow(dead_code)]

use std::env;
use std::fs::read_to_string;

mod lexer;
mod parser;
mod syntax_tree;
mod type_checker;
mod util;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use parser::*;
use type_checker::*;

fn run_on_file(file: &str) {
    let input = read_to_string(file).unwrap();

    let mut buckets = util::Buckets::new();
    let mut files = SimpleFiles::new();

    let mut parser = Parser::new(&mut buckets, &input);
    let file_id = files.add(file, &input);
    let parse_result = parser.try_parse_program();

    let program = match parse_result {
        Ok(p) => buckets.add_array(p),
        Err(e) => {
            let diagnostic = Diagnostic::error()
                .with_message(e.message)
                .with_labels(vec![Label::primary(
                    file_id,
                    (e.location.start as usize)..(e.location.end as usize),
                )]);

            let writer = StandardStream::stderr(ColorChoice::Always);
            let config = codespan_reporting::term::Config::default();

            codespan_reporting::term::emit(&mut writer.lock(), &config, &files, &diagnostic)
                .expect("why did this fail?");
            return;
        }
    };

    let mut t = TypeChecker::new(&mut buckets);
    match t.check_program(program) {
        Ok(()) => println!("success!"),
        Err(e) => {
            let diagnostic = Diagnostic::error()
                .with_message(e.message)
                .with_labels(vec![Label::primary(
                    file_id,
                    (e.location.start as usize)..(e.location.end as usize),
                )]);

            let writer = StandardStream::stderr(ColorChoice::Always);
            let config = codespan_reporting::term::Config::default();

            codespan_reporting::term::emit(&mut writer.lock(), &config, &files, &diagnostic)
                .expect("why did this fail?");
            return;
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    for arg in args.iter().skip(1) {
        run_on_file(arg);
    }
}
