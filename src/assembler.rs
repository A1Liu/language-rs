use crate::runtime::*;
use crate::syntax_tree::*;
use crate::util::OffsetTable;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
struct OpLoc {
    pub function_index: u32,
    pub offset: u32,
}
struct FuncInfo<'a> {
    uid: u32,
    offsets: OffsetTable,
    stmts: &'a [TStmt<'a>],
    return_index: i32,
}

pub struct Assembler {
    functions: HashMap<u32, Vec<Opcode>>,
    labels: Vec<OpLoc>,
}

impl Assembler {
    pub fn new() -> Self {
        return Self {
            functions: HashMap::new(),
            labels: Vec::new(),
        };
    }

    pub fn create_label(&mut self, function_index: u32) -> u32 {
        let idx = self.labels.len() as u32;
        self.labels.push(OpLoc {
            function_index,
            offset: !0,
        });
        return idx;
    }

    pub fn attach_label(&mut self, label_idx: u32, location: u32) {
        self.labels[label_idx as usize].offset = location;
    }

    pub fn assemble_program(&mut self, stmts: &[TStmt]) -> Vec<Opcode> {
        let mut program = Vec::new();
        let offsets = OffsetTable::new_global(HashMap::new());
        self.assemble_function(0, &mut program, offsets, stmts, 0);

        let mut function_translations = HashMap::new();
        function_translations.insert(0, 0);

        for (id, mut stmts) in self.functions.drain() {
            let function_offset = program.len() as u32;
            function_translations.insert(id, function_offset);
            program.append(&mut stmts);
        }

        for op in &mut program {
            match op {
                Opcode::Call(func) => *func = function_translations[func],
                Opcode::JumpIf(label) => {
                    let op_loc = self.labels[*label as usize];
                    *label = function_translations[&op_loc.function_index] + op_loc.offset;
                }
                Opcode::Jump(label) => {
                    let op_loc = self.labels[*label as usize];
                    *label = function_translations[&op_loc.function_index] + op_loc.offset;
                }

                _ => {}
            }
        }

        return program;
    }

    fn assemble_function(
        &mut self,
        function_index: u32,
        current: &mut Vec<Opcode>,
        mut offsets: OffsetTable,
        stmts: &[TStmt],
        return_index: i32,
    ) {
        let function_queue =
            self.assemble_block(function_index, current, offsets, stmts, return_index);
        current.push(Opcode::Return);

        for FuncInfo {
            uid,
            stmts,
            offsets,
            return_index,
        } in function_queue
        {
            let mut current_function = Vec::new();
            self.assemble_function(
                uid,
                &mut current_function,
                OffsetTable::new(&offsets),
                stmts,
                return_index,
            );
            self.functions.insert(uid, current_function);
        }
    }

    fn assemble_block<'a>(
        &mut self,
        function_index: u32,
        current: &mut Vec<Opcode>,
        mut offsets: OffsetTable,
        stmts: &'a [TStmt<'a>],
        return_index: i32,
    ) -> Vec<FuncInfo<'a>> {
        let mut function_queue = Vec::new();
        let mut decl_index = 0;
        for stmt in stmts {
            match stmt {
                TStmt::Expr(expr) => {
                    convert_expression_to_ops(current, &offsets, expr);
                    current.push(Opcode::Pop);
                }
                TStmt::Declare { decl_type, uid } => {
                    current.push(Opcode::PushNone);
                    offsets.declare(*uid, decl_index);
                    decl_index += 1;
                }
                TStmt::Assign { uid, value } => {
                    convert_expression_to_ops(current, &offsets, value);
                    current.push(Opcode::SetLocal {
                        stack_offset: offsets.search(*uid).unwrap(),
                    });
                }
                TStmt::Return { ret_val } => {
                    convert_expression_to_ops(current, &offsets, ret_val);
                    current.push(Opcode::SetLocal {
                        stack_offset: return_index,
                    });

                    current.push(Opcode::Return);
                }
                TStmt::If {
                    condition,
                    if_true,
                    if_false,
                } => {
                    convert_expression_to_ops(current, &offsets, condition);

                    let true_label = self.create_label(function_index);
                    let end_label = self.create_label(function_index);

                    current.push(Opcode::JumpIf(true_label));
                    self.assemble_block(
                        function_index,
                        current,
                        OffsetTable::new(&offsets),
                        if_false,
                        return_index,
                    );
                    current.push(Opcode::Jump(end_label));
                    self.attach_label(true_label, current.len() as u32);
                    self.assemble_block(
                        function_index,
                        current,
                        OffsetTable::new(&offsets),
                        if_true,
                        return_index,
                    );
                    self.attach_label(end_label, current.len() as u32);
                }
                TStmt::Function {
                    uid,
                    return_type,
                    argument_uids,
                    argument_types,
                    stmts,
                } => {
                    let mut offset = -1;
                    let mut offsets = OffsetTable::new(&offsets);
                    for uid in argument_uids.iter() {
                        offsets.declare(*uid, offset);
                        offset -= 1;
                    }
                    function_queue.push(FuncInfo {
                        uid: *uid,
                        stmts: *stmts,
                        offsets,
                        return_index: -(argument_types.len() as i32) - 1,
                    });
                }
            }
        }
        return function_queue;
    }
}

pub fn convert_expression_to_ops(ops: &mut Vec<Opcode>, offsets: &OffsetTable, expr: &TExpr) {
    match expr {
        TExpr::Ident { uid, .. } => {
            ops.push(Opcode::GetLocal {
                stack_offset: offsets.search(*uid).unwrap(),
            });
        }
        TExpr::Add { left, right, type_ } => {
            convert_expression_to_ops(ops, offsets, left);
            convert_expression_to_ops(ops, offsets, right);
            if *type_ == Type::Float {
                ops.push(Opcode::AddFloat);
            } else {
                ops.push(Opcode::AddInt);
            }
        }
        TExpr::Int(value) => {
            ops.push(Opcode::MakeInt(*value as i64));
        }
        TExpr::Float(value) => {
            ops.push(Opcode::MakeFloat(*value));
        }
        TExpr::Call {
            callee_uid,
            arguments,
            ..
        } => {
            ops.push(Opcode::PushNone);
            for arg in arguments.iter().rev() {
                convert_expression_to_ops(ops, offsets, arg);
            }
            ops.push(Opcode::Call(*callee_uid));
            for _ in 0..arguments.len() {
                ops.push(Opcode::Pop);
            }
        }
        TExpr::ECall { arguments } => {
            for arg in arguments.iter().rev() {
                convert_expression_to_ops(ops, offsets, arg);
            }
            ops.push(Opcode::ECall);
        }
    }
}
