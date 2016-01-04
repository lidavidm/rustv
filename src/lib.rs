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

#![feature(braced_empty_structs, clone_from_slice, raw, step_by)]
pub mod isa;
pub mod binary;
pub mod memory;
pub mod simulator;

#[test]
fn it_works() {
    use std::rc::Rc;
    use std::cell::RefCell;

    use std::path::Path;
    match binary::Binary::new_from_hex_file(Path::new("../riscv/kernel.hex")) {
        Ok(b) => {
            let mmu = memory::IdentityMmu::new();
            let memory = memory::Memory::new_from_binary(0x2000, b);
            let memory_ref = Rc::new(RefCell::new(Box::new(memory) as Box<memory::MemoryInterface>));
            let cache = Rc::new( RefCell::new( Box::new( memory::DirectMappedCache::new(4, 4, memory_ref.clone())) as Box<memory::MemoryInterface>) );
            let core = simulator::Core::new(cache.clone(), Box::new(mmu));
            let mut simulator = simulator::Simulator::new(vec![core], memory_ref.clone());
            simulator.run();
        },
        Err(err) => println!("Error: {:?}", err),
    }
}

#[test]
fn test_elfloader() {
    use std::io::prelude::*;
    use std::fs::File;
    extern crate elfloader;

    let mut f = File::open("../riscv/kernel").unwrap();
    let mut buffer = Vec::new();

    f.read_to_end(&mut buffer).unwrap();

    let elf = elfloader::ElfBinary::new("test", &buffer).unwrap();
    println!("HEADERS");
    for p in elf.program_headers() {
        println!("{}", p);
    }
    for p in elf.section_headers() {
        println!("{}", p);
        if p.name.0 == 0x1b {
            let data = elf.section_data(p);
            print!("\t");
            for x in data[0..8].iter() {
                print!("{:02x}", x);
            }
            println!("");
        }
    }

    println!("{:?}", elf);
}

#[cfg(test)]
mod tests {
    #[test]
    fn cache_address_parsing() {
        use memory::*;
        use std::rc::Rc;
        use std::cell::RefCell;

        let memory = Memory::new(16);
        let memory_ref = Rc::new(RefCell::new(Box::new(memory) as Box<MemoryInterface>));
        let dm_cache_word = DirectMappedCache::new(4, 1, memory_ref.clone());
        let dm_cache_doubleword = DirectMappedCache::new(4, 2, memory_ref.clone());

        assert_eq!(dm_cache_word.parse_address(0xFFFFFFFD),
                   (0xFFFFFFF, 3, 1));
        assert_eq!(dm_cache_doubleword.parse_address(0xFFFFFFFD),
                   (0x7FFFFFF, 3, 5));
    }

    #[test]
    fn memory_rw() {
        use std::rc::Rc;
        use std::cell::RefCell;
        use memory::*;

        let size = 0xFF;
        let mut memory = Memory::new(size);

        assert_eq!(memory.write_word(0, 0xF0),
                   Err(MemoryError::InvalidAddress));
        assert_eq!(memory.write_byte(0, 0xF0),
                   Err(MemoryError::InvalidAddress));
        assert_eq!(memory.write_byte(1, 0xF0),
                   Err(MemoryError::InvalidAddress));
        assert_eq!(memory.write_byte(2, 0xF0),
                   Err(MemoryError::InvalidAddress));

        for address in (4..size).step_by(4) {
            assert_eq!(memory.write_word(address, 0xF0), Ok(()));
            assert_eq!(memory.read_word(address), Ok(0xF0));
        }

        assert_eq!(memory.write_word(0x10, 0x01234567), Ok(()));
        assert_eq!(memory.write_word(0x14, 0xDEADBEEF), Ok(()));
        assert_eq!(memory.read_byte(0x10), Ok(0x67));
        assert_eq!(memory.read_byte(0x11), Ok(0x45));
        assert_eq!(memory.read_byte(0x12), Ok(0x23));
        assert_eq!(memory.read_byte(0x13), Ok(0x01));

        let stall = Err(MemoryError::CacheMiss {
            stall_cycles: memory.latency(),
        });
        let write_stall = Err(MemoryError::CacheMiss {
            stall_cycles: memory.latency(),
        });

        let memory_box = Box::new(memory) as Box<MemoryInterface>;
        let memory_ref = Rc::new(RefCell::new(memory_box));
        let mut dm_cache = DirectMappedCache::new(4, 4, memory_ref.clone());

        assert_eq!(dm_cache.read_word(0x10), stall);

        for _ in 0..100 {
            dm_cache.step();
        }

        assert_eq!(dm_cache.write_word(0x20, 0x123), write_stall);
        assert_eq!(dm_cache.read_word(0x10), Ok(0x01234567));
        assert_eq!(dm_cache.read_word(0x14), Ok(0xDEADBEEF));
        assert_eq!(dm_cache.read_word(0x18), Ok(0xF0));
        assert_eq!(dm_cache.read_word(0x1C), Ok(0xF0));
        assert_eq!(dm_cache.read_byte(0x10), Ok(0x67));
        assert_eq!(dm_cache.read_byte(0x11), Ok(0x45));
        assert_eq!(dm_cache.read_byte(0x12), Ok(0x23));
        assert_eq!(dm_cache.read_byte(0x13), Ok(0x01));
        assert_eq!(dm_cache.write_word(0x18, 0xBEEFBEEF), Ok(()));
        assert_eq!(dm_cache.read_word(0x18), Ok(0xBEEFBEEF));
        assert_eq!(memory_ref.borrow_mut().read_word(0x18), Ok(0xBEEFBEEF));

        for _ in 0..100 {
            dm_cache.step();
        }

        assert_eq!(dm_cache.write_word(0x20, 0x123), Ok(()));
        assert_eq!(memory_ref.borrow_mut().read_word(0x20), Ok(0x123));
        // Should not have been evicted
        assert_eq!(dm_cache.read_word(0x10), Ok(0x01234567));
        assert_eq!(dm_cache.read_word(0x14), Ok(0xDEADBEEF));
        assert_eq!(dm_cache.read_word(0x18), Ok(0xBEEFBEEF));
        assert_eq!(dm_cache.read_word(0x1C), Ok(0xF0));
    }
}
