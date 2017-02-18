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
use std::time::Duration;
use slippy_map_tiles::{LatLon, BBox, Tile};
use hyper::Client;
use image::GenericImage;
use image::Rgba;
use imageproc::drawing::{draw_line_segment_mut, draw_convex_polygon_mut};
use imageproc::rect::Rect;


const USAGE: &'static str = "
MTB tool.

Usage:
  mtb draw <path> [--zoom=<z> --imagewidth=<w> --linewidth=<w>]
  mtb (-h | --help)
  mtb --version

Options:
  -h --help         Show this screen.
  --version         Show version.
  --linewidth=<w>   Line width in pixels [default: 8].
  --zoom=<z>        Zoom level to use [default: 13].
  --imagewidth=<w>  Image width in pixels [default: 2048].
";

#[derive(Debug, RustcDecodable)]
struct Args {
    arg_path: String,
    flag_linewidth: u8,
    flag_zoom: u8,
    flag_imagewidth: u32,
    cmd_draw: bool,
}

#[derive(Debug, PartialEq, Clone)]
enum LLPointType {
    LatLon,
    Break,
}

#[derive(Debug)]
struct LLPoint {
    p_type: LLPointType,
    point: Option<LatLon>,
}

impl LLPoint {
    fn new_point(lat: f32, lon: f32) -> LLPoint {
        LLPoint { p_type: LLPointType::LatLon, point: LatLon::new(lat, lon) }
    }

    fn new_break() -> LLPoint {
        LLPoint { p_type: LLPointType::Break, point: None }
    }

    fn lat(&self) -> f32 {
        match self.point {
            Some(ref ll) => { ll.lat() },
            None => { 0f32 }
        }
    }

    fn lon(&self) -> f32 {
        match self.point {
            Some(ref ll) => { ll.lon() },
            None => { 0f32 }
        }
    }

    fn point(&self) -> Option<LatLon> {
        self.point.clone()
    }

    fn is_latlon(&self) -> bool {
        self.p_type == LLPointType::LatLon
    }

    fn is_break(&self) -> bool {
        self.p_type == LLPointType::Break
    }
}

fn load_gpx(path: &String) -> Vec<LLPoint> {
    let path = Path::new(path);
    let mut ret: Vec<LLPoint> = Vec::new();
    println!("Loading path {:?}", path);
    if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            println!("Loading file {:?}", entry);
            ret.push(LLPoint::new_break());
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
                            ret.push(LLPoint::new_point(lat, lon));
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


fn load_bbox(all: &Vec<LLPoint>) -> BBox {
    let mut latmin: f32 = 999f32;
    let mut latmax: f32 = -999f32;
    let mut lonmin: f32 = 999f32;
    let mut lonmax: f32 = -999f32;

    for p in all {
        if p.is_latlon() {
            if p.lat() < latmin { latmin = p.lat() }
            if p.lat() > latmax { latmax = p.lat() }
            if p.lon() < lonmin { lonmin = p.lon() }
            if p.lon() > lonmax { lonmax = p.lon() }
        }
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
    let mut client = Client::new();
    client.set_read_timeout(Some(Duration::new(3, 0)));
    //  http://[abc].tile.openstreetmap.org/zoom/x/y.png 
    //  http://[abc].tile.opencyclemap.org/cycle/zoom/x/y.png
    let url = format!("http://a.tile.opencyclemap.org/cycle/{}/{}/{}.png",
                      tile.zoom(), tile.x(), tile.y());
    let mut res = client.get(&url).send().unwrap();
    let mut buf = Vec::new();
    if res.read_to_end(&mut buf).is_ok() {
        if let Some(img) = image::load_from_memory(&buf).ok() {
            // println!("Loaded {} {}!", img.width(), img.height());
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

fn draw(bbox: &BBox, all: &Vec<LLPoint>, zoom: u8, imagewidth: u32, linewidth: u8) {
    let w = imagewidth;
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
    let resx = w / (xmax.x() - xmin.x() + 1);
    let h = w * ((ymax.y() - ymin.y() + 1) / (xmax.x() - xmin.x() + 1));
    let resy = h / (ymax.y() - ymin.y() + 1);
    let count = tiles.len();
    println!("{} {} {} {}", resx, resy, w, h);
    let mut img = image::ImageBuffer::<Rgba<u8>, Vec<u8>>::new(w, h);

    let mut oldperc = 0;
    for (i, tile) in tiles.iter().enumerate() {
        let perc = (i * 100) / count;
        if perc % 2 == 0 && oldperc != perc {
            println!("Downloading: {}%", perc);
            oldperc = perc;
        }
        'o: loop {
            let mut maxtry = 5;
            match download_tile(&tile) {
                Some(imgtile) => {
                    let imgtile = imgtile.resize_exact(resx, resy, image::FilterType::Nearest);
                    img.copy_from(&imgtile, (tile.x() - xmin.x()) * resx, (tile.y() - ymin.y()) * resy);
                    break 'o;
                },
                None => {
                    maxtry = maxtry - 1;
                    if maxtry == 0 {
                        println!("Can't download this tile {:?}", tile);
                        continue 'o;
                    }
                }
            }
        }
    }

    let mut iter = all.iter().peekable();
    let alw: i8 = linewidth as i8 / 2 as i8;
    loop {
        match iter.next() {
            Some(p) => {
                if let Some(new) = iter.peek() {
                    if p.is_latlon() && new.is_latlon() {
                        // println!("draw {:?} {:?}", p, new);
                        let (x1, y1) = lat_lon_to_xy(p.point().unwrap(), zoom);
                        let (x2, y2) = lat_lon_to_xy(new.point().unwrap(), zoom);
                        for lw in -alw..alw {
                            for lh in -alw..alw {
                                draw_line_segment_mut(&mut img,
                                                      ((x1 - xmin.x() as f32) * resx as f32 + lw as f32,
                                                       (y1 - ymin.y() as f32) * resy as f32 + lh as f32),
                                                      ((x2 - xmin.x() as f32) * resx as f32 + lw as f32,
                                                       (y2 - ymin.y() as f32) * resy as f32 + lh as f32 ),
                                                      Rgba([255u8, 0u8, 0u8, 255u8]));
                            }
                        }
                        // println!("draw {} {} {} {}", x1, y1, x2, y2);
                    }
                }
            },
            None => { break }
        }
    }
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
        draw(&bbox, &all, args.flag_zoom, args.flag_imagewidth, args.flag_linewidth);
    }
}
