use std::{
    env::args,
    path::PathBuf,
};

use getopts::Options;

use crate::versions::*;

pub struct CmdLine {
    pub version: ActiveVersion,
    pub xml_path: PathBuf,
    pub extensions: Vec<String>,
    pub use_libc: bool,
    pub used_identifiers_path: Option<PathBuf>,
}

fn print_usage(program: &str, opts: &Options) {
    let brief = format!("Usage: {} [options] path/to/gl.xml extensions... \
                         >.../gl.rs",
                        program);
    eprint!("{}", opts.usage(&brief));
}

pub fn parse_cmdline() -> Option<CmdLine> {
    let argv: Vec<String> = args().collect();
    let program = &argv[0];
    let mut opts = Options::new();
    opts.optopt("t", "target-version", "change the targeted API and version (e.g. gl2.1, glcore4.0, gles2.0; default is gles2.0)", "VERSION");
    opts.optopt("u", "used-identifiers", "path to a text file that contains identifiers, one per line, that your program uses. If this option is not specified, ALL identifiers will be exposed. Using this option saves a lot of runtime memory and a LOT of compile time, and is STRONGLY RECOMMENDED. If an identifier is in this text file but not found in this version of the GL, it is simply ignored.", "PATH");
    opts.optflag("C", "without-libc", "disable the use of the `libc` crate for correctly matching GL types (dangerous!)");
    if argv.len() < 2 {
        print_usage(program, &opts);
        return None
    }
    let matches = match opts.parse(&argv[1..]) {
        Ok(matches) => matches,
        Err(fail) => panic!("{}", fail.to_string()),
    };
    if matches.free.is_empty() {
        eprintln!("No gl.xml path specified");
        print_usage(program, &opts);
        return None
    }
    let ret = CmdLine{
        version: match parse_version(matches.opt_str("t").as_ref().map(|x| x.as_str()).unwrap_or("gles2.0")) {
            Err(wat) => {
                eprintln!("Invalid glversion: {}", wat);
                return None
            },
            Ok(version) => version,
        },
        xml_path: PathBuf::from(&matches.free[0]),
        extensions: matches.free[1..].into_iter().map(|x| x.clone()).collect(),
        use_libc: !matches.opt_present("C"),
        used_identifiers_path: match matches.opt_str("u") {
            Some(path) => Some(PathBuf::from(path)),
            None => None,
        },
    };
    Some(ret)
}
