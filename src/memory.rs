use isa::{Instruction};

pub struct Memory {
    memory: Vec<u32>,
}

impl Memory {
    pub fn new(size: usize) -> Memory {
        Memory {
            memory: Vec::with_capacity(size),
        }
    }

    pub fn read_word(&self, address: usize) -> Option<u32> {
        self.memory.get(address).map(Clone::clone)
    }

    pub fn read_instruction(&self, pc: usize) -> Option<Instruction> {
        self.memory.get(pc).map(Clone::clone).map(Instruction::new)
    }
}
