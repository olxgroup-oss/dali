use std::collections::HashMap;

use async_trait::async_trait;

#[cfg(feature = "reqwest")]
use crate::image_provider::reqwest::client::ReqwestImageProvider;
use crate::{commons::config::Configuration, routes::image::ImageProcessingError};

#[cfg(feature = "s3")]
use self::s3::s3::S3ImageProvider;

pub mod reqwest;
pub mod s3;

#[cfg(not(any(feature = "reqwest", feature = "s3")))]
compile_error!("only 's3' is available as an extra feature for the image storage service");

pub struct ImageResponse {
    pub bytes: Vec<u8>,
    pub response_headers: HashMap<String, Vec<u8>>,
}

#[async_trait]
pub trait ImageProvider: Send + Sync {
    async fn get_file(&self, resource: &str, config: &Configuration) -> Result<ImageResponse, ImageProcessingError>;
}

#[allow(unreachable_code)]
pub async fn create_image_provider(config: &Configuration) -> Box<dyn ImageProvider> {
    #[cfg(feature = "s3")]
    {
        return Box::new(S3ImageProvider::new(config).await);
    }

    #[cfg(feature = "reqwest")]
    {
        return Box::new(ReqwestImageProvider::new(config).await);
    }
}
