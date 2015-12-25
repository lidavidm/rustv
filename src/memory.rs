use isa::{self, Instruction};
use binary::{Binary};

#[derive(Debug)]
pub enum MemoryError {
    InvalidAddress,
    CacheMiss,
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
struct CacheBlock {
    valid: bool,
    tag: u32,
    contents: Vec<u32>,
}

// TODO: probably want different caches for different strategies, and
// investigate how LRU is implemented
// TODO: use hashtable for a way?
// TODO: hashtable-based FA cache?
pub struct Cache {
    num_sets: usize,
    num_ways: usize,
    block_words: usize,
    cache: Vec<Vec<CacheBlock>>,
}

impl Memory {
    pub fn new(size: isa::Address, binary: Binary) -> Memory {
        let mut memory = binary.words.clone();
        if size > memory.len() {
            let remainder = size - memory.len();
            memory.reserve(remainder);
        }
        Memory {
            memory: memory,
        }
    }

    pub fn read_instruction(&self, pc: isa::Address) -> Option<Instruction> {
        self.memory.get(pc / 4).map(Clone::clone).map(Instruction::new)
    }
}

impl MemoryInterface for Memory {
    const LATENCY: u32 = 100;

    fn read_word(&self, address: isa::Address) -> Result<isa::Word> {
        // memory is word-addressed but addresses are byte-addressed
        self.memory.get(address / 4).map(Clone::clone).ok_or(MemoryError::InvalidAddress)
    }

    fn write_word(&mut self, address: isa::Address, value: isa::Word) -> Result<()> {
        let address = address / 4;
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
        Cache {
            num_sets: sets,
            num_ways: ways,
            block_words: block_words,
            cache: vec![vec![CacheBlock {
                valid: false,
                tag: 0,
                contents: vec![0; block_words],
            }; ways]; sets],
        }
    }

    fn invalidate(&mut self, address: isa::Address) {

    }
}

impl MemoryInterface for Cache {
    const LATENCY: u32 = 1;

    fn read_word(&self, address: isa::Address) -> Result<isa::Word> {
        Err(MemoryError::InvalidAddress)
    }

    fn write_word(&mut self, address: isa::Address, value: isa::Word) -> Result<()> {
        Err(MemoryError::InvalidAddress)
    }

}
