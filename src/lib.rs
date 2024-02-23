use std::ops::{Deref, DerefMut};

use async_trait::async_trait;
use axum_core::{
	extract::{FromRequest, Request},
	response::{IntoResponse, Response},
};
use bytes::Bytes;
use http::{header, HeaderValue, StatusCode};
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

pub struct Bson<T>(pub T);

#[derive(Debug, Error)]
pub enum BsonRejection {
	#[error("bytes read error: {}",.0)]
	BytesRead(#[from] axum_core::extract::rejection::BytesRejection),
	#[error("missing octet-stream content type")]
	MissingContentType,
	#[error("bson parse error: {}",.0)]
	BsonError(#[from] bson::de::Error),
}

impl IntoResponse for BsonRejection {
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

#[async_trait]
impl<S, T> FromRequest<S> for Bson<T>
where
	T: DeserializeOwned,
	S: Send + Sync
{
	type Rejection = BsonRejection;

	async fn from_request(req: Request, _s: &S) -> Result<Self, Self::Rejection> {
		if bson_content_type(&req) {
			let bytes = Bytes::from_request(req,_s).await?;
			match bson::from_slice(&bytes) {
				Ok(value) => Ok(Bson(value)),
				Err(err) => Err(err.into()),
			}
		} else {
			Err(BsonRejection::MissingContentType)
		}
	}
}

fn bson_content_type<B>(req: &Request<B>) -> bool {
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

impl<T> Deref for Bson<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
			&self.0
	}
}

impl<T> DerefMut for Bson<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
			&mut self.0
	}
}

impl<T> From<T> for Bson<T> {
	fn from(inner: T) -> Self {
			Self(inner)
	}
}

impl<T> IntoResponse for Bson<T>
where
	T: Serialize,
{
	fn into_response(self) -> Response {
		match bson::to_raw_document_buf(&self.0) {
			Ok(buf) => 
				(
					[(
						header::CONTENT_TYPE,
						HeaderValue::from_static(mime::APPLICATION_OCTET_STREAM.as_ref()),
					)],
					Bytes::from(buf.into_bytes())
				).into_response(),
			Err(err) => 
				(
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


/*
#[cfg(test)]
mod tests {
	use super::*;
	//use axum_core::{routing::post, test_helpers::*, Router};
	use serde::Deserialize;

	#[tokio::test]
	async fn deserialize_body() {
			#[derive(Debug, Deserialize)]
			struct Input {
					foo: String,
			}

			let app = Router::new().route("/", post(|input: Bson<Input>| async { input.0.foo }));

			let client = TestClient::new(app);
			let res = client.post("/").json(&json!({ "foo": "bar" })).send().await;
			let body = res.text().await;

			assert_eq!(body, "bar");
	}
}
*/
