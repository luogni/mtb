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
extern crate imageproc;


use docopt::Docopt;
use std::fs;
use std::path::Path;
use xml::reader::{EventReader, XmlEvent};
use std::fs::File;
use std::io::{BufReader, Read, Write};
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

fn get_cached_tile(tile: &Tile) -> Option<image::DynamicImage> {
    let path = Path::new("./cache/").join(tile.ts_path("png"));
    if let Some(img) = image::open(&path).ok() {
        return Some(img);
    }
    None
}

fn cache_tile(tile: &Tile, buf: &Vec<u8>) {
    let path = Path::new("./cache/").join(tile.ts_path("png"));
    let p = path.parent().unwrap();
    fs::create_dir_all(p).unwrap();

    let mut file = File::create(&path).unwrap();
    file.write_all(buf.as_slice()).unwrap();
}

fn download_tile(tile: &Tile) -> Option<image::DynamicImage> {
    if let Some(img) = get_cached_tile(&tile) {
        return Some(img);
    }
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
            cache_tile(&tile, &buf);
            return Some(img);
        }
    }
    None
}

fn lat_lon_to_xy(p: LatLon, zoom: u8) -> (f32, f32) {
    let n: f32 = 2f32.powi(zoom as i32);
    let x: f32 = n * (p.lon() + 180f32) / 360f32;
    let lat_rad = p.lat().to_radians();
    let y: f32 = (1f32 - (lat_rad.tan() + (1f32 / lat_rad.cos())).ln() * std::f32::consts::FRAC_1_PI) / 2f32 * n;
        
    (x, y)
}

fn draw(bbox: &BBox, all: &Vec<LatLon>, zoom: u8) {
    let w = 512;
    let h = 512;
    let mut tiles: Vec<Tile> = Vec::new();

    for tile in bbox.tiles() {
        if tile.zoom() == zoom {
            tiles.push(tile);
        }else if tile.zoom() > zoom { break }
    }
    
    let xmin = tiles.iter().min_by(|t1, t2| t1.x().cmp(&t2.x())).unwrap();
    let xmax = tiles.iter().max_by(|t1, t2| t1.x().cmp(&t2.x())).unwrap();
    let ymin = tiles.iter().min_by(|t1, t2| t1.y().cmp(&t2.y())).unwrap();
    let ymax = tiles.iter().max_by(|t1, t2| t1.y().cmp(&t2.y())).unwrap();
    println!("{} {} {} {}", xmin.x(), xmax.x(), ymin.y(), ymax.y());
    let mut img = image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::new(w, h);
    let res = 512 / std::cmp::min(xmax.x() - xmin.x() + 1, ymax.y() - ymin.y() + 1);
    println!("{}", res);
    
    for tile in &tiles {
        // println!("{:?}", tile);
        if let Some(imgtile) = download_tile(&tile) {
            let imgtile = imgtile.resize_exact(res, res, image::FilterType::Nearest);
            img.copy_from(&imgtile, (tile.x() - xmin.x()) * res, (tile.y() - ymin.y()) * res);
        }
    }
    
    // FIXME: for each points find x/y and draw! (how? linear?)
    let _ = img.save("output.png");
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
        println!("Draw!");
        draw(&bbox, &all, 14);
    }
}
