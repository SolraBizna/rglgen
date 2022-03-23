use std::{
    collections::HashMap,
    io::Write,
};

use lazy_static::lazy_static;
use regex::bytes::{Regex, Captures};

use crate::{
    dom::{Node, Element},
    cmdline::CmdLine,
};

#[derive(Debug)]
pub struct Type {
    code: Option<String>,
}

impl Type {
    pub fn output(&self, _opts: &CmdLine) {
        if let Some(ref code) = self.code {
            println!("{}", code);
        }
    }
}

fn gather_text_content_and_search_for_name<W: Write>
    (root: &Element, type_name: &mut Option<String>, out: &mut W) {
    if root.get_name() == "name" {
        match *type_name {
            None => *type_name = Some(root.get_text()),
            Some(ref name) => panic!("type {} has multiple names!", name),
        }
    }
    for child in root.get_children() {
        match child {
            &Node::Text(ref text) => {
                out.write(text.as_bytes()).unwrap();
            },
            &Node::Element(ref element) => {
                gather_text_content_and_search_for_name(element,type_name,out);
            },
        }
    }
}

fn space_to_underscore(x: &u8) -> u8 {
    if *x == b' ' { b'_' } else { *x }
}

fn c_type_to_rust_type(map: &mut HashMap<String, Type>,
                       c_type: &[u8],
                       requires: &mut Vec<String>,
                       opts: &CmdLine) -> Vec<u8> {
    lazy_static! {
        static ref CONDENSE_SPACES_1: Regex
            = Regex::new(r#"^ +"#).unwrap();
        static ref CONDENSE_SPACES_2: Regex
            = Regex::new(r#" +(\*|$)"#).unwrap();
        static ref STRUCT_MUNCHER: Regex
            = Regex::new(r#"struct [_a-zA-Z][_a-zA-Z0-9]*"#).unwrap();
        static ref POINTER_MUNCHER: Regex
            = Regex::new(r#"^((?:const )?)([^\*]+) *(\**)$"#).unwrap();
        static ref STATIC_TYPES_LIBC: HashMap<&'static [u8], &'static [u8]>
            = [
                // We can't count on these to map to particular Rust types for
                // the most part. For most PC/mobile platforms it would be
                // acceptable to do so, but I intend to support even
                // Deathstation 9000s with this library. So... use libc types.
                (&b"void"[..], &b"libc::c_void"[..]),
                (&b"char"[..], &b"libc::c_char"[..]),
                (&b"unsigned char"[..], &b"libc::c_uchar"[..]),
                (&b"signed char"[..], &b"libc::c_schar"[..]),
                (&b"short"[..], &b"libc::c_short"[..]),
                (&b"unsigned short"[..], &b"libc::c_ushort"[..]),
                (&b"int"[..], &b"libc::c_int"[..]),
                (&b"unsigned int"[..], &b"libc::c_uint"[..]),
                (&b"long"[..], &b"libc::c_long"[..]),
                (&b"unsigned long"[..], &b"libc::c_ulong"[..]),
                (&b"float"[..], &b"libc::c_float"[..]),
                (&b"double"[..], &b"libc::c_double"[..]),
                // C99-ish types
                (&b"ptrdiff_t"[..], &b"libc::ptrdiff_t"[..]),
                (&b"intptr_t"[..], &b"libc::intptr_t"[..]),
                (&b"size_t"[..], &b"libc::size_t"[..]),
                (&b"ssize_t"[..], &b"libc::ssize_t"[..]),
                (&b"int8_t"[..], &b"libc::int8_t"[..]),
                (&b"int16_t"[..], &b"libc::int16_t"[..]),
                (&b"int32_t"[..], &b"libc::int32_t"[..]),
                (&b"int64_t"[..], &b"libc::int64_t"[..]),
                (&b"uint8_t"[..], &b"libc::uint8_t"[..]),
                (&b"uint16_t"[..], &b"libc::uint16_t"[..]),
                (&b"uint32_t"[..], &b"libc::uint32_t"[..]),
                (&b"uint64_t"[..], &b"libc::uint64_t"[..]),
                // "khrplatform.h" types... sigh...
                (&b"khronos_ptrdiff_t"[..], &b"isize"[..]),
                (&b"khronos_intptr_t"[..], &b"usize"[..]),
                (&b"khronos_size_t"[..], &b"usize"[..]),
                (&b"khronos_ssize_t"[..], &b"isize"[..]),
                (&b"khronos_int8_t"[..], &b"i8"[..]),
                (&b"khronos_int16_t"[..], &b"i16"[..]),
                (&b"khronos_int32_t"[..], &b"i32"[..]),
                (&b"khronos_int64_t"[..], &b"i64"[..]),
                (&b"khronos_uint8_t"[..], &b"u8"[..]),
                (&b"khronos_uint16_t"[..], &b"u16"[..]),
                (&b"khronos_uint32_t"[..], &b"u32"[..]),
                (&b"khronos_uint64_t"[..], &b"u64"[..]),
                (&b"khronos_float_t"[..], &b"f32"[..]),
                (&b"khronos_double_t"[..], &b"f64"[..]),
            ].into_iter().collect();
        static ref STATIC_TYPES_NO_LIBC: HashMap<&'static [u8], &'static [u8]>
            = [
                (&b"void"[..], &b"()"[..]),
                (&b"char"[..], &b"u8"[..]),
                (&b"unsigned char"[..], &b"u8"[..]),
                (&b"signed char"[..], &b"i8"[..]),
                (&b"short"[..], &b"i16"[..]),
                (&b"unsigned short"[..], &b"u16"[..]),
                (&b"int"[..], &b"i32"[..]),
                (&b"unsigned int"[..], &b"u32"[..]),
                (&b"long"[..], &b"i32"[..]),
                (&b"unsigned long"[..], &b"u32"[..]),
                (&b"float"[..], &b"f32"[..]),
                (&b"double"[..], &b"f64"[..]),
                // C99-ish types
                (&b"ptrdiff_t"[..], &b"isize"[..]),
                (&b"intptr_t"[..], &b"usize"[..]),
                (&b"size_t"[..], &b"usize"[..]),
                (&b"ssize_t"[..], &b"isize"[..]),
                (&b"int8_t"[..], &b"i8"[..]),
                (&b"int16_t"[..], &b"i16"[..]),
                (&b"int32_t"[..], &b"i32"[..]),
                (&b"int64_t"[..], &b"i64"[..]),
                (&b"uint8_t"[..], &b"u8"[..]),
                (&b"uint16_t"[..], &b"u16"[..]),
                (&b"uint32_t"[..], &b"u32"[..]),
                (&b"uint64_t"[..], &b"u64"[..]),
                // "khrplatform.h" types... sigh...
                (&b"khronos_ptrdiff_t"[..], &b"isize"[..]),
                (&b"khronos_intptr_t"[..], &b"usize"[..]),
                (&b"khronos_size_t"[..], &b"usize"[..]),
                (&b"khronos_ssize_t"[..], &b"isize"[..]),
                (&b"khronos_int8_t"[..], &b"i8"[..]),
                (&b"khronos_int16_t"[..], &b"i16"[..]),
                (&b"khronos_int32_t"[..], &b"i32"[..]),
                (&b"khronos_int64_t"[..], &b"i64"[..]),
                (&b"khronos_uint8_t"[..], &b"u8"[..]),
                (&b"khronos_uint16_t"[..], &b"u16"[..]),
                (&b"khronos_uint32_t"[..], &b"u32"[..]),
                (&b"khronos_uint64_t"[..], &b"u64"[..]),
                (&b"khronos_float_t"[..], &b"f32"[..]),
                (&b"khronos_double_t"[..], &b"f64"[..]),
            ].into_iter().collect();
    }
    let static_types =
        if opts.use_libc { &*STATIC_TYPES_LIBC }
        else { &*STATIC_TYPES_NO_LIBC };
    let temp = CONDENSE_SPACES_1
        .replace_all(&c_type, |_caps:&Captures| Vec::new());
    let temp = CONDENSE_SPACES_2
        .replace_all(&temp, |caps:&Captures| caps[1].to_vec());
    let temp = STRUCT_MUNCHER
        .replace_all(&temp, |_caps:&Captures| b"void".to_vec());
    let caps = POINTER_MUNCHER.captures(&temp).unwrap();
    let point = if !caps[1].is_empty() { &b"*const"[..] }
    else { &b"*mut"[..] };
    let num_pointers = caps[3].len();
    let mut ret = Vec::new();
    for _ in 0..num_pointers {
        ret.write_all(point).unwrap();
    }
    let old_type = &caps[2];
    if let Some(result) = static_types.get(old_type) {
        if *result != b"()" && num_pointers > 0 {
            ret.push(b' ');
        }
        ret.write_all(result).unwrap();
    }
    else {
        let old_type_as_string = String::from_utf8(old_type.to_vec())
            .unwrap();
        if let Some(_) = map.get(&old_type_as_string) {
            if !requires.contains(&old_type_as_string) {
                requires.push(old_type_as_string);
            }
            if old_type != b"()" && num_pointers > 0 {
                ret.push(b' ');
            }
            ret.write_all(old_type).unwrap();
        }
        else {
            panic!("Can't find the Rust equivalent to: {}",
                   unsafe { String::from_utf8_unchecked(old_type.to_vec()) });
        }
    }
    ret.to_vec()
}

fn gather_type(tag: &Element, map: &mut HashMap<String,Type>,
               order: &mut Vec<String>, opts: &CmdLine) {
    let mut name: Option<String> = tag.get_attributes().get("name").cloned();
    let mut text = Vec::new();
    let mut requires = Vec::new();
    if let Some(req) = tag.get_attributes().get("requires").cloned() {
        requires.push(req);
    }
    gather_text_content_and_search_for_name(tag, &mut name, &mut text);
    let name = match name {
        None => panic!("nameless type! text is:\n{}", unsafe { String::from_utf8_unchecked(text) }),
        Some(name) => name,
    };
    let code;
    lazy_static! {
        static ref SIMPLE_TYPEDEF: Regex
            = Regex::new(r#"^typedef (.* \**)([_a-zA-Z][_a-zA-Z0-9]*);$"#)
            .unwrap();
        static ref OPAQUE_STRUCT: Regex
            = Regex::new(r#"^(struct [_a-zA-Z][_a-zA-Z0-9]*);$"#).unwrap();
        static ref FUNCTION_POINTER: Regex
            = Regex::new(r#"^typedef (.*) *\( *\* *([_a-zA-Z][_a-zA-Z0-9]*) *\) *\((.*)\);$"#).unwrap();
        static ref NAME_AND_TYPE_EXTRACTOR: Regex
            = Regex::new(r#"^ *(.* \**)([_a-zA-Z][_a-zA-Z0-9]*)$"#)
            .unwrap();
    }
    let text = text.as_slice();
    if let Some(result) = SIMPLE_TYPEDEF.captures(text) {
        if result[2] != *name.as_bytes() {
            panic!("{}'s name isn't its name!? ({})", name,
                   unsafe { String::from_utf8_unchecked(result[2].to_vec()) });
        }
        let new_type: Vec<u8> = result[2].into_iter().map(space_to_underscore)
            .collect();
        let underlying_type = c_type_to_rust_type(map, &result[1], &mut requires, opts);
        let mut vec = Vec::new();
        vec.write_all(b"pub type ").unwrap();
        vec.write_all(new_type.as_slice()).unwrap();
        vec.write_all(b" = ").unwrap();
        vec.write_all(&underlying_type).unwrap();
        vec.write_all(b";").unwrap();
        code = Some(vec);
    }
    else if let Some(result) = OPAQUE_STRUCT.captures(text) {
        if result[1] != *name.as_bytes() {
            panic!("{}'s name isn't its name!? ({})", name,
                   unsafe { String::from_utf8_unchecked(result[1].to_vec()) });
        }
        let new_type: Vec<u8> = result[1].into_iter().map(space_to_underscore)
            .collect();
        let mut vec = Vec::new();
        vec.write_all(b"type ").unwrap();
        vec.write_all(new_type.as_slice()).unwrap();
        vec.write_all(b" = ();").unwrap();
        code = Some(vec);
    }
    else if let Some(result) = FUNCTION_POINTER.captures(text) {
        if result[2] != *name.as_bytes() {
            panic!("{}'s name isn't its name!? ({})", name,
                   unsafe { String::from_utf8_unchecked(result[2].to_vec()) });
        }
        let new_type: Vec<u8> = result[2].into_iter().map(space_to_underscore)
            .collect();
        let return_type = c_type_to_rust_type(map, &result[1], &mut requires, opts);
        let mut vec = Vec::new();
        vec.write_all(b"pub type ").unwrap();
        vec.write_all(new_type.as_slice()).unwrap();
        vec.write_all(b" = Option<extern \"C\" fn(").unwrap();
        let mut first_param = true;
        if result[3] != b"void"[..] {
            for param in result[3].split(|x| *x == b',') {
                if first_param {
                    first_param = false;
                }
                else {
                    vec.write_all(b", ").unwrap();
                }
                if let Some(caps) = NAME_AND_TYPE_EXTRACTOR.captures(param) {
                    let param_type = c_type_to_rust_type(map, &caps[1],
                                                         &mut requires, opts);
                    let param_name: &[u8] = match &caps[2] {
                        b"type" => b"r#type",
                        b"ref" => b"r#ref",
                        x => x,
                    };
                    vec.write_all(param_name).unwrap();
                    vec.write_all(b": ").unwrap();
                    vec.write_all(&param_type[..]).unwrap();
                }
                else {
                    let param_type=c_type_to_rust_type(map, &param,
                                                       &mut requires, opts);
                    vec.write_all(b"_: ").unwrap();
                    vec.write_all(&param_type[..]).unwrap();
                }
            }
        }
        vec.write_all(b") -> ").unwrap();
        vec.write_all(&return_type).unwrap();
        vec.write_all(b">;").unwrap();
        code = Some(vec);
    }
    else if name == "GLhandleARB" {
        assert_eq!(text, &br#"#ifdef __APPLE__
typedef void *GLhandleARB;
#else
typedef unsigned int GLhandleARB;
#endif"#[..]);
        code = Some(br#"// For historical reasons, this definition differs between macOS and other
// platforms. When the extension was promoted to core in GL 2.0, the definition
// was tightened. It's best to use the core versions of the routines that need
// handles rather than the extensions.
#[cfg(target_os = "macos")]
type GLhandleARB = *mut();
#[cfg(target_os != "macos")]
type GLhandleARB = libc::c_uint;"#.to_vec());
    }
    else if name == "stddef" || name == "khrplatform" || name == "inttypes" {
        // These are "dependencies GL types require to be declared legally".
        // They mainly consist of preprocessor directives. We don't make any
        // use of them.
        code = None;
    }
    else {
        eprintln!("!!!!!!!!!!!!!!!!!!!!!!");
        eprintln!("! TYPE PARSE FAILURE !");
        eprintln!("!!!!!!!!!!!!!!!!!!!!!!");
        eprintln!("We couldn't understand the definition for `{}`.", name);
        panic!("Type parse failure");
    }
    /*if name.starts_with("GL") {
        if let Some(ref mut code) = code {
            code.write_all(b"\npub use ").unwrap();
            code.write_all(name.as_str().as_bytes()).unwrap();
            code.write_all(b" as ").unwrap();
            code.write_all(&name.as_str().as_bytes()[2..]).unwrap();
            code.write_all(b";").unwrap();
        }
    }*/
    let mut result = Type {
        code: unsafe { code.map(|x| String::from_utf8_unchecked(x)) },
    };
    if let Some(ref comment) = tag.get_attributes().get("comment") {
        let mut new_code = Vec::new();
        for line in comment.as_bytes().split(|x| *x == b'\n') {
            new_code.write_all(b"// ").unwrap();
            new_code.write_all(line).unwrap();
            new_code.push(b'\n');
        }
        if let Some(code) = result.code {
            new_code.write_all(code.as_bytes()).unwrap();
        }
        result.code = Some(String::from_utf8(new_code).unwrap());
    }
    if !map.contains_key(&name) {
        order.push(name.clone());
    }
    map.insert(name, result);
}

pub fn gather_types(root: &Element, opts: &CmdLine)
                    -> (HashMap<String,Type>,Vec<String>) {
    let mut map = HashMap::new();
    let mut order = Vec::new();
    for child in root.get_children() {
        match child {
            &Node::Element(ref element) => {
                if element.get_name() == "types" {
                    for child in element.get_children() {
                        match child {
                            &Node::Element(ref element) =>
                                if element.get_name() == "type"
                                && opts.version.correct_api(element) {
                                    gather_type(element, &mut map, &mut order, opts)
                                },
                            _ => (),
                        }
                    }
                }
            },
            _ => (),
        }
    }
    (map,order)
}
