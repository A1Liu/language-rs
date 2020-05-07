#![allow(unused_variables)]
#![allow(dead_code)]

use std::env;
use std::fs::read_to_string;
use std::io::Write;

extern crate codespan_reporting;

mod assembler;
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

fn run_on_file<'a, 'b>(
    stdout: impl Write,
    stderr: impl Write,
    buckets: &mut util::Buckets<'b>,
    files: &mut SimpleFiles<&'a str, &'b str>,
    filename: &'a str,
) -> Result<(), Diagnostic<usize>> {
    let input = buckets.add_str(&read_to_string(filename).unwrap());
    let file_id = files.add(filename, input);

    return run_on_string(stdout, stderr, buckets, file_id, &input);
}

fn run_on_string<'b>(
    output: impl Write,
    mut stderr: impl Write,
    buckets: &mut util::Buckets<'b>,
    file_id: usize,
    input: &str,
) -> Result<(), Diagnostic<usize>> {
    let mut parser = parser::Parser::new(buckets, input);
    let parse_result = parser.try_parse_program();

    let program = match parse_result {
        Ok(p) => buckets.add_array(p),
        Err(e) => {
            return Err(Diagnostic::error()
                .with_message(e.message)
                .with_labels(vec![Label::primary(
                    file_id,
                    (e.location.start as usize)..(e.location.end as usize),
                )]));
        }
    };

    write!(stderr, "{:?}\n\n", program).expect("why did this fail?");

    let mut t = type_checker::TypeChecker::new(buckets);
    let program = match t.check_program(program) {
        Ok(p) => p,
        Err(e) => {
            return Err(Diagnostic::error()
                .with_message(e.message)
                .with_labels(vec![Label::primary(
                    file_id,
                    (e.location.start as usize)..(e.location.end as usize),
                )]));
        }
    };

    write!(stderr, "{:?}\n\n", program).expect("why did this fail?");

    let mut asmer = assembler::Assembler::new();
    let ops = asmer.assemble_program(program);
    // let ops = assembler::convert_program_to_ops(program);
    buckets.drop();
    write!(stderr, "{:?}\n\n", ops).expect("why did this fail?");
    let mut run = runtime::Runtime::new(output);
    run.run(&ops);

    return Ok(());
}

fn test_file_should_succeed(filename: &str) {
    let writer = StandardStream::stderr(ColorChoice::Always);
    let config = codespan_reporting::term::Config::default();

    let mut buckets = util::Buckets::new();
    let mut files = SimpleFiles::new();
    let mut output = util::StringWriter::new();

    match run_on_file(
        &mut output,
        util::Void::new(),
        &mut buckets,
        &mut files,
        filename,
    ) {
        Err(diagnostic) => {
            codespan_reporting::term::emit(&mut writer.lock(), &config, &files, &diagnostic)
                .expect("why did this fail?");
            panic!();
        }
        _ => {}
    }

    let filename = String::from(filename);
    assert!(output.to_string() == read_to_string(filename + ".out").expect("why did this fail?"));
}

#[test]
fn test_expr() {
    test_file_should_succeed("test_data/expressions.py");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let writer = StandardStream::stderr(ColorChoice::Always);
    let config = codespan_reporting::term::Config::default();

    for arg in args.iter().skip(1) {
        let mut buckets = util::Buckets::new();
        let mut files = SimpleFiles::new();
        match run_on_file(
            std::io::stdout(),
            std::io::stderr(),
            &mut buckets,
            &mut files,
            arg,
        ) {
            Err(diagnostic) => {
                codespan_reporting::term::emit(&mut writer.lock(), &config, &files, &diagnostic)
                    .expect("why did this fail?")
            }
            _ => {}
        }
    }
}
