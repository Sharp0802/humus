//!
//! A module that contains abstraction of stateless tokens
//!

use std::ops::Deref;
use chrono::{DateTime, Utc};
use cookie::CookieBuilder;
use headers::{Cookie, HeaderMapExt};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::header::SET_COOKIE;
use hyper::http::response::Builder;
use hyper::Request;
use lazy_static::lazy_static;
use rand::random;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use crate::encrypt::Aes;
use crate::error::Error;
use crate::response::ResponseBuilder;

lazy_static!{
    static ref CONFIG: RwLock<Option<TokenConfig>> = RwLock::new(None);
}

///
/// A struct that contains configuration for tokens
///
pub struct TokenConfig {
    key: String,
    secure_cookie: bool
}

impl TokenConfig {
    fn new(key: &str, secure_cookie: bool) -> Self {
        Self{
            key: key.to_string(),
            secure_cookie
        }
    }

    ///
    /// Override configuration by given configuration
    ///
    pub fn set(config: Self) {
        *CONFIG.blocking_write() = Some(config);
    }
}



#[derive(Serialize, Deserialize, Debug, Clone)]
struct Token {
    who: String,
    timestamp: i64,
    nonce: i64
}

///
/// A struct that represents stateless access-token
///
pub struct AccessToken {
    inner: Token
}

///
/// A struct that represents stateless refresh-token
///
pub struct RefreshToken {
    inner: Token
}

impl Token {
    fn new(who: &str, timestamp: i64) -> Token {
        Self {
            who: who.to_string(),
            timestamp,
            nonce: random()
        }
    }

    fn from(encrypted: &str) -> Result<Token, Error> {
        let config = CONFIG.blocking_read();
        let key = match config.deref() {
            None => return Err(Error::from("Token system not configured")),
            Some(config) => config.key.as_ref()
        };

        let decrypted = Aes::decrypt(encrypted, key)?;

        serde_json::from_str::<Token>(&decrypted).map_err(Error::from)
    }

    fn to_string(&self) -> Result<String, Error> {
        let config = CONFIG.blocking_read();
        let key = match config.deref() {
            None => return Err(Error::from("Token system not configured")),
            Some(config) => config.key.as_ref()
        };

        let json = serde_json::to_string(self).unwrap();

        Aes::encrypt(&json, key)
    }
}

impl AccessToken {
    fn new(who: &str, timestamp: i64) -> Self {
        Self {
            inner: Token::new(who, timestamp)
        }
    }

    fn from(encrypted: &str) -> Result<Self, Error> {
        Ok(Self {
            inner: Token::from(encrypted)?
        })
    }

    fn to_string(&self) -> Result<String, Error> {
        self.inner.to_string()
    }

    ///
    /// Get who has this token
    ///
    pub fn who(&self) -> &str {
        &self.inner.who
    }

    ///
    /// Get timestamp
    ///
    pub fn timestamp(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.inner.timestamp, 0).unwrap()
    }
}

impl RefreshToken {
    fn new(who: &str, timestamp: i64) -> Self {
        Self {
            inner: Token::new(who, timestamp)
        }
    }

    fn from(encrypted: &str) -> Result<Self, Error> {
        Ok(Self {
            inner: Token::from(encrypted)?
        })
    }

    fn to_string(&self) -> Result<String, Error> {
        self.inner.to_string()
    }

    ///
    /// Get who has this token
    ///
    pub fn who(&self) -> &str {
        &self.inner.who
    }

    ///
    /// Get timestamp
    ///
    pub fn timestamp(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.inner.timestamp, 0).unwrap()
    }
}


///
/// A struct that represents stateless session token
///
pub struct Session {
    access_token: AccessToken,
    refresh_token: RefreshToken
}

impl Session {
    fn read_cookie(key: &str, request: &Request<Full<Bytes>>) -> Option<String> {
        let cookie = request.headers().typed_get::<Cookie>()?;
        Some(cookie.get(key)?.to_string())
    }

    ///
    /// Create new session
    ///
    pub fn new(who: &str) -> Self {
        let timestamp = Utc::now().timestamp();

        Self {
            access_token: AccessToken::new(who, timestamp),
            refresh_token: RefreshToken::new(who, timestamp)
        }
    }

    ///
    /// Retrieve session information from request
    ///
    pub fn from_request(request: &Request<Full<Bytes>>) -> Result<Self, Error> {
        let access_token_str = Self::read_cookie("__HT_ACCESS_TOKEN", request)
            .ok_or(Error::from("Missing access token"))?;
        let mut access_token = AccessToken::from(&access_token_str)?;

        let refresh_token_str = Self::read_cookie("__HT_REFRESH_TOKEN", request)
            .ok_or(Error::from("Missing refresh token"))?;
        let mut refresh_token = RefreshToken::from(&refresh_token_str)?;

        let now = Utc::now();

        if access_token.who() != refresh_token.who() {
            return Err(Error::from("Token owner mismatched"))
        }

        // Access tokens are always generated after or at same time with Refresh token
        // If Refresh token's timestamp is later than Access token's one,
        // It may be client reuses refresh token after token refreshed
        if refresh_token.timestamp() > access_token.timestamp() {
            return Err(Error::from("Refresh token reused"))
        }

        if refresh_token.timestamp().signed_duration_since(now).num_days() > 90 {
            return Err(Error::from("Refresh token expired"))
        }

        if access_token.timestamp().signed_duration_since(now).num_minutes() > 15 {
            let timestamp = now.timestamp();
            access_token = AccessToken::new(access_token.who(), timestamp);
            refresh_token = RefreshToken::new(access_token.who(), timestamp);
        }

        Ok(Self {
            access_token,
            refresh_token
        })
    }

    ///
    /// Apply session information to response
    ///
    pub fn to_response(&self) -> Result<Builder, Error> {
        let secure = CONFIG.blocking_read().as_ref().unwrap().secure_cookie;

        Ok(ResponseBuilder::new()
            .header(SET_COOKIE, CookieBuilder::new("__HT_ACCESS_TOKEN", self.access_token.to_string()?)
                .http_only(true).secure(secure).to_string())
            .header(SET_COOKIE, CookieBuilder::new("__HT_REFRESH_TOKEN", self.refresh_token.to_string()?)
                .http_only(true).secure(secure).to_string()))
    }
}
