extern crate image;

use std::fs;
use slippy_map_tiles::{Tile};
use hyper::Client;
use std::path::Path;
use std::fs::File;
use std::time::Duration;
use std::io::{Read, Write};


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

pub fn download_tile(tile: &Tile) -> Option<image::DynamicImage> {
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

