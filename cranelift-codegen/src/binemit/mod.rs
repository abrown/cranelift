//! Binary machine code emission.
//!
//! The `binemit` module contains code for translating Cranelift's intermediate representation into
//! binary machine code.

mod memorysink;
mod relaxation;
mod shrink;

pub use self::memorysink::{MemoryCodeSink, NullTrapSink, RelocSink, TrapSink};
pub use self::relaxation::relax_branches;
pub use self::shrink::shrink_instructions;
pub use crate::regalloc::RegDiversions;

use crate::ir::{ConstantOffset, ExternalName, Function, Inst, JumpTable, SourceLoc, TrapCode};
use core::fmt;
#[cfg(feature = "enable-serde")]
use serde::{Deserialize, Serialize};

/// Offset in bytes from the beginning of the function.
///
/// Cranelift can be used as a cross compiler, so we don't want to use a type like `usize` which
/// depends on the *host* platform, not the *target* platform.
pub type CodeOffset = u32;

/// Addend to add to the symbol value.
pub type Addend = i64;

/// Relocation kinds for every ISA
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub enum Reloc {
    /// absolute 4-byte
    Abs4,
    /// absolute 8-byte
    Abs8,
    /// x86 PC-relative 4-byte
    X86PCRel4,
    /// x86 PC-relative 4-byte offset to trailing rodata
    X86PCRelRodata4,
    /// x86 call to PC-relative 4-byte
    X86CallPCRel4,
    /// x86 call to PLT-relative 4-byte
    X86CallPLTRel4,
    /// x86 GOT PC-relative 4-byte
    X86GOTPCRel4,
    /// Arm32 call target
    Arm32Call,
    /// Arm64 call target
    Arm64Call,
    /// RISC-V call target
    RiscvCall,
}

impl fmt::Display for Reloc {
    /// Display trait implementation drops the arch, since its used in contexts where the arch is
    /// already unambiguous, e.g. clif syntax with isa specified. In other contexts, use Debug.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Reloc::Abs4 => write!(f, "Abs4"),
            Reloc::Abs8 => write!(f, "Abs8"),
            Reloc::X86PCRel4 => write!(f, "PCRel4"),
            Reloc::X86PCRelRodata4 => write!(f, "PCRelRodata4"),
            Reloc::X86CallPCRel4 => write!(f, "CallPCRel4"),
            Reloc::X86CallPLTRel4 => write!(f, "CallPLTRel4"),
            Reloc::X86GOTPCRel4 => write!(f, "GOTPCRel4"),
            Reloc::Arm32Call | Reloc::Arm64Call | Reloc::RiscvCall => write!(f, "Call"),
        }
    }
}

/// Container for information about a vector of compiled code and its supporting read-only data.
///
/// The code starts at offset 0 and is followed optionally by relocatable jump tables and copyable
/// (raw binary) read-only data.  Any padding between sections is always part of the section that
/// precedes the boundary between the sections.
#[derive(PartialEq)]
pub struct CodeInfo {
    /// Number of bytes of machine code (the code starts at offset 0).
    pub code_size: CodeOffset,

    /// Number of bytes of jumptables.
    pub jumptables_size: CodeOffset,

    /// Number of bytes of rodata.
    pub rodata_size: CodeOffset,

    /// Number of bytes in total.
    pub total_size: CodeOffset,
}

impl CodeInfo {
    /// Offset of any relocatable jump tables, or equal to rodata if there are no jump tables.
    pub fn jumptables(&self) -> CodeOffset {
        self.code_size
    }

    /// Offset of any copyable read-only data, or equal to total_size if there are no rodata.
    pub fn rodata(&self) -> CodeOffset {
        self.code_size + self.jumptables_size
    }
}

/// Abstract interface for adding bytes to the code segment.
///
/// A `CodeSink` will receive all of the machine code for a function. It also accepts relocations
/// which are locations in the code section that need to be fixed up when linking.
pub trait CodeSink {
    /// Get the current position.
    fn offset(&self) -> CodeOffset;

    /// Add 1 byte to the code section.
    fn put1(&mut self, _: u8);

    /// Add 2 bytes to the code section.
    fn put2(&mut self, _: u16);

    /// Add 4 bytes to the code section.
    fn put4(&mut self, _: u32);

    /// Add 8 bytes to the code section.
    fn put8(&mut self, _: u64);

    /// Add a relocation referencing an EBB at the current offset.
    fn reloc_ebb(&mut self, _: Reloc, _: CodeOffset);

    /// Add a relocation referencing an external symbol plus the addend at the current offset.
    fn reloc_external(&mut self, _: Reloc, _: &ExternalName, _: Addend);

    /// Add a relocation referencing a jump table.
    fn reloc_constant(&mut self, _: Reloc, _: ConstantOffset);

    /// Add a relocation referencing a jump table.
    fn reloc_jt(&mut self, _: Reloc, _: JumpTable);

    /// Add trap information for the current offset.
    fn trap(&mut self, _: TrapCode, _: SourceLoc);

    /// Machine code output is complete, jump table data may follow.
    fn begin_jumptables(&mut self);

    /// Jump table output is complete, raw read-only data may follow.
    fn begin_rodata(&mut self);

    /// Read-only data output is complete, we're done.
    fn end_codegen(&mut self);
}

/// Report a bad encoding error.
#[cold]
pub fn bad_encoding(func: &Function, inst: Inst) -> ! {
    panic!(
        "Bad encoding {} for {}",
        func.encodings[inst],
        func.dfg.display_inst(inst, None)
    );
}

/// Emit a function to `sink`, given an instruction emitter function.
///
/// This function is called from the `TargetIsa::emit_function()` implementations with the
/// appropriate instruction emitter.
pub fn emit_function<CS, EI>(func: &Function, emit_inst: EI, sink: &mut CS)
where
    CS: CodeSink,
    EI: Fn(&Function, Inst, &mut RegDiversions, &mut CS),
{
    let mut divert = RegDiversions::new();
    for ebb in func.layout.ebbs() {
        divert.clear();
        debug_assert_eq!(func.offsets[ebb], sink.offset());
        for inst in func.layout.ebb_insts(ebb) {
            emit_inst(func, inst, &mut divert, sink);
        }
    }

    sink.begin_jumptables();

    // output jump tables
    for (jt, jt_data) in func.jump_tables.iter() {
        let jt_offset = func.jt_offsets[jt];
        for ebb in jt_data.iter() {
            let rel_offset: i32 = func.offsets[*ebb] as i32 - jt_offset as i32;
            sink.put4(rel_offset as u32)
        }
    }

    sink.begin_rodata();
    // TODO: No read-only data (constant pools) at this time.

    sink.end_codegen();
}
