use proc_macro::TokenStream;

enum Format {
    REX(RexFormat),
    VEX(VexFormat),
    // EVEX(EvexFormat),
}

// TODO need multi-sized arrays, e.g. a 1-, 2-, or, 3-byte opcode.
// TODO tighter bit packing, e.g. don't use u8 in ModR/M, SIB
// TODO add magic #[derive] to convert Encodable implementations to generatable strings.
// TODO move CodeSink trait to a higher level (shared?)

trait CodeSink {
    fn put1(&mut self, _: u8);
    fn put2(&mut self, _: u16);
    fn put4(&mut self, _: u32);
}

macro_rules! generate {
    (impl Encodable for $type:ident {
        fn encode(&$self:ident, $sink:ident : &mut dyn CodeSink) {
            $($body: stmt)+
        }
    }) => {

    impl Encodable for $type {
        fn encode(&$self, $sink: &mut dyn CodeSink) {
            $($body)+
        }
    }

    impl GenerateEncodable for $type {
        fn generate(&$self) -> String {
            return String::from("...");
        }
    }

    }
}

trait Encodable {
    fn encode(&self, sink: &mut dyn CodeSink);
}

trait GenerateEncodable {
    fn generate(&self) -> String;
}

generate! {
    impl Encodable for u16 {
        fn encode(&self, sink: &mut dyn CodeSink) {
            sink.put2(*self)
        }
    }
}

generate! {
    impl Encodable for u8 {
        fn encode(&self, sink: &mut dyn CodeSink) {
            sink.put1(*self)
        }
    }
}

impl<T: Encodable> Encodable for Vec<T> {
    fn encode(&self, sink: &mut dyn CodeSink) {
        for e in self {
            e.encode(sink);
        }
    }
}

impl<T: GenerateEncodable> GenerateEncodable for Vec<T> {
    fn generate(&self) -> String {
        let mut out = String::new();
        for e in self {
            out.push_str(&e.generate());
            out.push_str(";\n");
        }
        out
    }
}

impl<T: Encodable> Encodable for Option<T> {
    fn encode(&self, sink: &mut dyn CodeSink) {
        match self {
            None => {}
            Some(e) => e.encode(sink),
        }
    }
}

impl<T: GenerateEncodable> GenerateEncodable for Option<T> {
    fn generate(&self) -> String {
        match self {
            None => String::new(),
            Some(e) => e.generate(),
        }
    }
}

/// See REX Prefixes (section 2.2.1, Intel Software Developer's Manual vol. 2A).
struct RexFormat {
    legacy_prefixes: Vec<LegacyPrefix>,
    rex: RexPrefix,
    /// A 1-, 2-, or, 3-byte opcode.
    opcode: Vec<u8>,
    modrm: Option<ModRM>,
    sib: Option<SIB>,
    displacement: Option<Vec<u8>>,
    immediate: Option<Vec<u8>>,
}

impl Encodable for RexFormat {
    fn encode(&self, sink: &mut dyn CodeSink) {
        self.legacy_prefixes.encode(sink);
        self.rex.encode(sink);
        // TODO
    }
}

/// See Instruction Prefixes (section 2.1.1, Intel Software Developer's Manual vol. 2A).
enum LegacyPrefix {}

impl Encodable for LegacyPrefix {
    fn encode(&self, sink: &mut dyn CodeSink) {
        unimplemented!()
    }
}

/// The REX prefix byte has 0100 in bits 7:4.
struct RexPrefix {
    /// Bit 3: 0 = operand size determined by CS.D, 1 = 64-bit operand size.
    w: bool,
    /// Bit 2: extension of the ModR/M `reg` field.
    r: bool,
    /// Bit 1: extension of the SIB `index` field.
    x: bool,
    /// Bit 0: extension of the ModR/M `rm` field, SIB base field, or opcode reg field.
    b: bool,
}

impl Encodable for RexPrefix {
    fn encode(&self, sink: &mut dyn CodeSink) {
        unimplemented!()
    }
}

