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

    fn emit_proto_names(&self) -> Tokens {
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
        let name = Ident::from(name);
        quote!(#name)
    }

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
    fn emit_struct_name(&self) -> Tokens {
        let name = Ident::from(format!("{}_DATA", self.name));
        quote!(#name)
    }
    fn emit_field_names(&self) -> Vec<Tokens> {
        self.fields
            .iter()
            .map(|field| field.emit_name())
            .collect::<Vec<Tokens>>()
    }

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

    fn emit_proto_writers(&self) -> Vec<Tokens> {
        let mut cnt = 1;
        self.fields
            .iter()
            .map(|msg_field| {
                let name = Ident::from(msg_field.name.clone());
                let id = Ident::from(cnt.to_string());
                cnt += 1;
                let writer = match msg_field.mavtype {
                    MavType::Array(ref t, size) => match *t.clone() {
                        MavType::Array(_, _) => {
                            panic!("error");
                        }
                        _ => {
                            quote!{
                                let dummy_vec = vec![];
                                os.write_bytes(#id, dummy_vec.as_ref())?;
                                //os.write_bytes(#id, self.#name.as_ref())?;
                            }
                        }
                    },
                    _ => {
                        let write =
                            Ident::from(format!("write_{}", msg_field.mavtype.proto_type()));
                        quote!{
                            os.#write(#id, self.#name.into())?;
                        }
                    }
                };
                writer
            })
            .collect::<Vec<Tokens>>()
    }

    fn emit_rust(&self) -> Tokens {
        let msg_name = self.emit_struct_name();

        let field_names = self.emit_field_names();
        let field_types = self.emit_field_types();
        let readers = self.emit_rust_readers();
        let writers = self.emit_rust_writers();
        let proto_writers = self.emit_proto_writers();

        quote!{
            #[derive(Clone, Debug)]
            pub struct #msg_name {
                #(pub #field_names : #field_types ,)*
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

            impl ProtobufParsable for #msg_name {
                fn read_from_protostream(&mut self, is: Vec<u8>) -> ::protobuf::ProtobufResult<()> {
                    // Todo
                    ::std::result::Result::Ok(())
                }
                fn write_to_protostream(&self) -> ::protobuf::ProtobufResult<Vec<u8>> {
                    let mut v = vec![]; // TODO: allocated with capacity
                    {
                        let mut os = ::protobuf::stream::CodedOutputStream::vec(&mut v);
                        #(#proto_writers)*
                    }
                    ::std::result::Result::Ok(v)
                }
            }
        }
    }

    fn emit_proto_name(&self) -> Tokens {
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
        let name = Ident::from(name);
        quote!(#name)
    }

    fn emit_proto_defs(&self) -> Vec<Tokens> {
        let mut cnt = 1;
        self.fields
            .iter()
            .map(|msg_field| {
                let name = Ident::from(msg_field.name.clone());
                let value = Ident::from(cnt.to_string());
                cnt += 1;
                let mavtype = match msg_field.enumtype {
                    Some(ref enumtype) => {
                        let enum_name = enumtype
                            .split("_")
                            .map(|x| x.to_lowercase())
                            .map(|x| {
                                let mut v: Vec<char> = x.chars().collect();
                                v[0] = v[0].to_uppercase().nth(0).unwrap();
                                v.into_iter().collect()
                            })
                            .collect::<Vec<String>>()
                            .join("");
                        //Ident::from(enum_name)
                        // Create a uint32 instead of enum for now
                        Ident::from("uint32".to_string())
                    }
                    None => Ident::from(msg_field.mavtype.proto_type()),
                };
                quote!(required #mavtype #name = #value;)
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

    pub fn rust_type(&self) -> String {
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
            Array(t, size) => format!("Vec<{}> /* {} */", t.rust_type(), size),
        }
    }

    pub fn proto_type(&self) -> String {
        use parser::MavType::*;
        match self {
            UInt8MavlinkVersion | Char | UInt8 | UInt16 | UInt32 => "uint32".into(),
            Int8 | Int16 | Int32 => "int32".into(),
            Float => "float".into(),
            UInt64 => "uint64".into(),
            Int64 => "int64".into(),
            Double => "double".into(),
            Array(_, _) => "bytes".into(),
        }
    }

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

    fn emit_writer(&self) -> Tokens {
        let name = self.emit_name();
        let mut writer = quote!();
        match self.mavtype {
            MavType::Char | MavType::UInt8 | MavType::Int8 | MavType::UInt8MavlinkVersion => {
                let write = Ident::from(format!("write_{}", self.mavtype.rust_type()));
                writer = quote!{
                    wtr.#write(self.#name).unwrap();
                };
            }
            MavType::Array(ref t, size) => {
                for idx in 0..size {
                    match *t.clone() {
                        MavType::Char
                        | MavType::UInt8
                        | MavType::Int8
                        | MavType::UInt8MavlinkVersion => {
                            let write = Ident::from(format!("write_{}", t.rust_type()));
                            let index = Ident::from(idx.to_string());
                            writer.append(quote!{
                                    wtr.#write(self.#name[#index]).unwrap();
                            });
                        }
                        MavType::Array(_, _) => {
                            panic!("error");
                        }
                        _ => {
                            let write = Ident::from(format!("write_{}", t.rust_type()));
                            let index = Ident::from(idx.to_string());
                            writer.append(quote!{
                                wtr.#write::<LittleEndian>(self.#name[#index]).unwrap();
                            });
                        }
                    }
                }
            }
            _ => {
                let write = Ident::from(format!("write_{}", self.mavtype.rust_type()));
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
                let read = Ident::from(format!("read_{}", self.mavtype.rust_type()));
                reader = quote!{
                    #name : cur. #read ().unwrap(),
                };
            }
            MavType::Array(ref t, size) => {
                let read;
                match *t.clone() {
                    MavType::Char
                    | MavType::UInt8
                    | MavType::Int8
                    | MavType::UInt8MavlinkVersion => {
                        read = Ident::from(format!("read_{}", t.rust_type()));
                    }
                    MavType::Array(_, _) => {
                        panic!("error parsing message field");
                    }
                    _ => {
                        read = Ident::from(format!("read_{}::<LittleEndian>", t.rust_type()));
                    }
                }
                let size = Ident::from(size.to_string());
                reader = quote!{
                    #name: {
                        let mut v = Vec::with_capacity(#size);
                        for _ in 0..#size {
                            v.push(cur.#read().unwrap());
                        }
                        v
                    },
                };
            }
            _ => {
                let read = Ident::from(format!("read_{}", self.mavtype.rust_type()));
                reader = quote!{
                    #name : cur. #read::<LittleEndian> ().unwrap(),
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

    fn emit_proto(&self) -> Tokens {
        let enums = self.emit_proto_enums();
        let msgs = self.emit_proto_msgs();

        let comment = Ident::from(format!(
            "// This file was automatically generated, do not edit \n"
        ));
        quote!{
            #comment
            //#(#enums)*
            #(#msgs)*
            message MavlinkMessage {
                required uint32 msg_id = 1;
                required bytes msg_data = 2;
            }
        }
    }

    fn emit_rust(&self) -> Tokens {
        let comment = Ident::from(format!(
            "// This file was automatically generated, do not edit \n"
        ));

        let msgs = self
            .messages
            .iter()
            .map(|d| d.emit_rust())
            .collect::<Vec<Tokens>>();

        let enum_names = self
            .messages
            .iter()
            .map(|msg| {
                let name = Ident::from(msg.name.clone());
                quote!(#name)
            })
            .collect::<Vec<Tokens>>();

        let struct_names = self
            .messages
            .iter()
            .map(|msg| msg.emit_struct_name())
            .collect::<Vec<Tokens>>();

        let msg_ids = self
            .messages
            .iter()
            .map(|msg| {
                let id = Ident::from(msg.id.to_string());
                quote!(#id)
            })
            .collect::<Vec<Tokens>>();

        let msg_crc = self
            .messages
            .iter()
            .map(|msg| {
                let crc = Ident::from(extra_crc(&msg).to_string());
                quote!(#crc)
            })
            .collect::<Vec<Tokens>>();

        /*
        let msgs = self.messages[4].emit_rust();
        let enum_names = { let name = Ident::from(self.messages[4].name.clone()); quote!(#name) };
        let struct_names = self.messages[4].emit_struct_name();
        */
        /*
        //// test
        let mut msgs = vec![];
        let mut enum_names = vec![];
        let mut struct_names = vec![];
        let mut msg_ids = vec![];
        let mut msg_crc = vec![];
        for idx in 0..5 {
            msgs.push(self.messages[idx].emit_rust());
            enum_names.push({
                let name = Ident::from(self.messages[idx].name.clone());
                quote!(#name)
            });
            struct_names.push(self.messages[idx].emit_struct_name());
            msg_ids.push({
                let id = Ident::from(self.messages[idx].id.to_string());
                quote!(#id)
            });
            msg_crc.push({
                let crc = Ident::from(extra_crc(&self.messages[idx]).to_string());
                quote!(#crc)
            });
        }
        ///// test
*/
        let mav_message = self.emit_mav_message(enum_names.clone(), struct_names.clone());
        let mav_message_parse =
            self.emit_mav_message_parse(enum_names.clone(), struct_names, msg_ids.clone());
        let mav_message_id = self.emit_mav_message_id(enum_names.clone(), msg_ids.clone());
        let mav_message_serialize = self.emit_mav_message_serialize(enum_names);

        quote!{
            #comment
            use std::io::Cursor;
            use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

            use protobuf::Message as Message_imported_for_functions;
            use protobuf::ProtobufEnum as ProtobufEnum_imported_for_functions;

            pub trait Parsable {
                fn parse(payload: &[u8]) -> Self;
                fn serialize(&self) -> Vec<u8>;
            }

            pub trait ProtobufParsable {
                fn read_from_protostream(&mut self, is: Vec<u8>) -> ::protobuf::ProtobufResult<()>;
                fn write_to_protostream(&self) -> ::protobuf::ProtobufResult<Vec<u8>>;
            }

            #(#msgs)*
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
        }
    }

    fn emit_mav_message(&self, enums: Vec<Tokens>, structs: Vec<Tokens>) -> Tokens {
        quote!{
            #[derive(Clone, Debug)]
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
                                mavenum.name = attr.value.clone();
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
                                    message.name = attr.value.clone();
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
                                    field.enumtype = Some(attr.value.clone());
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
    //writeln!(output_rust, "{}", rust_tokens);
    let rust_src = rust_tokens.into_string();
    let mut cfg = rustfmt::config::Config::default();
    cfg.set().write_mode(rustfmt::config::WriteMode::Display);
    rustfmt::format_input(rustfmt::Input::Text(rust_src), &cfg, Some(output_rust)).unwrap();
}

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

/*
fn mavtype_parser(input: &MavType, field_name: &str) -> String {
    let s = match input {
        MavType::UInt8MavlinkVersion | MavType::UInt8 => format!("data.{}.into()", field_name),
        MavType::UInt16 | MavType::UInt32 | MavType::Char => format!("data.{}.into()", field_name),
        MavType::UInt64 => format!("data.{}.into()", field_name),
        MavType::Int8 | MavType::Int16 | MavType::Int32 | MavType::Int64 => {
            format!("data.{}.into()", field_name)
        }
        MavType::Float => format!("data.{}.into()", field_name),
        MavType::Double => format!("data.{}.into()", field_name),
        MavType::Array(boxed_type, _) => match **boxed_type {
            MavType::Float => format!("serialize_vec_f32(data.{})", field_name),
            MavType::Int16 => format!("serialize_vec_i16(data.{})", field_name),
            MavType::Int32 => format!("serialize_vec_i32(data.{})", field_name),
            MavType::UInt16 => format!("serialize_vec_u16(data.{})", field_name),
            MavType::UInt32 => format!("serialize_vec_u32(data.{})", field_name),
            MavType::Char | MavType::UInt8 => format!("data.{}.into()", field_name),
            MavType::Int8 => format!("serialize_vec_i8(data.{})", field_name),
            _ => panic!("Unknown format: {:?}", boxed_type),
        },
    };
    String::from(s)
}
*/
/*
#[allow(unused_must_use)] // TODO fix
pub fn generate_connector<R: Read, W: Write>(input: &mut R, output: &mut W) {
    let profile = parse_profile(input);
    writeln!(
        output,
        "
// Autogenerated code, do not edit
use protobuf::stream::*;
use protobuf::Message;
use protobuf::ProtobufEnum;

use mavlink::common::*;

use byteorder::{{LittleEndian, WriteBytesExt}};


mod mavlink_common_proto;

fn serialize_vec_i8(v: Vec<i8>) -> Vec<u8> {{
    let mut wtr = vec![];
    for val in v {{
        wtr.push(val as u8);
    }}
    wtr
}}

fn serialize_vec_f32(v: Vec<f32>) -> Vec<u8> {{
    let mut wtr = vec![];
    for val in v {{
        wtr.write_f32::<LittleEndian>(val).unwrap();
    }}
    wtr
}}

fn serialize_vec_i16(v: Vec<i16>) -> Vec<u8> {{
    let mut wtr = vec![];
    for val in v {{
        wtr.write_i16::<LittleEndian>(val).unwrap();
    }}
    wtr
}}

fn serialize_vec_u16(v: Vec<u16>) -> Vec<u8> {{
    let mut wtr = vec![];
    for val in v {{
        wtr.write_u16::<LittleEndian>(val).unwrap();
    }}
    wtr
}}

fn serialize_vec_i32(v: Vec<i32>) -> Vec<u8> {{
    let mut wtr = vec![];
    for val in v {{
        wtr.write_i32::<LittleEndian>(val).unwrap();
    }}
    wtr
}}

fn serialize_vec_u32(v: Vec<u32>) -> Vec<u8> {{
    let mut wtr = vec![];
    for val in v {{
        wtr.write_u32::<LittleEndian>(val).unwrap();
    }}
    wtr
}}


pub fn mavlink2protobuf(msg: MavMessage) -> mavlink_common_proto::MavlinkMessage {{
    let mut proto = mavlink_common_proto::MavlinkMessage::new();
    let mut msg_data = vec![];
    let id = msg.message_id();
    match msg {{"
    );

    for item in &profile.messages {
        writeln!(output, "        MavMessage::{}(data) => {{", item.name);
        writeln!(output, "            println!(\"Got message {{:?}}\",data);");
        let msg_name = item
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
        writeln!(
            output,
            "            let mut inner = mavlink_common_proto::{}::new();",
            msg_name
        );

        for field in &item.fields {
            let mut field_name = field.name.clone();
            if &field.name == "type" {
                field_name = "mavtype".to_string();
            }
            let field_conversion;
            match field.enumtype {
                Some(ref enumtype) => {
                    // we have an enum, have to get .value()
                    // MavMode.from_i32(data.base_mode.into())
                    let enumtype = enumtype
                        .split("_")
                        .map(|x| x.to_lowercase())
                        .map(|x| {
                            let mut v: Vec<char> = x.chars().collect();
                            v[0] = v[0].to_uppercase().nth(0).unwrap();
                            v.into_iter().collect()
                        })
                        .collect::<Vec<String>>()
                        .join("");
                    field_conversion = format!(
                        "mavlink_common_proto::{}::from_i32(data.{}.into()).unwrap()",
                        enumtype, field_name
                    );
                }
                None => {
                    // use only into() as we are converting a primitive data type
                    field_conversion = mavtype_parser(&field.mavtype, &field_name);
                    //field_conversion = format!("data.{}.into()",mavtype_parser(&field.mavtype));
                }
            } //
            writeln!(
                output,
                "            inner.set_{}({});",
                field_name, field_conversion
            );
        }

        writeln!(
            output,
            "            assert!(inner.is_initialized());
            let mut stream = protobuf::stream::CodedOutputStream::vec(&mut msg_data);
            inner.write_to_with_cached_sizes(&mut stream).unwrap();
        }}"
        );
    }

    writeln!(
        output,
        "    }}
    proto.set_msg_id(id.into());
    proto.set_msg_data(msg_data);
    assert!(proto.is_initialized());
    proto
}}"
    );
}
*/
/*
#[allow(unused_must_use)] // TODO fix
pub fn generate_mod<R: Read, W: Write>(input: &mut R, output: &mut W) {
    let profile = parse_profile(input);

    // writeln!(output, "#![allow(non_camel_case_types)]");
    // writeln!(output, "#![allow(non_snake_case)]");
    // writeln!(output, "");

    writeln!(output, "use std::io::Cursor;");
    writeln!(
        output,
        "use byteorder::{{LittleEndian, ReadBytesExt, WriteBytesExt}};"
    );
    writeln!(output, "");

    writeln!(output, "pub trait Parsable {{");
    writeln!(output, "    fn parse(payload: &[u8]) -> Self;");
    writeln!(output, "    fn serialize(&self) -> Vec<u8>;");
    writeln!(output, "}}");
    writeln!(output, "");

    for item in &profile.messages {
        let mut f = item.fields.clone();
        f.sort_by(|a, b| a.mavtype.compare(&b.mavtype));

        writeln!(output, "#[derive(Clone, Debug)]");
        writeln!(output, "pub struct {}_DATA {{", item.name);
        for field in &f {
            let fname = if field.name == "type" {
                "mavtype".into()
            } else {
                field.name.clone()
            };

            writeln!(output, "    pub {}: {},", fname, field.mavtype.rust_type());
        }
        writeln!(output, "}}");
        writeln!(output, "");

        writeln!(output, "impl Parsable for {}_DATA {{", item.name);
        writeln!(
            output,
            "    fn parse(payload: &[u8]) -> {}_DATA {{",
            item.name
        );
        writeln!(output, "        let mut cur = Cursor::new(payload);");
        writeln!(output, "        {}_DATA {{", item.name);
        for field in &f {
            let fname = if field.name == "type" {
                "mavtype".into()
            } else {
                field.name.clone()
            };
            match field.mavtype {
                MavType::Char | MavType::UInt8 | MavType::Int8 | MavType::UInt8MavlinkVersion => {
                    writeln!(
                        output,
                        "            {}: cur.read_{}().unwrap(),",
                        fname,
                        field.mavtype.rust_type()
                    );
                }
                MavType::Array(ref t, size) => {
                    writeln!(output, "            {}: vec![", fname);
                    for _ in 0..size {
                        match *t.clone() {
                            MavType::Char
                            | MavType::UInt8
                            | MavType::Int8
                            | MavType::UInt8MavlinkVersion => {
                                println!("                cur.read_{}().unwrap(),", t.rust_type());
                            }
                            MavType::Array(_, _) => {
                                panic!("error");
                            }
                            _ => {
                                println!(
                                    "                cur.read_{}::<LittleEndian>().unwrap(),",
                                    t.rust_type()
                                );
                            }
                        }
                    }
                    writeln!(output, "            ],");
                }
                _ => {
                    writeln!(
                        output,
                        "            {}: cur.read_{}::<LittleEndian>().unwrap(),",
                        fname,
                        field.mavtype.rust_type()
                    );
                }
            }
        }
        writeln!(output, "        }}");
        writeln!(output, "    }}");
        writeln!(output, "    fn serialize(&self) -> Vec<u8> {{");
        writeln!(output, "        let mut wtr = vec![];");
        for field in &f {
            let fname = if field.name == "type" {
                "mavtype".into()
            } else {
                field.name.clone()
            };
            match field.mavtype {
                MavType::Char | MavType::UInt8 | MavType::Int8 | MavType::UInt8MavlinkVersion => {
                    writeln!(
                        output,
                        "        wtr.write_{}(self.{}).unwrap();",
                        field.mavtype.rust_type(),
                        fname
                    );
                }
                MavType::Array(ref t, size) => {
                    for i in 0..size {
                        match *t.clone() {
                            MavType::Char
                            | MavType::UInt8
                            | MavType::Int8
                            | MavType::UInt8MavlinkVersion => {
                                writeln!(
                                    output,
                                    "        wtr.write_{}(self.{}[{}]).unwrap();",
                                    t.rust_type(),
                                    fname,
                                    i
                                );
                            }
                            MavType::Array(_, _) => {
                                panic!("error");
                            }
                            _ => {
                                writeln!(
                                    output,
                                    "        wtr.write_{}::<LittleEndian>(self.{}[{}]).\
                                     unwrap();",
                                    t.rust_type(),
                                    fname,
                                    i
                                );
                            }
                        }
                    }
                }
                _ => {
                    writeln!(
                        output,
                        "        wtr.write_{}::<LittleEndian>(self.{}).unwrap();",
                        field.mavtype.rust_type(),
                        fname
                    );
                }
            }
        }
        writeln!(output, "        wtr");
        writeln!(output, "    }}");
        writeln!(output, "}}");
        writeln!(output, "");
    }

    writeln!(output, "#[derive(Clone, Debug)]");
    writeln!(output, "pub enum MavMessage {{");
    for item in &profile.messages {
        writeln!(output, "  {}({}_DATA),", item.name, item.name);
    }
    writeln!(output, "}}");
    writeln!(output, "");

    writeln!(output, "impl MavMessage {{");
    writeln!(
        output,
        "    pub fn parse(id: u8, payload: &[u8]) -> Option<MavMessage> {{"
    );
    writeln!(output, "        match id {{");
    for item in &profile.messages {
        writeln!(
            output,
            "            {} => Some(MavMessage::{}({}_DATA::parse(payload))),",
            item.id, item.name, item.name
        );
    }
    writeln!(output, "            _ => None,");
    writeln!(output, "        }}");
    writeln!(output, "    }}");
    writeln!(output, "");
    writeln!(output, "    pub fn message_id(&self) -> u8 {{");
    writeln!(output, "        match self {{");
    for item in &profile.messages {
        writeln!(
            output,
            "            &MavMessage::{}(..) => {},",
            item.name, item.id
        );
    }
    writeln!(output, "        }}");
    writeln!(output, "    }}");
    writeln!(output, "");
    writeln!(output, "    pub fn extra_crc(id: u8) -> u8 {{");
    writeln!(output, "        match id {{");
    for item in &profile.messages {
        writeln!(output, "            {} => {},", item.id, extra_crc(item));
    }
    writeln!(output, "            _ => 0,");
    writeln!(output, "        }}");
    writeln!(output, "    }}");
    writeln!(output, "");
    writeln!(output, "    pub fn serialize(&self) -> Vec<u8> {{");
    writeln!(output, "        match self {{");
    for item in &profile.messages {
        writeln!(
            output,
            "            &MavMessage::{}(ref body) => body.serialize(),",
            item.name
        );
    }
    writeln!(output, "        }}");
    writeln!(output, "    }}");
    writeln!(output, "}}");
    writeln!(output, "");
}
*/
