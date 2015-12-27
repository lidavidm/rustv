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

pub const BRANCH: u32 = 0x63;
pub const JALR: u32 = 0x67;
pub const JAL: u32 = 0x6F;
pub const INTEGER_IMMEDIATE: u32 = 0x13;
pub const INTEGER_REGISTER: u32 = 0x33;
pub const LOAD: u32 = 0x3;
pub const STORE: u32 = 0x23;
pub const SYSTEM: u32 = 0x73;
