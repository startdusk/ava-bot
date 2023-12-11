use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use axum_extra::extract::CookieJar;

use crate::COOKIE_NAME_DEVICE_ID;

#[derive(Debug, Clone)]
pub struct AppContext {
    pub(crate) device_id: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for AppContext
where
    S: Send + Sync,
{
    /// If the extractor fails it'll use this "rejection" type. A rejection is
    /// a kind of error that can be converted into a response.
    type Rejection = (StatusCode, &'static str);

    /// Perform the extraction.
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_request_parts(parts, state).await.unwrap();
        let Some(device_id) = jar.get(COOKIE_NAME_DEVICE_ID) else {
            return Err((StatusCode::BAD_REQUEST, "cookie `device_id` is missing"));
        };
        Ok(AppContext {
            device_id: device_id.value().to_string(),
        })
    }
}
