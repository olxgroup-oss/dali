#[cfg(feature = "reqwest")]
pub mod client {
    use std::time::Duration;

    use async_trait::async_trait;
    use log::error;
    use reqwest::{Client, Url};

    use crate::commons::config::Configuration;
    use crate::image_provider::ImageProcessingError::{
        ClientReturnedErrorStatusCode, ImageDownloadFailed, ImageDownloadTimedOut,
        InvalidResourceUriProvided,
    };
    use crate::image_provider::ImageProvider;
    use crate::routes::image::ImageProcessingError;

    pub struct ReqwestImageProvider {
        client: Client,
    }

    impl ReqwestImageProvider {
        pub async fn new(config: &Configuration) -> ReqwestImageProvider {
            let reqwest_client = Client::builder()
                .timeout(Duration::from_millis(u64::from(
                    config.reqwest_timeout_millis.unwrap_or(2000),
                )))
                .connect_timeout(Duration::from_millis(u64::from(
                    config.reqwest_connection_timeout_millis.unwrap_or(2000),
                )))
                .pool_max_idle_per_host(usize::from(
                    config.reqwest_pool_max_idle_per_host.unwrap_or(10),
                ))
                .pool_idle_timeout(Duration::from_millis(u64::from(
                    config.reqwest_pool_idle_timeout_millis.unwrap_or(60000),
                )))
                .build();
            match reqwest_client {
                Ok(c) => ReqwestImageProvider { client: c },
                Err(e) => {
                    error!(
                        "failed to instantiate the 'reqwest' client with the error: {}",
                        e
                    );
                    panic!()
                }
            }
        }
    }

    #[async_trait]
    impl ImageProvider for ReqwestImageProvider {
        async fn get_file(&self, resource: &str) -> Result<Vec<u8>, ImageProcessingError> {
            let url = Url::parse(resource).map_err(|_| {
                error!(
                    "the provided resource uri is not a valid http url: '{}'",
                    resource
                );
                InvalidResourceUriProvided(String::from(resource))
            })?;
            let response = self.client.get(url).send().await.map_err(|e| {
                if e.is_timeout() {
                    error!(
                        "request for downloading the image '{}' timed out. error: {}",
                        resource, e
                    );
                    ImageDownloadTimedOut
                } else {
                    error!("error downloading the image: '{}'. error: {}", resource, e);
                    ImageDownloadFailed
                }
            })?;

            let status = response.status();
            if status.is_success() {
                let bytes = response.bytes().await.map_err(|e| {
                    error!(
                        "failed to read the binary payload of the image '{}'. error: {}",
                        resource, e
                    );
                    ImageDownloadFailed
                })?;
                Ok(bytes.to_vec())
            } else if status.is_client_error() {
                error!(
                    "the requested image '{}' couldn't be downloaded. received status code: {}",
                    resource, status
                );
                Err(ClientReturnedErrorStatusCode(
                    status.as_u16(),
                    String::from(resource),
                ))
            } else {
                error!(
                    "failed to download the specified resource. received status code: {}",
                    status.as_str()
                );
                Err(ImageDownloadFailed)
            }
        }
    }
}
