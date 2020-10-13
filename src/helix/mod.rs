//! Helix endpoints or the [New Twitch API](https://dev.twitch.tv/docs/api)
use serde::{Deserialize, Serialize};
use std::{convert::TryInto, str::FromStr};
use twitch_oauth2::TwitchToken;

pub mod channels;
pub mod clips;
pub mod games;
pub mod moderation;
pub mod streams;
pub mod subscriptions;
pub mod tags;
pub mod users;

pub(crate) mod ser;
pub use ser::Error;

#[doc(no_inline)]
pub use twitch_oauth2::Scope;

/// Client for Helix or the [New Twitch API](https://dev.twitch.tv/docs/api)
///
/// Provides [HelixClient::req_get] for requesting endpoints which uses [GET method][RequestGet].
///
///
/// Most [clients][crate::HttpClient] will be able to use the `'static` lifetime
///
/// ```rust,no_run
/// # use twitch_api2::{HelixClient}; pub mod reqwest {pub type Client = twitch_api2::client::DummyHttpClient;}
/// pub struct MyStruct {
///     twitch: HelixClient<'static, reqwest::Client>,
///     token: twitch_oauth2::AppAccessToken,
/// }
/// // etc
/// ```
///
/// See [HttpClient][crate::HttpClient] for implemented http clients, you can also define your own if needed.
#[cfg(all(feature = "client"))]
#[cfg_attr(nightly, doc(all(feature = "helix", feature = "client")))]
#[derive(Clone)]
pub struct HelixClient<'a, C>
where C: crate::HttpClient<'a> {
    client: C,
    _pd: std::marker::PhantomData<&'a ()>, // TODO: Implement rate limiter...
}

#[derive(PartialEq, Deserialize, Debug)]
struct InnerResponse<D> {
    data: Vec<D>,
    /// A cursor value, to be used in a subsequent request to specify the starting point of the next set of results.
    #[serde(default)]
    pagination: Pagination,
}
#[derive(Deserialize, Clone, Debug)]
struct HelixRequestError {
    error: String,
    status: u16,
    message: String,
}

