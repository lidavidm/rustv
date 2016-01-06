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

#![feature(braced_empty_structs, step_by)]
extern crate elfloader32 as elfloader_lib;

pub mod isa;
pub mod binary;
pub mod memory;
pub mod simulator;

pub use elfloader_lib as elfloader;

#[test]
fn test_elfloader() {
    use std::io::prelude::*;
    use std::fs::File;
    use std::rc::Rc;
    use std::cell::RefCell;
    use memory::{Mmu, MemoryInterface};

    let mut f = File::open("../riscv/kernel").unwrap();
    let mut buffer = Vec::new();

    f.read_to_end(&mut buffer).unwrap();

    let elf = elfloader::ElfBinary::new("test", &buffer).unwrap();
    let start = elf.file_header().entry as isa::Address;

    let mut text = None;
    let mut data = None;
    for p in elf.section_headers() {
        if p.name.0 == 0x1b {
            text = Some((elf.section_data(p), p.addr));
        }
        else if p.name.0 == 0x33 {
            data = Some((elf.section_data(p), p.addr));
        }
    }

    let (text, text_offset) = text.unwrap();
    let (data, data_offset) = data.unwrap();

    let mmu = memory::IdentityMmu::new();
    let mmu2 = memory::ReverseMmu::new(0x8000);
    let mut memory = memory::Memory::new(0x10000);
    memory.write_segment(&mmu, text, text_offset as usize);
    memory.write_segment(&mmu, data, data_offset as usize);
    memory.write_segment(&mmu2, text, text_offset as usize);
    memory.write_segment(&mmu2, data, data_offset as usize);

    let memory_ref = Rc::new(RefCell::new(memory));
    let cache = memory::DirectMappedCache::new(4, 4, memory_ref.clone());
    let cache_ref = Rc::new(RefCell::new(cache));
    let core = simulator::Core::new(
        start, 0x1000,
        cache_ref.clone(), Box::new(mmu));
    let core2 = simulator::Core::new(
        start, 0x3000,
        cache_ref.clone(), Box::new(mmu2));
    let cores = vec![core, core2];
    let mut simulator = simulator::Simulator::new(cores, memory_ref.clone());
    simulator.run();
}

#[cfg(test)]
mod tests {
    #[test]
    fn cache_address_parsing() {
        use memory::*;
        use std::rc::Rc;
        use std::cell::RefCell;

        let memory = Memory::new(16);
        let memory_ref = Rc::new(RefCell::new(memory));
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

        let memory_ref = Rc::new(RefCell::new(memory));
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
