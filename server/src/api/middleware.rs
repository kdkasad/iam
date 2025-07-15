//! # Custom HTTP middleware

use axum::http::{HeaderValue, header::CACHE_CONTROL};
use chrono::Duration;
use tower_http::set_header::SetResponseHeaderLayer;

/// Publicity value used in the [`CacheControlLayer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Publicity {
    Public,
    Private,
}

impl Publicity {
    pub fn to_str(self) -> &'static str {
        match self {
            Publicity::Public => "public",
            Publicity::Private => "private",
        }
    }
}

/// # `Cache-Control` middleware layer
///
/// This layer sets the `Cache-Control` HTTP header on responses which do not already contain
/// a value for that header.
///
/// # Examples
///
/// ```ignore
/// # // FIXME: see if we can doctest private items
/// # use iam_server::api::middleware::{CacheControlLayer, Publicity};
/// # use chrono::Duration;
/// CacheControlLayer::new()
///     .max_age(Duration::days(1))
///     .publicity(Publicity::Public)
///     .finish()
/// ```
/// The above layer will add the following header to responses:
/// ```text
/// Cache-Control: public, max-age=86400
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CacheControlLayer {
    max_age: Option<Duration>,
    publicity: Option<Publicity>,
    no_store: bool,
}

impl CacheControlLayer {
    /// Constructs a new [`CacheControlLayer`] builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the `max-age` directive to the given duration, truncated to second precision.
    pub fn max_age(mut self, value: Duration) -> Self {
        self.max_age = Some(value);
        self
    }

    /// Sets a publicity directive (i.e. `public`/`private`).
    pub fn publicity(mut self, value: Publicity) -> Self {
        self.publicity = Some(value);
        self
    }

    /// Sets the `no-store` directive.
    ///
    /// Setting this to `true` clears `max_age` and `publicity`.
    pub fn no_store(mut self, value: bool) -> Self {
        if value {
            self.max_age = None;
            self.publicity = None;
        }
        self.no_store = value;
        self
    }

    /// Finishes the builder, returning a [`SetResponseHeaderLayer`] which adds the proper `Cache-Control` header.
    ///
    /// # Panics
    ///
    /// Panics if the intended value of the `Cache-Control` header is not a valid [`HeaderValue`].
    pub fn finish(self) -> SetResponseHeaderLayer<HeaderValue> {
        let mut value = String::new();
        if self.no_store {
            value.push_str("no-store");
        } else {
            if let Some(p) = self.publicity {
                value.push_str(p.to_str());
            }
            if let Some(ma) = self.max_age {
                value.push_str(", max-age=");
                value.push_str(&ma.num_seconds().to_string());
            }
        }
        SetResponseHeaderLayer::if_not_present(
            CACHE_CONTROL,
            HeaderValue::from_str(&value).expect("expected header value to be valid"),
        )
    }
}

impl From<CacheControlLayer> for SetResponseHeaderLayer<HeaderValue> {
    fn from(value: CacheControlLayer) -> Self {
        value.finish()
    }
}
