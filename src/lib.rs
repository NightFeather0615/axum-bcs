use std::ops::{Deref, DerefMut};

use axum_core::{
  extract::{FromRequest, Request},
  response::{IntoResponse, Response},
};
use bytes::Bytes;
use http::{HeaderValue, StatusCode, header};
use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;


pub struct Bcs<T>(pub T);

#[derive(Debug, Error)]
pub enum BcsRejection {
  #[error("Bytes read error: {}",.0)]
  BytesRead(#[from] axum_core::extract::rejection::BytesRejection),
  #[error("Missing octet-stream content type")]
  MissingContentType,
  #[error("BCS parse error: {}",.0)]
  BcsError(#[from] bcs::Error),
}

impl IntoResponse for BcsRejection {
  fn into_response(self) -> axum_core::response::Response {
    (
      StatusCode::BAD_REQUEST,
      [(
        header::CONTENT_TYPE,
        HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
      )],
      self.to_string(),
    ).into_response()
  }
}

impl<S, T> FromRequest<S> for Bcs<T>
where
  T: DeserializeOwned,
  S: Send + Sync,
{
  type Rejection = BcsRejection;

  async fn from_request(req: Request, _s: &S) -> Result<Self, Self::Rejection> {
    if bcs_content_type(&req) {
      let bytes = Bytes::from_request(req, _s).await?;
      match bcs::from_bytes(&bytes) {
        Ok(value) => Ok(Bcs(value)),
        Err(err) => Err(err.into()),
      }
    } else {
      Err(BcsRejection::MissingContentType)
    }
  }
}

fn bcs_content_type<B>(req: &Request<B>) -> bool {
  let content_type = if let Some(content_type) = req.headers().get(header::CONTENT_TYPE) {
    content_type
  } else {
    return false;
  };

  let content_type = if let Ok(content_type) = content_type.to_str() {
    content_type
  } else {
    return false;
  };

  let mime = if let Ok(mime) = content_type.parse::<mime::Mime>() {
    mime
  } else {
    return false;
  };

  let is_binary_content_type = mime.type_() == "application"
    && (mime.subtype() == "octet-stream"
      || mime.suffix().map_or(false, |name| name == "octet-stream"));

  is_binary_content_type
}

impl<T> Deref for Bcs<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T> DerefMut for Bcs<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl<T> From<T> for Bcs<T> {
  fn from(inner: T) -> Self {
    Self(inner)
  }
}

impl<T> IntoResponse for Bcs<T>
where
  T: Serialize,
{
  fn into_response(self) -> Response {
    match bcs::to_bytes(&self.0) {
      Ok(buf) => (
        [(
          header::CONTENT_TYPE,
          HeaderValue::from_static(mime::APPLICATION_OCTET_STREAM.as_ref()),
        )],
        Bytes::from(buf),
      ).into_response(),
      Err(err) => (
        StatusCode::INTERNAL_SERVER_ERROR,
        [(
          header::CONTENT_TYPE,
          HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
        )],
        err.to_string(),
      ).into_response(),
    }
  }
}