/// See ModR/M and SIB Bytes (section 2.1.3, Intel Software Developer's Manual vol. 2A).
struct ModRM {
    /// Bits 7:6: `mod` combines with the rm field to form 32 possible values: 8 registers and 24
    /// addressing modes.
    mod_: u8,
    /// Bits 5:3: `reg/opcode` specifies either a register number or three more bits of opcode
    /// information (e.g. with `/d` specifier).
    reg: u8,
    /// Bits 2:0: `r/m` specifies a register as an operand or can be combined with `mod` to
    /// encode an addressing mode. Sometimes, certain combinations of the `mod` and `r/m` fields
    /// are use to express opcode information for certain instructions.
    rm: u8,
}

/// See ModR/M and SIB Bytes (section 2.1.3, Intel Software Developer's Manual vol. 2A).
struct SIB {
    /// Bits 7:6: `scale` specifies the scale factor.
    scale: u8,
    /// Bits 5:3: `index` specifies the register number of the index register.
    index: u8,
    /// Bits 2:0: `base` specifies the register number of the base register.
    base: u8,
}

struct VexFormat {
    vex: VexPrefix,
    opcode: u8,
    modrm: ModRM,
    sib: Option<SIB>,
}

enum VexPrefix {
    C5(TwoByteVexPrefix),
    C4(ThreeByteVexPrefix),
}

/// See VEX Prefix (section 2.3.5, Intel Software Developer's Manual vol. 2A). The first byte
/// (byte 1) is always `0xC5`.
struct TwoByteVexPrefix {
    /// Byte 1, bit 7: REX.R in complement (inverted) form. VEX.R=1 is equivalent to REX.R=0
    /// (must be 1 in 32-bit mode); VEX.R=0 is equivalent to REX.R=1 (64-bit mode only)
    r: bool,
    /// Byte 1, bits 6:3: Source or destination register specifier. For instructions with two or
    /// more source registers, encodes the first source register specifier. `VEX.vvvv` is not  
    /// used by instructions with one source (except certain shifts) or on instructions with no  
    /// XMM or YMM or memory destination. If an instruction does not use `VEX.vvvv` it should be
    /// set to `0b1111`; otherwise the instruction will be `#UD`.
    vvvvv: u8,
    /// Byte 1, bit 2:
    l: VexVectorLength,
    /// Byte 1, bits 1:0: Opcode extension providing equivalent functionality of a SIMD prefix.
    pp: VexSimdPrefix,
}

/// See VEX Prefix (section 2.3.5, Intel Software Developer's Manual vol. 2A). The first byte
/// (byte 1) is always `0xC4`.
struct ThreeByteVexPrefix {
    /// Byte 1, bit 7: REX.R in complement (inverted) form. VEX.R=1 is equivalent to REX.R=0
    /// (must be 1 in 32-bit mode); VEX.R=0 is equivalent to REX.R=1 (64-bit mode only)
    r: bool,
    /// Byte 1, bit 6: REX.X in complement (inverted) form. VEX.X=1 is equivalent to REX.X=0
    /// (must be 1 in 32-bit mode); VEX.X=0 is equivalent to REX.X=1 (64-bit mode only)
    x: bool,
    /// Byte 1, bit 5: REX.B in complement (inverted) form. VEX.B=1 is equivalent to REX.B=0
    /// (must be 1 in 32-bit mode); VEX.B=0 is equivalent to REX.B=1 (64-bit mode only)
    b: bool,
    /// Byte 1, bits 4:0: Replaces the REX leading opcode bytes.
    mmmmmm: VexLeadingOpcodeBytes,
    /// Byte 2, bit 7:
    w: bool,
    /// Byte 2, bits 6:3: Source or destination register specifier. For instructions with two or
    /// more source registers, encodes the first source register specifier. `VEX.vvvv` is not  
    /// used by instructions with one source (except certain shifts) or on instructions with no
    /// XMM or YMM or memory destination. If an instruction does not use `VEX.vvvv` it hould be  
    /// set to `0b1111`; otherwise the instruction will be `#UD`.
    vvvvv: u8,
    /// Byte 2, bit 2:
    l: VexVectorLength,
    /// Byte 2, bits 1:0: Opcode extension providing equivalent functionality of a SIMD prefix.
    pp: VexSimdPrefix,
}

enum VexLeadingOpcodeBytes {
    Implied0F,
    Implied0F38,
    Implied0F3A,
}

enum VexVectorLength {
    ScalarOrVector128Bit,
    Vector256Bit,
}

enum VexSimdPrefix {
    None,
    Prefix66,
    PrefixF3,
    PrefixF2,
}
