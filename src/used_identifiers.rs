use cmdline::CmdLine;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};

pub enum UsedIdentifiers {
    NotApplicable,
    Applicable(HashSet<String>),
}

impl UsedIdentifiers {
    pub fn contains(&self, key: &str) -> bool {
        match self {
            &UsedIdentifiers::NotApplicable => true,
            &UsedIdentifiers::Applicable(ref set) => set.contains(key),
        }
    }
}

pub fn get_used_identifiers(opts: &CmdLine) -> UsedIdentifiers {
    match opts.used_identifiers_path {
        None => UsedIdentifiers::NotApplicable,
        Some(ref path) => {
            let file = BufReader::new(File::open(path).unwrap());
            let mut set = HashSet::new();
            for line in file.lines() {
                let mut line = line.unwrap();
                // thanks, DOS!
                if line.ends_with("\r") {
                    let llen = line.len();
                    line.truncate(llen-1);
                }
                set.insert(line);
            }
            UsedIdentifiers::Applicable(set)
        },
    }
}
