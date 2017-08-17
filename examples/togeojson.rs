/// This example use the "togeojson" feature of the library to generate
/// a geojson FeatureCollection of the night zone of the earth at each hour during one year.
/// Each hour is represented by a geosjon Feature.
extern crate nightcoords;
extern crate geojson;
extern crate chrono;
extern crate serde_json;

use nightcoords::{Mode, night_coord_geojson};
use chrono::{TimeZone, Utc};
use std::io::Write;


fn main() {
    let mut v = Vec::new();
    let mut c = 0;
    // month and day count start at 1 in `chrono` library:
    for month in 1..13 {
        for day in 1..8 {
            for hour in 0..24 {
                let dt = Utc.ymd(2017, month, day).and_hms(hour, 0, 10);
                let mut geojson_feature =
                    night_coord_geojson(&dt, 10., 90., 180., -90.0, -180., &Mode::Night).unwrap();
                geojson_feature.id = Some(serde_json::Value::from(c));
                v.push(geojson_feature);
                c += 1;
            }
        }
    }
    let feature_collection = geojson::FeatureCollection {
        bbox: None,
        features: v,
        foreign_members: None,
    };
    let mut file = ::std::fs::File::create("/tmp/daynight.geojson").unwrap();
    file.write(geojson::GeoJson::from(feature_collection)
                   .to_string()
                   .as_bytes())
        .unwrap();
}
