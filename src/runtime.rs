use std::io::Write;
use std::slice;

const DEBUG: bool = false;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectHeader {
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
    // Data segment opcodes
    BeginStringData(u64), // take the next many bytes of string data
    StringData(u64),

    // Text segment opcodes
    MakeInt(i64),
    MakeFloat(f64),
    MakeBool(bool),
    AddFloat,
    AddInt,
    SubFloat,
    SubInt,
    PushNone,
    Pop,
    GetGlobal { stack_offset: u32 },
    SetGlobal { stack_offset: u32 },
    GetLocal { stack_offset: i32 },
    SetLocal { stack_offset: i32 },
    HeapRead { offset: u32 },
    HeapWrite { offset: u32 },
    HeapAlloc { header: ObjectHeader },
    Return,
    Call(u32),      // absolute address
    JumpIf(u32),    // absolute address
    JumpNotIf(u32), // absolute address
    Jump(u32),      // absolute address
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
const STRING_TYPE_INDEX: u32 = 3;
pub const STACK_FRAME_TYPE_INDEX: u32 = 4;

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
        if DEBUG {
            println!("DEBUG: {:?}", op);
        }

        use Opcode::*;
        match op {
            BeginStringData(_) | StringData(_) => {
                panic!("StringDdata not supported in text section");
            }
            MakeInt(int) => self.make_int(int),
            MakeFloat(float) => self.make_float(float),
            MakeBool(boolean) => {
                self.heap.push(BOOL_HEADER.to_bits());
                let ret_val = self.heap.len();
                self.heap.push(boolean as u64);
                self.stack.push(ret_val);
            }
            SubFloat => {
                let float2 = f64::from_bits(self.heap[self.stack.pop().unwrap()]);
                let float1 = f64::from_bits(self.heap[self.stack.pop().unwrap()]);
                self.make_float(float1 - float2);
            }
            SubInt => {
                let int2 = self.heap[self.stack.pop().unwrap()] as i64;
                let int1 = self.heap[self.stack.pop().unwrap()] as i64;
                self.make_int(int1 - int2);
            }
            AddFloat => {
                let float2 = f64::from_bits(self.heap[self.stack.pop().unwrap()]);
                let float1 = f64::from_bits(self.heap[self.stack.pop().unwrap()]);
                self.make_float(float1 + float2);
            }
            AddInt => {
                let int2 = self.heap[self.stack.pop().unwrap()] as i64;
                let int1 = self.heap[self.stack.pop().unwrap()] as i64;
                self.make_int(int1 + int2);
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
            HeapRead { offset } => {
                let ptr = self.stack.pop().unwrap();
                self.stack.push(self.heap[ptr + offset as usize] as usize);
            }
            HeapWrite { offset } => {
                let ptr = self.stack.pop().unwrap();
                let value = self.stack.pop().unwrap();
                self.heap[ptr + offset as usize] = value as u64;
            }
            HeapAlloc { header } => {
                self.heap.push(header.to_bits());
                let ret_val = self.heap.len();
                for _ in 0..header.object_size {
                    self.heap.push(NONE_VALUE as u64);
                }
                self.stack.push(ret_val);
            }
            PushNone => {
                self.stack.push(NONE_VALUE);
            }
            Jump(address) => {
                self.pc = address as usize;
                return;
            }
            JumpNotIf(address) => {
                let arg = self.stack.pop().unwrap();

                if !self.eval_bool(arg) {
                    self.pc = address as usize;
                    return;
                }
            }
            JumpIf(address) => {
                let arg = self.stack.pop().unwrap();

                if self.eval_bool(arg) {
                    self.pc = address as usize;
                    return;
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
                while self.stack.len() > self.fp {
                    self.stack.pop();
                }

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
                            let float_value = f64::from_bits(arg_value);
                            if float_value as i64 as f64 == float_value {
                                write!(self.stdout, "{:.p$}\n", f64::from_bits(arg_value), p = 1)
                                    .expect("should not have failed");
                            } else {
                                write!(self.stdout, "{}\n", f64::from_bits(arg_value))
                                    .expect("should not have failed");
                            }
                        }
                        BOOL_HEADER => {
                            let value = if arg_value != 0 { "True" } else { "False" };
                            write!(self.stdout, "{}\n", value).expect("should not have failed");
                        }
                        ObjectHeader {
                            type_index: STRING_TYPE_INDEX,
                            object_size,
                        } => {
                            let str_begin = (&self.heap[arg]) as *const u64 as *const u8;
                            let str_bytes =
                                unsafe { slice::from_raw_parts(str_begin, object_size as usize) };
                            write!(self.stdout, "{}\n", unsafe {
                                std::str::from_utf8_unchecked(str_bytes)
                            })
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

    fn make_int(&mut self, value: i64) {
        self.heap.push(INT_HEADER.to_bits());
        let ret_val = self.heap.len();
        self.heap.push(value as u64);
        self.stack.push(ret_val);
    }

    fn make_float(&mut self, value: f64) {
        self.heap.push(FLOAT_HEADER.to_bits());
        let ret_val = self.heap.len();
        self.heap.push(value.to_bits());
        self.stack.push(ret_val);
    }

    fn eval_bool(&self, value: usize) -> bool {
        if value == NONE_VALUE {
            return true;
        }
        return match self.get_obj_header(value) {
            INT_HEADER | BOOL_HEADER => self.heap[value] != 0,
            FLOAT_HEADER => f64::from_bits(self.heap[value]) != 0.0,
            x => {
                panic!("attempting to use value as boolean with type: {:?}\n", x);
            }
        };
    }
}
