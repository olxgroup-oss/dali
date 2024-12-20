#[cfg(feature = "s3")]
pub mod s3 {
    use aws_config::default_provider::credentials::DefaultCredentialsChain;
    use aws_sdk_s3::config::Builder;
    use aws_sdk_s3::error::SdkError;
    use axum::http::StatusCode;
    use log::error;
    use std::collections::HashMap;
    use std::io::Write;
    use thiserror::Error;

    use async_trait::async_trait;
    use aws_config::{BehaviorVersion, Region};
    use aws_sdk_s3::error::ProvideErrorMetadata;

    use crate::commons::config::Configuration;
    use crate::image_provider::ImageResponse;
    use crate::image_provider::{
        ImageProcessingError::{
            self, ClientReturnedErrorStatusCode, ImageDownloadFailed, ImageDownloadTimedOut,
            InvalidResourceUriProvided, FileSizeExceeded,
        },
        ImageProvider,
    };

    #[derive(Error, Debug)]
    pub enum S3ProviderError {}

    pub struct S3ImageProvider {
        s3_client: aws_sdk_s3::Client,
        bucket: String,
    }

    impl S3ImageProvider {
        pub async fn new(configuration: &Configuration) -> S3ImageProvider {
            let Configuration {
                s3_region,
                s3_endpoint,
                s3_bucket,
                s3_key,
                s3_secret,
                ..
            } = configuration;

            if s3_region.is_none() {
                panic!("cannot instantiate the S3 client without having the region provided in the config files with the 's3_region' key");
            }

            if s3_bucket.is_none() {
                panic!("cannot instaitate the S3 client without having the bucket provided in the config files with the 's3_bucket' key")
            }

            let region = s3_region.as_ref().unwrap().clone();
            let mut s3_config = Builder::new()
                .behavior_version(BehaviorVersion::v2023_11_09())
                .region(Region::new(region));

            if let (Some(key), Some(secret)) = (s3_key, s3_secret) {
                // first we prioritze explicitely configure static credentials
                s3_config = s3_config.credentials_provider(aws_sdk_s3::config::Credentials::new(
                    key, secret, None, None, "",
                ));
            } else {
                // if no static credentials have been set, we rely on the default behaviour reccomended by AWS
                s3_config = s3_config
                    .credentials_provider(DefaultCredentialsChain::builder().build().await);
            }

            // only needed for the local development environemnt
            if let Some(endpoint) = s3_endpoint {
                s3_config = s3_config.endpoint_url(endpoint);
                s3_config = s3_config.force_path_style(true);
            }
            S3ImageProvider {
                s3_client: aws_sdk_s3::Client::from_conf(s3_config.build()),
                bucket: s3_bucket.as_ref().unwrap().clone(),
            }
        }
    }

    #[async_trait]
    impl ImageProvider for S3ImageProvider {
        async fn get_file(&self, resource: &str, config: &Configuration) -> Result<ImageResponse, ImageProcessingError> {
            if String::from(resource).is_empty() {
                error!("the provided resource uri is empty");
                return Err(InvalidResourceUriProvided(String::new()));
            }

            let mut result = self
                .s3_client
                .get_object()
                .bucket(self.bucket.clone())
                .key(resource)
                .send()
                .await
                .map_err(|e| {
                    error!(
                        "failed to download the file '{}' from s3. error: '{}'",
                        resource,
                        e.message().unwrap_or("no message available")
                    );
                    match e {
                        SdkError::TimeoutError(_) => {
                            error!("the s3 request has timed out for the file: '{}'", resource);
                            ImageDownloadTimedOut
                        }
                        SdkError::ServiceError(_) => match e.code() {
                            Some(err_code) if err_code == "NoSuchKey" => {
                                ClientReturnedErrorStatusCode(
                                    StatusCode::NOT_FOUND.as_u16(),
                                    String::from(resource),
                                )
                            }
                            Some(err_code) if err_code == "AccessDenied" => {
                                ClientReturnedErrorStatusCode(
                                    StatusCode::FORBIDDEN.as_u16(),
                                    String::from(resource),
                                )
                            }
                            // the next case is only needed for the local development environemnt
                            Some(err_code) if err_code == "XMinioInvalidObjectName" => {
                                error!("invalid S3 key was provided: '{}'", resource);
                                InvalidResourceUriProvided(String::from(resource))
                            }
                            _ => {
                                error!("image download has failed with the error: {}", e);
                                ImageDownloadFailed
                            }
                        },
                        _ => ImageDownloadFailed,
                    }
                })?;

            let headers = match result.metadata() {
                None => HashMap::new(),
                Some(metadata) => metadata
                    .into_iter()
                    .map(|(key, value)| {
                        let mut response_header_key = String::from("x-amz-meta-");
                        response_header_key.push_str(key);
                        (
                            response_header_key,
                            value.as_bytes().to_vec(),
                        )
                    })
                    .collect(),
            };
            let mut binary_payload: Vec<u8> = Vec::new();
            let mut total_bytes = 0;
            while let Some(bytes) = result.body.try_next().await.map_err(|e| {
                error!(
                    "failed to read the response for the file '{}'. error: '{}'",
                    resource, e
                );
                ImageDownloadFailed
            })? {
                if let Some(max_size) = config.max_file_size {
                    total_bytes += bytes.len() as u32;
                    if total_bytes > max_size {
                        error!(
                            "the downloaded image '{}' exceeds the maximum allowed size of {} bytes",
                            resource, max_size
                        );
                        return Err(FileSizeExceeded(max_size));
                    }
                }
                binary_payload.write_all(&bytes).map_err(|e| {
                    error!(
                        "failed to read the response for the file '{}'. error: '{}'",
                        resource, e
                    );
                    ImageDownloadFailed
                })?;
            }
            Ok(ImageResponse {
                bytes: binary_payload,
                response_headers: headers,
            })
        }
    }
}