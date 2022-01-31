use std::{
    collections::{HashMap,HashSet},
    io::Write,
};

use regex::bytes::Regex;

use crate::{
    dom::{Node, Element},
    cmdline::CmdLine,
};

#[derive(Debug)]
pub struct Command {
    name: String,
    returns: String,
    params: String,
    ignored_params: String,
    param_names: String,
    param_count: u32,
    param_types: HashSet<String>,
    index: Option<u32>,
}

impl Command {
    pub fn touch_types<'a>(&'a self, map: &mut HashMap<&'a str, &'a str>,
                           ext: &'a str) {
        for param in &self.param_types {
            map.insert(param, ext);
        }
    }
    pub fn output_imp(&self, _opts: &CmdLine, procid: u32) {
        println!(r#"    #[inline(always)] pub unsafe fn {}(&self, {}) -> {} {{ (transmute::<_, extern "C" fn({}) -> {}>(self.procs[{}]))({}) }}"#, if self.name.starts_with("gl") { &(&self.name)[2..] } else { &(&self.name)[..] }, self.params, self.returns, self.params, self.returns, procid, self.param_names);
    }
    pub fn output_dummy_imp(&self, ext: &str, _opts: &CmdLine) {
        println!(r#"extern "C" fn {}_null_imp({}) -> {} {{ missing_ext_panic("{}", "{}"); }}"#, self.name, self.ignored_params, self.returns, self.name, ext);
    }
}

// Simpler than the logic in types.rs. Our types parsing is API/version
// dependent, while our commands parsing is not.
fn write_type(opts: &CmdLine, out: &mut Vec<u8>, mut ptype: &[u8],
              param_types: &mut HashSet<String>) {
    let ptrkind: &[u8] = if ptype.starts_with(b"const ") {
        ptype = &ptype[6..];
        b"*const"
    }
    else {
        b"*mut"
    };
    let mut pointer_levels: Vec<&[u8]> = Vec::new();
    if ptype.ends_with(b"*") {
        if ptype.ends_with(b"const*") {
            pointer_levels.push(b"*const");
            ptype = &ptype[..ptype.len()-6];
        }
        else {
            pointer_levels.push(ptrkind);
            ptype = &ptype[..ptype.len()-1];
        }
        while ptype.ends_with(b"*") {
            if ptype.ends_with(b"const*") {
                pointer_levels.push(b"*const");
                ptype = &ptype[..ptype.len()-6];
            }
            else {
                pointer_levels.push(b"*mut");
                ptype = &ptype[..ptype.len()-1];
            }
        }
    }
    if !pointer_levels.is_empty() {
        for level in pointer_levels.into_iter().rev() {
            out.write_all(level).unwrap();
        }
        out.push(b' ');
    }
    while ptype.ends_with(b" ") {
        ptype = &ptype[..ptype.len()-1];
    }
    if ptype == b"void" {
        if opts.use_libc {
            out.write_all(b"libc::c_void").unwrap();
        }
        else {
            out.write_all(b"()").unwrap();
        }
    }
    else {
        param_types.insert(String::from_utf8(ptype.to_vec()).unwrap());
        out.write_all(ptype).unwrap();
    }
}

fn gather_command(tag: &Element, opts: &CmdLine,
                  map: &mut HashMap<String,Command>) {
    let mut name: Option<String> = None;
    let mut returns: Option<String> = None;
    let mut params = Vec::new();
    let mut ignored_params = Vec::new();
    let mut param_names = Vec::new();
    let mut param_types = HashSet::new();
    let mut param_count = 0;
    lazy_static! {
        static ref TYPE_AND_NAME_EXTRACTOR: Regex
            = Regex::new(r#"^(.+?)([_a-zA-Z][_a-zA-Z0-9]*)((?:\[[0-9]+\])?)$"#).unwrap();
    }
    for child in tag.get_children() {
        match child {
            &Node::Element(ref element) => {
                if element.get_name() == "proto" {
                    assert!(name.is_none());
                    // we could parse out the <ptype> and <name> elements, but
                    // this way is simpler to write
                    let text = element.get_text_as_bytes();
                    let caps = TYPE_AND_NAME_EXTRACTOR.captures(&text[..])
                        .unwrap();
                    name = Some(String::from_utf8(caps[2].to_vec()).unwrap());
                    let mut rtype = Vec::new();
                    write_type(opts, &mut rtype, &caps[1], &mut param_types);
                    returns = Some(String::from_utf8(rtype).unwrap());
                }
                else if element.get_name() == "param" {
                    let text = element.get_text_as_bytes();
                    let caps = TYPE_AND_NAME_EXTRACTOR.captures(&text[..])
                        .unwrap();
                    let pname = &caps[2];
                    let pname: &[u8] = match pname {
                        b"type" => b"p_type",
                        b"ref" => b"p_ref",
                        x => x,
                    };
                    let mut ptype = caps[1].to_owned();
                    if !caps[3].is_empty() {
                        ptype.push(b'*');
                    }
                    param_count = param_count + 1;
                    if !params.is_empty() {
                        params.write_all(b", ").unwrap();
                        ignored_params.write_all(b", ").unwrap();
                        param_names.write_all(b", ").unwrap();
                    }
                    params.write_all(pname).unwrap();
                    param_names.write_all(pname).unwrap();
                    ignored_params.push(b'_');
                    params.write_all(b": ").unwrap();
                    ignored_params.write_all(b": ").unwrap();
                    write_type(opts, &mut params, &ptype, &mut param_types);
                    write_type(opts, &mut ignored_params, &ptype,
                               &mut param_types);
                }
            },
            _ => (),
        }
    }
    let result = Command {
        name: name.unwrap(),
        returns: returns.unwrap(),
        params: unsafe { String::from_utf8_unchecked(params) },
        ignored_params: unsafe { String::from_utf8_unchecked(ignored_params) },
        param_names: unsafe { String::from_utf8_unchecked(param_names) },
        param_types, param_count,
        index: None,
    };
    map.insert(result.name.clone(), result);
}

pub fn gather_commands(root: &Element, opts: &CmdLine)
                       -> (HashMap<String,Command>,Vec<String>) {
    let mut map = HashMap::new();
    for child in root.get_children() {
        match child {
            &Node::Element(ref element) => {
                if element.get_name() == "commands" {
                    for child in element.get_children() {
                        match child {
                            &Node::Element(ref element) =>
                                if element.get_name() == "command" {
                                    gather_command(element, opts, &mut map)
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
