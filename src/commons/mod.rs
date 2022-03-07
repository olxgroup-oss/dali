// (c) Copyright 2019-2020 OLX

pub mod actix_utils;
pub mod config;
pub mod errors;
pub mod http;

use errors::InvalidSizeError;
use libvips::ops::Angle;
use log::*;
use serde_derive::*;
use std::fmt;


pub fn timestamp_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis()
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProcessImageRequest {
    pub image_address: String,
    #[serde(default)]
    pub size: Size,
    #[serde(default)]
    pub format: ImageFormat,
    #[serde(default = "default_quality")]
    pub quality: i32,
    #[serde(default)]
    pub watermarks: Vec<Watermark>,
    #[serde(default)]
    pub rotation: Option<Rotation>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Watermark {
    pub image_address: String,
    #[serde(default)]
    pub position: Point,
    #[serde(default)]
    pub alpha: f64,
    #[serde(default = "default_watermark_size")]
    pub size: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Size {
    pub width: Option<i32>,
    pub height: Option<i32>,
}

#[derive(Debug, Deserialize, Clone)]
pub enum WatermarkPosition {
    Center,
    Point,
}

#[derive(Debug, Deserialize, Clone)]
pub enum Rotation {
    R90,
    R180,
    R270,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Point {
    x: HorizontalPosition,
    y: VerticalPosition,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "origin", content = "pos")]
pub enum HorizontalPosition {
    Left(i32),
    Right(i32),
    Center,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "origin", content = "pos")]
pub enum VerticalPosition {
    Top(i32),
    Bottom(i32),
    Center,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
    Heic,
}

fn default_quality() -> i32 {
    75
}

fn default_watermark_size() -> f64 {
    10.0
}

impl Into<Angle> for Rotation {
    fn into(self) -> Angle {
        // we want it inverted as we want it anti-clockwise
        match self {
            Rotation::R90 => Angle::D270,
            Rotation::R180 => Angle::D180,
            Rotation::R270 => Angle::D90,
        }
    }
}

impl fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let as_str = match self {
            ImageFormat::Jpeg => "jpeg",
            ImageFormat::Png => "png",
            ImageFormat::Webp => "webp",
            ImageFormat::Heic => "heic",
        };
        write!(f, "{}", as_str)
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "x: {}, y: {}", self.x, self.y)
    }
}

impl fmt::Display for HorizontalPosition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let as_str = match self {
            HorizontalPosition::Center => "Center".to_owned(),
            HorizontalPosition::Left(x) => format!("Left({})", x),
            HorizontalPosition::Right(x) => format!("Right({})", x),
        };
        write!(f, "{}", as_str)
    }
}

impl fmt::Display for VerticalPosition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let as_str = match self {
            VerticalPosition::Center => "Center".to_owned(),
            VerticalPosition::Top(x) => format!("Top({})", x),
            VerticalPosition::Bottom(x) => format!("Bottom({})", x),
        };
        write!(f, "{}", as_str)
    }
}

impl Default for Size {
    fn default() -> Self {
        Size {
            width: None,
            height: None,
        }
    }
}

impl Default for Point {
    fn default() -> Self {
        Point {
            x: HorizontalPosition::Left(0),
            y: VerticalPosition::Top(0),
        }
    }
}

impl Default for ImageFormat {
    fn default() -> Self {
        ImageFormat::Jpeg
    }
}

impl Default for WatermarkPosition {
    fn default() -> Self {
        WatermarkPosition::Point
    }
}

fn get_ratio(desired_measure: i32, original_measure: i32, opposite_orig_measure: i32) -> i32 {
    let ratio = desired_measure as f32 / original_measure as f32;
    (opposite_orig_measure as f32 * ratio) as i32
}

fn is_negative_or_zero(size: &Size) -> bool {
    (size.height.is_some() && size.height.unwrap() <= 0)
        || (size.width.is_some() && size.width.unwrap() <= 0)
}

