use std::{
    collections::{HashMap, HashSet},
    process::exit,
};

use crate::{
    cmdline::CmdLine,
    dom::{Element, Node},
};

fn process_feature<'a>(
    tag: &'a Element,
    ext: &'a str,
    handler: &fn(&mut HashMap<&'a str, &'a str>, &'a str, &'a str),
    type_set: &mut HashMap<&'a str, &'a str>,
    value_set: &mut HashMap<&'a str, &'a str>,
    command_set: &mut HashMap<&'a str, &'a str>,
) {
    for child in tag.get_children() {
        if let Node::Element(ref element) = child {
            match element.get_name() {
                "type" => {
                    handler(type_set, ext, &element.get_attributes()["name"])
                }
                "enum" => {
                    handler(value_set, ext, &element.get_attributes()["name"])
                }
                "command" => handler(
                    command_set,
                    ext,
                    &element.get_attributes()["name"],
                ),
                _ => (),
            }
        }
    }
}

fn gather_feature<'a>(
    tag: &'a Element,
    opts: &'a CmdLine,
    type_set: &mut HashMap<&'a str, &'a str>,
    value_set: &mut HashMap<&'a str, &'a str>,
    command_set: &mut HashMap<&'a str, &'a str>,
) {
    for child in tag.get_children() {
        if let Node::Element(ref element) = child {
            if (element.get_name() == "remove"
                || element.get_name() == "require")
                && opts.version.correct_profile(element)
            {
                process_feature(
                    element,
                    "",
                    &match element.get_name() {
                        "remove" => |set, _, wat| {
                            set.remove(wat);
                        },
                        "require" => |set, ext, wat| {
                            set.insert(wat, ext);
                        },
                        _ => panic!("?!"),
                    },
                    type_set,
                    value_set,
                    command_set,
                );
            }
        }
    }
}

fn gather_extension<'a>(
    tag: &'a Element,
    name: &'a str,
    opts: &'a CmdLine,
    type_set: &mut HashMap<&'a str, &'a str>,
    value_set: &mut HashMap<&'a str, &'a str>,
    command_set: &mut HashMap<&'a str, &'a str>,
) {
    for child in tag.get_children() {
        if let Node::Element(ref element) = child {
            if (element.get_name() == "remove"
                || element.get_name() == "require")
                && opts.version.correct_profile(element)
            {
                process_feature(
                    element,
                    name,
                    &match element.get_name() {
                        "remove" => |set, _, wat| {
                            set.remove(wat);
                        },
                        "require" => |set, ext, wat| {
                            set.insert(wat, ext);
                        },
                        _ => panic!("?!"),
                    },
                    type_set,
                    value_set,
                    command_set,
                );
            }
        }
    }
}

pub fn gather_features<'a>(
    root: &'a Element,
    opts: &'a CmdLine,
) -> [HashMap<&'a str, &'a str>; 3] {
    let mut type_set = HashMap::new();
    let mut value_set = HashMap::new();
    let mut command_set = HashMap::new();
    let mut found_extensions = HashSet::new();
    let mut errors_have_happened = false;
    for child in root.get_children() {
        if let Node::Element(ref element) = child {
            if element.get_name() == "feature"
                && opts.version.correct_api(element)
                && opts.version.correct_version(element)
            {
                gather_feature(
                    element,
                    opts,
                    &mut type_set,
                    &mut value_set,
                    &mut command_set,
                );
            } else if element.get_name() == "extensions" {
                for child in element.get_children() {
                    if let Node::Element(ref element) = child {
                        if element.get_name() == "extension" {
                            let name = &element.get_attributes()["name"];
                            let should_gather = if opts
                                .extensions
                                .contains(name)
                            {
                                if !opts.version.supported(element) {
                                    errors_have_happened = true;
                                    eprintln!("{} is not supported with the selected API", name);
                                }
                                true
                            } else {
                                false
                            };
                            if should_gather {
                                found_extensions.insert(name);
                                gather_extension(
                                    element,
                                    name,
                                    opts,
                                    &mut type_set,
                                    &mut value_set,
                                    &mut command_set,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    for ext in &opts.extensions {
        if !found_extensions.contains(ext) {
            errors_have_happened = true;
            eprintln!("Extension {} was not found", ext);
        }
    }
    if errors_have_happened {
        eprintln!("Errors have occurred, panicking");
        exit(1);
    }
    [type_set, value_set, command_set]
}
