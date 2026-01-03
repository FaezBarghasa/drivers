//! DEX File Parser
//!
//! Parses Dalvik Executable format for class loading.

use std::collections::BTreeMap;

use crate::errno::AndroidError;

/// DEX magic number
const DEX_MAGIC: [u8; 4] = [0x64, 0x65, 0x78, 0x0a]; // "dex\n"

/// DEX file header
#[derive(Debug, Clone)]
pub struct DexHeader {
    pub magic: [u8; 8],
    pub checksum: u32,
    pub signature: [u8; 20],
    pub file_size: u32,
    pub header_size: u32,
    pub endian_tag: u32,
    pub link_size: u32,
    pub link_off: u32,
    pub map_off: u32,
    pub string_ids_size: u32,
    pub string_ids_off: u32,
    pub type_ids_size: u32,
    pub type_ids_off: u32,
    pub proto_ids_size: u32,
    pub proto_ids_off: u32,
    pub field_ids_size: u32,
    pub field_ids_off: u32,
    pub method_ids_size: u32,
    pub method_ids_off: u32,
    pub class_defs_size: u32,
    pub class_defs_off: u32,
    pub data_size: u32,
    pub data_off: u32,
}

/// String ID item
#[derive(Debug, Clone)]
pub struct StringId {
    pub string_data_off: u32,
}

/// Type ID item
#[derive(Debug, Clone)]
pub struct TypeId {
    pub descriptor_idx: u32,
}

/// Method ID item
#[derive(Debug, Clone)]
pub struct MethodId {
    pub class_idx: u16,
    pub proto_idx: u16,
    pub name_idx: u32,
}

/// Field ID item
#[derive(Debug, Clone)]
pub struct FieldId {
    pub class_idx: u16,
    pub type_idx: u16,
    pub name_idx: u32,
}

/// Class definition
#[derive(Debug, Clone)]
pub struct ClassDef {
    pub class_idx: u32,
    pub access_flags: u32,
    pub superclass_idx: u32,
    pub interfaces_off: u32,
    pub source_file_idx: u32,
    pub annotations_off: u32,
    pub class_data_off: u32,
    pub static_values_off: u32,
}

/// Access flags for classes, fields, and methods
pub mod access_flags {
    pub const ACC_PUBLIC: u32 = 0x0001;
    pub const ACC_PRIVATE: u32 = 0x0002;
    pub const ACC_PROTECTED: u32 = 0x0004;
    pub const ACC_STATIC: u32 = 0x0008;
    pub const ACC_FINAL: u32 = 0x0010;
    pub const ACC_SYNCHRONIZED: u32 = 0x0020;
    pub const ACC_VOLATILE: u32 = 0x0040;
    pub const ACC_BRIDGE: u32 = 0x0040;
    pub const ACC_TRANSIENT: u32 = 0x0080;
    pub const ACC_VARARGS: u32 = 0x0080;
    pub const ACC_NATIVE: u32 = 0x0100;
    pub const ACC_INTERFACE: u32 = 0x0200;
    pub const ACC_ABSTRACT: u32 = 0x0400;
    pub const ACC_STRICT: u32 = 0x0800;
    pub const ACC_SYNTHETIC: u32 = 0x1000;
    pub const ACC_ANNOTATION: u32 = 0x2000;
    pub const ACC_ENUM: u32 = 0x4000;
    pub const ACC_CONSTRUCTOR: u32 = 0x10000;
    pub const ACC_DECLARED_SYNCHRONIZED: u32 = 0x20000;
}

/// Parsed DEX file
pub struct DexFile {
    pub header: DexHeader,
    pub strings: Vec<String>,
    pub types: Vec<TypeId>,
    pub methods: Vec<MethodId>,
    pub fields: Vec<FieldId>,
    pub classes: Vec<ClassDef>,
    data: Vec<u8>,
}

