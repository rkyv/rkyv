use rkyv::{
    access_unchecked, deserialize,
    rancor::{Error, Fallible},
    ser::{Allocator, Writer},
    vec::{ArchivedVec, VecResolver},
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Archived, Deserialize, Place, Serialize,
};

#[derive(Debug, PartialEq, Eq)]
pub enum Opcode {
    // A 1-byte opcode
    OneByte,
    // A 2-byte opcode
    TwoBytes(u8),
    // A 3-byte opcode
    ThreeBytes(u16),
    // A 5-byte opcode
    FiveBytes(u32),
    // A 9-byte opcode
    NineBytes(u64),
    // A variable-length opcode
    VariableLength(usize),
}

pub struct EncodeOpcodes;

pub struct OpcodesResolver {
    len: usize,
    inner: VecResolver,
}

impl ArchiveWith<Vec<Opcode>> for EncodeOpcodes {
    type Archived = ArchivedVec<u8>;
    type Resolver = OpcodesResolver;

    fn resolve_with(
        _: &Vec<Opcode>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        ArchivedVec::resolve_from_len(resolver.len, resolver.inner, out);
    }
}

impl<S> SerializeWith<Vec<Opcode>, S> for EncodeOpcodes
where
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize_with(
        field: &Vec<Opcode>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        // Encode opcodes into a compact binary format
        // We'll do it manually here, but you could just as easily proxy out to
        // a serialization framework like postcard
        let mut encoded = Vec::new();
        for opcode in field.iter() {
            match opcode {
                Opcode::OneByte => encoded.push(0),
                Opcode::TwoBytes(arg) => {
                    encoded.push(1);
                    encoded.extend(arg.to_le_bytes());
                }
                Opcode::ThreeBytes(arg) => {
                    encoded.push(2);
                    encoded.extend(arg.to_le_bytes());
                }
                Opcode::FiveBytes(arg) => {
                    encoded.push(3);
                    encoded.extend(arg.to_le_bytes());
                }
                Opcode::NineBytes(arg) => {
                    encoded.push(4);
                    encoded.extend(arg.to_le_bytes());
                }
                Opcode::VariableLength(arg) => {
                    let mut arg = *arg;
                    let bytes = arg.to_le_bytes();

                    let mut len = 1;
                    while arg >= 256 {
                        arg >>= 8;
                        len += 1;
                    }

                    encoded.push(4 + len as u8);
                    encoded.extend(&bytes[0..len]);
                }
            }
        }

        // Serialize encoded opcodes
        Ok(OpcodesResolver {
            len: encoded.len(),
            inner: ArchivedVec::serialize_from_slice(
                encoded.as_slice(),
                serializer,
            )?,
        })
    }
}

impl<D> DeserializeWith<Archived<Vec<u8>>, Vec<Opcode>, D> for EncodeOpcodes
where
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &Archived<Vec<u8>>,
        _: &mut D,
    ) -> Result<Vec<Opcode>, D::Error> {
        let mut result = Vec::new();

        // Decode opcodes from a compact binary format
        let mut bytes = field.iter().cloned();
        while let Some(op) = bytes.next() {
            match op {
                0 => result.push(Opcode::OneByte),
                1 => {
                    let arg = bytes.next().unwrap();
                    result.push(Opcode::TwoBytes(arg));
                }
                2 => {
                    let arg = bytes.next().unwrap() as u16
                        | (bytes.next().unwrap() as u16) << 8;
                    result.push(Opcode::ThreeBytes(arg));
                }
                3 => {
                    let arg = bytes.next().unwrap() as u32
                        | (bytes.next().unwrap() as u32) << 8
                        | (bytes.next().unwrap() as u32) << 16
                        | (bytes.next().unwrap() as u32) << 24;
                    result.push(Opcode::FiveBytes(arg));
                }
                4 => {
                    let arg = bytes.next().unwrap() as u64
                        | (bytes.next().unwrap() as u64) << 8
                        | (bytes.next().unwrap() as u64) << 16
                        | (bytes.next().unwrap() as u64) << 24
                        | (bytes.next().unwrap() as u64) << 32
                        | (bytes.next().unwrap() as u64) << 40
                        | (bytes.next().unwrap() as u64) << 48
                        | (bytes.next().unwrap() as u64) << 56;
                    result.push(Opcode::NineBytes(arg));
                }
                n @ 5..=12 => {
                    let len = n - 4;
                    let mut arg = 0;
                    for i in 0..len {
                        arg |= (bytes.next().unwrap() as usize) << (8 * i);
                    }
                    result.push(Opcode::VariableLength(arg));
                }
                // Either the deserializer can be bound to support decode
                // errors, or the opcodes can be checked during
                // validation with bytecheck
                _ => panic!("unexpected opcode"),
            }
        }

        Ok(result)
    }
}

#[derive(Archive, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Program {
    #[rkyv(with = EncodeOpcodes)]
    opcodes: Vec<Opcode>,
}

fn main() {
    let program = Program {
        opcodes: vec![
            Opcode::OneByte,
            Opcode::TwoBytes(42),
            Opcode::ThreeBytes(27774),
            Opcode::FiveBytes(31415926),
            Opcode::NineBytes(123456789123456789),
            Opcode::VariableLength(27774),
        ],
    };
    println!("opcodes: {:?}", program.opcodes);

    let buf = rkyv::to_bytes::<Error>(&program).unwrap();
    let archived_program = unsafe { access_unchecked::<ArchivedProgram>(&buf) };

    println!("encoded: {:?}", archived_program.opcodes);
    assert_eq!(archived_program.opcodes.len(), 23);

    let deserialized_program =
        deserialize::<Program, Error>(archived_program).unwrap();

    println!("deserialized opcodes: {:?}", deserialized_program.opcodes);
    assert_eq!(program, deserialized_program);
}
