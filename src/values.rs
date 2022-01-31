use std::collections::HashMap;

use crate::{
    dom::{Node, Element},
    cmdline::CmdLine,
};

pub enum Val {
    U32(u32),
    I32(i32),
    U64(u64),
}

impl Val {
    pub fn output(&self, name: &str, _opts: &CmdLine) {
        match *self {
            Val::U32(x) =>
                println!("pub const {}: u32 = 0x{:x};", name, x),
            Val::I32(x) =>
                println!("pub const {}: i32 = {};", name, x),
            Val::U64(x) =>
                println!("pub const {}: u64 = 0x{:x};", name, x),
        }
    }
}

fn parse_value(str: &str, typ: Option<&str>) -> Val {
    match typ {
        None => {
            if str.starts_with("-") {
                Val::I32(str.parse().unwrap())
            }
            else {
                parse_value(str, Some("u"))
            }
        },
        Some("u") => {
            Val::U32(if str.starts_with("0x") {
                u32::from_str_radix(&str[2..], 16).unwrap()
            }
            else {
                str.parse().unwrap()
            })
        },
        Some("ull") => {
            Val::U64(if str.starts_with("0x") {
                u64::from_str_radix(&str[2..], 16).unwrap()
            }
            else {
                str.parse().unwrap()
            })
        },
        Some(x) => {
            panic!("unknown <enum type=\"{}\">", x);
        },
    }
}

pub fn gather_values(root: &Element, _opts: &CmdLine)
                     -> (HashMap<String,Val>,Vec<String>) {
    let mut map = HashMap::new();
    for child in root.get_children() {
        match child {
            &Node::Element(ref element) => {
                if element.get_name() == "enums" {
                    for child in element.get_children() {
                        match child {
                            &Node::Element(ref element) =>
                                if element.get_name() == "enum" {
                                    if element.get_attributes().contains_key("alias") {
                                        // ignore!
                                    }
                                    else if let Some(ref enum_name) = element.get_attributes().get("name") {
                                        assert!(!map.contains_key(enum_name.as_str()));
                                        map.insert((*enum_name).clone(), parse_value(element.get_attributes()["value"].as_str(), element.get_attributes().get("type").map(|x| x.as_str())));
                                    }
                                },
                            _ => (),
                        }
                    }
                }
            },
            _ => (),
        }
    }
    let mut order = Vec::new();
    for key in map.keys() {
        order.push((*key).clone());
    }
    order.sort();
    (map,order)
}

