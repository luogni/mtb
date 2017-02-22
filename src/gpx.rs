use llpoint::LLPoint;
use std::path::Path;
use std::fs;
use xml::reader::{EventReader, XmlEvent};
use std::fs::File;
use std::io::{BufReader};

pub fn load_gpx(path: &String, onlyfirst: bool) -> Vec<LLPoint> {
    let path = Path::new(path);
    let mut ret: Vec<LLPoint> = Vec::new();
    println!("Loading path {:?}", path);
    if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let file = File::open(entry.path()).unwrap();
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
                                    lat = a.value.parse().unwrap()
                                }else if a.name.local_name == "lon" {
                                    lon = a.value.parse().unwrap()
                                }
                            }
                            if firstnow == false {
                                ret.push(LLPoint::new_break(entry.path().to_str().unwrap()));
                                firstnow = true;
                            }
                            ret.push(LLPoint::new_point(lat, lon));
                            if onlyfirst == true {
                                break
                            }
                        }
                    }
                    Ok(XmlEvent::EndElement { .. }) => {
                        //depth -= 1;
                        //println!("{}-{}", depth, name);
                    }
                    Err(e) => {
                        println!("Error: {:?} {}", entry, e);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
    ret
}
