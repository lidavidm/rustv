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

#![feature(augmented_assignments, braced_empty_structs,
           op_assign_traits, step_by)]
extern crate elfloader32 as elfloader_lib;

pub mod cache;
pub mod isa;
pub mod memory;
pub mod register_file;
pub mod simulator;
pub mod syscall;
pub mod trap;

pub use elfloader_lib as elfloader;

#[cfg(test)]
mod tests {
    #[test]
    fn cache_address_parsing() {
        use cache::*;
        use isa::*;
        use memory::*;
        use std::rc::Rc;
        use std::cell::RefCell;

        let memory = Memory::new(16);
        let memory_ref = Rc::new(RefCell::new(memory));
        let dm_cache_word = DirectMappedCache::new(
            4, 1, memory_ref.clone(), EmptyEventHandler {});
        let dm_cache_doubleword = DirectMappedCache::new(
            4, 2, memory_ref.clone(), EmptyEventHandler {});

        assert_eq!(dm_cache_word.parse_address(Word(0xFFFFFFFD)),
                   (0xFFFFFFF, 3, 1));
        assert_eq!(dm_cache_doubleword.parse_address(Word(0xFFFFFFFD)),
                   (0x7FFFFFF, 3, 5));
    }

    #[test]
    fn memory_rw() {
        use std::rc::Rc;
        use std::cell::RefCell;

        use cache::*;
        use isa::*;
        use memory::*;

        let size = 0xFF;
        let mut memory = Memory::new(size);

        assert_eq!(memory.write_word(Word(0), Word(0xF0)),
                   Err(MemoryError::InvalidAddress));
        assert_eq!(memory.write_byte(Word(0), Byte(0xF0)),
                   Err(MemoryError::InvalidAddress));
        assert_eq!(memory.write_byte(Word(1), Byte(0xF0)),
                   Err(MemoryError::InvalidAddress));
        assert_eq!(memory.write_byte(Word(2), Byte(0xF0)),
                   Err(MemoryError::InvalidAddress));
        assert_eq!(memory.write_byte(Word(3), Byte(0xF0)),
                   Err(MemoryError::InvalidAddress));
        assert_eq!(memory.write_halfword(Word(0), HalfWord(0xF0)),
                   Err(MemoryError::InvalidAddress));
        assert_eq!(memory.write_halfword(Word(2), HalfWord(0xF0)),
                   Err(MemoryError::InvalidAddress));

        for address in (4..size).step_by(4) {
            let address = Word(address as u32);
            assert_eq!(memory.write_word(address, Word(0xF0)), Ok(()));
            assert_eq!(memory.read_word(address), Ok(Word(0xF0)));
            assert_eq!(memory.read_halfword(address), Ok(HalfWord(0xF0)));
            assert_eq!(memory.read_halfword(address + 2), Ok(HalfWord(0x0)));
            assert_eq!(memory.read_byte(address), Ok(Byte(0xF0)));
            assert_eq!(memory.read_byte(address + 1), Ok(Byte(0x0)));
            assert_eq!(memory.read_byte(address + 2), Ok(Byte(0x0)));
            assert_eq!(memory.read_byte(address + 3), Ok(Byte(0x0)));
        }

        assert_eq!(memory.write_word(Word(0x10), Word(0x01234567)), Ok(()));
        assert_eq!(memory.write_word(Word(0x14), Word(0xDEADBEEF)), Ok(()));
        assert_eq!(memory.read_byte(Word(0x10)), Ok(Byte(0x67)));
        assert_eq!(memory.read_byte(Word(0x11)), Ok(Byte(0x45)));
        assert_eq!(memory.read_byte(Word(0x12)), Ok(Byte(0x23)));
        assert_eq!(memory.read_byte(Word(0x13)), Ok(Byte(0x01)));
        assert_eq!(memory.read_halfword(Word(0x10)), Ok(HalfWord(0x4567)));
        assert_eq!(memory.read_halfword(Word(0x12)), Ok(HalfWord(0x0123)));

        let stall = Err(MemoryError::CacheMiss {
            stall_cycles: memory.latency(),
            retry: true,
        });
        let write_stall = Err(MemoryError::CacheMiss {
            stall_cycles: memory.latency(),
            retry: true,
        });

        let memory_ref = Rc::new(RefCell::new(memory));
        let mut dm_cache = DirectMappedCache::new(
            4, 4, memory_ref.clone(), EmptyEventHandler {});

        assert_eq!(dm_cache.read_word(Word(0x10)), stall);

        for _ in 0..100 {
            dm_cache.step();
        }

        assert_eq!(dm_cache.write_word(Word(0x20), Word(0x123)), write_stall);
        assert_eq!(dm_cache.read_word(Word(0x10)), Ok(Word(0x01234567)));
        assert_eq!(dm_cache.read_word(Word(0x14)), Ok(Word(0xDEADBEEF)));
        assert_eq!(dm_cache.read_word(Word(0x18)), Ok(Word(0xF0)));
        assert_eq!(dm_cache.read_word(Word(0x1C)), Ok(Word(0xF0)));
        assert_eq!(dm_cache.read_byte(Word(0x10)), Ok(Byte(0x67)));
        assert_eq!(dm_cache.read_byte(Word(0x11)), Ok(Byte(0x45)));
        assert_eq!(dm_cache.read_byte(Word(0x12)), Ok(Byte(0x23)));
        assert_eq!(dm_cache.read_byte(Word(0x13)), Ok(Byte(0x01)));
        assert_eq!(dm_cache.write_word(Word(0x18), Word(0xBEEFBEEF)), Ok(()));
        assert_eq!(dm_cache.read_word(Word(0x18)), Ok(Word(0xBEEFBEEF)));
        assert_eq!(dm_cache.read_halfword(Word(0x10)), Ok(HalfWord(0x4567)));
        assert_eq!(dm_cache.read_halfword(Word(0x12)), Ok(HalfWord(0x0123)));
        assert_eq!(memory_ref.borrow_mut().read_word(Word(0x18)), Ok(Word(0xBEEFBEEF)));

        for _ in 0..100 {
            dm_cache.step();
        }

        assert_eq!(dm_cache.write_word(Word(0x20), Word(0x123)), Ok(()));
        assert_eq!(memory_ref.borrow_mut().read_word(Word(0x20)), Ok(Word(0x123)));
        // Should not have been evicted
        assert_eq!(dm_cache.read_word(Word(0x10)), Ok(Word(0x01234567)));
        assert_eq!(dm_cache.read_word(Word(0x14)), Ok(Word(0xDEADBEEF)));
        assert_eq!(dm_cache.read_word(Word(0x18)), Ok(Word(0xBEEFBEEF)));
        assert_eq!(dm_cache.read_word(Word(0x1C)), Ok(Word(0xF0)));
        assert_eq!(dm_cache.read_halfword(Word(0x10)), Ok(HalfWord(0x4567)));
        assert_eq!(dm_cache.read_halfword(Word(0x12)), Ok(HalfWord(0x0123)));

        assert_eq!(dm_cache.write_byte(Word(0x10), Byte(0x42)), Ok(()));
        assert_eq!(dm_cache.write_halfword(Word(0x12), HalfWord(0x4242)), Ok(()));
        assert_eq!(memory_ref.borrow_mut().read_word(Word(0x10)), Ok(Word(0x42424542)));
    }
}