#[cfg(feature = "client")]
impl<'a, C: crate::HttpClient<'a>> HelixClient<'a, C> {
    /// Create a new client with an existing client
    pub fn with_client(client: C) -> HelixClient<'a, C> {
        HelixClient {
            client,
            _pd: std::marker::PhantomData::default(),
        }
    }

    /// Create a new [HelixClient] with a default [HttpClient][crate::HttpClient]
    pub fn new() -> HelixClient<'a, C>
    where C: Default {
        let client = C::default();
        HelixClient::with_client(client)
    }

    /// Retrieve a clone of the [HttpClient][crate::HttpClient] inside this [HelixClient]
    pub fn clone_client(&self) -> C
    where C: Clone {
        self.client.clone()
    }

    /// Request on a valid [RequestGet] endpoint
    ///
    /// ```rust,no_run
    /// # #[tokio::main]
    /// # async fn main() {
    /// #   use twitch_api2::helix::{HelixClient, channels};
    /// #   let token = Box::new(twitch_oauth2::UserToken::from_existing_unchecked(
    /// #       twitch_oauth2::AccessToken::new("totallyvalidtoken".to_string()), None,
    /// #       twitch_oauth2::ClientId::new("validclientid".to_string()), None, None));
    ///     let req = channels::GetChannelInformationRequest::builder().broadcaster_id("123456").build();
    ///     let client = HelixClient::new();
    /// # let _: &HelixClient<twitch_api2::DummyHttpClient> = &client;
    ///
    ///     let response = client.req_get(req, &token).await;
    /// # }
    /// # // fn main() {run()}
    /// ```
    pub async fn req_get<R, D, T>(
        &'a self,
        request: R,
        token: &T,
    ) -> Result<Response<R, D>, ClientRequestError<<C as crate::HttpClient<'a>>::Error>>
    where
        R: Request<Response = D> + Request + RequestGet,
        D: serde::de::DeserializeOwned,
        T: TwitchToken + ?Sized,
    {
        let req = request.create_request(token.token().secret(), token.client_id().as_str())?;
        let uri = req.uri().clone();
        let response = self
            .client
            .req(req)
            .await
            .map_err(ClientRequestError::RequestError)?;
        request.parse_response(&uri, response).map_err(Into::into)
    }

    /// Request on a valid [RequestPost] endpoint
    pub async fn req_post<R, B, D, T>(
        &'a self,
        request: R,
        body: B,
        token: &T,
    ) -> Result<Response<R, D>, ClientRequestError<<C as crate::HttpClient<'a>>::Error>>
    where
        R: Request<Response = D> + Request + RequestPost<Body = B>,
        B: serde::Serialize,
        D: serde::de::DeserializeOwned,
        T: TwitchToken + ?Sized,
    {
        let req =
            request.create_request(body, token.token().secret(), token.client_id().as_str())?;
        let uri = req.uri().clone();
        let response = self
            .client
            .req(req)
            .await
            .map_err(ClientRequestError::RequestError)?;
        request.parse_response(&uri, response).map_err(Into::into)
    }

    /// Request on a valid [RequestPatch] endpoint
    pub async fn req_patch<R, B, D, T>(
        &'a self,
        request: R,
        body: B,
        token: &T,
    ) -> Result<D, ClientRequestError<<C as crate::HttpClient<'a>>::Error>>
    where
        R: Request<Response = D> + Request + RequestPatch<Body = B>,
        B: serde::Serialize,
        D: std::convert::TryFrom<http::StatusCode, Error = std::borrow::Cow<'static, str>>
            + serde::de::DeserializeOwned,
        T: TwitchToken + ?Sized,
    {
        let req =
            request.create_request(body, token.token().secret(), token.client_id().as_str())?;
        let uri = req.uri().clone();
        let response = self
            .client
            .req(req)
            .await
            .map_err(ClientRequestError::RequestError)?;
        request.parse_response(&uri, response).map_err(Into::into)
    }

    /// Request on a valid [RequestDelete] endpoint
    pub async fn req_delete<R, D, T>(
        &'a self,
        request: R,
        token: &T,
    ) -> Result<D, ClientRequestError<<C as crate::HttpClient<'a>>::Error>>
    where
        R: Request<Response = D> + Request + RequestDelete,
        D: std::convert::TryFrom<http::StatusCode, Error = std::borrow::Cow<'static, str>>
            + serde::de::DeserializeOwned,
        T: TwitchToken + ?Sized,
    {
        let req = request.create_request(token.token().secret(), token.client_id().as_str())?;
        let uri = req.uri().clone();
        let response = self
            .client
            .req(req)
            .await
            .map_err(ClientRequestError::RequestError)?;
        request.parse_response(&uri, response).map_err(Into::into)
    }
}

#[cfg(feature = "client")]
impl<'a, C> Default for HelixClient<'a, C>
where C: crate::HttpClient<'a> + Default
{
    fn default() -> HelixClient<'a, C> { HelixClient::new() }
}

/// A request is a Twitch endpoint, see [New Twitch API](https://dev.twitch.tv/docs/api/reference) reference
#[async_trait::async_trait]
pub trait Request: serde::Serialize {
    /// The path to the endpoint relative to the helix root. eg. `channels` for [Get Channel Information](https://dev.twitch.tv/docs/api/reference#get-channel-information)
    const PATH: &'static str;
    /// Scopes needed by this endpoint
    const SCOPE: &'static [twitch_oauth2::Scope];
    /// Optional scopes needed by this endpoint
    const OPT_SCOPE: &'static [twitch_oauth2::Scope] = &[];
    /// Response type. twitch's response will  deserialize to this.
    type Response: serde::de::DeserializeOwned;
    /// Defines layout of the url parameters.
    fn query(&self) -> Result<String, ser::Error> { ser::to_string(&self) }
}