impl DexFile {
    /// Parse a DEX file from bytes
    pub fn parse(data: Vec<u8>) -> Result<Self, AndroidError> {
        if data.len() < 112 {
            return Err(AndroidError::InvalidApk);
        }

        // Verify magic
        if &data[0..4] != DEX_MAGIC {
            return Err(AndroidError::InvalidApk);
        }

        // Parse header
        let header = DexHeader {
            magic: [
                data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
            ],
            checksum: u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
            signature: {
                let mut sig = [0u8; 20];
                sig.copy_from_slice(&data[12..32]);
                sig
            },
            file_size: u32::from_le_bytes([data[32], data[33], data[34], data[35]]),
            header_size: u32::from_le_bytes([data[36], data[37], data[38], data[39]]),
            endian_tag: u32::from_le_bytes([data[40], data[41], data[42], data[43]]),
            link_size: u32::from_le_bytes([data[44], data[45], data[46], data[47]]),
            link_off: u32::from_le_bytes([data[48], data[49], data[50], data[51]]),
            map_off: u32::from_le_bytes([data[52], data[53], data[54], data[55]]),
            string_ids_size: u32::from_le_bytes([data[56], data[57], data[58], data[59]]),
            string_ids_off: u32::from_le_bytes([data[60], data[61], data[62], data[63]]),
            type_ids_size: u32::from_le_bytes([data[64], data[65], data[66], data[67]]),
            type_ids_off: u32::from_le_bytes([data[68], data[69], data[70], data[71]]),
            proto_ids_size: u32::from_le_bytes([data[72], data[73], data[74], data[75]]),
            proto_ids_off: u32::from_le_bytes([data[76], data[77], data[78], data[79]]),
            field_ids_size: u32::from_le_bytes([data[80], data[81], data[82], data[83]]),
            field_ids_off: u32::from_le_bytes([data[84], data[85], data[86], data[87]]),
            method_ids_size: u32::from_le_bytes([data[88], data[89], data[90], data[91]]),
            method_ids_off: u32::from_le_bytes([data[92], data[93], data[94], data[95]]),
            class_defs_size: u32::from_le_bytes([data[96], data[97], data[98], data[99]]),
            class_defs_off: u32::from_le_bytes([data[100], data[101], data[102], data[103]]),
            data_size: u32::from_le_bytes([data[104], data[105], data[106], data[107]]),
            data_off: u32::from_le_bytes([data[108], data[109], data[110], data[111]]),
        };

        // Parse strings
        let strings = Self::parse_strings(&data, &header)?;

        // Parse types (simplified)
        let types = Vec::new();
        let methods = Vec::new();
        let fields = Vec::new();
        let classes = Vec::new();

        Ok(DexFile {
            header,
            strings,
            types,
            methods,
            fields,
            classes,
            data,
        })
    }

    fn parse_strings(data: &[u8], header: &DexHeader) -> Result<Vec<String>, AndroidError> {
        let mut strings = Vec::with_capacity(header.string_ids_size as usize);
        let off = header.string_ids_off as usize;

        for i in 0..header.string_ids_size as usize {
            let id_off = off + i * 4;
            if id_off + 4 > data.len() {
                break;
            }

            let string_data_off = u32::from_le_bytes([
                data[id_off],
                data[id_off + 1],
                data[id_off + 2],
                data[id_off + 3],
            ]) as usize;

            if string_data_off >= data.len() {
                strings.push(String::new());
                continue;
            }

            // Read ULEB128 length then MUTF-8 string
            let (len, bytes_read) = read_uleb128(&data[string_data_off..]);
            let start = string_data_off + bytes_read;
            let end = (start + len as usize).min(data.len());

            if let Ok(s) = String::from_utf8(data[start..end].to_vec()) {
                strings.push(s);
            } else {
                strings.push(String::from_utf8_lossy(&data[start..end]).to_string());
            }
        }

        Ok(strings)
    }

    /// Get a string by index
    pub fn get_string(&self, idx: u32) -> Option<&str> {
        self.strings.get(idx as usize).map(|s| s.as_str())
    }

    /// Get type name by index
    pub fn get_type_name(&self, idx: u32) -> Option<&str> {
        self.types
            .get(idx as usize)
            .and_then(|t| self.get_string(t.descriptor_idx))
    }
}

/// Read ULEB128 encoded value
fn read_uleb128(data: &[u8]) -> (u32, usize) {
    let mut result = 0u32;
    let mut shift = 0;
    let mut bytes_read = 0;

    for byte in data.iter() {
        bytes_read += 1;
        result |= ((byte & 0x7f) as u32) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }

    (result, bytes_read)
}

/// Dalvik bytecode opcodes
pub mod opcode {
    pub const NOP: u8 = 0x00;
    pub const MOVE: u8 = 0x01;
    pub const MOVE_FROM16: u8 = 0x02;
    pub const MOVE_16: u8 = 0x03;
    pub const MOVE_WIDE: u8 = 0x04;
    pub const MOVE_OBJECT: u8 = 0x07;
    pub const MOVE_RESULT: u8 = 0x0a;
    pub const MOVE_EXCEPTION: u8 = 0x0d;
    pub const RETURN_VOID: u8 = 0x0e;
    pub const RETURN: u8 = 0x0f;
    pub const RETURN_WIDE: u8 = 0x10;
    pub const RETURN_OBJECT: u8 = 0x11;
    pub const CONST_4: u8 = 0x12;
    pub const CONST_16: u8 = 0x13;
    pub const CONST: u8 = 0x14;
    pub const CONST_STRING: u8 = 0x1a;
    pub const CONST_CLASS: u8 = 0x1c;
    pub const NEW_INSTANCE: u8 = 0x22;
    pub const NEW_ARRAY: u8 = 0x23;
    pub const INVOKE_VIRTUAL: u8 = 0x6e;
    pub const INVOKE_SUPER: u8 = 0x6f;
    pub const INVOKE_DIRECT: u8 = 0x70;
    pub const INVOKE_STATIC: u8 = 0x71;
    pub const INVOKE_INTERFACE: u8 = 0x72;
    pub const IGET: u8 = 0x52;
    pub const IPUT: u8 = 0x59;
    pub const SGET: u8 = 0x60;
    pub const SPUT: u8 = 0x67;
}
