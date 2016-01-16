// Copyright 2015-2016 David Li
// This file is part of rustv.

// rustv is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// rustv is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with rustv.  If not, see <http://www.gnu.org/licenses/>.

use std::fmt;
use std::ops;

pub mod opcodes;
pub mod funct3;
pub mod funct7;

macro_rules! isa_type_op {
    ($name: ident, $ty: ty, $op: ident, $op_name: ident) => {
        impl ops::$op<$name> for $name {
            type Output = $name;

            fn $op_name(self, _rhs: $name) -> $name {
                $name(ops::$op::$op_name(self.0, _rhs.0))
            }
        }

        impl ops::$op<$ty> for $name {
            type Output = $name;

            fn $op_name(self, _rhs: $ty) -> $name {
                $name(ops::$op::$op_name(self.0, _rhs))
            }
        }
    }
}

macro_rules! isa_type_assign_op {
    ($name: ident, $ty: ty, $op: ident, $op_name: ident) => {
        impl ops::$op<$name> for $name {
            fn $op_name(&mut self, _rhs: $name) {
                ops::$op::$op_name(&mut self.0, _rhs.0)
            }
        }

        impl ops::$op<$ty> for $name {
            fn $op_name(&mut self, _rhs: $ty) {
                ops::$op::$op_name(&mut self.0, _rhs)
            }
        }
    }
}

macro_rules! isa_type {
    ($name: ident, $utype: ty) => {
        #[derive(Clone,Copy,Debug,Eq,Hash,Ord,PartialEq,PartialOrd)]
        pub struct $name(pub $utype);

        impl $name {
            pub fn wrapping_add(self, rhs: Self) -> Self {
                $name(self.0.wrapping_add(rhs.0))
            }

            pub fn wrapping_sub(self, rhs: Self) -> Self {
                $name(self.0.wrapping_sub(rhs.0))
            }

        }

        isa_type_op!($name, $utype, Add, add);
        isa_type_assign_op!($name, $utype, AddAssign, add_assign);
        isa_type_op!($name, $utype, Sub, sub);
        isa_type_op!($name, $utype, Mul, mul);
        isa_type_op!($name, $utype, Div, div);
        isa_type_op!($name, $utype, Rem, rem);
        isa_type_op!($name, $utype, Shr, shr);
        isa_type_op!($name, $utype, Shl, shl);
        isa_type_op!($name, $utype, BitAnd, bitand);
        isa_type_op!($name, $utype, BitOr, bitor);
        isa_type_op!($name, $utype, BitXor, bitxor);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                write!(f, "{}", self.0)
            }
        }

        impl fmt::LowerHex for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                write!(f, "{:x}", self.0)
            }
        }

        impl fmt::UpperHex for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                write!(f, "{:X}", self.0)
            }
        }
    }
}

pub trait IsaType {
    type Unsigned;
    type Signed;

    fn as_signed(self) -> Self::Signed;
    fn as_signed_word(self) -> SignedWord;
    fn as_word(self) -> Word;
    fn as_half_word(self) -> HalfWord;
    fn as_byte(self) -> Byte;
    fn as_address(self) -> Address;

    /// Converts the type into bytes, LSB-first.
    fn as_bytes(self) -> Vec<Byte>;
}

