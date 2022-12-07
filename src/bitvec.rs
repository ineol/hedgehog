use std::iter;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct BitVec {
    inner: Vec<u64>,
    hash: u64,
}

impl BitVec {
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
            hash: 0,
        }
    }

    pub fn from_elem(val: bool, len: usize) -> Self {
        let block = if val { u64::MAX } else { 0 };
        let count = (len + 63) / 64;
        let hash = if count % 2 == 0 { 0 } else { block };

        Self {
            inner: iter::repeat(block).take(count).collect(),
            hash,
        }
    }

    pub fn get(&self, i: usize) -> bool {
        let block = self.inner[i / 64];
        let masked = block & (1 << i % 64);
        masked != 0
    }

    pub fn set(&mut self, i: usize, val: bool) {
        let block = self.inner.get_mut(i / 64).unwrap();
        self.hash ^= *block;
        if val {
            *block = *block | (1 << i % 64);
        } else {
            *block = *block & !(1 << i % 64);
        }
        self.hash ^= *block;
    }
}

impl std::hash::Hash for BitVec {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}
