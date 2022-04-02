// (c) Copyright 2019-2020 OLX

#[macro_use]
extern crate lazy_static;
mod utils;

#[test]
fn test_get_simple() {
    let result = utils::make_request(utils::RequestParametersBuilder::new("img-test"))
        .expect("Unable to download file");
    utils::assert_result(&result[..], "raw.jpg");
}

#[test]
fn test_get_rotated() {
    let result = utils::make_request(
        utils::RequestParametersBuilder::new("img-test").with_rotation(utils::Rotation::R270),
    )
    .expect("Unable to download file");
    utils::assert_result(&result[..], "raw_rotated.jpg");
}

#[test]
fn test_get_resized() {
    let result =
        utils::make_request(utils::RequestParametersBuilder::new("img-test").with_size(100, 100))
            .expect("Unable to download file");
    utils::assert_result(&result[..], "resized.jpg");
}

#[test]
fn test_get_rounded_default() {
    let result = utils::make_request(
        utils::RequestParametersBuilder::new("img-test")
            .with_format(utils::ImageFormat::Png)
            .with_round(utils::RoundRect::default()),
    )
    .expect("Unable to download file");
    utils::assert_result_bytes(&result[..], "rounded_default.png");
}

#[test]
fn test_get_rounded_custom() {
    let result = utils::make_request(
        utils::RequestParametersBuilder::new("img-test")
            .with_format(utils::ImageFormat::Png)
            .with_round(utils::RoundRect::Custom {
                x: None,
                y: None,
                width: None,
                height: None,
                rx: Some(50),
                ry: Some(50),
            }),
    )
    .expect("Unable to download file");
    utils::assert_result_bytes(&result[..], "rounded_50.png");
}

#[test]
fn test_get_watermarked_left() {
    let result = utils::make_request(
        utils::RequestParametersBuilder::new("img-test").add_watermark(
            "watermark",
            40,
            0.5f64,
            10,
            10,
            utils::WatermarkPosition::Point,
        ),
    )
    .expect("Unable to download file");
    utils::assert_result(&result[..], "watermarked_left.jpg");
}

#[test]
fn test_get_watermarked_right() {
    let result = utils::make_request(
        utils::RequestParametersBuilder::new("img-test").add_watermark(
            "watermark",
            40,
            0.5f64,
            -10,
            -10,
            utils::WatermarkPosition::Point,
        ),
    )
    .expect("Unable to download file");
    utils::assert_result(&result[..], "watermarked_right.jpg");
}

#[test]
fn test_get_watermarked_center() {
    let result = utils::make_request(
        utils::RequestParametersBuilder::new("img-test").add_watermark(
            "watermark",
            40,
            0.5f64,
            10,
            10,
            utils::WatermarkPosition::Center,
        ),
    )
    .expect("Unable to download file");
    utils::assert_result(&result[..], "watermarked_center.jpg");
}

#[test]
fn test_get_watermarked_rotated() {
    let result = utils::make_request(
        utils::RequestParametersBuilder::new("img-test")
            .add_watermark(
                "watermark",
                40,
                0.5f64,
                10,
                10,
                utils::WatermarkPosition::Center,
            )
            .with_rotation(utils::Rotation::R90),
    )
    .expect("Unable to download file");
    utils::assert_result(&result[..], "rotated_watermarked.jpg");
}

#[test]
fn test_get_encoded_webp() {
    let result = utils::make_request(
        utils::RequestParametersBuilder::new("img-test").with_format(utils::ImageFormat::Webp),
    )
    .expect("Unable to download file");
    utils::assert_result(&result[..], "raw.webp");
}

#[test]
fn test_get_encoded_heic() {
    let result = utils::make_request(
        utils::RequestParametersBuilder::new("img-test").with_format(utils::ImageFormat::Heic),
    )
    .expect("Unable to download file");
    utils::assert_result(&result[..], "raw.heic");
}

#[test]
fn test_get_encoded_webp_bad_quality() {
    let result = utils::make_request(
        utils::RequestParametersBuilder::new("img-test")
            .with_format(utils::ImageFormat::Webp)
            .with_quality(10),
    )
    .expect("Unable to download file");
    utils::assert_result(&result[..], "raw_bad_quality.webp");
}

#[test]
fn test_get_raw_bad_quality() {
    let result = utils::make_request(
        utils::RequestParametersBuilder::new("img-test")
            .with_format(utils::ImageFormat::Jpeg)
            .with_quality(10),
    )
    .expect("Unable to download file");
    utils::assert_result(&result[..], "raw_bad_quality.jpg");
}

#[test]
fn test_get_multiple_watermarks() {
    let result = utils::make_request(
        utils::RequestParametersBuilder::new("img-test")
            .add_watermark(
                "watermark",
                20,
                0.3f64,
                -10,
                -10,
                utils::WatermarkPosition::Point,
            )
            .add_watermark(
                "watermark",
                20,
                0.3f64,
                10,
                10,
                utils::WatermarkPosition::Center,
            )
            .add_watermark(
                "watermark",
                20,
                0.3f64,
                10,
                10,
                utils::WatermarkPosition::Point,
            ),
    )
    .expect("Unable to download file");
    utils::assert_result(&result[..], "multiple_watermarks.jpg");
}

#[test]
fn test_get_watermark_no_alpha() {
    let result = utils::make_request(
        utils::RequestParametersBuilder::new("img-test")
            .add_watermark(
                "watermark",
                20,
                0.3f64,
                -10,
                -10,
                utils::WatermarkPosition::Point,
            )
            .add_watermark("lena", 20, 0.3f64, 10, 10, utils::WatermarkPosition::Center)
            .add_watermark("lena", 20, 0.3f64, 10, 10, utils::WatermarkPosition::Point),
    )
    .expect("Unable to download file");
    utils::assert_result(&result[..], "watermarks_no_alpha.jpg");
}

#[test]
fn test_get_exif_watermark() {
    let result = utils::make_request(
        utils::RequestParametersBuilder::new("exif")
            .add_watermark(
                "watermark",
                20,
                0.3f64,
                -10,
                -10,
                utils::WatermarkPosition::Point,
            )
            .add_watermark("lena", 20, 0.3f64, 10, 10, utils::WatermarkPosition::Center)
            .add_watermark("exif", 20, 0.3f64, 10, 10, utils::WatermarkPosition::Point),
    )
    .expect("Unable to download file");
    utils::assert_result(&result[..], "exif_watermark.jpg");
}

#[test]
fn test_get_all_features() {
    let result = utils::make_request(
        utils::RequestParametersBuilder::new("img-test")
            .with_format(utils::ImageFormat::Webp)
            .with_quality(50)
            .with_rotation(utils::Rotation::R180)
            .add_watermark(
                "watermark",
                33,
                0.3f64,
                -10,
                -10,
                utils::WatermarkPosition::Point,
            )
            .add_watermark(
                "watermark",
                33,
                0.3f64,
                10,
                10,
                utils::WatermarkPosition::Point,
            )
            .with_size(150, 150),
    )
    .expect("Unable to download file");
    utils::assert_result(&result[..], "all_features.webp");
}
