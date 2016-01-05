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

use isa::{self, Instruction};
use binary::{Binary};

#[derive(Clone, Debug, PartialEq)]
pub enum MemoryError {
    InvalidAddress,
    CacheMiss {
        stall_cycles: u32,
    },
}

pub type Result<T> = ::std::result::Result<T, MemoryError>;

pub trait MemoryInterface {
    fn latency(&self) -> u32;

    fn step(&mut self);

    // fn prefetch(&mut self, address: isa::Address);
    // fn invalidate(&mut self, address: isa::Address);

    fn read_word(&mut self, address: isa::Address) -> Result<isa::Word>;
    fn write_word(&mut self, address: isa::Address, value: isa::Word) -> Result<()>;

    fn read_instruction(&mut self, address: isa::Address) -> Option<Instruction> {
        match self.read_word(address / 4) {
            Ok(word) => Some(Instruction::new(word)),
            Err(_) => None,
        }
    }

    // fn read_halfword(&self, address: isa::Address) -> Result<isa::HalfWord>;
    // fn write_halfword(&self, address: isa::Address) -> Result<()>;

    fn read_byte(&mut self, address: isa::Address) -> Result<isa::Byte> {
        let result = self.read_word(address);
        let offset = address % 4;

        match result {
            Ok(word) => match offset {
                0 => Ok((word & 0xFF) as isa::Byte),
                1 => Ok(((word & 0xFF00) >> 8) as isa::Byte),
                2 => Ok(((word & 0xFF0000) >> 16) as isa::Byte),
                3 => Ok(((word & 0xFF000000) >> 24) as isa::Byte),
                _ => panic!(""),
            },
            Err(e) => Err(e),
        }
    }

    fn write_byte(&mut self, address: isa::Address, value: isa::Byte) -> Result<()> {
        let offset = address % 4;

        let result = self.read_word(address);
        let value = value as isa::Word;

        match result {
            Ok(word) => {
                let value = match offset {
                    0 => (word & !(0xFF)) | value,
                    1 => (word & !(0xFF00)) | (value << 8),
                    2 => (word & !(0xFF0000)) | (value << 16),
                    3 => (word & !(0xFF000000)) | (value << 24),
                    _ => panic!(""),
                };
                self.write_word(address, value)
            },
            Err(e) => Err(e),
        }
    }
}

pub type SharedMemory<'a> = Rc<RefCell<Box<MemoryInterface + 'a>>>;

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
        self.top - address
    }
}

pub struct Memory {
    memory: Vec<u32>,
}

#[derive(Clone)]
struct FetchRequest {
    address: isa::Address,
    prefetch: bool, // is this a prefetch
    cycles_left: u32,
    tag: u32,
    data: Vec<u32>, // hold data temporarily while we wait for an entire line
    error: Option<MemoryError>, // in case next level returns an error
    waiting_on: u32, // which word of the block are we waiting on
}

#[derive(Clone)]
struct CacheBlock {
    valid: bool,
    tag: u32,
    contents: Vec<u32>,
    fetch_request: Option<FetchRequest>,
}

// TODO: probably want different caches for different strategies, and
// investigate how LRU is implemented
// TODO: use hashtable for a way?
// TODO: hashtable-based FA cache?
pub struct DirectMappedCache<'a> {
    num_sets: u32,
    block_words: u32,
    cache: Vec<CacheBlock>,
    next_level: SharedMemory<'a>,
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

        let addr = mmu.translate((base as isa::Address) + ((4 * offset) as isa::Address)) / 4;
        dst[addr as usize] = word;
    }
}

impl Memory {
    pub fn new(size: isa::Address) -> Memory {
        Memory {
            memory: vec![0; size as usize],
        }
    }

