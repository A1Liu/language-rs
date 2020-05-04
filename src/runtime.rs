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
    ECall,
}

pub const INT_TYPE: u64 = 0;
pub const FLOAT_TYPE: u64 = 1;

pub const PRINT: u64 = 0;
pub const FLOAT_CAST: u64 = 1;

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
            ECall => match self.heap[self.stack.pop().unwrap()] {
                PRINT => {
                    println!("print primitive ecall");
                }
                _ => {
                    println!("invalid ecall");
                    panic!();
                }
            },
        }
        // println!(":{}", self.stack.len());
    }
}
