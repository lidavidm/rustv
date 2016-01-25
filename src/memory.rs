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

use std::rc::Rc;
use std::cell::RefCell;

use isa::{self, Instruction, IsaType};

#[derive(Clone, Debug, PartialEq)]
pub enum MemoryError {
    InvalidAddress,
    CacheMiss {
        /// How many cycles to stall
        stall_cycles: u32,
        /// Whether the load or store should be retried
        retry: bool,
    },
}

pub type Result<T> = ::std::result::Result<T, MemoryError>;

pub trait MemoryInterface {
    fn latency(&self) -> u32;

    fn step(&mut self);

    // fn prefetch(&mut self, address: isa::Address);
    // fn invalidate(&mut self, address: isa::Address);

    fn is_address_accessible(&self, address: isa::Address) -> bool;

    fn read_word(&mut self, address: isa::Address) -> Result<isa::Word>;
    fn write_word(&mut self, address: isa::Address, value: isa::Word) -> Result<()>;

    fn read_instruction(&mut self, address: isa::Address) -> Option<Instruction> {
        match self.read_word(address) {
            Ok(word) => Some(Instruction::new(word)),
            Err(_) => None,
        }
    }

    // TODO: check address more thoroughly

    fn read_halfword(&mut self, address: isa::Address) -> Result<isa::HalfWord> {
        let result = self.read_word(address);
        let offset = (address & 0b10).0;

        match result {
            Ok(word) => match offset {
                0 => Ok((word & 0xFFFF).as_half_word()),
                2 => Ok(((word & 0xFFFF0000) >> 16).as_half_word()),
                _ => panic!("Invalid halfword offset: address {:x}", address),
            },
            Err(e) => Err(e),
        }
    }

    fn write_halfword(&mut self, address: isa::Address, value: isa::HalfWord) -> Result<()> {
        let result = self.read_word(address);
        let offset = (address & 0b10).0;
        let value = value.as_word();

        match result {
            Ok(word) => {
                let value = match offset {
                    0 => (word & 0xFFFF0000) | value,
                    2 => (word & 0x0000FFFF) | (value << 16),
                    _ => panic!("Invalid halfword offset: address {:x}", address),
                };
                self.write_word(address, value)
            },
            Err(e) => Err(e),
        }
    }

    fn read_byte(&mut self, address: isa::Address) -> Result<isa::Byte> {
        let result = self.read_word(address);
        let offset = (address % 4).0;

        match result {
            Ok(word) => match offset {
                0 => Ok((word & 0xFF).as_byte()),
                1 => Ok(((word & 0xFF00) >> 8).as_byte()),
                2 => Ok(((word & 0xFF0000) >> 16).as_byte()),
                3 => Ok(((word & 0xFF000000) >> 24).as_byte()),
                _ => panic!("Invalid byte offset: {:x}", address),
            },
            Err(e) => Err(e),
        }
    }

    fn write_byte(&mut self, address: isa::Address, value: isa::Byte) -> Result<()> {
        let result = self.read_word(address);
        let offset = (address % 4).0;
        let value = value.as_word();

        match result {
            Ok(word) => {
                let value = match offset {
                    0 => (word & !(0xFF)) | value,
                    1 => (word & !(0xFF00)) | (value << 8),
                    2 => (word & !(0xFF0000)) | (value << 16),
                    3 => (word & !(0xFF000000)) | (value << 24),
                    _ => panic!("Invalid byte offset: {:x}", address),
                };
                self.write_word(address, value)
            },
            Err(e) => Err(e),
        }
    }
}


pub type SharedMemory<'a> = Rc<RefCell<MemoryInterface + 'a>>;

// TODO: rename to BijectiveMmu?
pub trait Mmu {
    fn translate(&self, address: isa::Address) -> isa::Address;
}

pub struct IdentityMmu {}
pub struct ReverseMmu {
    top: isa::Address,
}

impl IdentityMmu {
    pub fn new() -> IdentityMmu {
        IdentityMmu {}
    }
}

impl Mmu for IdentityMmu {
    fn translate(&self, address: isa::Address) -> isa::Address {
        address
    }
}

impl ReverseMmu {
    pub fn new(top: isa::Address) -> ReverseMmu {
        ReverseMmu {
            top: top,
        }
    }
}

impl Mmu for ReverseMmu {
    fn translate(&self, address: isa::Address) -> isa::Address {
        let offset = address % 4;
        (self.top - 4 - (address - offset)) + offset
    }
}

pub struct Memory {
    memory: Vec<u32>,
}

fn copy_u8_into_u32<T: Mmu>(mmu: &T, base: usize, src: &[u8], dst: &mut [u32]) {
    for (offset, word) in src.chunks(4).enumerate() {
        let word = if word.len() == 4 {
            (word[0] as u32) |
            ((word[1] as u32) << 8) |
            ((word[2] as u32) << 16) |
            ((word[3] as u32) << 24)
        }
        else if word.len() == 3 {
            (word[0] as u32) |
            ((word[1] as u32) << 8) |
            ((word[2] as u32) << 16)
        }
        else if word.len() == 2 {
            (word[0] as u32) |
            ((word[1] as u32) << 8)
        }
        else {
            word[0] as u32
        };

        let addr = isa::Word((base as u32) + ((4 * offset) as u32));
        let addr = mmu.translate(addr) / 4;
        dst[addr.0 as usize] = word;
    }
}

impl Memory {
    pub fn new(size: usize) -> Memory {
        Memory {
            memory: vec![0; size as usize],
        }
    }

    pub fn write_segment<T: Mmu>(&mut self, mmu: &T,
                                 data: &[u8], offset: usize) {
        copy_u8_into_u32(mmu, offset, data, &mut self.memory);
    }
}

impl MemoryInterface for Memory {
    fn latency(&self) -> u32 {
        100
    }

    fn step(&mut self) {}

    fn is_address_accessible(&self, address: isa::Address) -> bool {
        ((address / 4).0 as usize) < self.memory.len()
    }

    fn read_word(&mut self, address: isa::Address) -> Result<isa::Word> {
        // memory is word-addressed but addresses are byte-addressed
        self.memory.get((address / 4).0 as usize)
            .map(Clone::clone)
            .map(isa::Word)
            .ok_or(MemoryError::InvalidAddress)
    }

    fn write_word(&mut self, address: isa::Address, value: isa::Word)
                  -> Result<()> {
        let address = (address / 4).0 as usize;
        if address >= self.memory.len() || address <= 0 {
            Err(MemoryError::InvalidAddress)
        }
        else {
            self.memory[address] = value.0;
            Ok(())
        }
    }

    fn read_instruction(&mut self, pc: isa::Address) -> Option<Instruction> {
        self.memory.get((pc / 4).0 as usize)
            .map(Clone::clone)
            .map(isa::Word)
            .map(Instruction::new)
    }
}
