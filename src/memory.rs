use isa::{self, Instruction};
use binary::{Binary};

pub type Address = usize;

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

// TODO: refactor impls into a MemoryController(?) trait

impl Memory {
    pub fn new(size: Address, binary: Binary) -> Memory {
        let mut memory = binary.words.clone();
        if size > memory.len() {
            let remainder = size - memory.len();
            memory.reserve(remainder);
        }
        Memory {
            memory: memory,
        }
    }

    pub fn read_word(&self, address: Address) -> Option<isa::Word> {
        // memory is word-addressed but addresses are byte-addressed
        self.memory.get(address / 4).map(Clone::clone)
    }

    pub fn write_word(&mut self, address: Address, value: isa::Word) -> Option<()> {
        let address = address / 4;
        if address >= self.memory.len() {
            None
        }
        else {
            self.memory[address] = value;
            Some(())
        }
    }

    pub fn read_instruction(&self, pc: Address) -> Option<Instruction> {
        self.memory.get(pc / 4).map(Clone::clone).map(Instruction::new)
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

    fn read_word(&self, address: Address) -> Option<isa::Word> {
        None
    }

    fn write_word(&mut self, address: Address, value: isa::Word) -> Option<()> {
        None
    }

    fn invalidate(&mut self, address: Address) {
        
    }
}
