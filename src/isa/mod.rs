pub mod opcodes;
pub mod funct3;

pub enum Register {
    X0 = 0,
    X1 = 1,
    X2 = 2,
    X3 = 3,
    X4 = 4,
    X5 = 5,
    X6 = 6,
    X7 = 7,
    X8 = 8,
    X9 = 9,
    X10 = 10,
    X11 = 11,
}

impl Register {
    pub fn as_num(self) -> usize {
        self as usize
    }
}

pub struct Instruction {
    word: u32,
}

impl Instruction {
    pub fn new(word: u32) -> Instruction {
        Instruction {
            word: word,
        }
    }

    pub fn opcode(&self) -> u32 {
        self.word & 0x7F
    }

    pub fn funct3(&self) -> u32 {
        (self.word >> 12) & 0x3
    }
}
