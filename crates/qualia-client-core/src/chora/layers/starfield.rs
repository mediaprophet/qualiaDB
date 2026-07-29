use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Star {
    pub hip: u32,
    pub ra_deg: f64,
    pub dec_deg: f64,
    pub magnitude: f32,
    pub bv_color: f32,
    pub name: Option<&'static str>,
}

pub fn bv_to_rgb(bv: f32) -> [f32; 4] {
    let bv = bv.clamp(-0.4, 2.0);
    let t = (bv + 0.4) / 2.4;
    let r = 1.0 - (t * 0.3).min(0.5);
    let g = 1.0 - (t * 0.5).abs();
    let b = 1.0 - ((1.0 - t) * 0.4).min(0.5);
    let brightness = 1.0 - (bv.abs() * 0.1);
    [r * brightness, g * brightness, b * brightness, 1.0]
}

pub fn mag_to_size(magnitude: f32) -> f32 {
    ((6.5 - magnitude) / 6.5).max(0.1) * 2.0
}

pub fn celestial_to_cartesian(ra_deg: f64, dec_deg: f64, radius: f64) -> [f32; 3] {
    let ra = ra_deg.to_radians();
    let dec = dec_deg.to_radians();
    let x = radius * dec.cos() * ra.cos();
    let y = radius * dec.sin();
    let z = radius * dec.cos() * ra.sin();
    [x as f32, y as f32, z as f32]
}

