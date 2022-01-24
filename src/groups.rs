use dom::{Node, Element};
use std::collections::HashMap;
use cmdline::CmdLine;

#[derive(Debug)]
enum GroupType {
    Bitmask,
    Ordinary
}

#[derive(Debug)]
pub struct Group {
    elements: Vec<String>,
    comment: Option<String>,
    group_type: Option<GroupType>,
}

fn new_group() -> Group {
    Group {
        elements: Vec::new(),
        comment: None,
        group_type: None,
    }
}

fn gather_group(tag: &Element, map: &mut HashMap<String,Group>) {
    let name = tag.get_attributes()["name"].clone();
    let mut result = new_group();
    for child in tag.get_children() {
        match child {
            &Node::Element(ref element) => {
                if element.get_name() == "enum" {
                    result.elements.push(element.get_attributes()["name"].clone())
                }
            },
            _ => (),
        }
    }
    map.insert(name, result);
}

pub fn gather_groups(root: &Element, _opts: &CmdLine)
                     -> (HashMap<String,Group>,Vec<String>) {
    let mut map = HashMap::new();
    for child in root.get_children() {
        match child {
            &Node::Element(ref element) => {
                if element.get_name() == "groups" {
                    for child in element.get_children() {
                        match child {
                            &Node::Element(ref element) =>
                                if element.get_name() == "group" {
                                    gather_group(element, &mut map)
                                },
                            _ => (),
                        }
                    }
                }
                else if element.get_name() == "enums" {
                    if let Some(group_name) = element.get_attributes().get("group") {
                        let map = &mut map;
                        let mut group =
                        if let Some(group) = map.remove(group_name.as_str()) {
                            group
                        }
                        else {
                            new_group()
                        };
                        assert!(group.group_type.is_none());
                        assert!(group.comment.is_none());
                        group.comment = element.get_attributes().get("comment").cloned();
                        group.group_type = Some(match element.get_attributes().get("type") {
                            Some(x) if x == "bitmask" =>GroupType::Bitmask,
                            None => GroupType::Ordinary,
                            Some(x) => panic!("Unknown enum type: {}", x),
                        });
                        for child in element.get_children() {
                            match child {
                                &Node::Element(ref element) =>
                                    if element.get_attributes().contains_key("alias") {
                                        // ignore!
                                    }
                                    else if element.get_name() == "enum" {
                                        if let Some(ref enum_name) = element.get_attributes().get("name") {
                                            if !group.elements.contains(enum_name) {
                                                group.elements.push((*enum_name).clone());
                                            }
                                        }
                                    },
                                _ => (),
                            }
                        }
                        // grumble grumble...
                        map.insert(group_name.clone(), group);
                    }
                }
            },
            _ => (),
        }
    }
    let mut order = Vec::new();
    for (key, value) in &mut map {
        value.elements.sort();
        order.push(key.clone());
    }
    order.sort();
    (map,order)
}

