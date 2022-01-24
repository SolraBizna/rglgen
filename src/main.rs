// Yikes!

extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate xml;
extern crate getopts;

mod dom;

mod versions;

mod cmdline;
use cmdline::*;

mod comments;
use comments::*;

mod types;
use types::*;

mod groups;
use groups::*;

mod values;
use values::*;

mod commands;
use commands::*;

mod features;
use features::*;

mod used_identifiers;
use used_identifiers::*;

use std::process::exit;
use std::io;
use std::fs;
use std::collections::HashMap;
use std::cmp::Ordering;

/// Sorts the commands such that each required extension corresponds to a
/// contiguous range of procs.
fn sort_commands<'a>(used_identifier_set: &UsedIdentifiers,
                     command_map: &'a HashMap<String,Command>,
                     command_exts: &'a HashMap<&'a str, &'a str>,
                     command_order: &'a Vec<String>)
                     -> (Vec<&'a str>, HashMap<&'a str, u32>,
                         HashMap<&'a str, (u32, u32)>) {
    let mut unsorted = Vec::with_capacity(command_map.len());
    for command in command_order {
        if used_identifier_set.contains(command.as_str()) {
            if let Some(ext) = command_exts.get(command.as_str()) {
                let i = unsorted.len() as u32;
                unsorted.push((command.as_str(), ext, i));
            }
        }
    }
    // Sort by extension first, then order within gl.xml
    unsorted.as_mut_slice().sort_unstable_by(|a, b| {
        match a.1.cmp(b.1) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => a.2.cmp(&b.2),
        }
    });
    let mut sorted = Vec::with_capacity(unsorted.len());
    let mut indices = HashMap::new();
    let mut ranges = HashMap::new();
    let mut cur_range: Option<(&str, u32)> = None;
    for i in 0..unsorted.len() {
        let (command_name, required_extension, _) = unsorted[i];
        let i = i as u32;
        indices.insert(command_name, i);
        sorted.push(command_name);
        match cur_range {
            None => {
                assert_eq!(i, 0);
                cur_range = Some((required_extension, i));
            },
            Some((oext, oi)) => {
                if oext != *required_extension {
                    ranges.insert(oext, (oi, i));
                    cur_range = Some((required_extension, i));
                }
            },
        }
    }
    if let Some((oext, oi)) = cur_range {
        ranges.insert(oext, (oi, unsorted.len() as u32));
    }
    (sorted, indices, ranges)
}

