/*
 1) load gpxs from command line(dir with files in)
 2) get total bbox
 3) create output image
 4) download tiles
 5) put tiles and routes on output image

// FIXME:
 * no unwrap
 * error handling (gpx parsing, http requests, disk...)
 * libraries

 */
extern crate rustc_serialize;
extern crate docopt;
extern crate xml;
extern crate slippy_map_tiles;
extern crate hyper;
extern crate image;


use docopt::Docopt;
use std::fs;
use std::path::Path;
use xml::reader::{EventReader, XmlEvent};
use std::fs::File;
use std::io::{BufReader, Read};
use slippy_map_tiles::{LatLon, BBox, Tile};
use hyper::Client;
use image::GenericImage;


const USAGE: &'static str = "
MTB tool.

Usage:
  mtb draw <path>
  mtb (-h | --help)
  mtb --version

Options:
  -h --help     Show this screen
  --version     Show version
";

#[derive(Debug, RustcDecodable)]
struct Args {
    arg_path: String,
    cmd_draw: bool,
}

fn load_gpx(path: &String) -> Vec<LatLon> {
    let path = Path::new(path);
    let mut ret: Vec<LatLon> = Vec::new();
    println!("Loading path {:?}", path);
    if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            println!("Loading file {:?}", entry);
            let file = File::open(entry.path()).unwrap();
            let file = BufReader::new(file);
            let parser = EventReader::new(file);
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
                            ret.push(LatLon::new(lat, lon).unwrap());
                            //println!("{} {}", lat, lon);

                        }
                    }
                    Ok(XmlEvent::EndElement { .. }) => {
                        //depth -= 1;
                        //println!("{}-{}", depth, name);
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
    ret
}


fn load_bbox(all: &Vec<LatLon>) -> BBox {
    let mut latmin: f32 = 999f32;
    let mut latmax: f32 = -999f32;
    let mut lonmin: f32 = 999f32;
    let mut lonmax: f32 = -999f32;

    for point in all {
        if point.lat() < latmin { latmin = point.lat() }
        if point.lat() > latmax { latmax = point.lat() }
        if point.lon() < lonmin { lonmin = point.lon() }
        if point.lon() > lonmax { lonmax = point.lon() }
    }
    println!("{} {} {} {}", latmin, latmax, lonmin, lonmax);
    let bb: BBox = BBox::new(latmax, lonmin, latmin, lonmax).unwrap();
    bb
}

fn download_tile(tile: &Tile) -> Option<image::DynamicImage> {
    let client = Client::new();
    //  http://[abc].tile.openstreetmap.org/zoom/x/y.png 
    //  http://[abc].tile.opencyclemap.org/cycle/zoom/x/y.png
    let url = format!("http://a.tile.opencyclemap.org/cycle/{}/{}/{}.png",
                      tile.zoom(), tile.x(), tile.y());
    let mut res = client.get(&url).send().unwrap();
    let mut buf = Vec::new();
    if res.read_to_end(&mut buf).is_ok() {
        if let Some(img) = image::load_from_memory(&buf).ok() {
            println!("Loaded {} {}!", img.width(), img.height());
            return Some(img);
        }
    }
    None
}

fn draw(bbox: &BBox, all: &Vec<LatLon>, zoom: u8) {
    let mut first = true;
    
    for tile in bbox.tiles() {
        if tile.zoom() == zoom {
            println!("{:?}", tile);
            if first == true {
                if let Some(img) = download_tile(&tile) {
                    
                    first = false;
                }
            }
        }else if tile.zoom() > zoom { break }
    }
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
    println!("Hello, world!: {:?}", args);
    if args.cmd_draw == true {
        let all = load_gpx(&args.arg_path);
        println!("Loaded {} points", all.len());
        let bbox = load_bbox(&all);
        draw(&bbox, &all, 15);
    }
}
