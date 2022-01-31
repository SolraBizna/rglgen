use std::{
    collections::HashMap,
    io, io::Write,
};

use xml::reader::{EventReader, XmlEvent};

#[derive(Debug)]
pub enum Node {
    Element(Element),
    Text(String),
}

#[derive(Debug)]
pub struct Element {
    name: String,
    attributes: HashMap<String, String>,
    contents: Vec<Node>
}

fn sub_get_text(root: &Element, out: &mut Vec<u8>) {
    for child in root.get_children() {
        match child {
            &Node::Text(ref text) => {
                out.write(text.as_bytes()).unwrap();
            },
            &Node::Element(ref element) => {
                sub_get_text(element, out);
            },
        }
    }
}

impl Element {
    pub fn get_name(&self) -> &str { &self.name }
    pub fn get_children(&self) -> &[Node] { &self.contents[..] }
    pub fn get_attributes(&self) -> &HashMap<String,String> { &self.attributes}
    pub fn get_text(&self) -> String {
        let mut ret = Vec::new();
        sub_get_text(self, &mut ret);
        String::from_utf8(ret).unwrap()
    }
    pub fn get_text_as_bytes(&self) -> Vec<u8> {
        let mut ret = Vec::new();
        sub_get_text(self, &mut ret);
        ret
    }
}

pub fn read_xml<R: io::Read>(input: R) -> Element {
    let mut stack = Vec::new();
    let mut ret: Option<Element> = None;
    for e in EventReader::new(input) {
        match e {
            Ok(XmlEvent::StartDocument{..}) |
            Ok(XmlEvent::ProcessingInstruction{..}) |
            Ok(XmlEvent::Comment(_)) => (),
            Err(e) => panic!("XML parsing error: {}", e),
            Ok(XmlEvent::EndDocument) => {
                match ret {
                    Some(x) => return x,
                    None => {
                        panic!("EndDocument received, but the document was \
                                not complete!");
                    }
                }
            },
            Ok(XmlEvent::StartElement{name, attributes, ..}) => {
                stack.push(Element {
                    name: name.local_name,
                    attributes: attributes.into_iter()
                        .map(|x| (x.name.to_string(), x.value)).collect(),
                    contents: Vec::new()
                });
            },
            Ok(XmlEvent::EndElement{name}) => {
                debug_assert!(stack.len() > 0);
                let el = stack.pop().unwrap();
                assert!(el.name == name.local_name);
                if stack.is_empty() {
                    debug_assert!(ret.is_none());
                    ret = Some(el);
                }
                else {
                    stack.last_mut().unwrap().contents.push(Node::Element(el));
                }
            },
            Ok(XmlEvent::CData(text)) |
            Ok(XmlEvent::Characters(text)) |
            Ok(XmlEvent::Whitespace(text)) => {
                if !stack.is_empty() {
                    stack.last_mut().unwrap().contents.push(Node::Text(text));
                }
            },
        }
    }
    panic!("EndDocument was not received!")
}