    pub fn new_from_binary(size: isa::Address, binary: Binary) -> Memory {
        let mut memory = binary.words.clone();
        let size = size as usize;
        if size > memory.len() {
            let remainder = size - memory.len();
            memory.reserve(remainder);
        }
        Memory {
            memory: memory,
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

    fn read_word(&mut self, address: isa::Address) -> Result<isa::Word> {
        // memory is word-addressed but addresses are byte-addressed
        self.memory.get((address / 4) as usize)
            .map(Clone::clone)
            .ok_or(MemoryError::InvalidAddress)
    }

    fn write_word(&mut self, address: isa::Address, value: isa::Word)
                  -> Result<()> {
        let address = (address / 4) as usize;
        if address >= self.memory.len() || address <= 0 {
            Err(MemoryError::InvalidAddress)
        }
        else {
            self.memory[address] = value;
            Ok(())
        }
    }

    fn read_instruction(&mut self, pc: isa::Address) -> Option<Instruction> {
        self.memory.get((pc / 4) as usize)
            .map(Clone::clone)
            .map(Instruction::new)
    }
}

impl<'a> DirectMappedCache<'a> {
    pub fn new(sets: u32, block_words: u32, next_level: SharedMemory<'a>)
               -> DirectMappedCache<'a> {
        let set = CacheBlock {
            valid: false,
            tag: 0,
            contents: vec![0; block_words as usize],
            fetch_request: None,
        };
        DirectMappedCache {
            num_sets: sets,
            block_words: block_words,
            cache: vec![set; sets as usize],
            next_level: next_level,
        }
    }

    pub fn parse_address(&self, address: isa::Address) -> (u32, u32, u32) {
        // TODO: use constant in ISA module for word->byte conversion
        let offset_mask = (self.block_words * 4 - 1) as u32;
        let offset = address & offset_mask;
        let index_mask = (self.num_sets - 1) as u32;
        let index_shift = 32 - (self.block_words * 4).leading_zeros() - 1;
        let index = (address >> index_shift) & index_mask;
        let tag_shift = index_shift + (32 - self.num_sets.leading_zeros()) - 1;
        let tag = address >> tag_shift;

        (tag, index, offset)
    }

    fn normalize_address(&self, address: isa::Address) -> isa::Address {
        let offset_mask = !(self.block_words * 4 - 1);
        address & offset_mask
    }
}

impl<'a> MemoryInterface for DirectMappedCache<'a> {
    fn latency(&self) -> u32 {
        0
    }

    fn step(&mut self) {
        for set in self.cache.iter_mut() {
            if let Some(ref mut fetch_request) = set.fetch_request {
                // Start filling the cache once the cycles_left would
                // have hit 0, so that the consumer never gets
                // stall_cycles = 0
                if fetch_request.cycles_left > 1 {
                    fetch_request.cycles_left -= 1;
                    return;
                }
                // read all the words in a line from the next
                // level, until we get a stall

                for offset in fetch_request.waiting_on..self.block_words {
                    let result = self.next_level
                        .borrow_mut()
                        .read_word(fetch_request.address + (4 * offset));
                    match result {
                        Ok(data) => {
                            fetch_request.data[offset as usize] = data;
                            fetch_request.waiting_on += 1;
                        },
                        Err(MemoryError::CacheMiss { stall_cycles }) => {
                            fetch_request.cycles_left = stall_cycles;
                            continue;
                        },
                        Err(MemoryError::InvalidAddress) => {
                            fetch_request.error =
                                Some(MemoryError::InvalidAddress);
                            continue;
                        }
                    }
                }

                // All words fetched, write to cache
                set.tag = fetch_request.tag;
                set.contents = fetch_request.data.clone();
                set.valid = true;
            }

            set.fetch_request = None;
        }
    }

    fn read_word(&mut self, address: isa::Address) -> Result<isa::Word> {
        let normalized = self.normalize_address(address);
        let (new_tag, _, _) = self.parse_address(address);
        let stall = self.next_level.borrow().latency();
        let (tag, index, offset) = self.parse_address(address);
        let ref mut set = self.cache[index as usize];

        if set.valid && set.tag == tag {
            return Ok(set.contents[(offset / 4) as usize]);
        }
        else if let None = set.fetch_request {
            set.fetch_request = Some(FetchRequest {
                address: normalized,
                prefetch: false,
                cycles_left: stall,
                tag: new_tag,
                data: vec![0; self.block_words as usize],
                error: None,
                waiting_on: 0,
            });
        }
        else if let Some(ref mut fetch_request) = set.fetch_request {
            if let Some(ref err) = fetch_request.error {
                if fetch_request.address == normalized {
                    return Err(err.clone());
                }
                else {
                    fetch_request.address = normalized;
                    fetch_request.prefetch = false;
                    fetch_request.cycles_left = stall;
                    fetch_request.tag = new_tag;
                    fetch_request.waiting_on = 0;
                }
            }
            // Do the assignment outside the borrow of the error
            fetch_request.error = None;

            return Err(MemoryError::CacheMiss {
                stall_cycles: fetch_request.cycles_left
            });
        }

        Err(MemoryError::CacheMiss {
            stall_cycles: stall,
        })
    }

    fn write_word(&mut self, address: isa::Address, value: isa::Word)
                  -> Result<()> {
        // Write-allocate policy
        match self.read_word(address) {
            Ok(_) => {
                let (tag, index, offset) = self.parse_address(address);
                let ref mut set = self.cache[index as usize];

                if set.valid && set.tag == tag {
                    set.contents[(offset / 4) as usize] = value;
                    // Write-through policy
                    let result = self.next_level.borrow_mut()
                        .write_word(address, value);
                    match result {
                        Ok(()) => Ok(()),
                        Err(e) => Err(e),
                    }
                }
                else {
                    panic!("Could not find supposedly read word");
                }
            },
            Err(e) => Err(e),
        }
    }
}