fn main() {
    let opts = match parse_cmdline() {
        None => exit(1),
        Some(opts) => opts,
    };
    let used_identifier_set = get_used_identifiers(&opts);
    let xml = dom::read_xml(io::BufReader::new(fs::File::open(&opts.xml_path).unwrap()));
    assert!(xml.get_name() == "registry");
    let (type_map, type_order) = gather_types(&xml, &opts);
    let (_group_map, _group_order) = gather_groups(&xml, &opts);
    let (value_map, value_order) = gather_values(&xml, &opts);
    let (command_map, command_order) = gather_commands(&xml, &opts);
    let (mut type_set, value_set, command_exts) = gather_features(&xml, &opts);
    for (command, ext) in &command_exts {
        if used_identifier_set.contains(command) {
            let command = &command_map[*command];
            command.touch_types(&mut type_set, ext);
        }
    }
    print!(r"#![allow(dead_code,non_snake_case,non_upper_case_globals,unused_imports)]

/// This module was generated using the rglgen crate.
/// It is a {}binding for {}.
", match opts.used_identifiers_path { Some(_) => "partial ", _ => ""},
           opts.version);
    if !opts.extensions.is_empty() {
        print!(r"///
/// It includes support for the following extensions:
");
        for ext in &opts.extensions {
            print!("/// - {}\n", ext);
        }
    }
    else {
        print!(r"/// It does not support any extensions.
");
    }
print!("
// The following comments are from the source XML file. It refers to that file,
// not this generated Rust code. Nevertheless, valuable copyright and
// provenance data may be present.
");
    output_comment_elements(&xml);
    println!("\n// *** TYPES ***");
    if opts.use_libc {
        println!("use libc;");
    }
    for typ in &type_order {
        if type_set.contains_key(typ.as_str()) {
            type_map[typ].output(&opts);
        }
    }
    println!("\n// *** VALUES ***");
    for value in &value_order {
        if used_identifier_set.contains(value.as_str()) {
            if value_set.contains_key(value.as_str()) {
                value_map[value].output(value, &opts);
            }
        }
    }
    println!("\n// *** COMMANDS ***\npub struct Procs {{");

    let (sorted_commands, proc_indices, ext_proc_ranges)
        = sort_commands(&used_identifier_set, &command_map, &command_exts,
                        &command_order);

    println!("    procs: [*const (); {}],", sorted_commands.len());
    
    for ext in &opts.extensions {
        println!("    has_{}: bool,",
                 if ext.starts_with("GL_") { &ext[3..] }
                 else { &ext[..] });
    }
    print!("{}", r#"}

use std::fmt;
impl fmt::Debug for Procs {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Procs{{...}}")?;
        Ok(())
    }
}
"#);
    let mut need_missing_ext_panic = false;
    for command in &command_order {
        if used_identifier_set.contains(command.as_str()) {
            if let Some(ext) = command_exts.get(command.as_str()) {
                if !ext.is_empty() {
                    command_map[command].output_dummy_imp(ext, &opts);
                    need_missing_ext_panic = true;
                }
            }
        }
    }
    if need_missing_ext_panic {
        print!("{}",
               r#"
#[inline(never)] fn missing_ext_panic(name: &str, ext: &str) -> ! {
    panic!("{} called, but the requisite extension ({}) is not present",
        name, ext);
}

"#);
    }
print!("{}", r#"use std::mem::transmute;
use std::mem::uninitialized;
use std::ffi::CStr;
"#);
    print!("{}", r#"impl Procs {
    pub fn new<E, F: Fn(&[u8])->Result<*const(),E>>(get_proc: F)
                 -> Result<Procs, E> {
        let mut ret = Procs {
            procs: unsafe { uninitialized() },
"#);
    for ext in &opts.extensions {
        println!("            has_{}: false,",
                 if ext.starts_with("GL_") { &ext[3..] }
                 else { &ext[..] });
    }
    println!("{}", r#"        };"#);
    // if you *really* want a GL binding with no GL entry points in it, I'm not
    // gonna get in your way.
    let mut need_getprocs = false;
    // initialize the procs before we try calling glGetString (duh)
    if let Some(&(start, stop)) = ext_proc_ranges.get("") {
        need_getprocs = true;
        print!("        Procs::getprocs(get_proc, &mut ret.procs[{}..{}], &[\n",
               start, stop);
        for i in start..stop {
            print!("            b\"{}\\0\",\n", sorted_commands[i as usize]);
        }
        print!("        ])?;\n");
    }
    if !opts.extensions.is_empty() {
        print!("{}",r#"        let extensions = unsafe {CStr::from_ptr(transmute(ret.GetString(GL_EXTENSIONS)))};
        let extensions = extensions.to_bytes();
        for ext in extensions.split(|x| *x == b' ') {
            match ext {
"#);
        for ext in &opts.extensions {
            println!(r#"                b"{}" => ret.has_{} = true,"#,
                     ext,
                     if ext.starts_with("GL_") { &ext[3..] }
                     else { &ext[..] });
        }
        print!("{}",r#"            _ => (),
            }
        }
"#);
    }
    for ext in &opts.extensions {
        if let Some(&(start, stop)) = ext_proc_ranges.get(ext.as_str()) {
            let name_for_has =
                if ext.starts_with("GL_") { &ext[3..] }
                else { &ext[..] };
            need_getprocs = true;
            print!(r#"        if ret.has_{} {{
            Procs::getprocs(get_proc, &mut ret.procs[{}..{}], &["#,
                   name_for_has, start, stop);
            for i in start..stop {
                print!("                b\"{}\0\",\n",
                       sorted_commands[i as usize]);
            }
            print!(r#"            ])?;
        }}
        else {{
            ret.procs[{}..{}].copy_from_slice(&[
"#, start, stop);
            for i in start..stop {
                print!("                {}_null_imp as *const (),\n",
                       sorted_commands[i as usize]);
            }
            print!(r#"            ]);
        }}
"#);
        }
    }
    
    print!("{}",r#"        Ok(ret)
    }
"#);
    if need_getprocs {
        print!("{}",r#"    fn getprocs<E, F: Fn(&[u8])->Result<*const(),E>>(get_proc: F, range: &mut[*const ()], names: &[&[u8]]) -> Result<(), E> {
        debug_assert_eq!(range.len(), names.len());
        for i in 0..range.len() {
            range[i] = unsafe {transmute(get_proc(names[i])?)};
        }
        Ok(())
    }
"#);
    }
    for command in &command_order {
        if used_identifier_set.contains(command.as_str())
            && command_exts.contains_key(command.as_str()) {
            command_map[command].output_imp(&opts,
                                            proc_indices[command.as_str()]);
        }
    }
    println!("{}","}");
}
