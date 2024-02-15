// (c) Copyright 2019-2024 OLX

use crate::commons::*;
use libvips::ops;
use libvips::Result;
use libvips::VipsImage;
use log::*;

pub fn process_image(
    buffer: Vec<u8>,
    wm_buffers: Vec<Vec<u8>>,
    parameters: ProcessImageRequest,
) -> Result<Vec<u8>> {
    let ProcessImageRequest {
        image_address: _addr,
        size,
        format,
        quality,
        watermarks,
        rotation,
    } = parameters;
    let needs_rotation = rotation.is_some()
        || match rexif::parse_buffer_quiet(&buffer[..]).0 {
            Ok(data) => data.entries.into_iter().any(|e| {
                e.tag == rexif::ExifTag::Orientation
                    && e.value.to_i64(0).is_some()
                    && e.value.to_i64(0).unwrap() != 0
                    && e.value.to_i64(0).unwrap() != 1
            }),
            Err(_) => false,
        };
    let options = if !needs_rotation {
        "[access=VIPS_ACCESS_SEQUENTIAL]"
    } else {
        ""
    };
    let source = VipsImage::new_from_buffer(&buffer[..], options)?;

    let mut final_image = if needs_rotation {
        let exif_rotated = ops::autorot(&source)?;
        debug!("Rotating image to {:?}", rotation);
        if let Some(rotation) = rotation {
            let resized = resize_image(exif_rotated, &size)?;
            ops::rot(&resized, rotation.into())?
        } else {
            resize_image(exif_rotated, &size)?
        }
    } else {
        resize_image(source, &size)?
    };

    let image_width = final_image.get_width();
    let image_height = final_image.get_height();

    for (i, wm_buffer) in wm_buffers.iter().enumerate() {
        let watermark = &watermarks[i];
        debug!("Applying watermark: {:?}", watermark);
        let wm =
            VipsImage::new_from_buffer(&wm_buffer[..], "[access=VIPS_ACCESS_SEQUENTIAL]")?;

        let wm_width = wm.get_width();
        let wm_height = wm.get_height();

        let (wm_target_width, wm_target_height) = get_watermark_target_size(
            image_width,
            image_height,
            wm_width,
            wm_height,
            watermark.size,
        )?;

        let target_smaller = wm_width * wm_height > wm_target_width * wm_target_height;
        let wm = if target_smaller {
            ops::resize(&wm, f64::from(wm_target_width) / f64::from(wm_width))?
        } else {
            wm
        };

        let mut alpha = [1.0, 1.0, 1.0, watermark.alpha];
        let mut add = [0.0, 0.0, 0.0, 0.0];

        let wm = if !wm.image_hasalpha() {
            ops::bandjoin_const(&wm, &mut [255.0])?
        } else {
            wm
        };

        let wm = ops::linear(&wm, &mut alpha, &mut add)?;
        let (left, top, right, bottom) = get_watermark_borders(
            image_width,
            image_height,
            wm_target_width,
            wm_target_height,
            &watermark.position,
        );
        debug!(
            "Watermark position - Padding: top: {}, left: {}, bottom: {}, right: {}",
            top, left, bottom, right
        );
        let options = ops::Composite2Options {
            x: left,
            y: top,
            ..ops::Composite2Options::default()
        };
        let wm = if !target_smaller {
            ops::resize(&wm, f64::from(wm_target_width) / f64::from(wm_width))?
        } else {
            wm
        };
        final_image =
            ops::composite_2_with_opts(&final_image, &wm, ops::BlendMode::Over, &options)?;
    }

    debug!("Encoding to: {}", format);
    match format {
        ImageFormat::Jpeg => {
            let options = ops::JpegsaveBufferOptions {
                q: quality,
                background: vec![255.0],
                strip: true,
                optimize_coding: true,
                optimize_scans: true,
                interlace: true,
                ..ops::JpegsaveBufferOptions::default()
            };
            ops::jpegsave_buffer_with_opts(&final_image, &options)
        }
        ImageFormat::Webp => {
            let options = ops::WebpsaveBufferOptions {
                q: quality,
                strip: true,
                effort: 2,
                ..ops::WebpsaveBufferOptions::default()
            };
            ops::webpsave_buffer_with_opts(&final_image, &options)
        }
        ImageFormat::Png => {
            let options = ops::PngsaveBufferOptions {
                q: quality,
                strip: true,
                bitdepth: 8,
                ..ops::PngsaveBufferOptions::default()
            };
            ops::pngsave_buffer_with_opts(&final_image, &options)
        }
        ImageFormat::Heic => {
            let options = ops::HeifsaveBufferOptions {
                q: quality,
                strip: true,
                ..ops::HeifsaveBufferOptions::default()
            };
            ops::heifsave_buffer_with_opts(&final_image, &options)
        }
    }
}

fn resize_image(img: VipsImage, size: &Size) -> Result<VipsImage> {
    if size.height.is_none() && size.width.is_none() {
        return Ok(img);
    }
    debug!("Resizing image to {:?}", size);
    let original_width = img.get_width();
    let original_height = img.get_height();

    debug!(
        "Resizing image. Original size: {}x{}. Desired: {:?}",
        original_width, original_height, size
    );

    let (target_width, target_height) = get_target_size(original_width, original_height, &size)?;

    debug!("Final size: {}x{}", target_width, target_height);

    ops::resize(&img, f64::from(target_width) / f64::from(original_width))
}
