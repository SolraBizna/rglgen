// Yikes!

use std::io::BufRead;

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

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::process::exit;

/// Sorts the commands such that each required extension corresponds to a
/// contiguous range of procs.
#[allow(clippy::type_complexity)]
fn sort_commands<'a>(
    used_identifier_set: &UsedIdentifiers,
    command_map: &'a HashMap<String, Command>,
    command_exts: &'a HashMap<&'a str, &'a str>,
    command_order: &'a Vec<String>,
) -> (
    Vec<&'a str>,
    HashMap<&'a str, u32>,
    HashMap<&'a str, (u32, u32)>,
) {
    let mut unsorted = Vec::with_capacity(command_map.len());
    for command in command_order {
        if used_identifier_set.contains(command.as_str()) {
            if let Some(ext) = command_exts.get(command.as_str()) {
                let i = unsorted.len() as u32;
                unsorted.push((command.as_str(), *ext, i));
            }
        }
    }
    // Sort by extension first, then order within gl.xml
    unsorted
        .as_mut_slice()
        .sort_unstable_by(|a, b| match a.1.cmp(b.1) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => a.2.cmp(&b.2),
        });
    let mut sorted = Vec::with_capacity(unsorted.len());
    let mut indices = HashMap::new();
    let mut ranges = HashMap::new();
    let mut cur_range: Option<(&str, u32)> = None;
    for (i, (command_name, required_extension, _)) in
        unsorted.iter().enumerate()
    {
        let i = i as u32;
        indices.insert(*command_name, i);
        sorted.push(*command_name);
        match cur_range {
            None => {
                assert_eq!(i, 0);
                cur_range = Some((required_extension, i));
            }
            Some((oext, oi)) => {
                if oext != *required_extension {
                    ranges.insert(oext, (oi, i));
                    cur_range = Some((required_extension, i));
                }
            }
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
    let mut file = io::BufReader::new(fs::File::open(&opts.xml_path).unwrap());
    // skip a byte order mark if there is one
    {
        let top = file.fill_buf().unwrap();
        if top.starts_with(b"\xEF\xBB\xBF") {
            file.consume(3);
        }
    }
    let xml = dom::read_xml(file);
    assert!(xml.get_name() == "registry");
    let (type_map, type_order) = gather_types(&xml, &opts);
    let (_group_map, _group_order) = gather_groups(&xml, &opts);
    let (value_map, value_order) = gather_values(&xml, &opts);
    let (command_map, command_order) = gather_commands(&xml, &opts);
    let [mut type_set, value_set, command_exts] = gather_features(&xml, &opts);
    for (command, ext) in &command_exts {
        if used_identifier_set.contains(command) {
            let command = &command_map[*command];
            command.touch_types(&mut type_set, ext);
        }
    }
    print!(
        r"#![allow(dead_code,non_snake_case,non_upper_case_globals,unused_imports,clippy::all)]

//! This module was generated using the rglgen crate.
//! It is a {}binding for {}.
",
        match opts.used_identifiers_path {
            Some(_) => "partial ",
            _ => "",
        },
        opts.version
    );
    if !opts.extensions.is_empty() {
        print!(
            r"//!
//! It includes support for the following extensions:
"
        );
        for ext in &opts.extensions {
            println!("//! - {}", ext);
        }
    } else {
        println!(r"//! It does not support any extensions.");
    }
    print!(
        "
// The following comments are from the source XML file. It refers to that file,
// not this generated Rust code. Nevertheless, valuable copyright and
// provenance data may be present.
"
    );
    output_comment_elements(&xml);
    println!("\n// *** TYPES ***");
    // no longer helpful in Rust 2018
    /*
    if opts.use_libc {
        println!("use libc;");
    }
    */
    for typ in &type_order {
        if type_set.contains_key(typ.as_str()) {
            type_map[typ].output(&opts);
        }
    }
    println!("\n// *** VALUES ***");
    for value in &value_order {
        if used_identifier_set.contains(value.as_str())
            && value_set.contains_key(value.as_str())
        {
            value_map[value].output(value, &opts);
        }
    }
    println!("\n// *** COMMANDS ***\npub struct Procs {{");

    let (sorted_commands, proc_indices, ext_proc_ranges) = sort_commands(
        &used_identifier_set,
        &command_map,
        &command_exts,
        &command_order,
    );

    println!("    procs: [*const (); {}],", sorted_commands.len());

    for ext in &opts.extensions {
        println!(
            "    pub has_{}: bool,",
            ext.strip_prefix("GL_").unwrap_or(ext)
        )
    }
    print!(
        r#"}}

use std::fmt;
impl fmt::Debug for Procs {{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {{
        write!(f, "Procs{{{{...}}}}")?;
        Ok(())
    }}
}}
"#
    );
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
        print!(
            r#"
#[inline(never)] fn missing_ext_panic(name: &str, ext: &str) -> ! {{
    panic!("{{}} called, but the requisite extension ({{}}) is not present",
        name, ext);
}}

"#
        );
    }
    print!(
        r#"use std::mem::{{transmute, MaybeUninit}};
use std::ffi::CStr;
"#
    );
    print!(
        r#"impl Procs {{
    pub fn new<E, F: Fn(&[u8])->Result<*const(),E>>(get_proc: F)
                 -> Result<Procs, E> {{
"#
    );
    // if you *really* want a GL binding with no GL entry points in it, I'm not
    // gonna get in your way.
    let mut need_getprocs = false;
    print!(
        r#"        let mut procs: [MaybeUninit<*const()>; {}] = unsafe {{
        MaybeUninit::uninit().assume_init()
    }};
"#,
        sorted_commands.len()
    );
    // initialize the procs before we try calling glGetString (duh)
    if let Some(&(start, stop)) = ext_proc_ranges.get("") {
        need_getprocs = true;
        println!(
            r#"        Procs::getprocs(&get_proc, &mut procs[{}..{}], &["#,
            start, stop
        );
        for i in start..stop {
            println!("            b\"{}\\0\",", sorted_commands[i as usize]);
        }
        println!(r#"        ])?;"#);
    }
    for ext in &opts.extensions {
        if let Some(&(start, stop)) = ext_proc_ranges.get(ext.as_str()) {
            for i in start..stop {
                println!(
                    "        procs[{}].write({}_null_imp as *const ());",
                    i, sorted_commands[i as usize]
                );
            }
        }
    }
    print!(
        r#"        let procs = unsafe {{ transmute(procs) }};
        #[allow(unused_mut)] let mut ret = Procs {{
            procs,
"#
    );
    for ext in &opts.extensions {
        println!(
            "            has_{}: false,",
            ext.strip_prefix("GL_").unwrap_or(ext)
        );
    }
    println!("        }};");
    if !opts.extensions.is_empty() {
        print!(
            r#"        let disabled_extensions = std::env::var("GL_DISABLED_EXTENSIONS");
        let disabled_extensions = disabled_extensions.as_ref()
            .map(|x| x.as_bytes()).unwrap_or(b"");
        let disabled_extensions
            = build_disabled_extension_list(disabled_extensions);
"#
        );
        if opts.version.needs_getstringi_extensions() {
            // both OpenGL and OpenGL ES switched to this method in version 3.0
            // and deprecated the previous one
            print!(
                r#"        let mut num_extensions = 0;
        unsafe {{ ret.GetIntegerv(GL_NUM_EXTENSIONS, &mut num_extensions) }};
        for i in 0 .. num_extensions as GLuint {{
            let ext = unsafe {{CStr::from_ptr(transmute(ret.GetStringi(GL_EXTENSIONS, i)))}}.to_bytes();
"#
            );
        } else {
            print!(
                r#"        let extensions = unsafe {{CStr::from_ptr(transmute(ret.GetString(GL_EXTENSIONS)))}};
        let extensions = extensions.to_bytes();
        for ext in extensions.split(|x| *x == b' ') {{
"#
            );
        }
        print!(
            r#"            if disabled_extensions.contains(ext) {{ continue }}
            match ext {{
"#
        );
        for ext in &opts.extensions {
            println!(
                r#"                b"{}" => ret.has_{} = true,"#,
                ext,
                ext.strip_prefix("GL_").unwrap_or(ext)
            );
        }
        print!(
            r#"            _ => (),
            }}
        }}
"#
        );
    }
    for ext in &opts.extensions {
        if let Some(&(start, stop)) = ext_proc_ranges.get(ext.as_str()) {
            let name_for_has = ext.strip_prefix("GL_").unwrap_or(ext);
            need_getprocs = true;
            print!(
                r#"        if ret.has_{} {{
            Procs::getprocs(&get_proc,
                            unsafe {{ transmute(&mut ret.procs[{}..{}]) }}, &[
"#,
                name_for_has, start, stop
            );
            for i in start..stop {
                println!(
                    "                b\"{}\\0\",",
                    sorted_commands[i as usize]
                );
            }
            print!(
                r#"            ])?;
        }}
"#
            );
        }
    }

    print!(
        r#"        Ok(ret)
    }}
"#
    );
    if need_getprocs {
        print!(
            r#"    fn getprocs<E, F: Fn(&[u8])->Result<*const(),E>>(get_proc: &F, range: &mut[MaybeUninit<*const ()>], names: &[&[u8]]) -> Result<(), E> {{
        debug_assert_eq!(range.len(), names.len());
        for i in 0..range.len() {{
            range[i].write(unsafe {{transmute(get_proc(names[i])?)}});
        }}
        Ok(())
    }}
"#
        );
    }
    for command in &command_order {
        if used_identifier_set.contains(command.as_str())
            && command_exts.contains_key(command.as_str())
        {
            command_map[command]
                .output_imp(&opts, proc_indices[command.as_str()]);
        }
    }
    println!("}}");
    if !opts.extensions.is_empty() {
        print!(
            r#"
fn build_disabled_extension_list(disabled_extensions: &[u8])
            -> std::collections::HashSet<&[u8]> {{
    disabled_extensions.split(|&x| {{
        !((x >= b'0' && x <= b'9')
          || (x >= b'A' && x <= b'Z')
          || (x >= b'a' && x <= b'z')
          || (x == b'_'))
    }}).filter_map(|x| {{
        match x {{
            b"" => None,
            x => Some(x)
        }}
    }}).collect()
}}
"#
        );
    }
}
