use llpoint::LLPoint;
use std::path::Path;
use std::fs;
use xml::reader::{EventReader, XmlEvent};
use std::fs::File;
use std::io::{BufReader};
use errors::*;

pub fn load_gpx(path: &str, onlyfirst: bool) -> Result<Vec<LLPoint>> {
    let path = Path::new(path);
    let mut ret: Vec<LLPoint> = Vec::new();
    println!("Loading path {:?}", path);
    for entry in try!(fs::read_dir(path).chain_err(|| "Directory not valid")) {
        let entry = try!(entry.chain_err(|| "Path not valid"));
        let file = try!(File::open(entry.path()).chain_err(|| "File path not valid"));
        let file = BufReader::new(file);
        let parser = EventReader::new(file);
        let mut firstnow = false;
        for e in parser {
            match e {
                Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                    if name.local_name == "trkpt" {
                        let mut lat: f32 = 0.0;
                        let mut lon: f32 = 0.0;
                        
                        for a in attributes {
                            if a.name.local_name == "lat" {
                                lat = try!(a.value.parse().chain_err(|| "Can't parse lat"));
                            }else if a.name.local_name == "lon" {
                                lon = try!(a.value.parse().chain_err(|| "Can't parse lon"));
                            }
                        }
                        if !firstnow {
                            entry.path().to_str().
                                and_then(|p| Some(LLPoint::new_break(p))).
                                and_then(|p| Some(ret.push(p)));
                            firstnow = true;
                        }
                        ret.push(LLPoint::new_point(lat, lon));
                        if onlyfirst {
                            break
                        }
                    }
                }
                Err(_) => {
                    // println!("Error: {:?} {}", entry, e);
                    break;
                }
                _ => {
                }
            }
        }
    }
    Ok(ret)
}
