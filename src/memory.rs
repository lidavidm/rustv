// Copyright 2015 David Li
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

#[derive(Clone, Debug)]
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

    // fn read_byte(&self, address: isa::Address) -> Result<isa::Byte>;
    // fn write_byte(&self, address: isa::Address) -> Result<()>;
}

pub type SharedMemory = Rc<RefCell<MemoryInterface>>;

// TODO: should be a trait
pub struct Mmu<T: MemoryInterface> {
    memory: T,
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
pub struct DirectMappedCache {
    num_sets: u32,
    block_words: u32,
    cache: Vec<CacheBlock>,
    next_level: SharedMemory,
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

impl DirectMappedCache {
    pub fn new(sets: u32, block_words: u32, next_level: SharedMemory)
               -> DirectMappedCache {
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

    pub fn prefetch(&mut self, address: isa::Address) {

    }

    pub fn invalidate(&mut self, address: isa::Address) {

    }
}

impl MemoryInterface for DirectMappedCache {
    fn latency(&self) -> u32 {
        100
    }

    fn step(&mut self) {
        for set in self.cache.iter_mut() {
            if let Some(ref mut fetch_request) = set.fetch_request {
                if fetch_request.cycles_left > 0 {
                    fetch_request.cycles_left -= 1;
                    continue;
                }
                else {
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
            }

            set.fetch_request = None;
        }
    }

    fn read_word(&mut self, address: isa::Address) -> Result<isa::Word> {
        let normalized = self.normalize_address(address);
        let (new_tag, _, _) = self.parse_address(address);
        let stall = self.latency();
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
                data: vec![0, self.block_words],
                error: None,
                waiting_on: 0,
            })
        }
        else if let Some(ref fetch_request) = set.fetch_request {
            if let Some(ref err) = fetch_request.error {
                // TODO: check to make sure the fetch request is for
                // this address, else just clear the request
                // TODO: clear the fetch request
                return Err(err.clone());
            }
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
        // XXX: temporary
        self.next_level.borrow_mut().write_word(address, value)
    }
}