pub static BRIGHT_STARS: &[Star] = &[
    Star {
        hip: 32349,
        ra_deg: 101.287,
        dec_deg: -16.716,
        magnitude: -1.46,
        bv_color: 0.0,
        name: Some("Sirius"),
    },
    Star {
        hip: 24608,
        ra_deg: 95.676,
        dec_deg: -52.696,
        magnitude: -0.01,
        bv_color: 0.71,
        name: Some("Canopus"),
    },
    Star {
        hip: 69673,
        ra_deg: 213.915,
        dec_deg: 19.182,
        magnitude: 0.03,
        bv_color: -0.01,
        name: Some("Arcturus"),
    },
    Star {
        hip: 91262,
        ra_deg: 279.234,
        dec_deg: 38.784,
        magnitude: 0.08,
        bv_color: 0.42,
        name: Some("Vega"),
    },
    Star {
        hip: 30438,
        ra_deg: 79.172,
        dec_deg: -45.999,
        magnitude: 0.13,
        bv_color: -0.16,
        name: Some("Rigel Kentaurus"),
    },
    Star {
        hip: 71683,
        ra_deg: 219.484,
        dec_deg: -60.834,
        magnitude: 0.61,
        bv_color: 1.23,
        name: Some("Hadar"),
    },
    Star {
        hip: 37826,
        ra_deg: 116.109,
        dec_deg: 28.072,
        magnitude: 0.77,
        bv_color: 0.42,
        name: Some("Capella"),
    },
    Star {
        hip: 57632,
        ra_deg: 178.457,
        dec_deg: -1.943,
        magnitude: 0.85,
        bv_color: -0.23,
        name: Some("Procyon"),
    },
    Star {
        hip: 97649,
        ra_deg: 297.696,
        dec_deg: 8.868,
        magnitude: 0.97,
        bv_color: 1.63,
        name: Some("Altair"),
    },
    Star {
        hip: 65378,
        ra_deg: 200.981,
        dec_deg: -54.058,
        magnitude: 1.09,
        bv_color: 0.09,
        name: Some("Achernar"),
    },
    Star {
        hip: 45555,
        ra_deg: 138.299,
        dec_deg: -58.976,
        magnitude: 1.25,
        bv_color: 1.17,
        name: Some("Acrux"),
    },
    Star {
        hip: 68702,
        ra_deg: 211.097,
        dec_deg: 49.313,
        magnitude: 1.26,
        bv_color: 0.03,
        name: Some("Alkaid"),
    },
    Star {
        hip: 80783,
        ra_deg: 246.969,
        dec_deg: -26.432,
        magnitude: 1.33,
        bv_color: 1.59,
        name: Some("Antares"),
    },
    Star {
        hip: 62956,
        ra_deg: 194.294,
        dec_deg: -63.099,
        magnitude: 1.35,
        bv_color: 0.07,
        name: Some("Atria"),
    },
    Star {
        hip: 92855,
        ra_deg: 283.816,
        dec_deg: -37.104,
        magnitude: 1.50,
        bv_color: 0.42,
        name: Some("Nunki"),
    },
    Star {
        hip: 53409,
        ra_deg: 163.533,
        dec_deg: 55.860,
        magnitude: 1.58,
        bv_color: 0.80,
        name: Some("Alcaid"),
    },
    Star {
        hip: 74624,
        ra_deg: 228.071,
        dec_deg: -58.749,
        magnitude: 1.62,
        bv_color: 0.09,
        name: Some("Peacock"),
    },
    Star {
        hip: 66006,
        ra_deg: 203.841,
        dec_deg: -42.367,
        magnitude: 1.63,
        bv_color: 1.17,
        name: Some("Mimosa"),
    },
    Star {
        hip: 62434,
        ra_deg: 191.930,
        dec_deg: -59.689,
        magnitude: 1.68,
        bv_color: 0.07,
        name: Some("Acrux B"),
    },
    Star {
        hip: 54061,
        ra_deg: 166.259,
        dec_deg: 56.537,
        magnitude: 1.70,
        bv_color: 0.22,
        name: Some("Dubhe"),
    },
    Star {
        hip: 67301,
        ra_deg: 206.885,
        dec_deg: 49.313,
        magnitude: 1.77,
        bv_color: 0.03,
        name: Some("Mizar"),
    },
    Star {
        hip: 53910,
        ra_deg: 165.932,
        dec_deg: 61.751,
        magnitude: 1.79,
        bv_color: 1.07,
        name: Some("Polaris"),
    },
    Star {
        hip: 33579,
        ra_deg: 104.656,
        dec_deg: -27.935,
        magnitude: 1.80,
        bv_color: 0.42,
        name: Some("Alnilam"),
    },
    Star {
        hip: 25428,
        ra_deg: 80.627,
        dec_deg: -1.943,
        magnitude: 1.84,
        bv_color: 0.42,
        name: Some("Alnitab"),
    },
    Star {
        hip: 36850,
        ra_deg: 114.825,
        dec_deg: -40.003,
        magnitude: 1.85,
        bv_color: 0.18,
        name: Some("Mintaka"),
    },
    Star {
        hip: 97649,
        ra_deg: 297.696,
        dec_deg: 8.868,
        magnitude: 0.97,
        bv_color: 1.63,
        name: Some("Altair"),
    },
    Star {
        hip: 102098,
        ra_deg: 312.497,
        dec_deg: -7.789,
        magnitude: 1.74,
        bv_color: 0.09,
        name: Some("Sadr"),
    },
    Star {
        hip: 98036,
        ra_deg: 299.085,
        dec_deg: 29.580,
        magnitude: 1.81,
        bv_color: 0.42,
        name: Some("Deneb"),
    },
    Star {
        hip: 113368,
        ra_deg: 345.944,
        dec_deg: -52.696,
        magnitude: 1.74,
        bv_color: 0.09,
        name: Some("Suhail"),
    },
    Star {
        hip: 113963,
        ra_deg: 347.587,
        dec_deg: -42.998,
        magnitude: 1.83,
        bv_color: 1.45,
        name: Some("Wezen"),
    },
    Star {
        hip: 11767,
        ra_deg: 37.954,
        dec_deg: 89.264,
        magnitude: 1.98,
        bv_color: 0.60,
        name: Some("Polaris B"),
    },
    Star {
        hip: 21421,
        ra_deg: 68.980,
        dec_deg: 16.510,
        magnitude: 0.85,
        bv_color: 0.42,
        name: Some("Aldebaran"),
    },
    Star {
        hip: 24436,
        ra_deg: 75.244,
        dec_deg: -1.943,
        magnitude: 1.62,
        bv_color: -0.22,
        name: Some("Bellatrix"),
    },
    Star {
        hip: 25930,
        ra_deg: 81.573,
        dec_deg: 6.350,
        magnitude: 3.39,
        bv_color: 0.13,
        name: Some("Mintaka B"),
    },
    Star {
        hip: 72607,
        ra_deg: 222.720,
        dec_deg: -47.288,
        magnitude: 1.92,
        bv_color: 1.13,
        name: Some("Sargas"),
    },
    Star {
        hip: 74824,
        ra_deg: 228.539,
        dec_deg: -58.749,
        magnitude: 1.86,
        bv_color: 0.09,
        name: Some("Peacock B"),
    },
    Star {
        hip: 42913,
        ra_deg: 131.052,
        dec_deg: -54.708,
        magnitude: 1.86,
        bv_color: 0.42,
        name: Some("Avior"),
    },
    Star {
        hip: 49669,
        ra_deg: 151.830,
        dec_deg: 16.399,
        magnitude: 1.35,
        bv_color: 0.42,
        name: Some("Regulus"),
    },
    Star {
        hip: 62184,
        ra_deg: 191.279,
        dec_deg: -62.672,
        magnitude: 1.86,
        bv_color: 0.07,
        name: Some("Acrux C"),
    },
    Star {
        hip: 87833,
        ra_deg: 268.383,
        dec_deg: -34.074,
        magnitude: 1.92,
        bv_color: 1.17,
        name: Some("Kaus Australis"),
    },
    Star {
        hip: 92848,
        ra_deg: 283.274,
        dec_deg: -26.432,
        magnitude: 2.05,
        bv_color: 1.38,
        name: Some("Sargas B"),
    },
    Star {
        hip: 95947,
        ra_deg: 291.539,
        dec_deg: -63.099,
        magnitude: 1.91,
        bv_color: 0.42,
        name: Some("Atria B"),
    },
    Star {
        hip: 98002,
        ra_deg: 298.828,
        dec_deg: 22.721,
        magnitude: 2.23,
        bv_color: 0.42,
        name: Some("Sadr B"),
    },
    Star {
        hip: 104732,
        ra_deg: 318.234,
        dec_deg: -9.482,
        magnitude: 2.05,
        bv_color: 1.17,
        name: Some("Nunki B"),
    },
    Star {
        hip: 108386,
        ra_deg: 330.795,
        dec_deg: -0.299,
        magnitude: 2.74,
        bv_color: 0.42,
        name: Some("Sadalsuud"),
    },
    Star {
        hip: 109074,
        ra_deg: 333.820,
        dec_deg: 1.765,
        magnitude: 2.83,
        bv_color: 0.42,
        name: Some("Sadalmelik"),
    },
    Star {
        hip: 109139,
        ra_deg: 334.054,
        dec_deg: 18.154,
        magnitude: 3.27,
        bv_color: 0.42,
        name: Some("Enif"),
    },
    Star {
        hip: 111841,
        ra_deg: 341.373,
        dec_deg: -5.099,
        magnitude: 2.93,
        bv_color: 1.45,
        name: Some("Markab"),
    },
    Star {
        hip: 112740,
        ra_deg: 343.734,
        dec_deg: -1.431,
        magnitude: 3.53,
        bv_color: 0.42,
        name: Some("Algenib"),
    },
    Star {
        hip: 113883,
        ra_deg: 346.190,
        dec_deg: -3.432,
        magnitude: 2.49,
        bv_color: 1.53,
        name: Some("Diphda"),
    },
    Star {
        hip: 43209,
        ra_deg: 132.249,
        dec_deg: -9.482,
        magnitude: 3.53,
        bv_color: 0.42,
        name: Some("Zaurak"),
    },
    Star {
        hip: 44816,
        ra_deg: 136.999,
        dec_deg: -9.482,
        magnitude: 3.04,
        bv_color: 0.42,
        name: Some("Mirzam"),
    },
    Star {
        hip: 57651,
        ra_deg: 178.227,
        dec_deg: -3.432,
        magnitude: 2.90,
        bv_color: 0.42,
        name: Some("Alhena"),
    },
    Star {
        hip: 63125,
        ra_deg: 194.294,
        dec_deg: -63.099,
        magnitude: 2.81,
        bv_color: 0.07,
        name: Some("Atria C"),
    },
    Star {
        hip: 71957,
        ra_deg: 220.625,
        dec_deg: -47.288,
        magnitude: 2.39,
        bv_color: 0.42,
        name: Some("Sabik"),
    },
    Star {
        hip: 72622,
        ra_deg: 222.720,
        dec_deg: -47.288,
        magnitude: 2.43,
        bv_color: 0.42,
        name: Some("Sabik B"),
    },
    Star {
        hip: 76267,
        ra_deg: 233.672,
        dec_deg: -26.432,
        magnitude: 2.89,
        bv_color: 1.17,
        name: Some("Kaus Borealis"),
    },
    Star {
        hip: 79593,
        ra_deg: 243.454,
        dec_deg: -34.074,
        magnitude: 2.81,
        bv_color: 1.17,
        name: Some("Kaus Media"),
    },
    Star {
        hip: 82396,
        ra_deg: 251.498,
        dec_deg: -34.074,
        magnitude: 2.84,
        bv_color: 1.17,
        name: Some("Kaus Meridionalis"),
    },
    Star {
        hip: 85927,
        ra_deg: 262.688,
        dec_deg: -34.074,
        magnitude: 2.81,
        bv_color: 1.17,
        name: Some("Alnasl"),
    },
    Star {
        hip: 86670,
        ra_deg: 264.329,
        dec_deg: -34.385,
        magnitude: 3.32,
        bv_color: 1.17,
        name: Some("Albaldah"),
    },
];

