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

use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead};
use std::num;
use std::path::Path;
use std::str;

/// Representation of a binary
pub struct Binary {
    pub words: Vec<u32>,
}

#[derive(Debug)]
pub enum BinaryError {
    Io(io::Error),
    Utf8(str::Utf8Error),
    ParseInt(num::ParseIntError),
}

impl fmt::Display for BinaryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BinaryError::Io(ref err) => err.fmt(f),
            BinaryError::Utf8(ref err) => err.fmt(f),
            BinaryError::ParseInt(ref err) => err.fmt(f),
        }
    }
}

impl Error for BinaryError {
    fn description(&self) -> &str {
        match *self {
            BinaryError::Io(ref err) => err.description(),
            BinaryError::Utf8(ref err) => err.description(),
            BinaryError::ParseInt(ref err) => err.description(),
        }
    }
}

impl From<io::Error> for BinaryError {
    fn from(err: io::Error) -> BinaryError {
        BinaryError::Io(err)
    }
}

impl From<str::Utf8Error> for BinaryError {
    fn from(err: str::Utf8Error) -> BinaryError {
        BinaryError::Utf8(err)
    }
}

impl From<num::ParseIntError> for BinaryError {
    fn from(err: num::ParseIntError) -> BinaryError {
        BinaryError::ParseInt(err)
    }
}

impl Binary {
    /// Load a binary from a hex file (generated with elf2hex)
    pub fn new_from_hex_file(path: &Path) -> Result<Binary, BinaryError> {
        let file = try!(File::open(path));
        let file = io::BufReader::new(file);

        let mut words: Vec<u32> = Vec::new();
        for line in file.lines() {
            let line = try!(line);
            for bytes in line.as_bytes().chunks(8).rev() {
                let word = try!(str::from_utf8(bytes));
                words.push(try!(u32::from_str_radix(word, 16)));
            }
        }

        Ok(Binary {
            words: words,
        })
    }
}
