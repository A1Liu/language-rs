#![allow(unused_variables)]
#![allow(dead_code)]

use std::env;
use std::fs::read_to_string;

extern crate codespan_reporting;

mod builtins;
mod lexer;
mod parser;
mod runtime;
mod syntax_tree;
mod type_checker;
mod util;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};

fn run_on_file(filename: &str) {
    let input = read_to_string(filename).unwrap();
    run_on_string(filename, &input);
}

fn run_on_string(filename: &str, input: &str) {
    let mut buckets = util::Buckets::new();
    let mut files = SimpleFiles::new();

    let mut parser = parser::Parser::new(&mut buckets, input);
    let file_id = files.add(filename, input);
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

    let mut t = type_checker::TypeChecker::new(&mut buckets);
    match t.check_program(program) {
        Ok(()) => {}
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

    let mut rscope = runtime::RuntimeScope::new();
    for stmt in program {
        rscope.run_stmt(stmt);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    for arg in args.iter().skip(1) {
        run_on_file(arg);
    }
}
