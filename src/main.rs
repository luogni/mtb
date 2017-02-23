/*
 1) load gpxs from command line(dir with files in)
 2) get total bbox
 3) create output image
 4) download tiles
 5) put tiles and routes on output image

// FIXME:
 * modules
  * tile downloader to library?
 * commands:
  * nicer draw image with some statistics, mountains, h+, h-, d_total
  * statistics for year/month/week, graphs..
  * auto split with clustering (how? dir? name?)
  * http://wiki.openstreetmap.org/wiki/Nominatim reverse, from latlon get a name
  * auto name with higher points, start point, street names..
  * after cluster split we can split a pbf file to small size to get info faster from it (there is a crate for parsing pbf files). split con osmconvert per ex.
  * auto random names (names crate)
  * telegram bot as gui
 */
#![recursion_limit = "1024"]
#[macro_use]
extern crate error_chain;
extern crate rustc_serialize;
extern crate docopt;
extern crate xml;
extern crate slippy_map_tiles;
extern crate hyper;
extern crate image;
extern crate imageproc;
extern crate rayon;

mod llpoint;
mod gpx;
mod tiledl;
mod cluster;

mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! { }
}

use tiledl::download_tile;
use llpoint::LLPoint;
use gpx::load_gpx;
use docopt::Docopt;
use slippy_map_tiles::{LatLon, BBox, Tile};
use image::GenericImage;
use image::Rgba;
use imageproc::drawing::{draw_line_segment_mut};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use errors::*;

const USAGE: &'static str = "
MTB tool.

Usage:
  mtb draw <path> [--zoom=<z> --imagewidth=<w> --linewidth=<w>]
  mtb cluster <path>
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
    cmd_cluster: bool,
}

fn load_bbox(all: &[LLPoint]) -> BBox {
    let fvec = all.into_iter().
        filter(|&p| p.is_latlon()).
        filter_map(|p| p.point()).
        collect::<Vec<_>>();
    BBox::new_from_points_list(&fvec).unwrap()
}

fn lat_lon_to_xy(p: LatLon, zoom: u8) -> (f32, f32) {
    let n: f32 = 2f32.powi(zoom as i32);
    let x: f32 = n * (p.lon() + 180f32) / 360f32;
    let lat_rad = p.lat().to_radians();
    let y: f32 = (1f32 - (lat_rad.tan() + (1f32 / lat_rad.cos())).ln() * std::f32::consts::FRAC_1_PI) / 2f32 * n;
        
    (x, y)
}

fn draw(bbox: &BBox, all: &[LLPoint], zoom: u8, imagewidth: u32, linewidth: u8) {
    let mut w = imagewidth / 256 * 256;
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
    let resx = ((w as f32) / (xmax.x() as f32 - xmin.x() as f32 + 1f32)).floor() as u32;
    w = resx * (xmax.x() - xmin.x() + 1);
    let h = (w as f32 * ((ymax.y() as f32 - ymin.y() as f32 + 1f32) / (xmax.x() as f32 - xmin.x() as f32 + 1f32))).ceil() as u32;
    let resy = resx;
    let count = tiles.len();
    println!("{} {} {} {} {}", resx, resy, w, h, count);
    let mut img = image::ImageBuffer::<Rgba<u8>, Vec<u8>>::new(w, h);

    {
        let img_thread = Arc::new(Mutex::new(&mut img));
        for (chi, ch) in tiles.chunks(count / 100 + 1).enumerate() {
            println!("{}", chi);

            ch.par_iter().map(|tile| {
                'o: loop {
                    let mut maxtry = 5;
                    match download_tile(tile) {
                        Ok(imgtile) => {
                            let imgtile = imgtile.resize_exact(resx, resy, image::FilterType::Triangle);
                            img_thread.lock().unwrap().copy_from(&imgtile, (tile.x() - xmin.x()) * resx,
                                                                 (tile.y() - ymin.y()) * resy);
                            break 'o;
                        },
                        Err(err) => {
                            maxtry -= 1;
                            if maxtry == 0 {
                                println!("Can't download this tile {:?} {:?}", tile, err);
                                continue 'o;
                            }
                        }
                    }
                }
            }).collect::<Vec<_>>();
        }
    }
                                    
    let mut iter = all.iter().peekable();
    let alw: i8 = linewidth as i8 / 2 as i8;
    while let Some(p) = iter.next() {
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
    }
    let _ = img.save("output.png");
}

// main function from error-chain
fn main() {
    if let Err(ref e) = run() {
        println!("error: {}", e);

        for e in e.iter().skip(1) {
            println!("caused by: {}", e);
        }

        // The backtrace is not always generated. Try to run this example
        // with `RUST_BACKTRACE=1`.
        if let Some(backtrace) = e.backtrace() {
            println!("backtrace: {:?}", backtrace);
        }

        ::std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
    println!("Hello, world!: {:?}", args);
    if args.cmd_draw {
        let all = try!(load_gpx(&args.arg_path, false));
        println!("Loaded {} points", all.len());
        let bbox = load_bbox(&all);
        println!("Draw!");
        draw(&bbox, &all, args.flag_zoom, args.flag_imagewidth, args.flag_linewidth);
    }else if args.cmd_cluster {
        let all = try!(load_gpx(&args.arg_path, true));
        println!("Loaded {} points", all.len());
        try!(cluster::cluster(&all));
    }
    Ok(())
}
