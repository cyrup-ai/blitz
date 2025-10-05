//! Networking (HTTP, filesystem, Data URIs) for Blitz
//!
//! Provides an implementation of the [`blitz_traits::net::NetProvider`] trait.

use std::sync::Arc;

use blitz_traits::net::{BoxedHandler, Bytes, NetCallback, NetProvider, Request, SharedCallback};
use data_url::DataUrl;
use reqwest::Client;
use tokio::{
    runtime::Handle,
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:60.0) Gecko/20100101 Firefox/81.0";

pub struct Provider<D> {
    rt: Handle,
    client: Client,
    resource_callback: SharedCallback<D>,
}
impl<D: 'static> Provider<D> {
    pub fn new(resource_callback: SharedCallback<D>) -> Self {
        #[cfg(feature = "cookies")]
        let client = Client::builder().cookie_store(true).build().unwrap();
        #[cfg(not(feature = "cookies"))]
        let client = Client::new();

        Self {
            rt: Handle::current(),
            client,
            resource_callback,
        }
    }
    pub fn shared(res_callback: SharedCallback<D>) -> Arc<dyn NetProvider<D>> {
        Arc::new(Self::new(res_callback))
    }
    pub fn is_empty(&self) -> bool {
        Arc::strong_count(&self.resource_callback) == 1
    }
}
impl<D: 'static> Provider<D> {
    async fn fetch_inner(
        client: Client,
        request: Request,
    ) -> Result<(String, Bytes), ProviderError> {
        Ok(match request.url.scheme() {
            "data" => {
                let data_url = DataUrl::process(request.url.as_str())?;
                let decoded = data_url.decode_to_vec()?;
                (request.url.to_string(), Bytes::from(decoded.0))
            }
            "file" => {
                let file_content = std::fs::read(request.url.path())?;
                (request.url.to_string(), Bytes::from(file_content))
            }
            _ => {
                let response = client
                    .request(request.method, request.url)
                    .headers(request.headers)
                    .header("User-Agent", USER_AGENT)
                    .body(request.body)
                    .send()
                    .await?;

                (response.url().to_string(), response.bytes().await?)
            }
        })
    }

    async fn fetch_with_handler(
        client: Client,
        doc_id: usize,
        request: Request,
        handler: BoxedHandler<D>,
        res_callback: SharedCallback<D>,
    ) -> Result<(), ProviderError> {
        let (_response_url, bytes) = Self::fetch_inner(client, request).await?;
        handler.bytes(doc_id, bytes, res_callback);
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    pub fn fetch_with_callback(
        &self,
        request: Request,
        callback: Box<dyn FnOnce(Result<(String, Bytes), ProviderError>) + Send + Sync + 'static>,
    ) {
        let client = self.client.clone();
        self.rt.spawn(async move {
            let url = request.url.to_string();
            let result = Self::fetch_inner(client, request).await;
            if let Err(e) = &result {
                eprintln!("Error fetching {url}: {e:?}");
            } else {
                println!("Success {url}");
            }
            callback(result);
        });
    }

    pub async fn fetch_async(&self, request: Request) -> Result<(String, Bytes), ProviderError> {
        let client = self.client.clone();
        let url = request.url.to_string();
        let result = Self::fetch_inner(client, request).await;
        if let Err(e) = &result {
            eprintln!("Error fetching {url}: {e:?}");
        } else {
            println!("Success {url}");
        }
        result
    }
}

impl<D: 'static> NetProvider<D> for Provider<D> {
    fn fetch(&self, doc_id: usize, request: Request, handler: BoxedHandler<D>) {
        let client = self.client.clone();
        let callback = Arc::clone(&self.resource_callback);
        
        #[cfg(feature = "tracing")]
        tracing::debug!("Fetching {}", &request.url);
        
        self.rt.spawn(async move {
            let url = request.url.to_string();
            let res = Self::fetch_with_handler(client, doc_id, request, handler, callback.clone()).await;
            
            if let Err(e) = res {
                // Structured logging with context
                #[cfg(feature = "tracing")]
                tracing::error!(
                    url = %url,
                    doc_id = doc_id,
                    error = %e,
                    "Network fetch failed"
                );
                
                #[cfg(not(feature = "tracing"))]
                eprintln!("Error fetching {url}: {e}");
                
                // Propagate error to callback consumers
                let error_msg = format!("Failed to fetch {}: {}", url, e);
                callback.call(doc_id, Err(Some(error_msg)));
            } else {
                #[cfg(feature = "tracing")]
                tracing::debug!("Success {}", url);
            }
        });
    }
}

#[derive(Debug)]
pub enum ProviderError {
    Io(std::io::Error),
    DataUrl(data_url::DataUrlError),
    DataUrlBase64(data_url::forgiving_base64::InvalidBase64),
    ReqwestError(reqwest::Error),
}

impl From<std::io::Error> for ProviderError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<data_url::DataUrlError> for ProviderError {
    fn from(value: data_url::DataUrlError) -> Self {
        Self::DataUrl(value)
    }
}

impl From<data_url::forgiving_base64::InvalidBase64> for ProviderError {
    fn from(value: data_url::forgiving_base64::InvalidBase64) -> Self {
        Self::DataUrlBase64(value)
    }
}

impl From<reqwest::Error> for ProviderError {
    fn from(value: reqwest::Error) -> Self {
        Self::ReqwestError(value)
    }
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {}", e),
            Self::DataUrl(e) => write!(f, "Data URL parsing error: {}", e),
            Self::DataUrlBase64(e) => write!(f, "Base64 decode error: {}", e),
            Self::ReqwestError(e) => write!(f, "HTTP request error: {}", e),
        }
    }
}

impl std::error::Error for ProviderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::ReqwestError(e) => Some(e),
            _ => None,
        }
    }
}

pub struct MpscCallback<T>(UnboundedSender<(usize, Result<T, String>)>);
impl<T> MpscCallback<T> {
    pub fn new() -> (UnboundedReceiver<(usize, Result<T, String>)>, Self) {
        let (send, recv) = unbounded_channel();
        (recv, Self(send))
    }
}

impl<T: Send + Sync + 'static> NetCallback<T> for MpscCallback<T> {
    fn call(&self, doc_id: usize, result: Result<T, Option<String>>) {
        // Convert Option<String> error to String for channel
        let result_to_send = result.map_err(|opt_err| {
            opt_err.unwrap_or_else(|| "Unknown network error".to_string())
        });
        
        if let Err(e) = self.0.send((doc_id, result_to_send)) {
            #[cfg(feature = "tracing")]
            tracing::error!(
                doc_id = doc_id,
                error = ?e,
                "Failed to send network result through channel"
            );
            
            #[cfg(not(feature = "tracing"))]
            eprintln!("Failed to send network result for doc {doc_id}: {e:?}");
        }
    }
}
