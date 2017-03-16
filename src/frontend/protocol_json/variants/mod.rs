use ::{Type, TypeData, TypeContainer};
use ::variants::{Variant, SimpleScalarVariant, ContainerVariant, ContainerField, SwitchVariant,
                 SwitchCase, PrefixedStringVariant};
use ::field_reference::FieldReference;
use super::FromProtocolJson;
use ::json::JsonValue;
use super::errors::*;
use super::type_from_json;
use super::count::{CountMode, read_count};

use std::rc::Rc;
use std::cell::RefCell;

mod array;
pub use self::array::ArrayReader;

pub struct ScalarReader;
impl FromProtocolJson for ScalarReader {
    fn from_json(name: String, _arg: &JsonValue) -> Result<TypeContainer> {
        let mut data = TypeData::default();
        data.name = name;

        Ok(Rc::new(RefCell::new(Type {
            data: data,
            variant: Variant::SimpleScalar(SimpleScalarVariant {}),
        })))
    }
}

pub struct ContainerReader;
impl FromProtocolJson for ContainerReader {
    fn from_json(name: String, arg: &JsonValue) -> Result<TypeContainer> {
        ensure!(arg.is_array(),
                "argument for 'container' must be array, got {:?}",
                arg);

        let mut children_fields: Vec<(TypeContainer, ContainerField)> = arg.members()
            .enumerate()
            .map(|(idx, member)| {
                ensure!(member.is_object(),
                        "'container' child must be object, got {:?}",
                        member);
                ensure!(member.has_key("name"),
                        "'container' child #{} missing 'name' field",
                        idx);
                ensure!(member.has_key("type"),
                        "'container' child #{} missing 'type' field",
                        idx);

                let typ = &member["type"];
                let name = &member["name"];

                let final_type = type_from_json(typ)?;
                let field = ContainerField {
                    name: name.to_string(),
                    child: Rc::downgrade(&final_type),
                    child_index: idx,
                };

                Ok((final_type, field))
            })
            .collect::<Result<Vec<(TypeContainer, ContainerField)>>>()?;

        let (children, fields): (Vec<TypeContainer>, Vec<ContainerField>) =
            children_fields.drain(0..).unzip();

        let mut data = TypeData::default();
        data.name = name;
        data.children = children;

        Ok(Rc::new(RefCell::new(Type {
            data: data,
            variant: Variant::Container(ContainerVariant { fields: fields }),
        })))
    }
}

pub struct SwitchReader;
impl FromProtocolJson for SwitchReader {
    fn from_json(name: String, arg: &JsonValue) -> Result<TypeContainer> {
        ensure!(arg.is_object(),
                "argument for 'switch' must be object, got {:?}",
                arg);
        ensure!(arg.has_key("compareTo"),
                "argument for 'switch' must have 'compareTo' key");
        ensure!(arg.has_key("fields"),
                "argument for 'switch' must have 'fields' key");
        ensure!(arg["fields"].is_object(),
                "'fields' field in 'switch' must be object");

        let compareToStr = arg["compareTo"].as_str()
            .ok_or("'compareTo' field in 'switch' must be string")?;
        let compareTo = FieldReference::parse(compareToStr)
            .ok_or("'compareTo' field in 'switch' must contain a valid field reference")?;

        let fields_key = &arg["fields"];

        let mut children_options: Vec<(TypeContainer, SwitchCase)> = fields_key.entries()
            .enumerate()
            .map(|(idx, (key, typ))| {
                let final_type = type_from_json(typ)?;
                let field = SwitchCase {
                    raw_value: key.to_string(),
                    child: Rc::downgrade(&final_type),
                    child_index: idx,
                };

                Ok((final_type, field))
            })
            .collect::<Result<Vec<(TypeContainer, SwitchCase)>>>()?;

        let (mut children, fields): (Vec<TypeContainer>, Vec<SwitchCase>) =
            children_options.drain(0..).unzip();

        if arg.has_key("default") {
            let typ = type_from_json(&arg["default"])?;
            let idx = children.len();
            children.push(typ);
        }

        unreachable!();
    }
}

pub struct StringReader;
impl FromProtocolJson for StringReader {
    fn from_json(name: String, arg: &JsonValue) -> Result<TypeContainer> {
        match read_count(arg)? {
            CountMode::Prefixed(prefix_type) => {
                let mut data = TypeData::default();
                data.name = name;
                data.children.push(prefix_type.clone());

                Ok(Rc::new(RefCell::new(Type {
                    data: data,
                    variant: Variant::PrefixedString(PrefixedStringVariant {
                        length: Rc::downgrade(&prefix_type),
                        length_index: 0,
                    }),
                })))
            }
            _ => unimplemented!(),
        }
    }
}