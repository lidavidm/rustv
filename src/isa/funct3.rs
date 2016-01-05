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

pub const ADDI: u32 = 0x0;
pub const SLLI: u32 = 0x1;
pub const SLTI: u32 = 0x2;
pub const SLTIU: u32 = 0x3;
pub const XORI: u32 = 0x4;
pub const SRLI_SRAI: u32 = 0x5;
pub const ORI: u32 = 0x6;
pub const ANDI: u32 = 0x7;

pub const ADD_SUB: u32 = 0x0;
pub const SLL: u32 = 0x1;
pub const SLT: u32 = 0x2;
pub const SLTU: u32 = 0x3;
pub const XOR: u32 = 0x4;
pub const SRL_SRA: u32 = 0x5;
pub const OR: u32 = 0x6;
pub const AND: u32 = 0x7;

pub const BEQ: u32 = 0b000;
pub const BNE: u32 = 0b001;
pub const BLT: u32 = 0b100;
pub const BGE: u32 = 0b101;
pub const BLTU: u32 = 0b110;
pub const BGEU: u32 = 0b111;

pub const LW: u32 = 0x2;

pub const SW: u32 = 0x2;
