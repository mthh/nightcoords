#[cfg(feature = "togeojson")]
extern crate serde;
#[cfg(feature = "togeojson")]
extern crate serde_json;
#[cfg(feature = "togeojson")]
extern crate geojson;

extern crate chrono;


use chrono::{DateTime, Datelike, Timelike, Utc};

#[cfg(feature = "togeojson")]
use geojson::{GeoJson, Geometry, Value, Feature};

pub type UtcDateTime = DateTime<Utc>;

pub enum Mode {
    Night,
    Day,
}

enum Calendar {
    Julian,
    Gregorian,
    ProlepticGregorian,
}

/// Compute the coordinates of the "night" part of the earth at a given `datetime`,
/// within a bbox defined by its `latmax`, `lonmax`, `latmin`, `lonmin` and return
/// the corresponding `geojson::Feature`, containing the geometry as a Polygon
/// and the date in `date` field of its properties, formated according to the
/// RFC 3339.
#[cfg(feature = "togeojson")]
pub fn night_coord_geojson(date: &UtcDateTime,
                           delta: f64,
                           latmax: f64,
                           lonmax: f64,
                           latmin: f64,
                           lonmin: f64,
                           mode: &Mode)
                           -> Result<geojson::Feature, &'static str> {
    let exterior_ring = match night_coord(&date, delta, latmax, lonmax, latmin, lonmin, &mode) {
        Ok(v) => v,
        Err(e) => return Err(e),
    };
    let geometry = Geometry::new(Value::Polygon(vec![exterior_ring]));
    let mut prop = serde_json::Map::new();
    prop.insert(String::from("date"),
                serde_json::to_value(date.to_rfc3339()).unwrap());
    Ok(Feature {
           bbox: None,
           geometry: Some(geometry),
           id: None,
           foreign_members: None,
           properties: Some(prop),
       })
}

/// Compute the coordinates of the polygon delimiting the "night" part
/// of the earth at a given `datetime`,
/// within a bbox defined by its `latmax`, `lonmax`, `latmin`, `lonmin`.
pub fn night_coord(datetime: &UtcDateTime,
                   delta: f64,
                   mut latmax: f64,
                   mut lonmax: f64,
                   mut latmin: f64,
                   mut lonmin: f64,
                   mode: &Mode)
                   -> Result<Vec<Vec<f64>>, &'static str> {
    if latmax < 80. {
        latmax = 80.;
    } else if latmax > 90. {
        latmax = 90.;
    }
    if latmin > -80. {
        latmin = -80.;
    } else if latmin < -90. {
        latmin = -90.;
    }
    if lonmin >= lonmax || lonmax - lonmin < 10. {
        lonmax = 180.;
        lonmin = -180.;
    }
    if lonmax > 180. {
        lonmax = 180.;
    }
    if lonmin < -180. {
        lonmin = -180.;
    }

    let (c_lon, c_lat, _, dec) = daynight_terminator(&datetime, lonmin, lonmax, delta)?;

    let n = c_lon.len();
    let mut exterior_ring = Vec::with_capacity(n + 3usize);

    for i in 0..n {
        exterior_ring.push(vec![c_lon[i], c_lat[i]]);
    }
    let lat_close = get_lat_close(dec, latmax, latmin, &mode);
    exterior_ring.push(vec![lonmax, lat_close]);
    exterior_ring.push(vec![lonmin, lat_close]);
    exterior_ring.push(vec![c_lon[0], c_lat[0]]);
    Ok(exterior_ring)
}

fn julian_day_from_date(dt: &UtcDateTime, calendar: &Calendar) -> Result<i32, &'static str> {
    let mut year = dt.year() as f64;
    let mut month = dt.month() as f64;
    let mut day = dt.day0() as f64;
    let hour = dt.hour();
    let minute = dt.minute();
    let second = dt.second();
    day = day + hour as f64 / 24.0 + minute as f64 / 1440.0 + second as f64 / 86400.0;
    if month < 3.0 {
        month += 12.0;
        year -= 1.0;
    }
    let a = (year / 100.0).floor();
    let jd = (365.25 * (year + 4716.0)).floor() + (30.6001 * (month + 1.0)).floor() + day - 1524.5;
    let b = match *calendar {
        Calendar::Gregorian => {
            if jd >= 2299170.5 {
                2.0 - a + (a / 4.).floor()
            } else if jd < 2299160.5 {
                0.0
            } else {
                return Err("Impossible date error".into());
            }
        }
        Calendar::ProlepticGregorian => 2.0 - a + (a / 4.).floor(),
        Calendar::Julian => 0.0,
    };
    Ok((jd + b).floor() as i32)
}

fn epem(dt: &UtcDateTime) -> Result<(f64, f64), &'static str> {
    let jd = julian_day_from_date(dt, &Calendar::ProlepticGregorian)?;
    // UTC hour:
    let ut = dt.hour() as f64 + dt.minute() as f64 / 60.0 + dt.second() as f64 / 3600.0;
    // Number of centuries from J2000:
    let t = (jd as f64 + (ut / 24.0) - 2451545.0) / 36525.;
    // Mean longitude corrected:
    let l = (280.460 + 36000.770 * t) % 360.;
    // Mean anomaly:
    let g = 357.528 + 35999.050 * t;
    // Ecliptic longitude:
    let lm = l + 1.915 * (g.to_radians()).sin() + 0.020 * (2.0 * g.to_radians()).sin();
    // Obliquity of the ecliptic:
    let ep = 23.4393 - 0.01300 * t;

    // Equation of time
    let eqt01 = -1.915 * (g.to_radians()).sin();
    let eqt02 = 0.020 * (2.0 * g.to_radians()).sin();
    let eqt03 = 2.466 * (2.0 * lm.to_radians()).sin();
    let eqt04 = 0.053 * (4.0 * lm.to_radians()).sin();
    let eqtime = eqt01 - eqt02 + eqt03 - eqt04;

    // Greenwich hour angle
    let gha = 15. * ut - 180. + eqtime;

    // Declination of sun
    let dec = ((ep.to_radians()).sin() * (lm.to_radians()).sin())
        .asin()
        .to_degrees();
    Ok((gha, dec))
}

fn daynight_terminator(dt: &UtcDateTime,
                       lonmin: f64,
                       lonmax: f64,
                       delta: f64)
                       -> Result<(Vec<f64>, Vec<f64>, f64, f64), &'static str> {
    let mut min_lon = lonmin;
    let (tau, dec) = epem(&dt)?;
    let mut lons = Vec::new();
    let mut lats = Vec::new();
    while min_lon <= lonmax {
        lons.push(min_lon);
        min_lon += delta;
    }
    for ln in &lons {
        lats.push((-((ln + tau).to_radians()).cos() / (dec.to_radians().tan()))
                      .atan()
                      .to_degrees());
    }
    Ok((lons, lats, tau, dec))
}

fn get_lat_close(dec: f64, latmax: f64, latmin: f64, mode: &Mode) -> f64 {
    match mode {
        &Mode::Night => if dec > 0.0 { latmax } else { latmin },
        &Mode::Day => if dec > 0.0 { latmin } else { latmax },
    }
}
