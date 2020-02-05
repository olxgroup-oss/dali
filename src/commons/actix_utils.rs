// (c) Copyright 2019-2020 OLX

use crate::commons::ProcessImageRequest;
use actix_web::dev;
use actix_web::{Error, FromRequest, HttpRequest};
use futures::future::{err, ready, Ready};

impl FromRequest for ProcessImageRequest {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        match req.app_data::<actix_web::web::Data<serde_qs::Config>>() {
            Some(qs_config) => ready(
                qs_config
                    .deserialize_str::<ProcessImageRequest>(req.query_string())
                    .map_err(actix_web::error::ErrorBadRequest),
            ),
            None => err(actix_web::error::ErrorInternalServerError(
                "Config not defined",
            )),
        }
    }
}
