use slippy_map_tiles::LatLon;

#[derive(Debug, PartialEq, Clone)]
enum LLPointType {
    LatLon,
    Break,
}

#[derive(Debug)]
pub struct LLPoint {
    p_type: LLPointType,
    point: Option<LatLon>,
    name: String,
}

impl LLPoint {
    pub fn new_point(lat: f32, lon: f32) -> LLPoint {
        LLPoint { p_type: LLPointType::LatLon, point: LatLon::new(lat, lon) , name: String::new()}
    }

    pub fn new_break(name: &str) -> LLPoint {
        LLPoint { p_type: LLPointType::Break, point: None, name: String::from(name) }
    }

    pub fn lat(&self) -> f32 {
        match self.point {
            Some(ref ll) => { ll.lat() },
            None => { 0f32 }
        }
    }

    pub fn lon(&self) -> f32 {
        match self.point {
            Some(ref ll) => { ll.lon() },
            None => { 0f32 }
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn point(&self) -> Option<LatLon> {
        self.point.clone()
    }

    pub fn is_latlon(&self) -> bool {
        self.p_type == LLPointType::LatLon
    }
}