pub fn get_target_size(
    original_width: i32,
    original_height: i32,
    desired_size: &Size,
) -> Result<(i32, i32), InvalidSizeError> {
    match &desired_size {
        Size {
            width: None,
            height: None,
        } => Ok((original_width, original_height)),
        s if is_negative_or_zero(s) => Err(InvalidSizeError::new(&desired_size)),
        Size {
            width: Some(w),
            height: Some(h),
        } if *h > original_height && *w > original_width => Ok((original_width, original_height)),
        Size {
            width: Some(w),
            height: Some(h),
        } => {
            let diff_height = *h as f32 / original_height as f32;
            let diff_width = *w as f32 / original_width as f32;

            if diff_height < diff_width && diff_height <= 1.0 {
                Ok((get_ratio(*h, original_height, original_width), *h))
            } else {
                Ok((*w, get_ratio(*w, original_width, original_height)))
            }
        }
        Size {
            width: None,
            height: Some(h),
        } => {
            if *h > original_height {
                Ok((original_width, original_height))
            } else {
                Ok((get_ratio(*h, original_height, original_width), *h))
            }
        }
        Size {
            width: Some(w),
            height: None,
        } => {
            if *w > original_width {
                Ok((original_width, original_height))
            } else {
                Ok((*w, get_ratio(*w, original_width, original_height)))
            }
        }
    }
}

pub fn get_watermark_target_size(
    image_width: i32,
    image_height: i32,
    wm_width: i32,
    wm_height: i32,
    percentage: f64,
) -> Result<(i32, i32), InvalidSizeError> {
    if percentage <= 0.0 || percentage > 100.0 {
        Err(InvalidSizeError::new(&Size::default()))
    } else {
        let desired_width = f64::from(image_width) * (percentage / 100.0);
        let desired_height = f64::from(image_height) * (percentage / 100.0);
        debug!(
            "Desired watermark size: {}x{}",
            desired_width, desired_height
        );
        if f64::from(wm_width) / desired_width >= f64::from(wm_height) / desired_height {
            Ok((
                desired_width as i32,
                (f64::from(wm_height) * desired_width / f64::from(wm_width)) as i32,
            ))
        } else {
            Ok((
                (f64::from(wm_width) * desired_height / f64::from(wm_height)) as i32,
                desired_height as i32,
            ))
        }
    }
}

