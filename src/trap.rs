// Copyright 2016 David Li
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

use isa;

#[derive(Debug)]
pub enum Trap {
    IllegalInstruction {
        address: isa::Address,
        instruction: isa::Instruction,
    },
    IllegalRead {
        address: isa::Address,
        instruction: isa::Instruction,
        memory_address: isa::Address,
    },
    IllegalWrite {
        address: isa::Address,
        instruction: isa::Instruction,
        memory_address: isa::Address,
        memory_value: isa::Word,
    }
}