/// Helix endpoint POSTs information
pub trait RequestPost: Request {
    /// Body parameters
    type Body: serde::Serialize;

    /// Create body text from [RequestPost::Body]
    fn body(&self, body: &Self::Body) -> Result<String, serde_json::Error> {
        serde_json::to_string(body)
    }

    /// Get Request to use in your client
    fn create_request(
        &self,
        body: Self::Body,
        token: &str,
        client_id: &str,
    ) -> Result<http::Request<Vec<u8>>, CreateRequestError>
    {
        let uri = http::Uri::from_str(&format!(
            "{}{}?{}",
            crate::TWITCH_HELIX_URL,
            <Self as Request>::PATH,
            self.query()?
        ))?;

        let body = self.body(&body)?;
        // eprintln!("\n\nbody is ------------ {} ------------", body);

        let mut bearer =
            http::HeaderValue::from_str(&format!("Bearer {}", token)).map_err(|_| {
                CreateRequestError::Custom("Could not make token into headervalue".into())
            })?;
        bearer.set_sensitive(true);
        http::Request::builder()
            .method(http::Method::POST)
            .uri(uri)
            .header("Client-ID", client_id)
            .header("Content-Type", "application/json")
            .header(http::header::AUTHORIZATION, bearer)
            .body(body.into_bytes())
            .map_err(Into::into)
    }

    /// Parse response. Override for different behavior
    fn parse_response(
        self,
        uri: &http::Uri,
        response: http::Response<Vec<u8>>,
    ) -> Result<Response<Self, <Self as Request>::Response>, HelixRequestPostError>
    where
        Self: Sized,
    {
        let text = std::str::from_utf8(&response.body())
            .map_err(|e| HelixRequestPostError::Utf8Error(response.body().clone(), e))?;
        if let Ok(HelixRequestError {
            error,
            status,
            message,
        }) = serde_json::from_str::<HelixRequestError>(&text)
        {
            return Err(HelixRequestPostError::Error {
                error,
                status: status.try_into().unwrap_or(http::StatusCode::BAD_REQUEST),
                message,
                uri: uri.clone(),
                body: response.body().clone(),
            });
        }
        let response: InnerResponse<<Self as Request>::Response> = serde_json::from_str(&text)?;
        Ok(Response {
            data: response.data,
            pagination: response.pagination,
            request: self,
        })
    }
}

/// Helix endpoint PATCHs information
pub trait RequestPatch: Request {
    /// Body parameters
    type Body: serde::Serialize;

    /// Create body text from [RequestPost::Body]
    fn body(&self, body: &Self::Body) -> Result<String, serde_json::Error> {
        serde_json::to_string(body)
    }

    /// Get Request to use in your client
    fn create_request(
        &self,
        body: Self::Body,
        token: &str,
        client_id: &str,
    ) -> Result<http::Request<Vec<u8>>, CreateRequestError>
    {
        let uri = http::Uri::from_str(&format!(
            "{}{}?{}",
            crate::TWITCH_HELIX_URL,
            <Self as Request>::PATH,
            self.query()?
        ))?;

        let body = self.body(&body)?;
        // eprintln!("\n\nbody is ------------ {} ------------", body);

        let mut bearer =
            http::HeaderValue::from_str(&format!("Bearer {}", token)).map_err(|_| {
                CreateRequestError::Custom("Could not make token into headervalue".into())
            })?;
        bearer.set_sensitive(true);
        http::Request::builder()
            .method(http::Method::PATCH)
            .uri(uri)
            .header("Client-ID", client_id)
            .header("Content-Type", "application/json")
            .header(http::header::AUTHORIZATION, bearer)
            .body(body.into_bytes())
            .map_err(Into::into)
    }

