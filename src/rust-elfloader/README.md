[![Build Status](https://travis-ci.org/gz/rust-elfloader.svg?branch=master)](https://travis-ci.org/gz/rust-elfloader)

# rust-elfloader

A library to load and relocate ELF files in memory.
This library depends only on libcore so it can be used in kernel level code,
for example to load user-space programs.

This library reuses a modified version of the types.rs
file from [rust-elf](https://github.com/cole14/rust-elf)
by Christopher Cole.

This library was modified to handle 32-bit ELF files instead. You
probably want the original
[rust-elfloader](https://github.com/gz/rust-elfloader), if you're
looking at this.