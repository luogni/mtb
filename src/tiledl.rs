extern crate image;

use std::fs;
use slippy_map_tiles::{Tile};
use hyper::Client;
use std::path::Path;
use std::fs::File;
use std::time::Duration;
use std::io::{Read, Write};
use errors::*;


fn get_cached_tile(tile: &Tile) -> Result<image::DynamicImage> {
    let path = Path::new("./cache/").join(tile.ts_path("png"));
    image::open(&path).chain_err(|| "Error loading cache tile")
}

fn cache_tile(tile: &Tile, buf: &[u8]) -> Result<()> {
    let path = Path::new("./cache/").join(tile.ts_path("png"));
    let p = try!(path.parent().ok_or("No parent"));
    fs::create_dir_all(p).
        and_then(|_| File::create(&path)).
        and_then(|mut file| file.write_all(buf)).
        chain_err(|| "Error caching tile")
}

pub fn download_tile(tile: &Tile) -> Result<image::DynamicImage> {
    if let Ok(img) = get_cached_tile(tile) {
        return Ok(img);
    }
    let mut client = Client::new();
    client.set_read_timeout(Some(Duration::new(3, 0)));
    //  http://[abc].tile.openstreetmap.org/zoom/x/y.png 
    //  http://[abc].tile.opencyclemap.org/cycle/zoom/x/y.png
    let url = format!("http://a.tile.opencyclemap.org/cycle/{}/{}/{}.png",
                      tile.zoom(), tile.x(), tile.y());
    let mut res = try!(client.get(&url).send().chain_err(|| "Error getting tile"));
    let mut buf = Vec::new();
    if res.read_to_end(&mut buf).is_ok() {
        if let Ok(img) = image::load_from_memory(&buf) {
            // println!("Loaded {} {}!", img.width(), img.height());
            try!(cache_tile(&tile, &buf));
            return Ok(img);
        }
    }
    bail!("Error downloading tile");
}

