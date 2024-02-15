// (c) Copyright 2019-2024 OLX
use libvips::ops;
use libvips::VipsApp;
use libvips::VipsImage;
use log::error;
use reqwest::Client;
use std::env;
use std::fmt;
use thiserror::Error;

lazy_static! {
    static ref VIPS_APP: VipsApp = VipsApp::new("e2e tests", true).expect("Can't initialize Vips");
}

#[derive(Error, Debug)]
pub enum ImageDownloadError {
    #[error("received error response `{0}` while attempting to download the image `{1}`")]
    InvalidResponseStatusObtained(u16, String),
    #[error("the request for downloading the image `{0}` has failed")]
    RequestHasFailed(String),
}

pub struct RequestParametersBuilder {
    image_address: String,
    format: Option<ImageFormat>,
    quality: Option<i32>,
    w: Option<i32>,
    h: Option<i32>,
    watermarks: Vec<Watermark>,
    r: Option<Rotation>,
}

pub struct Watermark {
    image_address: String,
    x: i32,
    y: i32,
    origin: WatermarkPosition,
    alpha: f64,
    size: i32,
}

pub enum WatermarkPosition {
    Center,
    Point,
}

pub enum Rotation {
    R90,
    R180,
    R270,
}

pub enum ImageFormat {
    Jpeg,
    Webp,
    Heic,
}

impl RequestParametersBuilder {
    pub fn new(image_address: &str) -> Self {
        RequestParametersBuilder {
            image_address: image_address.to_string(),
            format: None,
            quality: None,
            w: None,
            h: None,
            watermarks: Vec::new(),
            r: None,
        }
    }

    pub fn with_format(mut self, format: ImageFormat) -> Self {
        self.format = Some(format);
        self
    }

    pub fn with_quality(mut self, quality: i32) -> Self {
        self.quality = Some(quality);
        self
    }

    pub fn with_rotation(mut self, rotation: Rotation) -> Self {
        self.r = Some(rotation);
        self
    }

    pub fn with_size(mut self, width: i32, height: i32) -> Self {
        self.w = Some(width);
        self.h = Some(height);
        self
    }

    pub fn add_watermark(
        mut self,
        file: &str,
        size: i32,
        alpha: f64,
        x: i32,
        y: i32,
        pos: WatermarkPosition,
    ) -> Self {
        self.watermarks.push(Watermark {
            image_address: file.to_string(),
            x,
            y,
            origin: pos,
            alpha,
            size,
        });
        self
    }
}

pub fn assert_result(img: &[u8], image_address: &str) {
    let file_expected = format!("tests/results/{}", image_address);
    let img_result = VipsImage::new_from_buffer(img, "").expect("Unable to read image from dali");
    let img_expected =
        VipsImage::new_from_file(&file_expected).expect("Cannot load file from disk");
    let result = ops::relational(&img_result, &img_expected, ops::OperationRelational::Equal)
        .expect("Cannot compare images");
    let min = ops::min(&result).expect("Can't get min from image");

    println!("Image diff: {}", min);
    assert!(min == 0.0);
}

pub async fn make_request(params: RequestParametersBuilder) -> Result<Vec<u8>, ImageDownloadError> {
    let client = Client::default();

    let url = get_url(&params);
    println!("URL: {}", url);

    let response = client.get(url).send().await.map_err(|e| {
        let img_address = params.image_address.clone();
        error!(
            "failed to downlaod the image from the url '{}'. received error is {}",
            img_address, e
        );
        ImageDownloadError::RequestHasFailed(img_address)
    })?;
    println!("Response: {:?}", response);
    let status = response.status();
    if status.is_success() {
        let bytes = response.bytes().await.unwrap();
        Ok(bytes.to_vec())
    } else {
        Err(ImageDownloadError::InvalidResponseStatusObtained(
            status.as_u16(),
            params.image_address,
        ))
    }
}

fn get_url(params: &RequestParametersBuilder) -> String {
    let mut query_string = Vec::new();
    let image_address = format!(
        "http://{}/{}",
        env::var("HTTP_HOST").unwrap_or("localhost:9000".into()),
        params.image_address
    );
    query_string.push(format!("image_address={}", image_address));
    if let Some(format) = &params.format {
        query_string.push(format!("format={}", format));
    }
    if let Some(w) = params.w {
        query_string.push(format!("size[width]={}", w));
    }
    if let Some(h) = params.h {
        query_string.push(format!("size[height]={}", h));
    }
    if let Some(quality) = params.quality {
        query_string.push(format!("quality={}", quality));
    }
    if let Some(rotation) = &params.r {
        query_string.push(format!("rotation={}", rotation));
    }
    for (i, item) in params.watermarks.iter().enumerate() {
        let image_address = format!(
            "http://{}/{}",
            env::var("HTTP_HOST").unwrap_or("localhost:9000".into()),
            item.image_address
        );
        query_string.push(format!(
            "watermarks[{}][image_address]={}",
            i, image_address
        ));
        match item.origin {
            WatermarkPosition::Center => {
                query_string.push(format!(
                    "watermarks[{}][position][x][origin]={}",
                    i, "Center"
                ));
                query_string.push(format!(
                    "watermarks[{}][position][y][origin]={}",
                    i, "Center"
                ));
            }
            WatermarkPosition::Point => {
                if item.x < 0 {
                    query_string.push(format!(
                        "watermarks[{}][position][x][origin]={}",
                        i, "Right"
                    ));
                    query_string.push(format!(
                        "watermarks[{}][position][x][pos]={}",
                        i,
                        item.x.abs()
                    ));
                } else {
                    query_string.push(format!("watermarks[{}][position][x][origin]={}", i, "Left"));
                    query_string.push(format!("watermarks[{}][position][x][pos]={}", i, item.x));
                }
                if item.y < 0 {
                    query_string.push(format!(
                        "watermarks[{}][position][y][origin]={}",
                        i, "Bottom"
                    ));
                    query_string.push(format!(
                        "watermarks[{}][position][y][pos]={}",
                        i,
                        item.y.abs()
                    ));
                } else {
                    query_string.push(format!("watermarks[{}][position][y][origin]={}", i, "Top"));
                    query_string.push(format!("watermarks[{}][position][y][pos]={}", i, item.y));
                }
            }
        }
        query_string.push(format!("watermarks[{}][alpha]={}", i, item.alpha));
        query_string.push(format!("watermarks[{}][size]={}", i, item.size));
    }

    format!(
        "http://{}:8080/?{}",
        env::var("DALI_HOST").unwrap_or("localhost".into()),
        query_string.join("&")
    )
}

impl fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let as_str = match self {
            ImageFormat::Jpeg => "Jpeg",
            ImageFormat::Webp => "Webp",
            ImageFormat::Heic => "Heic",
        };
        write!(f, "{}", as_str)
    }
}

impl fmt::Display for Rotation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let as_str = match self {
            Rotation::R90 => "R90",
            Rotation::R180 => "R180",
            Rotation::R270 => "R270",
        };
        write!(f, "{}", as_str)
    }
}
