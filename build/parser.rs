use crc16;
use std::cmp::Ordering;
use std::default::Default;
use std::io::{Read, Write};

use xml::reader::{EventReader, XmlEvent};

use quote::{Ident, Tokens};
use rustfmt;

#[derive(Debug, PartialEq, Clone)]
pub struct MavEnum {
    pub name: String,
    pub description: Option<String>,
    pub entries: Vec<MavEnumEntry>,
}

impl Default for MavEnum {
    fn default() -> MavEnum {
        MavEnum {
            name: "".into(),
            description: None,
            entries: vec![],
        }
    }
}

impl MavEnum {
    #[allow(dead_code)]
    fn has_enum_values(&self) -> bool {
        // add values for enums that don't have any value specified
        let sum = self.entries.iter().map(|x| x.value).fold(0, |mut sum, x| {
            sum += x;
            sum
        });
        if sum == 0 {
            false
        } else {
            true
        }
    }

    #[allow(dead_code)]
    fn emit_proto_defs(&self) -> Vec<Tokens> {
        let mut cnt = 0;
        self.entries
            .iter()
            .map(|enum_entry| {
                let name = Ident::from(enum_entry.name.clone());
                let value;
                if !self.has_enum_values() {
                    value = Ident::from(cnt.to_string());
                    cnt += 1;
                } else {
                    value = Ident::from(enum_entry.value.to_string());
                };
                quote!(#name = #value;)
            })
            .collect::<Vec<Tokens>>()
    }

    #[allow(dead_code)]
    fn emit_proto_names(&self) -> Tokens {
        /*
        let name = self
            .name
            .split("_")
            .map(|x| x.to_lowercase())
            .map(|x| {
                let mut v: Vec<char> = x.chars().collect();
                v[0] = v[0].to_uppercase().nth(0).unwrap();
                v.into_iter().collect()
            })
            .collect::<Vec<String>>()
            .join("");
            */
        let name = Ident::from(self.name.clone());
        quote!(#name)
    }

    #[allow(dead_code)]
    fn emit_proto(&self) -> Tokens {
        let defs = self.emit_proto_defs();
        let enum_name = self.emit_proto_names();

        quote!{
            enum #enum_name {
                #(#defs)*
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct MavEnumEntry {
    pub value: i32,
    pub name: String,
    pub description: Option<String>,
    pub params: Option<Vec<String>>,
}

impl Default for MavEnumEntry {
    fn default() -> MavEnumEntry {
        MavEnumEntry {
            value: 0,
            name: "".into(),
            description: None,
            params: None,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct MavMessage {
    pub id: u8,
    pub name: String,
    pub description: Option<String>,
    pub fields: Vec<MavField>,
}

impl Default for MavMessage {
    fn default() -> MavMessage {
        MavMessage {
            id: 0,
            name: "".into(),
            description: None,
            fields: vec![],
        }
    }
}

impl MavMessage {
    /// Return Token of "MESSAGE_NAME_DATA
    /// for mavlink struct data
    fn emit_struct_name(&self) -> Tokens {
        let name = Ident::from(format!("{}Data", self.name));
        quote!(#name)
    }
    fn emit_name_types(&self) -> Vec<Tokens> {
        let mut cnt = 1;
        self.fields
            .iter()
            .map(|field| {
                let nametype = field.emit_name_type();
                let val = Ident::from(format!("\"{}\"", cnt));
                let proto_type = Ident::from(field.mavtype.proto_type());
                let field_rule = match field.mavtype {
                    MavType::Array(_, _) => Ident::from(format!("repeated")),
                    _ => Ident::from(format!("required")),
                };
                cnt += 1;
                quote!{
                    #[prost(#proto_type, #field_rule, tag= #val )]
                    #nametype
                }
            })
            .collect::<Vec<Tokens>>()
    }

    #[allow(dead_code)]
    fn emit_field_names(&self) -> Vec<Tokens> {
        self.fields
            .iter()
            .map(|field| field.emit_name())
            .collect::<Vec<Tokens>>()
    }

    #[allow(dead_code)]
    fn emit_field_types(&self) -> Vec<Tokens> {
        self.fields
            .iter()
            .map(|field| field.emit_type())
            .collect::<Vec<Tokens>>()
    }

    fn emit_rust_readers(&self) -> Vec<Tokens> {
        self.fields
            .iter()
            .map(|field| field.emit_reader())
            .collect::<Vec<Tokens>>()
    }

    fn emit_rust_writers(&self) -> Vec<Tokens> {
        self.fields
            .iter()
            .map(|field| field.emit_writer())
            .collect::<Vec<Tokens>>()
    }

    fn emit_rust(&self) -> Tokens {
        let msg_name = self.emit_struct_name();
        let name_types = self.emit_name_types();
        let readers = self.emit_rust_readers();
        let writers = self.emit_rust_writers();
        let comment = Ident::from(format!("/// id: {}\n", self.id));

        quote!{
            #comment
            #[derive(Clone, PartialEq, Message)]
            #[derive(Serialize, Deserialize)]
            pub struct #msg_name {
                // to make deserialiation work (we need to insert serde_ attributes for f32 and f64 fields)
                #(#name_types)*
            }

            impl Parsable for #msg_name {
                fn parse(payload: &[u8]) -> #msg_name {
                    let mut cur = Cursor::new(payload);
                    #msg_name {
                        #(#readers)*
                    }
                }

                fn serialize(&self) -> Vec<u8> {
                    let mut wtr = vec![];
                    #(#writers)*
                    wtr
                }
            }
        }
    }

    /// Message name in protbuf format, i.e MessageName
    fn emit_proto_name(&self) -> Tokens {
        /*
        let name = self
            .name
            .split("_")
            .map(|x| x.to_lowercase())
            .map(|x| {
                let mut v: Vec<char> = x.chars().collect();
                v[0] = v[0].to_uppercase().nth(0).unwrap();
                v.into_iter().collect()
            })
            .collect::<Vec<String>>()
            .join("");
            */
        let name = Ident::from(self.name.clone());
        quote!(#name)
    }

    /// Create protobuf message fields definitions
    /// e.g. "required uint32 time_boot_ms = 1;"
    fn emit_proto_defs(&self) -> Vec<Tokens> {
        let mut cnt = 1;
        self.fields
            .iter()
            .map(|msg_field| {
                let name = Ident::from(msg_field.name.clone());
                let value = Ident::from(cnt.to_string());
                cnt += 1;
                match msg_field.enumtype {
                    Some(ref enumtype) => {
                        // TODO: generate enums too
                        let _enum_name = enumtype; /*
                            .split("_")
                            .map(|x| x.to_lowercase())
                            .map(|x| {
                                let mut v: Vec<char> = x.chars().collect();
                                v[0] = v[0].to_uppercase().nth(0).unwrap();
                                v.into_iter().collect()
                            })
                            .collect::<Vec<String>>()
                            .join("");
                            */
                        //let mavtype = Ident::from(_enum_name.clone());
                        // Create a uint32 instead of enum for now
                        let mavtype = Ident::from("uint32".to_string());
                        quote!(required #mavtype #name = #value;)
                    }
                    None => {
                        let mavtype = Ident::from(msg_field.mavtype.proto_type());
                        match msg_field.mavtype {
                            MavType::Array(_, _) => quote!(repeated #mavtype #name = #value;),
                            _ => quote!(required #mavtype #name = #value;),
                        }
                    }
                }
            })
            .collect::<Vec<Tokens>>()
    }

    fn emit_proto(&self) -> Tokens {
        let defs = self.emit_proto_defs();
        let msg_name = self.emit_proto_name();

        let comment = Ident::from(format!("\n// id: {} {} \n", self.id, self.name));
        quote!{
            #comment
            message #msg_name {
                #(#defs)*
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum MavType {
    UInt8MavlinkVersion,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Int8,
    Int16,
    Int32,
    Int64,
    Char,
    Float,
    Double,
    Array(Box<MavType>, usize),
}

fn parse_type(s: &str) -> Option<MavType> {
    use parser::MavType::*;
    match s {
        "uint8_t_mavlink_version" => Some(UInt8MavlinkVersion),
        "uint8_t" => Some(UInt8),
        "uint16_t" => Some(UInt16),
        "uint32_t" => Some(UInt32),
        "uint64_t" => Some(UInt64),
        "int8_t" => Some(Int8),
        "int16_t" => Some(Int16),
        "int32_t" => Some(Int32),
        "int64_t" => Some(Int64),
        "char" => Some(Char),
        "float" => Some(Float),
        "Double" => Some(Double),
        _ => {
            if s.ends_with("]") {
                let start = s.find("[").unwrap();
                let size = s[start + 1..(s.len() - 1)].parse::<usize>().unwrap();
                let mtype = parse_type(&s[0..start]).unwrap();
                Some(Array(Box::new(mtype), size))
            } else {
                panic!("UNHANDLED {:?}", s);
            }
        }
    }
}

impl MavType {
    /// Size of a given Mavtype
    fn len(&self) -> usize {
        use parser::MavType::*;
        match self.clone() {
            UInt8MavlinkVersion | UInt8 | Int8 | Char => 1,
            UInt16 | Int16 => 2,
            UInt32 | Int32 | Float => 4,
            UInt64 | Int64 | Double => 8,
            Array(t, size) => t.len() * size,
        }
    }

    /// Used for ordering of types
    fn order_len(&self) -> usize {
        use parser::MavType::*;
        match self.clone() {
            UInt8MavlinkVersion | UInt8 | Int8 | Char => 1,
            UInt16 | Int16 => 2,
            UInt32 | Int32 | Float => 4,
            UInt64 | Int64 | Double => 8,
            Array(t, _) => t.len(),
        }
    }

    /// Used for crc calculation
    pub fn primitive_type(&self) -> String {
        use parser::MavType::*;
        match self.clone() {
            UInt8MavlinkVersion => "uint8_t".into(),
            UInt8 => "uint8_t".into(),
            Int8 => "int8_t".into(),
            Char => "char".into(),
            UInt16 => "uint16_t".into(),
            Int16 => "int16_t".into(),
            UInt32 => "uint32_t".into(),
            Int32 => "int32_t".into(),
            Float => "float".into(),
            UInt64 => "uint64_t".into(),
            Int64 => "int64_t".into(),
            Double => "double".into(),
            Array(t, _) => t.primitive_type(),
        }
    }

    /// Return rust equivalent of a given Mavtype
    /// Used for generating struct fields.
    /// Note, the smallest type is u32 to make it compatible
    /// with protobuf protocol
    pub fn rust_type(&self) -> String {
        use parser::MavType::*;
        match self.clone() {
            UInt8 | UInt8MavlinkVersion | Char | UInt16 | UInt32 => "u32".into(),
            Int8 | Int16 | Int32 => "i32".into(),
            Float => "f32".into(),
            UInt64 => "u64".into(),
            Int64 => "i64".into(),
            Double => "f64".into(),
            // Buffer(n) => "u8".into(),
            Array(t, size) => format!("Vec<{}> /* {} */", t.rust_type(), size),
        }
    }

    /// Emit the type for serialization/deserialization
    pub fn rust_serde_type(&self) -> String {
        use parser::MavType::*;
        match self.clone() {
            UInt8 | UInt8MavlinkVersion => "u8".into(),
            Int8 => "i8".into(),
            Char => "u8".into(),
            UInt16 => "u16".into(),
            Int16 => "i16".into(),
            UInt32 => "u32".into(),
            Int32 => "i32".into(),
            Float => "f32".into(),
            UInt64 => "u64".into(),
            Int64 => "i64".into(),
            Double => "f64".into(),
            // Buffer(n) => "u8".into(),
            Array(t, size) => format!("Vec<{}> /* {} */", t.rust_serde_type(), size),
        }
    }

    /// Return protobuf equivalent of a given Mavtype
    /// Used for generating *.proto files
    pub fn proto_type(&self) -> String {
        use parser::MavType::*;
        match self {
            UInt8MavlinkVersion | Char | UInt8 | UInt16 | UInt32 => "uint32".into(),
            Int8 | Int16 | Int32 => "int32".into(),
            Float => "float".into(),
            UInt64 => "uint64".into(),
            Int64 => "int64".into(),
            Double => "double".into(),
            Array(t, _) => match *t.clone() {
                Array(_, _) => panic!("Error matching Mavtype"),
                UInt8MavlinkVersion | Char | UInt8 | UInt16 | UInt32 => "uint32".into(),
                Int8 | Int16 | Int32 => "int32".into(),
                Float => "float".into(),
                UInt64 => "uint64".into(),
                Int64 => "int64".into(),
                Double => "double".into(),
            },
        }
    }

    /// Compare two MavTypes
    pub fn compare(&self, other: &Self) -> Ordering {
        let len = self.order_len();
        (-(len as isize)).cmp(&(-(other.order_len() as isize)))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct MavField {
    pub mavtype: MavType,
    pub name: String,
    pub description: Option<String>,
    pub enumtype: Option<String>,
}

impl Default for MavField {
    fn default() -> MavField {
        MavField {
            mavtype: MavType::UInt8,
            name: "".into(),
            description: None,
            enumtype: None,
        }
    }
}

impl MavField {
    fn emit_name(&self) -> Tokens {
        let name = Ident::from(self.name.clone());
        quote!(#name)
    }

    fn emit_type(&self) -> Tokens {
        let mavtype = Ident::from(self.mavtype.rust_type());
        quote!(#mavtype)
    }

    fn emit_name_type(&self) -> Tokens {
        let name = self.emit_name();
        let mavtype = self.emit_type();
        match self.mavtype {
            MavType::Float => {
                quote!{
                    #[serde(deserialize_with="parse_f32")]
                    #name: #mavtype,
                }
            }
            MavType::Double => {
                quote!{
                    #[serde(deserialize_with="parse_f64")]
                    #name: #mavtype,
                }
            }
            _ => quote!(#name: #mavtype,),
        }
    }

    fn emit_writer(&self) -> Tokens {
        let name = self.emit_name();
        let mut writer = quote!();
        match self.mavtype {
            // we have to downcast to smaller types (u8/i8)
            MavType::Char | MavType::UInt8 | MavType::Int8 | MavType::UInt8MavlinkVersion => {
                let write = Ident::from(format!("write_{}", self.mavtype.rust_serde_type()));
                let cast_to = Ident::from(format!("{}", self.mavtype.rust_serde_type()));
                writer = quote!{
                    wtr.#write(self.#name as #cast_to).unwrap();
                };
            }
            // we have to downcast to smaller types (u16/i16)
            MavType::UInt16 | MavType::Int16 => {
                let write = Ident::from(format!("write_{}", self.mavtype.rust_serde_type()));
                let cast_to = Ident::from(format!("{}", self.mavtype.rust_serde_type()));
                writer = quote!{
                    wtr.#write::<LittleEndian>(self.#name as #cast_to).unwrap();
                };
            }
            MavType::Array(ref t, size) => {
                for idx in 0..size {
                    match *t.clone() {
                        // we have to downcast to a smaller type (u8/i8)
                        MavType::Char
                        | MavType::UInt8
                        | MavType::Int8
                        | MavType::UInt8MavlinkVersion => {
                            let write = Ident::from(format!("write_{}", t.rust_serde_type()));
                            let cast_to = Ident::from(format!("{}", t.rust_serde_type()));
                            let index = Ident::from(idx.to_string());
                            writer.append(quote!{
                                    wtr.#write(self.#name[#index] as #cast_to).unwrap();
                            });
                        }
                        // we have to downcast to a smaller type (u8/i8)
                        MavType::Int16 | MavType::UInt16 => {
                            let write = Ident::from(format!("write_{}", t.rust_serde_type()));
                            let cast_to = Ident::from(format!("{}", t.rust_serde_type()));
                            let index = Ident::from(idx.to_string());
                            writer.append(quote!{
                                    wtr.#write::<LittleEndian>(self.#name[#index] as #cast_to).unwrap();
                            });
                        }
                        MavType::Array(_, _) => {
                            panic!("error");
                        }
                        _ => {
                            let write = Ident::from(format!("write_{}", t.rust_serde_type()));
                            let index = Ident::from(idx.to_string());
                            writer.append(quote!{
                                wtr.#write::<LittleEndian>(self.#name[#index]).unwrap();
                            });
                        }
                    }
                }
            }
            _ => {
                let write = Ident::from(format!("write_{}", self.mavtype.rust_serde_type()));
                writer = quote!{
                    wtr.#write::<LittleEndian>(self.#name).unwrap();
                };
            }
        }
        quote!(#writer)
    }

    fn emit_reader(&self) -> Tokens {
        let name = self.emit_name();
        let reader;
        match self.mavtype {
            MavType::Char | MavType::UInt8 | MavType::Int8 | MavType::UInt8MavlinkVersion => {
                let read = Ident::from(format!("read_{}", self.mavtype.rust_serde_type()));
                let cast_to = Ident::from(format!("{}", self.mavtype.rust_type()));
                reader = quote!{
                    #name : cur. #read ().unwrap() as #cast_to,
                };
            }
            MavType::Array(ref t, size) => {
                let read;
                match *t.clone() {
                    MavType::Char
                    | MavType::UInt8
                    | MavType::Int8
                    | MavType::UInt8MavlinkVersion => {
                        read = Ident::from(format!("read_{}", t.rust_serde_type()));
                    }
                    MavType::Array(_, _) => {
                        panic!("error parsing message field");
                    }
                    _ => {
                        read = Ident::from(format!("read_{}::<LittleEndian>", t.rust_serde_type()));
                    }
                }
                let size = Ident::from(size.to_string());
                let cast_to = Ident::from(format!("{}", t.rust_type()));
                reader = quote!{
                    #name: {
                        let mut v = Vec::with_capacity(#size);
                        for _ in 0..#size {
                            v.push(cur.#read().unwrap() as #cast_to);
                        }
                        v
                    },
                };
            }
            _ => {
                let read = Ident::from(format!("read_{}", self.mavtype.rust_serde_type()));
                let cast_to = Ident::from(format!("{}", self.mavtype.rust_type()));
                reader = quote!{
                    #name : cur. #read::<LittleEndian> ().unwrap() as #cast_to,
                };
            }
        }
        quote!(#reader)
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum MavXmlElement {
    Version,
    Mavlink,
    Include,
    Enums,
    Enum,
    Entry,
    Description,
    Param,
    Messages,
    Message,
    Field,
}

fn identify_element(s: &str) -> Option<MavXmlElement> {
    use parser::MavXmlElement::*;
    match s {
        "version" => Some(Version),
        "mavlink" => Some(Mavlink),
        "include" => Some(Include),
        "enums" => Some(Enums),
        "enum" => Some(Enum),
        "entry" => Some(Entry),
        "description" => Some(Description),
        "param" => Some(Param),
        "messages" => Some(Messages),
        "message" => Some(Message),
        "field" => Some(Field),
        _ => None,
    }
}

fn is_valid_parent(p: Option<MavXmlElement>, s: MavXmlElement) -> bool {
    use parser::MavXmlElement::*;
    match s {
        Version => p == Some(Mavlink),
        Mavlink => p == None,
        Include => p == Some(Mavlink),
        Enums => p == Some(Mavlink),
        Enum => p == Some(Enums),
        Entry => p == Some(Enum),
        Description => p == Some(Entry) || p == Some(Message) || p == Some(Enum),
        Param => p == Some(Entry),
        Messages => p == Some(Mavlink),
        Message => p == Some(Messages),
        Field => p == Some(Message),
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct MavProfile {
    pub includes: Vec<String>,
    pub messages: Vec<MavMessage>,
    pub enums: Vec<MavEnum>,
}

impl MavProfile {
    #[allow(dead_code)]
    fn emit_proto_enums(&self) -> Vec<Tokens> {
        self.enums
            .iter()
            .map(|d| d.emit_proto())
            .collect::<Vec<Tokens>>()
    }

    fn emit_proto_msgs(&self) -> Vec<Tokens> {
        self.messages
            .iter()
            .map(|d| d.emit_proto())
            .collect::<Vec<Tokens>>()
    }

    fn emit_proto_msg_names(&self) -> Vec<Tokens> {
        let mut cnt = 1;
        self.messages
            .iter()
            .map(|msg| {
                let msg_type_name = msg.emit_proto_name();
                let msg_field_name = Ident::from(msg.name.to_lowercase());
                let msg_field_num = Ident::from(cnt.to_string());
                cnt += 1;
                quote!{
                    //optional #msg_type_name #msg_field_name = #msg_field_num;
                    #msg_type_name #msg_field_name = #msg_field_num;
                }
            })
            .collect::<Vec<Tokens>>()
    }

    /// Emit proto file
    fn emit_proto(&self) -> Tokens {
        let enums = self.emit_proto_enums();
        let msgs = self.emit_proto_msgs();
        let mav_msg_fields = self.emit_proto_msg_names();

        let comment = Ident::from(format!(
            "// This file was automatically generated, do not edit \n"
        ));
        quote!{
            #comment

            syntax = "proto2";
            package mavlink.common;

            // TODO: generate enums too
            //#(#enums)*

            // List of all messages
            #(#msgs)*

            // Union over all messages
            message MavlinkMessage {
                oneof msg_set {
                #(#mav_msg_fields)*
                }
            }
        }
    }

    /// Simple header comment
    fn emit_comments(&self) -> Ident {
        Ident::from(format!(
            "// This file was automatically generated, do not edit \n"
        ))
    }

    /// Emit rust messages
    fn emit_msgs(&self) -> Vec<Tokens> {
        self.messages
            .iter()
            .map(|d| d.emit_rust())
            .collect::<Vec<Tokens>>()
    }

    /// Get list of original message names
    fn emit_enum_names(&self) -> Vec<Tokens> {
        self.messages
            .iter()
            .map(|msg| {
                let name = Ident::from(msg.name.clone());
                quote!(#name)
            })
            .collect::<Vec<Tokens>>()
    }

    ///
    fn emit_struct_names(&self) -> Vec<Tokens> {
        self.messages
            .iter()
            .map(|msg| msg.emit_struct_name())
            .collect::<Vec<Tokens>>()
    }

    /// A list of message IDs
    fn emit_msg_ids(&self) -> Vec<Tokens> {
        self.messages
            .iter()
            .map(|msg| {
                let id = Ident::from(msg.id.to_string());
                quote!(#id)
            })
            .collect::<Vec<Tokens>>()
    }

    /// A list of tags for the encompassing Mavlink proto message
    fn emit_msg_tags(&self) -> Tokens {
        let mut tags = vec![];
        let id = Ident::from("\"");
        tags.push(quote!(#id));
        
        for idx in 0..self.messages.len() {
            let id = Ident::from((idx+1).to_string());
            tags.push(quote!(#id));
            let id = Ident::from(format!(","));
            tags.push(quote!(#id));
        }
        tags.pop();
        let id = Ident::from("\"");
        tags.push(quote!(#id));
        
        quote!(#(#tags)*)
    }

    /// Emit enum for the encomassing one-of Mavlink proto message
    fn emit_msg_set(&self) -> Vec<Tokens> {
        let mut cnt = 1;
        self.messages
            .iter()
            .map(|msg| {
                let nametype = Ident::from(format!("{}",msg.name));
                let nametype_data = Ident::from(format!("{}Data",msg.name));
                let val = Ident::from(format!("\"{}\"", cnt));
                cnt += 1;
                quote!{
                    #[prost(message, tag= #val)]
                    #nametype (super::#nametype_data),
                }
            })
            .collect::<Vec<Tokens>>()
    }

    /// CRC values needed for mavlink parsing
    fn emit_msg_crc(&self) -> Vec<Tokens> {
        self.messages
            .iter()
            .map(|msg| {
                let crc = Ident::from(extra_crc(&msg).to_string());
                quote!(#crc)
            })
            .collect::<Vec<Tokens>>()
    }

    fn emit_rust(&self) -> Tokens {
        let comment = self.emit_comments();
        let msgs = self.emit_msgs();
        let enum_names = self.emit_enum_names();
        let struct_names = self.emit_struct_names();
        let msg_ids = self.emit_msg_ids();
        let msg_crc = self.emit_msg_crc();
        let mav_message = self.emit_mav_message(enum_names.clone(), struct_names.clone());
        let mav_message_parse =
            self.emit_mav_message_parse(enum_names.clone(), struct_names, msg_ids.clone());
        let mav_message_id = self.emit_mav_message_id(enum_names.clone(), msg_ids.clone());
        let mav_message_serialize = self.emit_mav_message_serialize(enum_names);
        let protobuf_msg_tags = self.emit_msg_tags();
        let protobuf_msg_set = self.emit_msg_set();

        quote!{
            #comment
            // Cursor and byteorder is needed for parsing mavlink data
            use std::io::Cursor;
            use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

            // Serde imports are needed to handle parsing null fields in JSON
            use serde::Deserializer;
            use serde::de::Deserialize;
            
            // To encode and decode messages
            use prost::Message;

            // replace Null with NAN
            use std::{f32,f64};

            #[allow(dead_code)]
            fn parse_f32<'de, D>(d: D) -> Result<f32, D::Error> where D: Deserializer<'de> {
                Deserialize::deserialize(d)
                    .map(|x: Option<_>| {
                    x.unwrap_or(f32::NAN)
                })
            }

            #[allow(dead_code)]
            fn parse_f64<'de, D>(d: D) -> Result<f64, D::Error> where D: Deserializer<'de> {
                Deserialize::deserialize(d)
                    .map(|x: Option<_>| {
                    x.unwrap_or(f64::NAN)
                })
            }

            // For mavlink parsing
            pub trait Parsable {
                fn parse(payload: &[u8]) -> Self;
                fn serialize(&self) -> Vec<u8>;
            }

            #(#msgs)*

            // Below are defines for Mavlink part only
            #[derive(Clone, PartialEq, Debug)]
            #[derive(Serialize)]
            #mav_message

            impl MavMessage {
                #mav_message_parse
                #mav_message_id
                #mav_message_serialize
                pub fn extra_crc(id: u8) -> u8 {
                    match id {
                        #(#msg_ids => #msg_crc,)*
                        _ => 0,
                    }
                }
            }
            // End of mavlink only part

            // Below are defines for Protobuf part only
            #[derive(Clone, PartialEq, Message)]
            #[derive(Serialize,Deserialize)]
            pub struct MavlinkMessage {
                #[prost(oneof="mavlink_message::MsgSet", tags= #protobuf_msg_tags)]
                pub msg_set: ::std::option::Option<mavlink_message::MsgSet>,
            }

            pub mod mavlink_message {
                #[derive(Clone, Oneof, PartialEq)]
                #[derive(Serialize,Deserialize)]
                pub enum MsgSet {
                    #(#protobuf_msg_set)*
                }
            }
            // End of protobuf only part
            
            // Interoperability between protobuf and regular mavlink
            impl MavMessage {
                // Converts an enum variant into a proto message
                pub fn encode(&self) -> Vec<u8> {
                    match &self {
                        &MavMessage::Heartbeat(ref body) => {
                            let mut buf = Vec::new();
                            buf.reserve(body.encoded_len());
                            // Unwrap is safe, since we have reserved sufficient capacity in the vector.
                            body.encode(&mut buf).unwrap();
                            buf
                        },
                        _ => vec![]
                    }
                }
            }
            // end of the interop part
        }
    }

    fn emit_mav_message(&self, enums: Vec<Tokens>, structs: Vec<Tokens>) -> Tokens {
        quote!{
                pub enum MavMessage {
                    #(#enums(#structs)),*
                }
        }
    }

    fn emit_mav_message_parse(
        &self,
        enums: Vec<Tokens>,
        structs: Vec<Tokens>,
        ids: Vec<Tokens>,
    ) -> Tokens {
        quote!{
            pub fn parse(id: u8, payload: &[u8]) -> Option<MavMessage> {
                match id {
                    #(#ids => Some(MavMessage::#enums(#structs::parse(payload))),)*
                    _ => None,
                }
            }
        }
    }

    fn emit_mav_message_id(&self, enums: Vec<Tokens>, ids: Vec<Tokens>) -> Tokens {
        quote!{
            pub fn message_id(&self) -> u8 {
                match self {
                    #(MavMessage::#enums(..) => #ids,)*
                }
            }
        }
    }

    fn emit_mav_message_serialize(&self, enums: Vec<Tokens>) -> Tokens {
        quote!{
            pub fn serialize(&self) -> Vec<u8> {
                match self {
                    #(&MavMessage::#enums(ref body) => body.serialize(),)*
                }
            }
        }
    }
}

pub fn parse_profile(file: &mut Read) -> MavProfile {
    let mut stack: Vec<MavXmlElement> = vec![];

    let mut profile = MavProfile {
        includes: vec![],
        messages: vec![],
        enums: vec![],
    };

    let mut field: MavField = Default::default();
    let mut message: MavMessage = Default::default();
    let mut mavenum: MavEnum = Default::default();
    let mut entry: MavEnumEntry = Default::default();
    let mut paramid: Option<usize> = None;

    let parser = EventReader::new(file);
    for e in parser {
        match e {
            Ok(XmlEvent::StartElement {
                name,
                attributes: attrs,
                ..
            }) => {
                let id = match identify_element(&name.to_string()) {
                    None => {
                        panic!("unexpected element {:?}", name);
                    }
                    Some(kind) => kind,
                };

                if !is_valid_parent(
                    match stack.last().clone() {
                        Some(arg) => Some(arg.clone()),
                        None => None,
                    },
                    id.clone(),
                ) {
                    panic!("not valid parent {:?} of {:?}", stack.last(), id);
                }

                match id {
                    MavXmlElement::Message => {
                        message = Default::default();
                    }
                    MavXmlElement::Field => {
                        field = Default::default();
                    }
                    MavXmlElement::Enum => {
                        mavenum = Default::default();
                    }
                    MavXmlElement::Entry => {
                        entry = Default::default();
                    }
                    MavXmlElement::Param => {
                        paramid = None;
                    }
                    _ => (),
                }

                stack.push(id);

                for attr in attrs {
                    match stack.last() {
                        Some(&MavXmlElement::Enum) => match attr.name.local_name.clone().as_ref() {
                            "name" => {
                                mavenum.name =
                                    attr.value
                                        .clone()
                                        .split("_")
                                        .map(|x| x.to_lowercase())
                                        .map(|x| {
                                            let mut v: Vec<char> = x.chars().collect();
                                            v[0] = v[0].to_uppercase().nth(0).unwrap();
                                            v.into_iter().collect()
                                        })
                                        .collect::<Vec<String>>()
                                        .join("");
                                //mavenum.name = attr.value.clone();
                            }
                            _ => (),
                        },
                        Some(&MavXmlElement::Entry) => {
                            match attr.name.local_name.clone().as_ref() {
                                "name" => {
                                    entry.name = attr.value.clone();
                                }
                                "value" => {
                                    entry.value = attr.value.parse::<i32>().unwrap();
                                }
                                _ => (),
                            }
                        }
                        Some(&MavXmlElement::Message) => {
                            match attr.name.local_name.clone().as_ref() {
                                "name" => {
                                    message.name = attr
                                        .value
                                        .clone()
                                        .split("_")
                                        .map(|x| x.to_lowercase())
                                        .map(|x| {
                                            let mut v: Vec<char> = x.chars().collect();
                                            v[0] = v[0].to_uppercase().nth(0).unwrap();
                                            v.into_iter().collect()
                                        })
                                        .collect::<Vec<String>>()
                                        .join("");
                                    //message.name = attr.value.clone();
                                }
                                "id" => {
                                    message.id = attr.value.parse::<u8>().unwrap();
                                }
                                _ => (),
                            }
                        }
                        Some(&MavXmlElement::Field) => {
                            match attr.name.local_name.clone().as_ref() {
                                "name" => {
                                    field.name = attr.value.clone();
                                    if field.name == "type" {
                                        field.name = "mavtype".to_string();
                                    }
                                }
                                "type" => {
                                    field.mavtype = parse_type(&attr.value).unwrap();
                                }
                                "enum" => {
                                    field.enumtype = Some(
                                        attr.value
                                            .clone()
                                            .split("_")
                                            .map(|x| x.to_lowercase())
                                            .map(|x| {
                                                let mut v: Vec<char> = x.chars().collect();
                                                v[0] = v[0].to_uppercase().nth(0).unwrap();
                                                v.into_iter().collect()
                                            })
                                            .collect::<Vec<String>>()
                                            .join(""),
                                    );
                                    //field.enumtype = Some(attr.value.clone());
                                }
                                _ => (),
                            }
                        }
                        Some(&MavXmlElement::Param) => {
                            if let None = entry.params {
                                entry.params = Some(vec![]);
                            }
                            match attr.name.local_name.clone().as_ref() {
                                "index" => {
                                    paramid = Some(attr.value.parse::<usize>().unwrap());
                                }
                                _ => (),
                            }
                        }
                        _ => (),
                    }
                }
            }
            Ok(XmlEvent::Characters(s)) => {
                use parser::MavXmlElement::*;
                match (stack.last(), stack.get(stack.len() - 2)) {
                    (Some(&Description), Some(&Message)) => {
                        message.description = Some(s);
                        println!("message.description {:?}", message.description);
                    }
                    (Some(&Field), Some(&Message)) => {
                        field.description = Some(s);
                        println!("field.description {:?}", field.description);
                    }
                    (Some(&Description), Some(&Enum)) => {
                        mavenum.description = Some(s);
                    }
                    (Some(&Description), Some(&Entry)) => {
                        entry.description = Some(s);
                    }
                    (Some(&Param), Some(&Entry)) => {
                        if let Some(ref mut params) = entry.params {
                            params.insert(paramid.unwrap() - 1, s);
                        }
                    }
                    (Some(&Include), Some(&Mavlink)) => {
                        println!("TODO: include {:?}", s);
                    }
                    (Some(&Version), Some(&Mavlink)) => {
                        println!("TODO: version {:?}", s);
                    }
                    data => {
                        panic!("unexpected text data {:?} reading {:?}", data, s);
                    }
                }
            }
            Ok(XmlEvent::EndElement { .. }) => {
                match stack.last() {
                    Some(&MavXmlElement::Field) => message.fields.push(field.clone()),
                    Some(&MavXmlElement::Entry) => {
                        mavenum.entries.push(entry.clone());
                    }
                    Some(&MavXmlElement::Message) => {
                        // println!("message: {:?}", message);
                        let mut msg = message.clone();
                        msg.fields.sort_by(|a, b| a.mavtype.compare(&b.mavtype));
                        profile.messages.push(msg);
                    }
                    Some(&MavXmlElement::Enum) => {
                        profile.enums.push(mavenum.clone());
                    }
                    _ => (),
                }
                stack.pop();
                // println!("{}-{}", indent(depth), name);
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
            _ => {}
        }
    }

    profile
}

/// Generate protobuf represenation of mavlink message set
/// Generate rust representation of mavlink message set with appropriate conversion methods
pub fn generate<R: Read, W: Write>(input: &mut R, output_proto: &mut W, output_rust: &mut W) {
    let profile = parse_profile(input);

    // proto file
    let proto_tokens = profile.emit_proto();
    writeln!(output_proto, "{}", proto_tokens).unwrap();

    // rust file
    let rust_tokens = profile.emit_rust();
    //writeln!(output_rust, "{}", rust_tokens).unwrap();

    let rust_src = rust_tokens.into_string();
    let mut cfg = rustfmt::config::Config::default();
    cfg.set().write_mode(rustfmt::config::WriteMode::Display);
    rustfmt::format_input(rustfmt::Input::Text(rust_src), &cfg, Some(output_rust)).unwrap();
}
// TODO: CHECK CRC?!
pub fn extra_crc(msg: &MavMessage) -> u8 {
    // calculate a 8-bit checksum of the key fields of a message, so we
    // can detect incompatible XML changes
    let mut crc = crc16::State::<crc16::MCRF4XX>::new();
    crc.update(msg.name.as_bytes());
    crc.update(" ".as_bytes());

    let mut f = msg.fields.clone();
    f.sort_by(|a, b| a.mavtype.compare(&b.mavtype));
    for field in &f {
        crc.update(field.mavtype.primitive_type().as_bytes());
        crc.update(" ".as_bytes());
        crc.update(field.name.as_bytes());
        crc.update(" ".as_bytes());
        if let MavType::Array(_, size) = field.mavtype {
            crc.update(&[size as u8]);
        }
    }

    let crcval = crc.get();
    ((crcval & 0xFF) ^ (crcval >> 8)) as u8
}
