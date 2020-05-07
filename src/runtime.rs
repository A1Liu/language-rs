use std::io::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ObjectHeader {
    type_index: u32,
    object_size: u32,
}

impl ObjectHeader {
    pub fn to_bits(self) -> u64 {
        return ((self.type_index as u64) << 32) + self.object_size as u64;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Opcode {
    MakeInt(i64),
    MakeFloat(f64),
    AddFloat,
    AddInt,
    PushNone,
    Pop,
    GetGlobal { stack_offset: u32 },
    SetGlobal { stack_offset: u32 },
    GetLocal { stack_offset: i32 },
    SetLocal { stack_offset: i32 },
    Return,
    Call(u32),   // absolute address
    JumpIf(u32), // absolute address
    Jump(u32),   // absolute address
    ECall,
}

pub struct Runtime<Out>
where
    Out: Write,
{
    pub stack: Vec<usize>,
    pub heap: Vec<u64>,
    pub fp_ra_stack: Vec<usize>,
    pub stdout: Out,
    pub fp: usize,
    pub pc: usize,
}

pub const NONE_VALUE: usize = !0;

const INT_HEADER: ObjectHeader = ObjectHeader {
    type_index: 0,
    object_size: 1,
};
const FLOAT_HEADER: ObjectHeader = ObjectHeader {
    type_index: 1,
    object_size: 1,
};
const BOOL_HEADER: ObjectHeader = ObjectHeader {
    type_index: 2,
    object_size: 1,
};

pub const PRINT_PRIMITIVE: u64 = 0;
pub const FLOAT_CAST: u64 = 1;

impl<Out> Runtime<Out>
where
    Out: Write,
{
    pub fn new(stdout: Out) -> Self {
        return Self {
            stack: Vec::new(), // dummy frame pointer value
            heap: Vec::new(),
            fp_ra_stack: vec![NONE_VALUE, 0],
            stdout,
            fp: 0,
            pc: 0,
        };
    }

    pub fn run(&mut self, code: &Vec<Opcode>) {
        while self.pc != NONE_VALUE {
            self.run_op(code[self.pc]);
        }
    }

    fn get_obj_header(&self, idx: usize) -> ObjectHeader {
        let header = self.heap[idx - 1];
        return ObjectHeader {
            type_index: (header >> 32) as u32,
            object_size: header as u32,
        };
    }

    fn run_op(&mut self, op: Opcode) {
        use Opcode::*;
        match op {
            MakeInt(int) => {
                self.heap.push(INT_HEADER.to_bits());
                let ret_val = self.heap.len();
                self.heap.push(int as u64);
                self.stack.push(ret_val);
            }
            MakeFloat(float) => {
                self.heap.push(FLOAT_HEADER.to_bits());
                let ret_val = self.heap.len();
                self.heap.push(float.to_bits());
                self.stack.push(ret_val);
            }
            AddFloat => {
                let float2 = f64::from_bits(self.heap[self.stack.pop().unwrap()]);
                let float1 = f64::from_bits(self.heap[self.stack.pop().unwrap()]);
                self.heap.push(FLOAT_HEADER.to_bits());
                let ret_val = self.heap.len();
                self.heap.push((float1 + float2).to_bits());
                self.stack.push(ret_val);
            }
            AddInt => {
                let int2 = self.heap[self.stack.pop().unwrap()] as i64;
                let int1 = self.heap[self.stack.pop().unwrap()] as i64;
                self.heap.push(INT_HEADER.to_bits());
                let ret_val = self.heap.len();
                self.heap.push((int1 + int2) as u64);
                self.stack.push(ret_val);
            }
            Pop => {
                self.stack.pop();
            }
            GetGlobal { stack_offset } => {
                self.stack.push(self.stack[stack_offset as usize]);
            }
            SetGlobal { stack_offset } => {
                self.stack[stack_offset as usize] = self.stack.pop().unwrap();
            }
            GetLocal { stack_offset } => {
                self.stack
                    .push(self.stack[self.fp.wrapping_add(stack_offset as usize)]);
            }
            SetLocal { stack_offset } => {
                self.stack[self.fp.wrapping_add(stack_offset as usize)] = self.stack.pop().unwrap();
            }
            PushNone => {
                self.stack.push(NONE_VALUE);
            }
            Jump(address) => {
                self.pc = address as usize;
                return;
            }
            JumpIf(address) => {
                let arg = self.stack.pop().unwrap();

                if arg != NONE_VALUE {
                    let should_jump = match self.get_obj_header(arg) {
                        INT_HEADER | BOOL_HEADER => self.heap[arg] != 0,
                        FLOAT_HEADER => f64::from_bits(self.heap[arg]) != 0.0,
                        x => {
                            panic!("attempting to use value as boolean with type: {:?}\n", x);
                        }
                    };

                    if should_jump {
                        self.pc = address as usize;
                        return;
                    }
                }
            }
            Call(func) => {
                self.fp_ra_stack.push(self.pc + 1);
                self.fp_ra_stack.push(self.fp);

                self.pc = func as usize;
                self.fp = self.stack.len();
                return;
            }
            Return => {
                self.fp = self.fp_ra_stack.pop().unwrap();
                self.pc = self.fp_ra_stack.pop().unwrap();
                return;
            }
            ECall => match self.heap[self.stack.pop().unwrap()] {
                PRINT_PRIMITIVE => {
                    let arg = self.stack.pop().unwrap();
                    let type_id = self.get_obj_header(arg);
                    let arg_value = self.heap[arg];

                    match type_id {
                        INT_HEADER => {
                            write!(self.stdout, "{}\n", arg_value as i64)
                                .expect("should not have failed");
                        }
                        FLOAT_HEADER => {
                            write!(self.stdout, "{}\n", f64::from_bits(arg_value))
                                .expect("should not have failed");
                        }
                        x => {
                            panic!("got print_primitive ecall arg of invalid type {:?}", x);
                        }
                    }
                    self.stack.push(NONE_VALUE);
                }
                FLOAT_CAST => {
                    let arg = self.stack.pop().unwrap();
                    let type_id = self.get_obj_header(arg);
                    let arg_value = self.heap[arg];

                    match type_id {
                        INT_HEADER => {
                            self.heap.push(FLOAT_HEADER.to_bits());
                            let ret_val = self.heap.len();
                            self.heap.push((arg_value as i64 as f64).to_bits());
                            self.stack.push(ret_val);
                        }
                        x => {
                            panic!("[ECALL]: attempted to cast a non-int to float: {:?}", x);
                        }
                    }
                }
                _ => {
                    println!("invalid ecall");
                    panic!();
                }
            },
        }
        self.pc += 1;
    }
}
