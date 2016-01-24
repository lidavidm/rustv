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

use isa;
use memory::{MemoryError, MemoryInterface, Result, SharedMemory};

pub struct CacheMetadata {
    /// How many sets are in the cache
    pub num_sets: usize,
    /// How many ways are in a set
    pub num_ways: usize,
    /// How many words are in a block/line
    pub num_block_words: usize,
    /// The tags currently in the cache, in order of set, then way
    pub tags: Vec<Option<isa::Address>>,
}

pub trait CacheInterface : MemoryInterface {
    fn cache_metadata(&self) -> CacheMetadata;
}

pub type SharedCache<'a> = Rc<RefCell<CacheInterface + 'a>>;

#[derive(Clone,Copy)]
pub struct CacheLocation {
    pub tag: u32,
    pub index: u32,
    pub offset: u32,
    pub way: u32,
}

#[derive(Clone)]
struct FetchRequest {
    address: isa::Address,
    prefetch: bool, // is this a prefetch
    cycles_left: u32,
    location: CacheLocation,
    data: Vec<isa::Word>, // hold data temporarily while we wait for an entire line
    error: Option<MemoryError>, // in case next level returns an error
    waiting_on: u32, // which word of the block are we waiting on
}

#[derive(Clone)]
struct CacheBlock {
    valid: bool,
    tag: u32,
    contents: Vec<isa::Word>,
    fetch_request: Option<FetchRequest>,
}

pub trait CacheEventHandler {
    fn block_fetched(&self, location: CacheLocation);
}

pub struct EmptyCacheEventHandler {}

impl CacheEventHandler for EmptyCacheEventHandler {
    fn block_fetched(&self, _: CacheLocation) {}
}

// TODO: probably want different caches for different strategies, and
// investigate how LRU is implemented
// TODO: use hashtable for a way?
// TODO: hashtable-based FA cache?
pub struct DirectMappedCache<'a, T: CacheEventHandler> {
    num_sets: u32,
    block_words: u32,
    cache: Vec<CacheBlock>,
    next_level: SharedMemory<'a>,
    events: T,
}

impl<'a, T: CacheEventHandler> DirectMappedCache<'a, T> {
    pub fn new(sets: u32, block_words: u32,
               next_level: SharedMemory<'a>, events: T)
               -> DirectMappedCache<'a, T> {
        let set = CacheBlock {
            valid: false,
            tag: 0,
            contents: vec![isa::Word(0); block_words as usize],
            fetch_request: None,
        };
        DirectMappedCache {
            num_sets: sets,
            block_words: block_words,
            cache: vec![set; sets as usize],
            next_level: next_level,
            events: events,
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

        (tag.0, index.0, offset.0)
    }

    fn normalize_address(&self, address: isa::Address) -> isa::Address {
        let offset_mask = !(self.block_words * 4 - 1);
        address & offset_mask
    }
}

impl<'a, T: CacheEventHandler> MemoryInterface for DirectMappedCache<'a, T> {
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
                        Err(MemoryError::CacheMiss { stall_cycles, .. }) => {
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
                self.events.block_fetched(fetch_request.location);

                set.tag = fetch_request.location.tag;
                set.contents = fetch_request.data.clone();
                set.valid = true;
            }

            set.fetch_request = None;
        }
    }

    fn is_address_accessible(&self, address: isa::Address) -> bool {
        let (tag, index, _) = self.parse_address(address);
        let ref set = self.cache[index as usize];

        set.valid && set.tag == tag
    }

    fn read_word(&mut self, address: isa::Address) -> Result<isa::Word> {
        let normalized = self.normalize_address(address);
        let stall = self.next_level.borrow().latency();
        let (tag, index, offset) = self.parse_address(address);
        let location = CacheLocation {
            tag: tag,
            index: index,
            offset: offset,
            way: 0,
        };
        let ref mut set = self.cache[index as usize];

        if set.valid && set.tag == tag {
            return Ok(set.contents[(offset / 4) as usize]);
        }
        else if let None = set.fetch_request {
            set.fetch_request = Some(FetchRequest {
                address: normalized,
                prefetch: false,
                cycles_left: stall,
                location: location,
                data: vec![isa::Word(0); self.block_words as usize],
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
                    fetch_request.location = location;
                    fetch_request.waiting_on = 0;
                }
            }
            // Do the assignment outside the borrow of the error
            fetch_request.error = None;

            return Err(MemoryError::CacheMiss {
                stall_cycles: fetch_request.cycles_left,
                retry: true,
            });
        }

        Err(MemoryError::CacheMiss {
            stall_cycles: stall,
            retry: true,
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

impl<'a, T: CacheEventHandler> CacheInterface for DirectMappedCache<'a, T> {
    fn cache_metadata(&self) -> CacheMetadata {
        let tags = {
            let mut tags = Vec::new();

            for set in self.cache.iter() {
                if set.valid {
                    tags.push(Some(isa::Word(set.tag)));
                }
                else {
                    tags.push(None);
                }
            }

            tags
        };

        CacheMetadata {
            num_sets: self.num_sets as usize,
            num_ways: 1,
            num_block_words: self.block_words as usize,
            tags: tags,
        }
    }
}
