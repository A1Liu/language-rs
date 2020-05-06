pub struct Runtime {
    pub stack: Vec<usize>,
    pub heap: Vec<u64>,
    pub fp_ra_stack: Vec<usize>,
    pub fp: usize,
    pub pc: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum Opcode {
    MakeInt(i64),
    MakeFloat(f64),
    AddFloat,
    AddInt,
    PushNone,
    Pop,
    GetLocal { stack_offset: i32 },
    SetLocal { stack_offset: i32 },
    Return,
    Call(u32),
    ECall,
}

pub const NONE_VALUE: usize = !0;

pub const INT_TYPE: u64 = 0;
pub const FLOAT_TYPE: u64 = 1;

pub const PRINT_PRIMITIVE: u64 = 0;
pub const FLOAT_CAST: u64 = 1;

impl Runtime {
    pub fn new() -> Self {
        return Self {
            stack: Vec::new(), // dummy frame pointer value
            heap: Vec::new(),
            fp_ra_stack: vec![NONE_VALUE, 0],
            fp: 0,
            pc: 0,
        };
    }

    pub fn run(&mut self, code: &Vec<Opcode>) {
        while self.pc != NONE_VALUE {
            self.run_op(code[self.pc]);
        }
    }

    pub fn run_op(&mut self, op: Opcode) {
        use Opcode::*;
        match op {
            MakeInt(int) => {
                self.heap.push(INT_TYPE);
                let ret_val = self.heap.len();
                self.heap.push(int as u64);
                self.stack.push(ret_val);
            }
            MakeFloat(float) => {
                self.heap.push(FLOAT_TYPE);
                let ret_val = self.heap.len();
                self.heap.push(float.to_bits());
                self.stack.push(ret_val);
            }
            AddFloat => {
                let float2 = f64::from_bits(self.heap[self.stack.pop().unwrap()]);
                let float1 = f64::from_bits(self.heap[self.stack.pop().unwrap()]);
                self.heap.push(FLOAT_TYPE);
                let ret_val = self.heap.len();
                self.heap.push((float1 + float2).to_bits());
                self.stack.push(ret_val);
            }
            AddInt => {
                let int2 = self.heap[self.stack.pop().unwrap()] as i64;
                let int1 = self.heap[self.stack.pop().unwrap()] as i64;
                self.heap.push(INT_TYPE);
                let ret_val = self.heap.len();
                self.heap.push((int1 + int2) as u64);
                self.stack.push(ret_val);
            }
            Pop => {
                self.stack.pop();
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
                    let type_id = self.heap[arg - 1];
                    let arg_value = self.heap[arg];

                    match type_id {
                        INT_TYPE => {
                            println!("{}", arg_value as i64);
                        }
                        FLOAT_TYPE => {
                            println!("{}", f64::from_bits(arg_value));
                        }
                        x => {
                            println!("{}", x);
                            panic!();
                        }
                    }
                    self.stack.push(NONE_VALUE);
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
