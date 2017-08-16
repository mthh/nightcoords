# nightcoords

Get the night world geometry at a given date and time.

# Example:
Get the geometry of the night zone at 5pm (UTC) on 15th august 2017 and save it in a geojson Feature Collection:

```rust
extern crate nightcoords;
extern crate geojson;
extern crate chrono;

use nightcoords::night_coord_geojson;
use chrono::{TimeZone, Utc};
use std::io::Write;

fn main() {
    let dt = Utc.ymd(2017, 8, 15).and_hms(17, 0, 0);
    let geojson_feature = night_coord_geojson(&dt, 10., 90., 180., -90.0, -180.).unwrap();
    let feature_collection = geojson::FeatureCollection {
        bbox: None,
        features: vec![geojson_feature],
        foreign_members: None,
    };
    let mut file = ::std::fs::File::create("/tmp/daynight.geojson").unwrap();
    file.write(geojson::GeoJson::from(feature_collection)
                   .to_string()
                   .as_bytes())
        .unwrap();
}
```

### License and credits:
Licensed under ISC license, as [Basemap](https://github.com/matplotlib/basemap), the package from which this feature was taken
