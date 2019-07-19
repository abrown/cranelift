//! Constant representation.

use crate::ir::Constant;
use cranelift_entity::EntityRef;
use std::collections::btree_map::Iter;
use std::collections::{BTreeMap, HashMap};
use std::vec::Vec;

/// A reference to the actual constant data
//pub struct ConstantData2<'a>(&'a [u8]);
pub type ConstantData = Vec<u8>;

/// This type describes an offset in bytes within a constant pool
pub type ConstantOffset = u32;

/// Maintains the mapping between a constant handle (i.e. [Constant](ir::entities::Constant)) and
/// its constant data (i.e. [ConstantData](ir::constant::ConstantData))
#[derive(Clone)]
pub struct ConstantPool {
    handles_to_values: BTreeMap<Constant, ConstantData>,
    // would maintain insertion order if Constants are sequentially increasing
    values_to_handles: HashMap<ConstantData, Constant>, // no need to maintain lexicographic order of data
}

impl ConstantPool {
    /// Create a new constant pool instance
    pub fn new() -> Self {
        ConstantPool {
            handles_to_values: BTreeMap::new(),
            values_to_handles: HashMap::new(),
        }
    }

    /// Empty the constant pool of all data
    pub fn clear(&mut self) {
        self.handles_to_values.clear();
        self.values_to_handles.clear();
    }

    /// Insert constant data into the pool, returning a handle for later referencing; when constant
    /// data is inserted that is a duplicate of previous constant data, the existing handle will be
    /// returned
    pub fn insert(&mut self, constant_value: ConstantData) -> Constant {
        if self.values_to_handles.contains_key(&constant_value) {
            self.values_to_handles
                .get(&constant_value)
                .expect("The value must exist")
                .clone()
        } else {
            let constant_handle = Constant::new(self.len());
            self.values_to_handles
                .insert(constant_value.clone(), constant_handle.clone());
            self.handles_to_values
                .insert(constant_handle.clone(), constant_value);
            constant_handle
        }
    }

    /// Retrieve the offset of a given constant, where the offset is the number of bytes from the
    /// beginning of the constant pool to the beginning of the constant data
    pub fn get_offset(&self, constant_handle: Constant) -> ConstantOffset {
        let mut counted_bytes: ConstantOffset = 0;
        for (handle, value) in self.handles_to_values.iter() {
            if handle == &constant_handle {
                return counted_bytes;
            } else {
                counted_bytes += value.len() as u32;
            }
        }
        panic!("Unable to find the constant; was its value inserted first?") // TODO could return result here instead
    }

    /// Iterate over the constants in insertion order
    pub fn iter(&self) -> Iter<Constant, ConstantData> {
        self.handles_to_values.iter()
    }

    /// Return the number of constants in the pool
    pub fn len(&self) -> usize {
        self.handles_to_values.len()
    }

    /// Return the combined size of all of the constant values in the pool
    pub fn byte_size(&self) -> usize {
        self.values_to_handles.keys().map(|c| c.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let sut = ConstantPool::new();
        assert_eq!(sut.len(), 0);
    }

    #[test]
    fn insert() {
        let mut sut = ConstantPool::new();
        sut.insert(vec![1, 2, 3]);
        sut.insert(vec![4, 5, 6]);
        assert_eq!(sut.len(), 2);
    }

    #[test]
    fn insert_duplicate() {
        let mut sut = ConstantPool::new();
        let a = sut.insert(vec![1, 2, 3]);
        sut.insert(vec![4, 5, 6]);
        let b = sut.insert(vec![1, 2, 3]);
        assert_eq!(a, b);
    }

    #[test]
    fn clear() {
        let mut sut = ConstantPool::new();
        sut.insert(vec![1, 2, 3]);
        assert_eq!(sut.len(), 1);

        sut.clear();
        assert_eq!(sut.len(), 0);
    }

    #[test]
    fn iteration_order() {
        let mut sut = ConstantPool::new();
        sut.insert(vec![1, 2, 3]);
        sut.insert(vec![4, 5, 6]);
        sut.insert(vec![1, 2, 3]);
        let data = sut.iter().map(|(_, v)| v).collect::<Vec<&ConstantData>>();
        assert_eq!(data, vec![&vec![1, 2, 3], &vec![4, 5, 6]]);
    }
}
