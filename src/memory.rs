use isa::{Instruction};
use binary::{Binary};

pub struct Memory {
    memory: Vec<u32>,
}

impl Memory {
    pub fn new(size: usize, binary: Binary) -> Memory {
        let mut memory = binary.words.clone();
        if size > memory.len() {
            let remainder = size - memory.len();
            memory.reserve(remainder);
        }
        Memory {
            memory: memory,
        }
    }

    pub fn read_word(&self, address: usize) -> Option<u32> {
        // memory is word-addressed but addresses are byte-addressed
        self.memory.get(address / 4).map(Clone::clone)
    }

    pub fn read_instruction(&self, pc: usize) -> Option<Instruction> {
        self.memory.get(pc / 4).map(Clone::clone).map(Instruction::new)
    }
}
