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

    pub fn write_word(&mut self, address: usize, value: u32) -> Option<()> {
        let address = address / 4;
        if address >= self.memory.len() {
            None
        }
        else {
            self.memory[address] = value;
            Some(())
        }
    }

    pub fn read_instruction(&self, pc: usize) -> Option<Instruction> {
        self.memory.get(pc / 4).map(Clone::clone).map(Instruction::new)
    }
}
