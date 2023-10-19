use std::io;

use crate::dom::{Element, Node};

fn gather_comment_elements<W: io::Write>(root: &Element, out: &mut W) {
    let gather_text = root.get_name() == "comment";
    for child in root.get_children() {
        match child {
            Node::Text(ref text) => {
                if gather_text {
                    out.write_all(text.as_bytes()).unwrap();
                }
            }
            Node::Element(ref element) => {
                gather_comment_elements(element, out);
            }
        }
    }
}

pub fn output_comment_elements(root: &Element) {
    let mut comment_text = Vec::new();
    gather_comment_elements(root, &mut comment_text);
    let comment_text = String::from_utf8(comment_text).unwrap();
    if !comment_text.starts_with('\n') {
        println!("//");
    }
    for line in comment_text.split('\n') {
        let mut line = line;
        while line.ends_with(' ') {
            line = &line[..line.len() - 1];
        }
        if line.is_empty() {
            println!("//");
        } else {
            println!("// {}", line);
        }
    }
}
