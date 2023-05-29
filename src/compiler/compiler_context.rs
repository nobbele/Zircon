use std::collections::HashMap;

pub struct CompilerContext {
    pub address: usize,
    pub binary: Vec<u8>,

    pub declarations: HashMap<String, u16>,
}

impl CompilerContext {
    fn reserve_min(&mut self, min_length: usize) {
        if min_length > self.binary.len() {
            self.binary
                .extend(std::iter::repeat(0).take(min_length - self.binary.len()));
        }
    }

    pub fn write(&mut self, data: &[u8]) {
        let end_address = self.address + data.len();
        self.reserve_min(end_address);
        self.binary[self.address..end_address].copy_from_slice(data);
        self.address = end_address;
    }

    pub fn set_address(&mut self, new_address: u16) {
        self.address = new_address as usize;
    }

    pub fn set(&mut self, name: &str, value: u16) {
        if self.declarations.contains_key(name) {
            // TODO Compile error?
            panic!("Already declared");
        }

        self.declarations.insert(name.to_owned(), value);
    }

    pub fn get(&self, name: &str) -> Option<u16> {
        self.declarations.get(name).copied()
    }
}
