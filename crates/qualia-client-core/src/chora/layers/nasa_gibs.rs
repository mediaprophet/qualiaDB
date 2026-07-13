pub struct GibsRequest {
    pub layer: String,
    pub projection: String,
    pub width: u32,
    pub height: u32,
}

impl GibsRequest {
    pub fn url(&self) -> String {
        let bbox = if self.projection == "epsg4326" {
            "-90,-180,90,180"
        } else {
            "-20037508.34,-20037508.34,20037508.34,20037508.34"
        };
        let crs = if self.projection == "epsg4326" {
            "EPSG:4326"
        } else {
            "EPSG:3857"
        };
        format!(
            "https://gibs.earthdata.nasa.gov/wms/{}/best/wms.cgi?\
             SERVICE=WMS&REQUEST=GetMap&VERSION=1.3.0\
             &LAYERS={}&CRS={}&BBOX={}\
             &WIDTH={}&HEIGHT={}&FORMAT=image/jpeg\
             &STYLES=&TRANSPARENT=FALSE",
            self.projection, self.layer, crs, bbox, self.width, self.height
        )
    }
}

pub struct EarthTexture {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<[u8; 3]>,
}

impl EarthTexture {
    pub fn sample(&self, lat_deg: f32, lon_deg: f32) -> [f32; 3] {
        let lat = lat_deg.clamp(-90.0, 90.0);
        let lon = lon_deg.clamp(-180.0, 180.0);
        let v = ((90.0 - lat) / 180.0 * self.height as f32) as u32;
        let u = (((lon + 180.0) / 360.0) * self.width as f32) as u32;
        let v = v.min(self.height - 1);
        let u = u.min(self.width - 1);
        let idx = (v * self.width + u) as usize;
        let [r, g, b] = self.rgb.get(idx).copied().unwrap_or([0, 0, 80]);
        [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
    }

    pub fn sample_vertex(&self, x: f32, y: f32, z: f32) -> [f32; 3] {
        let lat = y.asin().to_degrees();
        let lon = z.atan2(x).to_degrees();
        self.sample(lat, lon)
    }
}

pub fn decode_jpeg_rgb(data: &[u8]) -> Result<EarthTexture, String> {
    let img = image::load_from_memory_with_format(data, image::ImageFormat::Jpeg)
        .map_err(|e| format!("JPEG decode: {e}"))?;
    let rgb_img = img.to_rgb8();
    let width = rgb_img.width();
    let height = rgb_img.height();
    let mut rgb = Vec::with_capacity((width * height) as usize);
    for pixel in rgb_img.pixels() {
        rgb.push([pixel[0], pixel[1], pixel[2]]);
    }
    Ok(EarthTexture { width, height, rgb })
}

pub async fn download_gibs_texture(req: &GibsRequest) -> Result<EarthTexture, String> {
    let url = req.url();
    let response = reqwest::get(&url)
        .await
        .map_err(|e| format!("GIBS request: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("GIBS returned {}", response.status()));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("GIBS body: {e}"))?;
    decode_jpeg_rgb(&bytes)
}
