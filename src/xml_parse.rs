extern crate xml;

use crate::component2::{Component};
use std::io::BufWriter;
use xml::{EventReader, EventWriter, EmitterConfig, reader::XmlEvent, writer::events::XmlEvent as XmlEventW};
use std::error::Error;

fn props_from_xml(parser: &mut XMLParser) -> Result<(String, String), Box<Error>> {
    let start_tag = parser.next().ok_or("bad props")?;
    let body = parser.next().ok_or("bad props")?;
    let end_tag = parser.next().ok_or("bad props")?;
    let mut name = String::new(); 
    let mut val = String::new(); 
    match start_tag? {
        XmlEvent::StartElement {name: _, attributes: attr, namespace: _} => {
            name += attr.iter().find(|x| x.name.local_name == "name").map(|on| &on.value[..]).ok_or("no name on props")?;
        },
        _ => { return Err("bad props".into()); }
    };
    match body? {
        XmlEvent::Characters(c) => {
            val += &c[..];
        },
        _ => { return Err("bad props".into()); }
    }
    match end_tag? {
        XmlEvent::EndElement {name: n} => {
            if &n.local_name[..] != "property" { return Err("bad props".into()); }
        },
        _ => { return Err("bad props".into()); }
    }
    Ok((name, val))
}

type XMLParser<'a> = std::iter::Peekable<xml::reader::Events<&'a[u8]>>;

impl Component {
    fn from_xml(parser: &mut XMLParser) -> Result<Component, Box<Error>> {
        let mut c = Component::empty();
        while let Some(e) = parser.peek().map(|x| x.clone()) {
            match e? {
                XmlEvent::StartElement {name: n, attributes: attr, namespace: _} => {
                    match &n.local_name[..] {
                        "object" => {
                            c.class = attr.iter().find(|x| x.name.local_name == "class").map(|on| on.value.clone()).unwrap_or("".to_string());
                            c.id = attr.iter().find(|x| x.name.local_name == "id").map(|on| on.value.clone()).unwrap_or("".to_string());
                        },
                        "property" => {
                            let p = props_from_xml(parser)?;
                            c.properties.insert(p.0, p.1);
                        },
                        "child" => {
                            parser.next();
                            let child = Component::from_xml(parser)?;
                            c.children.v.push(child.id.clone());
                            c.children.m.insert(child.id.clone(), child);
                        }
                        _ => {}
                    };
                },
                XmlEvent::EndElement {name: n} => {
                    if &n.local_name[..] == "object" {
                        return Ok(c);
                    }
                }
                _ => {}
            }
            parser.next();
        }
        Ok(c)
    }
    
    pub fn from_xml_string(xml_str: &str) -> Result<Component, Box<Error>> {
        let rdr = EventReader::from_str(xml_str);
        let mut parser = rdr.into_iter().peekable();
        let mut c = Component::empty();
        while let Some(tag) = parser.peek().map(|x| x.clone()) {
            match tag? {
                XmlEvent::StartElement {name: n, attributes: _, namespace: _} => {
                    match &n.local_name[..] {
                        "object" => {
                            c = Component::from_xml(&mut parser)?;
                        },
                        _ => {}
                    };
                },
                XmlEvent::EndElement {name: _} => {},
                _ => {}
            }
            parser.next();
        }
        Ok(c)
    }

    fn to_xml(&self, wtr: &mut EventWriter<BufWriter<Vec<u8>>>) -> Result<(), Box<Error>> {
        let start = XmlEventW::start_element("object").attr("id", &self.id[..]).attr("class", &self.class[..]);
        wtr.write(start)?;
        for (k,v) in self.properties.iter() {
            wtr.write(XmlEventW::start_element("property").attr("name", &k[..]))?;
            wtr.write(XmlEventW::characters(&v[..]))?;
            wtr.write(XmlEventW::end_element())?;
        };
        for child_id in self.children.v.iter() {
            wtr.write(XmlEventW::start_element("child"))?;
            self.children.m[child_id].to_xml(wtr)?;
            wtr.write(XmlEventW::end_element())?;
        }
        wtr.write(XmlEventW::end_element())?;
        Ok(())
    }

    pub fn to_xml_string(&self) -> Result<String, Box<Error>> {
        let buf: Vec<u8> = Vec::new();
        let config = EmitterConfig { write_document_declaration: false, perform_indent: true, ..EmitterConfig::new() };
        let mut wtr = EventWriter::new_with_config(BufWriter::new(buf), config);
        wtr.write(XmlEventW::start_element("interface"))?;
        self.to_xml(&mut wtr)?;
        wtr.write(XmlEventW::end_element())?;
        String::from_utf8(wtr.into_inner().into_inner()?).map_err(|e| e.into())
    }
}
