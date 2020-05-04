pub struct Runtime<'a> {
    pub stack: Vec<usize>,
    pub heap: Vec<u64>,
    pub fp_stack: Vec<usize>,
    pub code: &'a Vec<Vec<Opcode>>,
    pub fp: usize,
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
    Call(u32),
}

pub const INT_TYPE: u64 = 0;
pub const FLOAT_TYPE: u64 = 1;

pub const PRINT_FUNC: u32 = 0;
pub const FLOAT_CONSTRUCTOR: u32 = 1;

impl<'a> Runtime<'a> {
    pub fn new(code: &'a Vec<Vec<Opcode>>) -> Self {
        return Self {
            stack: Vec::new(), // dummy frame pointer value
            heap: Vec::new(),
            fp_stack: Vec::new(),
            code,
            fp: 0,
        };
    }

    pub fn run(&mut self) {
        let main = &self.code[0];
        for op in main {
            self.run_op(*op);
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
            Call(func_id) => match func_id {
                PRINT_FUNC => {
                    self.print_func();
                }
                FLOAT_CONSTRUCTOR => {
                    self.float_constructor();
                }
                x => {}
            },
            GetLocal { stack_offset } => {
                self.stack
                    .push(self.stack[self.fp.wrapping_add(stack_offset as usize)]);
            }
            SetLocal { stack_offset } => {
                self.stack[self.fp.wrapping_add(stack_offset as usize)] = self.stack.pop().unwrap();
            }
            PushNone => {
                self.stack.push(!0);
            }
        }
        // println!(":{}", self.stack.len());
    }

    pub fn print_func(&mut self) {
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
    }

    pub fn float_constructor(&mut self) {
        let arg = self.stack.pop().unwrap();
        let type_id = self.heap[arg - 1];
        let arg_value = self.heap[arg];

        self.heap.push(FLOAT_TYPE);
        let ret_val = self.heap.len();

        match type_id {
            INT_TYPE => {
                self.heap.push((arg_value as i64 as f64).to_bits());
            }
            FLOAT_TYPE => {
                self.heap.push(arg_value);
            }
            _ => panic!(),
        }

        *self.stack.last_mut().unwrap() = ret_val;
    }
}