pub fn get_watermark_borders(
    width: i32,
    height: i32,
    wm_width: i32,
    wm_height: i32,
    point: &Point,
) -> (i32, i32, i32, i32) {
    debug!(
        "Watermark parameters: original images {}x{}, watermark: {}x{}, params: {}",
        width, height, wm_width, wm_height, point
    );
    let (left, right) = match point.x {
        HorizontalPosition::Center => {
            let left = (width / 2) - (wm_width / 2);
            (left, left + (width % 2))
        }
        HorizontalPosition::Left(x) => {
            let right = width - x - wm_width;
            let left = x + if right < 0 { right } else { 0 };
            (left, if right > 0 { right } else { 0 })
        }
        HorizontalPosition::Right(x) => {
            let left = width - x - wm_width;
            let right = x + if left < 0 { left } else { 0 };
            (if left > 0 { left } else { 0 }, right)
        }
    };

    let (top, bottom) = match point.y {
        VerticalPosition::Center => {
            let top = (height / 2) - (wm_height / 2);
            (top, top + (height % 2))
        }
        VerticalPosition::Top(y) => {
            let bottom = height - y - wm_height;
            let top = y + if bottom < 0 { bottom } else { 0 };
            (top, if bottom > 0 { bottom } else { 0 })
        }
        VerticalPosition::Bottom(y) => {
            let top = height - y - wm_height;
            let bottom = y + if top < 0 { top } else { 0 };
            (if top > 0 { top } else { 0 }, bottom)
        }
    };
    (left, top, right, bottom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_size() {
        assert!(get_target_size(
            100,
            100,
            &Size {
                width: Some(-1),
                height: Some(-1)
            }
        )
        .is_err());
        assert!(get_target_size(
            100,
            100,
            &Size {
                width: Some(-1),
                height: Some(1)
            }
        )
        .is_err());
        assert!(get_target_size(
            100,
            100,
            &Size {
                width: Some(1),
                height: Some(-1)
            }
        )
        .is_err());
        assert!(get_target_size(
            100,
            100,
            &Size {
                width: None,
                height: Some(-1)
            }
        )
        .is_err());
        assert!(get_target_size(
            100,
            100,
            &Size {
                width: Some(-1),
                height: None
            }
        )
        .is_err());
    }

    #[test]
    fn test_size_square_img() {
        assert_eq!(
            get_target_size(
                100,
                100,
                &Size {
                    width: Some(100),
                    height: Some(100)
                }
            ),
            Ok((100, 100))
        );
        assert_eq!(
            get_target_size(
                100,
                100,
                &Size {
                    width: Some(10),
                    height: Some(10)
                }
            ),
            Ok((10, 10))
        );
        assert_eq!(
            get_target_size(
                100,
                100,
                &Size {
                    width: Some(10),
                    height: Some(20)
                }
            ),
            Ok((10, 10))
        );
        assert_eq!(
            get_target_size(
                100,
                100,
                &Size {
                    width: Some(20),
                    height: Some(10)
                }
            ),
            Ok((10, 10))
        );
        assert_eq!(
            get_target_size(
                100,
                100,
                &Size {
                    width: Some(100),
                    height: Some(50)
                }
            ),
            Ok((50, 50))
        );
        assert_eq!(
            get_target_size(
                100,
                100,
                &Size {
                    width: Some(50),
                    height: Some(100)
                }
            ),
            Ok((50, 50))
        );
        assert_eq!(
            get_target_size(
                100,
                100,
                &Size {
                    width: Some(120),
                    height: Some(100)
                }
            ),
            Ok((100, 100))
        );
        assert_eq!(
            get_target_size(
                100,
                100,
                &Size {
                    width: Some(100),
                    height: Some(120)
                }
            ),
            Ok((100, 100))
        );
    }

    #[test]
    fn test_size_rectangular_img() {
        assert_eq!(
            get_target_size(
                100,
                150,
                &Size {
                    width: Some(100),
                    height: Some(150)
                }
            ),
            Ok((100, 150))
        );
        assert_eq!(
            get_target_size(
                100,
                150,
                &Size {
                    width: Some(100),
                    height: Some(100)
                }
            ),
            Ok((66, 100))
        );
        assert_eq!(
            get_target_size(
                100,
                150,
                &Size {
                    width: Some(120),
                    height: Some(100)
                }
            ),
            Ok((66, 100))
        );
        assert_eq!(
            get_target_size(
                100,
                150,
                &Size {
                    width: Some(100),
                    height: Some(50)
                }
            ),
            Ok((33, 50))
        );
        assert_eq!(
            get_target_size(
                100,
                150,
                &Size {
                    width: Some(50),
                    height: Some(100)
                }
            ),
            Ok((50, 75))
        );
        assert_eq!(
            get_target_size(
                100,
                150,
                &Size {
                    width: Some(200),
                    height: Some(200)
                }
            ),
            Ok((100, 150))
        );
        assert_eq!(
            get_target_size(
                100,
                150,
                &Size {
                    width: Some(200),
                    height: Some(150)
                }
            ),
            Ok((100, 150))
        );
        assert_eq!(
            get_target_size(
                100,
                150,
                &Size {
                    width: Some(100),
                    height: Some(200)
                }
            ),
            Ok((100, 150))
        );
    }

    #[test]
    fn test_size_rectangular_img2() {
        assert_eq!(
            get_target_size(
                150,
                100,
                &Size {
                    width: Some(150),
                    height: Some(100)
                }
            ),
            Ok((150, 100))
        );
        assert_eq!(
            get_target_size(
                150,
                100,
                &Size {
                    width: Some(100),
                    height: Some(100)
                }
            ),
            Ok((100, 66))
        );
        assert_eq!(
            get_target_size(
                150,
                100,
                &Size {
                    width: Some(120),
                    height: Some(100)
                }
            ),
            Ok((120, 80))
        );
        assert_eq!(
            get_target_size(
                150,
                100,
                &Size {
                    width: Some(100),
                    height: Some(50)
                }
            ),
            Ok((75, 50))
        );
        assert_eq!(
            get_target_size(
                150,
                100,
                &Size {
                    width: Some(50),
                    height: Some(100)
                }
            ),
            Ok((50, 33))
        );
        assert_eq!(
            get_target_size(
                150,
                100,
                &Size {
                    width: Some(200),
                    height: Some(200)
                }
            ),
            Ok((150, 100))
        );
        assert_eq!(
            get_target_size(
                150,
                100,
                &Size {
                    width: Some(200),
                    height: Some(150)
                }
            ),
            Ok((150, 100))
        );
        assert_eq!(
            get_target_size(
                150,
                100,
                &Size {
                    width: Some(100),
                    height: Some(200)
                }
            ),
            Ok((100, 66))
        );
    }

    #[test]
    fn test_size_optional() {
        assert_eq!(
            get_target_size(
                100,
                100,
                &Size {
                    width: Some(100),
                    height: None
                }
            ),
            Ok((100, 100))
        );
        assert_eq!(
            get_target_size(
                100,
                100,
                &Size {
                    width: None,
                    height: Some(100)
                }
            ),
            Ok((100, 100))
        );
        assert_eq!(
            get_target_size(
                50,
                100,
                &Size {
                    width: Some(100),
                    height: None
                }
            ),
            Ok((50, 100))
        );
        assert_eq!(
            get_target_size(
                100,
                50,
                &Size {
                    width: None,
                    height: Some(100)
                }
            ),
            Ok((100, 50))
        );
        assert_eq!(
            get_target_size(
                150,
                100,
                &Size {
                    width: Some(100),
                    height: None
                }
            ),
            Ok((100, 66))
        );
        assert_eq!(
            get_target_size(
                100,
                150,
                &Size {
                    width: None,
                    height: Some(100)
                }
            ),
            Ok((66, 100))
        );
        assert_eq!(
            get_target_size(
                100,
                100,
                &Size {
                    width: None,
                    height: None
                }
            ),
            Ok((100, 100))
        );
    }

    #[test]
    fn test_center_watermark() {
        assert_eq!(
            get_watermark_borders(
                100,
                100,
                10,
                10,
                &Point {
                    x: HorizontalPosition::Center,
                    y: VerticalPosition::Center
                },
            ),
            (45, 45, 45, 45)
        );
        assert_eq!(
            get_watermark_borders(
                101,
                101,
                10,
                10,
                &Point {
                    x: HorizontalPosition::Center,
                    y: VerticalPosition::Center
                },
            ),
            (45, 45, 46, 46)
        );
    }

    #[test]
    fn test_left_top_watermark() {
        assert_eq!(
            get_watermark_borders(
                100,
                100,
                10,
                10,
                &Point {
                    x: HorizontalPosition::Left(10),
                    y: VerticalPosition::Top(10)
                }
            ),
            (10, 10, 80, 80)
        );
        assert_eq!(
            get_watermark_borders(
                100,
                100,
                10,
                10,
                &Point {
                    x: HorizontalPosition::Left(95),
                    y: VerticalPosition::Top(10)
                }
            ),
            (90, 10, 0, 80)
        );
        assert_eq!(
            get_watermark_borders(
                100,
                100,
                10,
                10,
                &Point {
                    x: HorizontalPosition::Left(10),
                    y: VerticalPosition::Top(95)
                }
            ),
            (10, 90, 80, 0)
        );
        assert_eq!(
            get_watermark_borders(
                100,
                100,
                10,
                10,
                &Point {
                    x: HorizontalPosition::Left(95),
                    y: VerticalPosition::Top(95)
                }
            ),
            (90, 90, 0, 0)
        );
    }

    #[test]
    fn test_right_bottom_watermark() {
        assert_eq!(
            get_watermark_borders(
                100,
                100,
                10,
                10,
                &Point {
                    x: HorizontalPosition::Right(10),
                    y: VerticalPosition::Bottom(10)
                }
            ),
            (80, 80, 10, 10)
        );
        assert_eq!(
            get_watermark_borders(
                100,
                100,
                10,
                10,
                &Point {
                    x: HorizontalPosition::Right(95),
                    y: VerticalPosition::Bottom(10)
                }
            ),
            (0, 80, 90, 10)
        );
        assert_eq!(
            get_watermark_borders(
                100,
                100,
                10,
                10,
                &Point {
                    x: HorizontalPosition::Right(10),
                    y: VerticalPosition::Bottom(95)
                }
            ),
            (80, 0, 10, 90)
        );
        assert_eq!(
            get_watermark_borders(
                100,
                100,
                10,
                10,
                &Point {
                    x: HorizontalPosition::Right(95),
                    y: VerticalPosition::Bottom(95)
                }
            ),
            (0, 0, 90, 90)
        );
    }
}
