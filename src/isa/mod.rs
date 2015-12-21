pub mod opcodes;
pub mod funct3;
pub mod funct7;

pub type Word = u32;
pub type SignedWord = i32;

// TODO: directly encode PC as u32, as architecturally specified
pub type Address = usize;

#[derive(Debug, PartialEq)]
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
    X12 = 12,
    X13 = 13,
    X14 = 14,
    X15 = 15,
    X16 = 16,
    X17 = 17,
    X18 = 18,
    X19 = 19,
    X20 = 20,
    X21 = 21,
    X22 = 22,
    X23 = 23,
    X24 = 24,
    X25 = 25,
    X26 = 26,
    X27 = 27,
    X28 = 28,
    X29 = 29,
    X30 = 30,
    X31 = 31,
}

impl Register {
    pub fn as_num(self) -> usize {
        self as usize
    }

    pub fn from_num(num: u32) -> Register {
        match num {
            0 => Register::X0,
            1 => Register::X1,
            2 => Register::X2,
            3 => Register::X3,
            4 => Register::X4,
            5 => Register::X5,
            6 => Register::X6,
            7 => Register::X7,
            8 => Register::X8,
            9 => Register::X9,
            10 => Register::X10,
            11 => Register::X11,
            12 => Register::X12,
            13 => Register::X13,
            14 => Register::X14,
            15 => Register::X15,
            16 => Register::X16,
            17 => Register::X17,
            18 => Register::X18,
            19 => Register::X19,
            20 => Register::X20,
            21 => Register::X21,
            22 => Register::X22,
            23 => Register::X23,
            24 => Register::X24,
            25 => Register::X25,
            26 => Register::X26,
            27 => Register::X27,
            28 => Register::X28,
            29 => Register::X29,
            30 => Register::X30,
            31 => Register::X31,
            _ => panic!("Invalid register number: {}", num),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Instruction {
    // TODO: rename word to something correct - instructions are not always a
    // word
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

    pub fn rd(&self) -> Register {
        Register::from_num((self.word >> 7) & 0x1F)
    }

    pub fn funct3(&self) -> u32 {
        (self.word >> 12) & 0x3
    }

    pub fn funct7(&self) -> u32 {
        (self.word >> 25) & 0x7F
    }

    pub fn shamt(&self) -> u32 {
        (self.word >> 20) & 0x1F
    }

    pub fn rs1(&self) -> Register {
        Register::from_num((self.word >> 15) & 0x1F)
    }

    pub fn rs2(&self) -> Register {
        Register::from_num((self.word >> 20) & 0x1F)
    }

    pub fn i_imm(&self) -> SignedWord {
        (self.word as SignedWord) >> 20
    }

    pub fn s_imm(&self) -> SignedWord {
        let low = (self.word >> 7) & 0x1F;
        let high = (((self.word as SignedWord) >> 25) & 0x7F) as Word;
        ((high << 7) | low) as SignedWord
    }

    pub fn uj_imm(&self) -> SignedWord {
        // Want zero-extension
        let low1 = (self.word >> 21) & 0x3FF;
        let low11 = (self.word >> 20) & 0x1;
        let low12 = (self.word >> 12) & 0xFF;
        // Want sign-extension
        let low20 = ((self.word as SignedWord) >> 30) as Word;
        ((low20 << 20) | (low12 << 12) | (low11 << 11) | (low1 << 1)) as SignedWord
    }

    pub fn sb_imm(&self) -> SignedWord {
        let low1 = (self.word >> 8) & 0xF;
        let low5 = (self.word >> 25) & 0x3F;
        let low11 = (self.word >> 7) & 0x1;
        let low12 = ((self.word as SignedWord) >> 31) as Word;
        ((low12 << 12) | (low11 << 11) | (low5 << 5) | (low1 << 1)) as SignedWord
    }
}
