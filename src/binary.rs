pub struct Binary<'a> {
    words: &'a [u32],
}

impl<'a> Binary<'a> {
    pub fn new(words: &'a [u32]) -> Binary<'a> {
        Binary {
            words: words,
        }
    }

    // pub fn new_from_hex_file() -> Binary<'a> {
        
    // }
}
