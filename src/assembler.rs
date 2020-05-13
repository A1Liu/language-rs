use crate::runtime::*;
use crate::syntax_tree::*;
use std::collections::HashMap;
use std::ptr::NonNull;

pub struct OffsetTable {
    pub uids: HashMap<u32, u32>,
    parent: Option<NonNull<OffsetTable>>,
}

pub fn offsets_(parent: &OffsetTable) -> OffsetTable {
    return OffsetTable {
        uids: HashMap::new(),
        parent: Some(NonNull::from(parent)),
    };
}

impl OffsetTable {
    pub fn new_global() -> Self {
        return Self {
            uids: HashMap::new(),
            parent: None,
        };
    }

    pub fn declare(&mut self, symbol: u32, offset: u32) {
        if self.uids.contains_key(&symbol) {
            println!("{}", symbol);
            panic!();
        }
        self.uids.insert(symbol, offset);
    }

    pub fn search(&self, symbol: u32) -> Option<u32> {
        return unsafe { self.search_unsafe(symbol) };
    }

    unsafe fn search_unsafe(&self, symbol: u32) -> Option<u32> {
        let mut current = NonNull::from(self);
        let mut uids = NonNull::from(&current.as_ref().uids);

        loop {
            if let Some(info) = uids.as_ref().get(&symbol) {
                return Some(*info);
            } else if let Some(parent) = current.as_ref().parent {
                current = parent;
                uids = NonNull::from(&current.as_ref().uids);
            } else {
                return None;
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct OpLoc {
    pub function_index: u32,
    pub offset: u32,
}

#[derive(Clone, Copy)]
struct FuncInfo<'a> {
    uid: u32,
    argument_uids: &'a [u32],
    declarations: &'a [Declaration],
    stmts: &'a [TStmt<'a>],
    parent: &'a OffsetTable,
}

#[derive(Debug, Clone, Copy)]
enum AsmContext {
    Function {
        function_index: u32,
        return_index: i32,
    },
    Global,
}

impl AsmContext {
    pub fn func_idx(&self) -> u32 {
        match self {
            Self::Global => 0,
            Self::Function { function_index, .. } => *function_index,
        }
    }
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

    pub fn assemble_program(&mut self, program_tree: TProgram) -> Vec<Opcode> {
        let mut program = Vec::new();
        let mut offsets = OffsetTable::new_global();

        program.push(Opcode::HeapAlloc {
            header: ObjectHeader {
                type_index: STACK_FRAME_TYPE_INDEX,
                object_size: program_tree.declarations.len() as u32,
            },
        });

        for (idx, decl) in program_tree.declarations.iter().enumerate() {
            offsets.declare(decl.name, idx as u32);
        }

        self.assemble_block(
            AsmContext::Global,
            None,
            &mut program,
            offsets,
            program_tree.stmts,
        );

        program.push(Opcode::Return);

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
                Opcode::JumpNotIf(label) => {
                    let op_loc = self.labels[*label as usize];
                    *label = function_translations[&op_loc.function_index] + op_loc.offset;
                }
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
        uid: u32,
        argument_uids: &[u32],
        declarations: &[Declaration],
        stmts: &[TStmt],
        parent: &OffsetTable,
    ) -> Vec<Opcode> {
        let mut current = Vec::new();
        let stack_frame_size = (argument_uids.len() + declarations.len()) as u32;

        current.push(Opcode::HeapAlloc {
            header: ObjectHeader {
                type_index: STACK_FRAME_TYPE_INDEX,
                object_size: stack_frame_size,
            },
        });

        let mut offsets = offsets_(parent);
        let mut arg_offset = -1;
        let mut offset = 0;
        for uid in argument_uids.iter() {
            offsets.declare(*uid, offset);
            current.push(Opcode::GetLocal {
                stack_offset: arg_offset,
            });
            current.push(Opcode::GetLocal { stack_offset: 0 });
            current.push(Opcode::HeapWrite { offset });

            offset += 1;
            arg_offset -= 1;
        }
        for decl in declarations.iter() {
            offsets.declare(decl.name, offset);
            offset += 1;
        }

        self.assemble_block(
            AsmContext::Function {
                function_index: uid,
                return_index: -(argument_uids.len() as i32) - 1,
            },
            None,
            &mut current,
            offsets,
            stmts,
        );
        current.push(Opcode::Return);
        return current;
    }

    fn assemble_block<'a>(
        &mut self,
        context: AsmContext,
        loop_label: Option<u32>,
        current: &mut Vec<Opcode>,
        offsets: OffsetTable,
        stmts: &'a [TStmt<'a>],
    ) {
        for stmt in stmts {
            match stmt {
                TStmt::Expr(expr) => {
                    convert_expression_to_ops(current, &offsets, expr);
                    current.push(Opcode::Pop);
                }
                TStmt::Assign { to, value } => {
                    convert_expression_to_ops(current, &offsets, value);
                    let offset = offsets.search(*to).unwrap();

                    current.push(Opcode::GetLocal { stack_offset: 0 });
                    current.push(Opcode::HeapWrite { offset });
                }
                TStmt::Return { ret_val } => {
                    convert_expression_to_ops(current, &offsets, ret_val);
                    if let AsmContext::Function {
                        function_index,
                        return_index,
                    } = context
                    {
                        current.push(Opcode::SetLocal {
                            stack_offset: return_index,
                        });
                        current.push(Opcode::Return);
                    } else {
                        panic!("shouldn't see return statement outside of function context");
                    }
                }
                TStmt::If {
                    condition,
                    if_true,
                    if_false,
                } => {
                    convert_expression_to_ops(current, &offsets, condition);

                    let false_label = self.create_label(context.func_idx());
                    let end_label = self.create_label(context.func_idx());

                    current.push(Opcode::JumpNotIf(false_label));
                    self.assemble_block(context, loop_label, current, offsets_(&offsets), if_true);
                    current.push(Opcode::Jump(end_label));
                    self.attach_label(false_label, current.len() as u32);
                    self.assemble_block(context, loop_label, current, offsets_(&offsets), if_false);
                    self.attach_label(end_label, current.len() as u32);
                }
                TStmt::Break => {
                    current.push(Opcode::Jump(loop_label.unwrap()));
                }
                TStmt::While {
                    condition,
                    block,
                    else_block: e_block,
                } => {
                    let begin = self.create_label(context.func_idx());
                    let else_branch = self.create_label(context.func_idx());
                    let end = self.create_label(context.func_idx());

                    self.attach_label(begin, current.len() as u32);
                    convert_expression_to_ops(current, &offsets, condition);
                    current.push(Opcode::JumpNotIf(else_branch));
                    self.assemble_block(context, Some(end), current, offsets_(&offsets), block);
                    current.push(Opcode::Jump(begin));
                    self.attach_label(else_branch, current.len() as u32);
                    self.assemble_block(context, loop_label, current, offsets_(&offsets), e_block);
                    self.attach_label(end, current.len() as u32);
                }
                TStmt::Function {
                    uid,
                    argument_names,
                    declarations,
                    stmts,
                    ..
                } => {
                    let func_body =
                        self.assemble_function(*uid, argument_names, declarations, stmts, &offsets);
                    self.functions.insert(*uid, func_body);
                }
            }
        }
    }
}

pub fn convert_expression_to_ops(ops: &mut Vec<Opcode>, offsets: &OffsetTable, expr: &TExpr) {
    match expr {
        TExpr::None => {
            ops.push(Opcode::PushNone);
        }
        TExpr::Bool(value) => {
            ops.push(Opcode::MakeBool(*value));
        }
        TExpr::Int(value) => {
            ops.push(Opcode::MakeInt(*value as i64));
        }
        TExpr::Float(value) => {
            ops.push(Opcode::MakeFloat(*value));
        }
        TExpr::Ident { id, .. } => {
            let offset = offsets.search(*id).unwrap();
            ops.push(Opcode::GetLocal { stack_offset: 0 });
            ops.push(Opcode::HeapRead { offset });
        }
        TExpr::Minus { left, right, type_ } => {
            convert_expression_to_ops(ops, offsets, left);
            convert_expression_to_ops(ops, offsets, right);
            if *type_ == Type::Float {
                ops.push(Opcode::SubFloat);
            } else {
                ops.push(Opcode::SubInt);
            }
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