    /// Parse response. Override for different behavior
    fn parse_response(
        self,
        uri: &http::Uri,
        response: http::Response<Vec<u8>>,
    ) -> Result<<Self as Request>::Response, HelixRequestPatchError>
    where
        <Self as Request>::Response:
            std::convert::TryFrom<http::StatusCode, Error = std::borrow::Cow<'static, str>>,
        Self: Sized,
    {
        match response.status().try_into() {
            Ok(result) => Ok(result),
            Err(err) => Err(HelixRequestPatchError {
                status: response.status(),
                message: err.to_string(),
                uri: uri.clone(),
                body: response.body().clone(),
            }),
        }
    }
}

/// Helix endpoint DELETEs information
pub trait RequestDelete: Request {
    /// Get Request to use in your client
    fn create_request(
        &self,
        token: &str,
        client_id: &str,
    ) -> Result<http::Request<Vec<u8>>, CreateRequestError>
    {
        let uri = http::Uri::from_str(&format!(
            "{}{}?{}",
            crate::TWITCH_HELIX_URL,
            <Self as Request>::PATH,
            self.query()?
        ))?;

        let mut bearer =
            http::HeaderValue::from_str(&format!("Bearer {}", token)).map_err(|_| {
                CreateRequestError::Custom("Could not make token into headervalue".into())
            })?;
        bearer.set_sensitive(true);
        http::Request::builder()
            .method(http::Method::DELETE)
            .uri(uri)
            .header("Client-ID", client_id)
            .header("Content-Type", "application/json")
            .header(http::header::AUTHORIZATION, bearer)
            .body(Vec::with_capacity(0))
            .map_err(Into::into)
    }

    /// Parse response. Override for different behavior
    fn parse_response(
        self,
        uri: &http::Uri,
        response: http::Response<Vec<u8>>,
    ) -> Result<<Self as Request>::Response, HelixRequestDeleteError>
    where
        <Self as Request>::Response:
            std::convert::TryFrom<http::StatusCode, Error = std::borrow::Cow<'static, str>>,
        Self: Sized,
    {
        let text = std::str::from_utf8(&response.body())
            .map_err(|e| HelixRequestDeleteError::Utf8Error(response.body().clone(), e))?;
        // eprintln!("\n\nmessage is ------------ {} ------------", text);

        if let Ok(HelixRequestError {
            error,
            status,
            message,
        }) = serde_json::from_str::<HelixRequestError>(&text)
        {
            return Err(HelixRequestDeleteError::Error {
                error,
                status: status.try_into().unwrap_or(http::StatusCode::BAD_REQUEST),
                message,
                uri: uri.clone(),
            });
        }

        match response.status().try_into() {
            Ok(result) => Ok(result),
            Err(err) => Err(HelixRequestDeleteError::Error {
                error: String::new(),
                status: response.status(),
                message: err.to_string(),
                uri: uri.clone(),
            }),
        }
    }
}

/// Helix endpoint GETs information
pub trait RequestGet: Request {
    /// Get Request to use in your client
    fn create_request(
        &self,
        token: &str,
        client_id: &str,
    ) -> Result<http::Request<Vec<u8>>, CreateRequestError>
    {
        let uri = http::Uri::from_str(&format!(
            "{}{}?{}",
            crate::TWITCH_HELIX_URL,
            <Self as Request>::PATH,
            self.query()?
        ))?;

        let mut bearer =
            http::HeaderValue::from_str(&format!("Bearer {}", token)).map_err(|_| {
                CreateRequestError::Custom("Could not make token into headervalue".into())
            })?;
        bearer.set_sensitive(true);
        http::Request::builder()
            .method(http::Method::GET)
            .uri(uri)
            .header("Client-ID", client_id)
            .header("Content-Type", "application/json")
            .header(http::header::AUTHORIZATION, bearer)
            .body(Vec::with_capacity(0))
            .map_err(Into::into)
    }

