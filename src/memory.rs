use isa::{self, Instruction};
use binary::{Binary};

#[derive(Debug)]
pub enum MemoryError {
    InvalidAddress,
    CacheMiss {
        stall_cycles: u32,
    },
}

pub type Result<T> = ::std::result::Result<T, MemoryError>;

pub trait MemoryInterface {
    const LATENCY: u32;

    fn read_word(&self, address: isa::Address) -> Result<isa::Word>;
    fn write_word(&mut self, address: isa::Address, value: isa::Word) -> Result<()>;

    // fn read_halfword(&self, address: isa::Address) -> Result<isa::HalfWord>;
    // fn write_halfword(&self, address: isa::Address) -> Result<()>;

    // fn read_byte(&self, address: isa::Address) -> Result<isa::Byte>;
    // fn write_byte(&self, address: isa::Address) -> Result<()>;
}

pub struct Mmu<T: MemoryInterface> {
    memory: T,
}

pub struct Memory {
    memory: Vec<u32>,
}

#[derive(Clone)]
struct FetchRequest {
    cycles_left: u32,
}

#[derive(Clone)]
struct CacheBlock {
    valid: bool,
    tag: u32,
    contents: Vec<u32>,
    fetch_request: Option<FetchRequest>,
}

type CacheSet = Vec<CacheBlock>;

// TODO: probably want different caches for different strategies, and
// investigate how LRU is implemented
// TODO: use hashtable for a way?
// TODO: hashtable-based FA cache?
pub struct Cache {
    num_sets: usize,
    num_ways: usize,
    block_words: usize,
    cache: Vec<CacheSet>,
}

impl Memory {
    pub fn new(size: isa::Address, binary: Binary) -> Memory {
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

    pub fn read_instruction(&self, pc: isa::Address) -> Option<Instruction> {
        self.memory.get((pc / 4) as usize)
            .map(Clone::clone)
            .map(Instruction::new)
    }
}

impl MemoryInterface for Memory {
    const LATENCY: u32 = 100;

    fn read_word(&self, address: isa::Address) -> Result<isa::Word> {
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
}

impl Cache {
    pub fn new(sets: usize, ways: usize, block_words: usize) -> Cache {
        let set = vec![CacheBlock {
            valid: false,
            tag: 0,
            contents: vec![0; block_words],
            fetch_request: None,
        }; ways];
        Cache {
            num_sets: sets,
            num_ways: ways,
            block_words: block_words,
            cache: vec![set; sets],
        }
    }

    fn parse_address(&self, address: isa::Address) -> (u32, u32, u32) {
        // TODO: use constant in ISA module for word->byte conversion
        let offset_mask = (self.block_words * 4 - 1) as u32;
        let offset = address & offset_mask;
        let index_mask = (self.num_sets - 1) as u32;
        let index_shift = 32 - (self.block_words * 4).leading_zeros();
        let index = (address >> index_shift) & index_mask;
        let tag_shift = index_shift + (32 - self.num_sets.leading_zeros());
        let tag = address >> tag_shift;

        (tag, index, offset)
    }

    fn prefetch(&mut self, address: isa::Address) {

    }

    fn invalidate(&mut self, address: isa::Address) {

    }
}

impl MemoryInterface for Cache {
    const LATENCY: u32 = 1;

    fn read_word(&self, address: isa::Address) -> Result<isa::Word> {
        Err(MemoryError::InvalidAddress)
    }

    fn write_word(&mut self, address: isa::Address, value: isa::Word)
                  -> Result<()> {
        Err(MemoryError::InvalidAddress)
    }

}