pub fn bright_stars_mesh(radius: f64) -> (Vec<[f32; 3]>, Vec<[f32; 4]>) {
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    for star in BRIGHT_STARS {
        let pos = celestial_to_cartesian(star.ra_deg, star.dec_deg, radius);
        positions.push(pos);
        colors.push(bv_to_rgb(star.bv_color));
    }
    (positions, colors)
}

pub fn generate_synthetic_starfield(
    count: u32,
    radius: f64,
    seed: u64,
) -> (Vec<[f32; 3]>, Vec<[f32; 4]>) {
    let mut positions = Vec::with_capacity(count as usize);
    let mut colors = Vec::with_capacity(count as usize);
    let mut state = seed;
    for _ in 0..count {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let u = ((state >> 32) as f64) / (u32::MAX as f64);
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let v = ((state >> 32) as f64) / (u32::MAX as f64);
        let theta = 2.0 * std::f64::consts::PI * u;
        let phi = (2.0 * v - 1.0).acos();
        let x = radius * phi.sin() * theta.cos();
        let y = radius * phi.cos();
        let z = radius * phi.sin() * theta.sin();
        positions.push([x as f32, y as f32, z as f32]);
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let bv = (((state >> 32) as f32) / (u32::MAX as f32)) * 2.0 - 0.4;
        colors.push(bv_to_rgb(bv));
    }
    (positions, colors)
}