    /// Parse response. Override for different behavior
    fn parse_response(
        self,
        uri: &http::Uri,
        response: http::Response<Vec<u8>>,
    ) -> Result<Response<Self, <Self as Request>::Response>, HelixRequestGetError>
    where
        Self: Sized,
    {
        let text = std::str::from_utf8(&response.body())
            .map_err(|e| HelixRequestGetError::Utf8Error(response.body().clone(), e))?;
        //eprintln!("\n\nmessage is ------------ {} ------------", text);
        if let Ok(HelixRequestError {
            error,
            status,
            message,
        }) = serde_json::from_str::<HelixRequestError>(&text)
        {
            return Err(HelixRequestGetError::Error {
                error,
                status: status.try_into().unwrap_or(http::StatusCode::BAD_REQUEST),
                message,
                uri: uri.clone(),
            });
        }
        let response: InnerResponse<_> = serde_json::from_str(&text)?;
        Ok(Response {
            data: response.data,
            pagination: response.pagination,
            request: self,
        })
    }
}

/// Response retrieved from endpoint. Data is the type in [Request::Response]
#[derive(PartialEq, Debug)]
pub struct Response<R, D>
where
    R: Request<Response = D>,
    D: serde::de::DeserializeOwned, {
    ///  Twitch's response field for `data`.
    pub data: Vec<D>,
    /// A cursor value, to be used in a subsequent request to specify the starting point of the next set of results.
    pub pagination: Pagination,
    /// The request that was sent, used for [Paginated]
    pub request: R,
}

#[cfg(feature = "client")]
impl<R, D> Response<R, D>
where
    R: Request<Response = D> + Clone + Paginated + RequestGet,
    D: serde::de::DeserializeOwned,
{
    /// Get the next page in the responses.
    pub async fn get_next<'a, C: crate::HttpClient<'a>>(
        self,
        client: &'a HelixClient<'a, C>,
        token: &impl TwitchToken,
    ) -> Result<Option<Response<R, D>>, ClientRequestError<<C as crate::HttpClient<'a>>::Error>>
    {
        let mut req = self.request.clone();
        if let Some(ref cursor) = self.pagination.cursor {
            req.set_pagination(cursor.clone());
            client.req_get(req, token).await.map(Some)
        } else {
            Ok(None)
        }
    }
}

/// Request can be paginated with a cursor
pub trait Paginated: Request {
    /// Should returns the current pagination cursor.
    ///
    /// # Notes
    ///
    /// Pass [Option::None] if no cursor is found.
    fn set_pagination(&mut self, cursor: Cursor);
}

/// A cursor for pagination. This is needed because of how pagination is represented in the [New Twitch API](https://dev.twitch.tv/docs/api)
#[derive(PartialEq, Deserialize, Serialize, Debug, Clone, Default)]
pub struct Pagination {
    #[serde(default)]
    cursor: Option<Cursor>,
}

/// A cursor is a pointer to the current "page" in the twitch api pagination
pub type Cursor = String;

