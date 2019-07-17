//! Constant representation.

use crate::ir::immediates::Uimm128;
use core::fmt::{self, Display, Formatter};
use core::slice::Iter;
use std::boxed::Box;
use std::collections::BTreeSet;
use std::vec::Vec;

/// Symbolizes a constant in the IR that will be stored separately (e.g. in a constant pool)
///
/// # Fields:
///
/// * data: holds the value of the constant as bytes
/// * offset: the location at which to store the constant in the constant pool
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)] // TODO
pub struct Constant {
    data: ConstantData,
    /// the the location (in bytes) at which to store the constant in the constant pool
    pub offset: Option<ConstantOffset>,
}

impl Constant {
    /// set the offset of the constant
    pub fn offset(mut self, new_offset: ConstantOffset) -> Constant {
        self.offset = Some(new_offset);
        self
    }

    /// iterate over the bytes stored in the constant
    pub fn bytes(&self) -> Iter<u8> {
        self.data.iter()
    }

    /// calculate the number of bytes stored by the constant
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl From<Uimm128> for Constant {
    fn from(imm: Uimm128) -> Self {
        let data = ConstantData::from(&imm.0[..]);
        Constant { data, offset: None }
    }
}

impl From<&Box<Uimm128>> for Constant {
    fn from(imm: &Box<Uimm128>) -> Self {
        imm.into()
    }
}

/// This type describes an offset in bytes within a constant pool
pub type ConstantOffset = u32;

/// Calculate the total number of bytes held in all of the constants in the set
///
/// # Parameters:
/// - set: the set of constants
///
/// # Return:
/// - the number of bytes contained in all of the constants
pub fn byte_len(set: &BTreeSet<Constant>) -> usize {
    set.iter().fold(0, |a, c| a + c.len())
}

/// Contents of a constant value
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct ConstantData {
    // raw bytes of constant TODO perhaps this could be more efficient with a &[u8] type
    data: Vec<u8>,
}

impl ConstantData {
    /// Create a new empty jump table.
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Create a new empty jump table with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    /// Get the number of table entries.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    ///
    pub fn add_byte(&mut self, byte: u8) {
        self.data.push(byte)
    }

    ///
    ///
    pub fn add_slice(&mut self, bytes: &[u8]) {
        for byte in bytes.iter() {
            self.add_byte(*byte)
        }
    }

    /// Returns an iterator over the table.
    pub fn iter(&self) -> Iter<u8> {
        self.data.iter()
    }
}

impl From<&[u8]> for ConstantData {
    fn from(slice: &[u8]) -> Self {
        Self {
            data: slice.to_vec(),
        }
    }
}

// TODO add impl From<[u8]> for ConstantData

impl Display for ConstantData {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "constant ")?;
        write!(fmt, "{:02X?}", self.data)
    }
}

#[cfg(test)]
mod tests {
    use std::string::ToString;

    use super::*;

    #[test]
    fn empty() {
        let c = ConstantData::new();
        assert_eq!(c.len(), 0);
    }

    #[test]
    fn insert() {
        let mut c = ConstantData::new();
        c.add_byte(1);
        c.add_byte(2);
        c.add_byte(3);
        assert_eq!(c.len(), 3);
    }

    #[test]
    fn stringify() {
        let mut c = ConstantData::new();
        assert_eq!(c.to_string(), "constant []");

        c.add_slice(&[1, 2, 42]);
        assert_eq!(c.to_string(), "constant [01, 02, 2A]");
    }
}