macro_rules! isa_utype {
    ($name: ident, $signed: ident, $utype: ty, $stype: ty) => {
        impl IsaType for $name {
            type Unsigned = $name;
            type Signed = $signed;

            fn as_signed(self) -> Self::Signed {
                $signed(self.0 as $stype)
            }

            fn as_signed_word(self) -> SignedWord {
                // Convert self to signed so that second cast will
                // sign-extend
                SignedWord((self.0 as $stype) as i32)
            }

            fn as_word(self) -> Word {
                Word(self.0 as u32)
            }

            fn as_half_word(self) -> HalfWord {
                HalfWord(self.0 as u16)
            }

            fn as_byte(self) -> Byte {
                Byte(self.0 as u8)
            }

            fn as_address(self) -> Address {
                self.as_word()
            }

            fn as_bytes(self) -> Vec<Byte> {
                use std::mem;

                let mut bytes = vec![];
                for offset in 0..mem::size_of::<$utype>() {
                    bytes.push(Byte(((self.0 >> (8 * offset)) & 0xFF) as u8));
                }

                bytes
            }
        }

        impl IsaType for $signed {
            type Unsigned = $name;
            type Signed = $signed;

            fn as_signed(self) -> Self::Signed {
                self
            }

            fn as_signed_word(self) -> SignedWord {
                SignedWord(self.0 as i32)
            }

            fn as_word(self) -> Word {
                Word(self.0 as u32)
            }

            fn as_half_word(self) -> HalfWord {
                HalfWord(self.0 as u16)
            }

            fn as_byte(self) -> Byte {
                Byte(self.0 as u8)
            }

            fn as_address(self) -> Address {
                self.as_word()
            }

            fn as_bytes(self) -> Vec<Byte> {
                use std::mem;

                let mut bytes = vec![];
                for offset in 0..mem::size_of::<$utype>() {
                    bytes.push(Byte((self.0 >> (8 * offset)) as u8));
                }

                bytes
            }
        }
    }
}

isa_type!(Word, u32);
isa_type!(SignedWord, i32);
isa_utype!(Word, SignedWord, u32, i32);
isa_type!(HalfWord, u16);
isa_type!(SignedHalfWord, i16);
isa_utype!(HalfWord, SignedHalfWord, u16, i16);
isa_type!(Byte, u8);
isa_type!(SignedByte, i8);
isa_utype!(Byte, SignedByte, u8, i8);

pub type Address = Word;

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
    word: Word,
}

impl Instruction {
    pub fn new(word: Word) -> Instruction {
        Instruction {
            word: word,
        }
    }

    pub fn opcode(&self) -> u32 {
        (self.word & 0x7F).0
    }

    pub fn rd(&self) -> Register {
        Register::from_num(((self.word >> 7) & 0x1F).0)
    }

    pub fn funct3(&self) -> u32 {
        ((self.word >> 12) & 0x7).0
    }

    pub fn funct7(&self) -> u32 {
        ((self.word >> 25) & 0x7F).0
    }

    pub fn shamt(&self) -> u32 {
        ((self.word >> 20) & 0x1F).0
    }

    pub fn rs1(&self) -> Register {
        Register::from_num(((self.word >> 15) & 0x1F).0)
    }

    pub fn rs2(&self) -> Register {
        Register::from_num(((self.word >> 20) & 0x1F).0)
    }

    pub fn i_imm(&self) -> SignedWord {
        (self.word.as_signed_word()) >> 20
    }

    pub fn s_imm(&self) -> SignedWord {
        let low = (self.word >> 7) & 0x1F;
        let high = ((self.word.as_signed_word()) >> 25).as_word();
        ((high << 5) | low).as_signed_word()
    }

    pub fn uj_imm(&self) -> SignedWord {
        // Want zero-extension
        let low1 = (self.word >> 21) & 0x3FF;
        let low11 = (self.word >> 20) & 0x1;
        let low12 = (self.word >> 12) & 0xFF;
        // Want sign-extension
        let low20 = ((self.word.as_signed_word()) >> 30).as_word();
        ((low20 << 20) | (low12 << 12) | (low11 << 11) | (low1 << 1)).as_signed_word()
    }

    pub fn sb_imm(&self) -> SignedWord {
        let low1 = (self.word >> 8) & 0xF;
        let low5 = (self.word >> 25) & 0x3F;
        let low11 = (self.word >> 7) & 0x1;
        let low12 = ((self.word.as_signed_word()) >> 31).as_word();
        ((low12 << 12) | (low11 << 11) | (low5 << 5) | (low1 << 1)).as_signed_word()
    }

    pub fn u_imm(&self) -> SignedWord {
        (self.word & 0xFFFFF000).as_signed_word()
    }
}