/// Errors for [HelixClient::req_get] and similar functions.
#[derive(thiserror::Error, Debug, displaydoc::Display)]
pub enum ClientRequestError<RE: std::error::Error + Send + Sync + 'static> {
    /// request failed from reqwests side
    RequestError(RE),
    /// no pagination found
    NoPage,
    /// Could not create request
    CreateRequestError(#[from] CreateRequestError),
    /// Could not parse GET response
    #[error(transparent)]
    HelixRequestGetError(#[from] HelixRequestGetError),
    //#[error(transparent)]
    /// Could not parse PUT response
    HelixRequestPutError(#[from] HelixRequestPutError),
    /// Could not parse POST response
    #[error(transparent)]
    HelixRequestPostError(#[from] HelixRequestPostError),
    //#[error(transparent)]
    /// Could not parse PATCH response
    HelixRequestPatchError(#[from] HelixRequestPatchError),
    /// Could not parse DELETE response
    #[error(transparent)]
    HelixRequestDeleteError(#[from] HelixRequestDeleteError),
    /// {0}
    Custom(std::borrow::Cow<'static, str>),
}
/// Could not create request
#[derive(thiserror::Error, Debug, displaydoc::Display)]
pub enum CreateRequestError {
    /// Could not serialize request to query
    QuerySerializeError(#[from] ser::Error),
    /// http crate returned an error
    HttpError(#[from] http::Error),
    /// URI could not be parsed
    UriParseError(#[from] http::uri::InvalidUri),
    /// serialization of body failed
    SerializeError(#[from] serde_json::Error),
    /// {0}
    Custom(std::borrow::Cow<'static, str>),
}

/// Could not parse GET response
#[derive(thiserror::Error, Debug, displaydoc::Display)]
pub enum HelixRequestGetError {
    /// helix returned error {status:?} - {error}: {message:?} when calling `GET {uri}`
    Error {
        /// Error message related to status code
        error: String,
        /// Status code of error, usually 400-499
        status: http::StatusCode,
        /// Error message from Twitch
        message: String,
        /// URI to the endpoint
        uri: http::Uri,
    },
    /// could not parse body as utf8: {1}
    Utf8Error(Vec<u8>, std::str::Utf8Error),
    /// deserialization failed when processing request result
    DeserializeError(#[from] serde_json::Error),
}

/// helix returned error {status:?} - {error}: {message:?} when calling `PUT {uri}` with a body
#[derive(thiserror::Error, Debug, displaydoc::Display)]
pub struct HelixRequestPutError {
    /// Error message related to status code
    error: String,
    /// Status code of error, usually 400-499
    status: http::StatusCode,
    /// Error message from Twitch
    message: String,
    /// URI to the endpoint
    uri: http::Uri,
    /// Body sent with PUT
    body: Vec<u8>,
}

/// Could not parse POST response
#[derive(thiserror::Error, Debug, displaydoc::Display)]
pub enum HelixRequestPostError {
    /// helix returned error {status:?} - {error}: {message:?} when calling `POST {uri}` with a body
    Error {
        /// Error message related to status code
        error: String,
        /// Status code of error, usually 400-499
        status: http::StatusCode,
        /// Error message from Twitch
        message: String,
        /// URI to the endpoint
        uri: http::Uri,
        /// Body sent with PUT
        body: Vec<u8>,
    },
    /// could not parse body as utf8: {1}
    Utf8Error(Vec<u8>, std::str::Utf8Error),
    /// deserialization failed when processing request result
    DeserializeError(#[from] serde_json::Error),
}

/// helix returned error {status:?}: {message:?} when calling `PATCH {uri}` with a body
#[derive(thiserror::Error, Debug, displaydoc::Display)]
pub struct HelixRequestPatchError {
    /// Status code of error, usually 400-499
    status: http::StatusCode,
    /// Error message from Twitch
    message: String,
    /// URI to the endpoint
    uri: http::Uri,
    /// Body sent with PATCH
    body: Vec<u8>,
}

/// Could not parse DELETE response
#[derive(thiserror::Error, Debug, displaydoc::Display)]
pub enum HelixRequestDeleteError {
    /// helix returned error {status:?}- {error}: {message:?} when calling `DELETE {uri}`
    Error {
        /// Error message related to status code
        error: String,
        /// Status code of error, usually 400-499
        status: http::StatusCode,
        /// Error message from Twitch
        message: String,
        /// URI to the endpoint
        uri: http::Uri,
    },
    /// could not parse body as utf8: {1}
    Utf8Error(Vec<u8>, std::str::Utf8Error),
}

/// Repeat url query items with name
///
/// ```rust
/// let users = &["emilgardis", "jtv", "tmi"].iter().map(<_>::to_string).collect::<Vec<_>>();
///  assert_eq!(&twitch_api2::helix::repeat_query("user", users), "user=emilgardis&user=jtv&user=tmi")
/// ```
pub fn repeat_query(name: &str, items: &[String]) -> String {
    let mut s = String::new();
    for (idx, item) in items.iter().enumerate() {
        s.push_str(&format!("{}={}", name, item));
        if idx + 1 != items.len() {
            s.push('&')
        }
    }
    s
}