use std::{
    sync::{atomic::AtomicU64, Arc, RwLock},
    time::Duration,
};

use anyhow::Context;
use dashmap::DashMap;
use futures::StreamExt;
use futures::{channel::oneshot, SinkExt};
use serde_json::{json, Value};
use solana_api_types::{Client as BasicClient, Hash, Pubkey, Signature, Transaction};

use url::Url;

use log::{debug, error, info};

#[derive(Debug)]
enum WsRequest {
    SubscribeSlot,
    SubscribeSignature(Signature),
}

struct RpcResponse {
    method: Option<String>,
    id: u64,
    result: Value,
    params: Value,
}

fn make_rpc_request(id: u64, method: &str, params: Option<Value>) -> Value {
    let mut request = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
    });

    if let Some(params) = params {
        request
            .as_object_mut()
            .unwrap()
            .insert("params".into(), params);
    }

    request
}

fn parse_rpc_response(mut value: Value) -> RpcResponse {
    let method = value["method"].as_str().map(|s| s.into());

    let id = value["id"].as_u64().unwrap_or(0);

    let result = (&mut value["result"]).take();
    let params = (&mut value["params"]).take();

    RpcResponse {
        method,
        id,
        result,
        params,
    }
}

#[cfg(feature = "native")]
use async_tungstenite::{tungstenite::Message, WebSocketStream as WsStream};

#[cfg(feature = "wasm")]
use ws_stream_wasm::{WsMessage as Message, WsStream};

impl WsRequest {
    fn to_message(&self, id: u64) -> Message {
        Message::Text(
            serde_json::to_string(&match self {
                WsRequest::SubscribeSlot => make_rpc_request(id, "slotSubscribe", None),
                WsRequest::SubscribeSignature(signature) => make_rpc_request(
                    id,
                    "signatureSubscribe",
                    Some(json!([signature.to_string()])),
                ),
            })
            .expect("json serialization"),
        )
    }
}

struct WsClient {
    request_sender: async_std::channel::Sender<WsRequest>,
    signature_notifiers: Arc<DashMap<Signature, oneshot::Sender<()>>>,
    last_slot: Arc<RwLock<u64>>,
}

#[cfg(feature = "wasm")]
async fn connect_ws(url: Url) -> Result<WsStream, anyhow::Error> {
    use ws_stream_wasm::WsMeta;
    let (_, stream) = WsMeta::connect(url, None)
        .await
        .context("couldn't establish ws connection")?;

    Ok(stream)
}

#[cfg(feature = "native")]
async fn connect_ws(url: Url) -> Result<WsStream<async_std::net::TcpStream>, anyhow::Error> {
    let (stream, _) = async_tungstenite::async_std::connect_async(url)
        .await
        .context("couldn't establish connection")?;

    Ok(stream)
}

impl WsClient {
    async fn start(url: Url) -> anyhow::Result<Self> {
        debug!("connecting to cluster at {}", url);
        let stream = connect_ws(url.clone()).await?;
        debug!("connected to cluster");
        let (mut sink, mut stream) = stream.split();
        let id = AtomicU64::new(1);

        let (request_sender, mut request_receiver) = async_std::channel::unbounded::<WsRequest>();
        let last_slot = Arc::new(RwLock::new(0));

        // Tracks subscriptions to the `signatureNotification` method on the RPC.
        // The keys are subscription IDs.
        let pending_signatures: Arc<DashMap<u64, Signature>> = Arc::new(DashMap::new());

        // Tracks pending subscription requests.
        // The keys are request IDs as specified in the 'id' field of the request.
        // Values are the recorded Requests for these ids.
        let pending_requests: Arc<DashMap<u64, WsRequest>> = Arc::new(DashMap::new());
        // Tracks notifiers for signature subscriptions.
        // When the signature has reached the requested commitment level, the provided Sender will be used to notify
        // the waiting task.
        let signature_notifiers: Arc<DashMap<Signature, oneshot::Sender<()>>> =
            Arc::new(DashMap::new());

        // This will handle WS subscription requests coming in from the client.
        let request_processor = {
            let pending_requests = Arc::clone(&pending_requests);

            async move {
                while let Some(request) = request_receiver.next().await {
                    debug!("received ws request: {:?}", request);
                    let id = id.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                    let message = request.to_message(id);
                    debug!("sending ws message: {:?}", message);
                    sink.send(request.to_message(id)).await?;
                    pending_requests.insert(id, request);
                }

                Result::<(), anyhow::Error>::Ok(())
            }
        };

        // This will handle WS responses coming in from the RPC.
        let response_processor = {
            let (pending_signatures, pending_requests, signature_notifiers) = (
                Arc::clone(&pending_signatures),
                Arc::clone(&pending_requests),
                Arc::clone(&signature_notifiers),
            );

            let slot_sender = last_slot.clone();
            async move {
                let slot_sender = slot_sender.clone();

                while let Some(message) = stream.next().await {
                    #[cfg(feature = "native")]
                    let message = match message {
                        Ok(message) => message,
                        Err(error) => {
                            error!("{}", error);
                            continue;
                        }
                    };

                    if let Message::Text(message) = message {
                        let result = async {
                            match serde_json::from_str::<serde_json::Value>(&message) {
                                Ok(root) => {
                                    let response = parse_rpc_response(root);
                                    if response
                                        .method
                                        .as_ref()
                                        .map(|method| method != "slotNotification")
                                        .unwrap_or(true)
                                    {
                                        debug!("received ws message: {}", message);
                                    }

                                    if let Some(method) = response.method {
                                        // If the response has a 'method' field, this is likely a notification for an active subscription. Handle it.
                                        match method.as_str() {
                                            "slotNotification" => {
                                                if let Some(slot) =
                                                    response.params["result"]["slot"].as_u64()
                                                {
                                                    *slot_sender.write().unwrap() = slot;
                                                }
                                            }
                                            "signatureNotification" => {
                                                if let Some((_, notifier)) = response.params
                                                    ["subscription"]
                                                    .as_u64()
                                                    .and_then(|id| pending_signatures.remove(&id))
                                                    .and_then(|(_, signature)| {
                                                        signature_notifiers.remove(&signature)
                                                    })
                                                {
                                                    // We don't really care whether the send was successful.
                                                    notifier.send(()).ok();
                                                }
                                            }
                                            _ => {}
                                        }
                                    } else {
                                        // An absence of a 'method' field indicates that this is a response to a subscription request containing the subscription id,
                                        // which we need to record to match the notification later on.
                                        let id = response.id;
                                        let subscription_id = response.result.as_u64();

                                        if let (Some(subscription_id), Some(request)) =
                                            (subscription_id, pending_requests.get(&id))
                                        {
                                            match *request {
                                                WsRequest::SubscribeSlot => {}
                                                WsRequest::SubscribeSignature(signature) => {
                                                    pending_signatures
                                                        .insert(subscription_id, signature);
                                                }
                                            }
                                        }

                                        pending_requests.remove(&id);
                                    }
                                }
                                Err(error) => {
                                    error!("error trying to parse json ({}): {}", message, error);
                                }
                            }

                            Result::<(), anyhow::Error>::Ok(())
                        }
                        .await;

                        if let Err(error) = result {
                            error!("{}", error);
                        }
                    }
                }

                Result::<(), anyhow::Error>::Ok(())
            }
        };

        // Spawn the processors onto a separate task.
        async_std::task::spawn(async {
            if let Err(error) = futures::try_join!(request_processor, response_processor) {
                error!("{}", error);
            };
        });

        Ok(WsClient {
            request_sender,
            signature_notifiers,
            last_slot,
        })
    }

    async fn register_signature(&self, signature: Signature) -> oneshot::Receiver<()> {
        let request = WsRequest::SubscribeSignature(signature);

        let (sender, receiver) = oneshot::channel();
        self.signature_notifiers.insert(signature, sender);
        self.request_sender
            .send(request)
            .await
            .expect("couldn't send request to an unbounded queue - is the receiver alive?");

        receiver
    }

    async fn register_slot(&self) {
        let request = WsRequest::SubscribeSlot;

        self.request_sender
            .send(request)
            .await
            .expect("couldn't send request to an unbounded queue - is the receiver alive?");
    }
}

struct SolanaClientInner<T: BasicClient + Send + Sync + 'static> {
    ws_client: WsClient,
    client: Arc<T>,
    last_slot: Arc<RwLock<u64>>,
    recent_blockhash: Arc<RwLock<Hash>>,
}

impl<T: BasicClient + Send + Sync + 'static> SolanaClientInner<T> {
    async fn new(
        client: T,
        ws_url: Url,
        recent_blockhash_interval: Duration,
    ) -> anyhow::Result<Self> {
        let client = Arc::new(client);
        let hash = client.get_recent_blockhash().await?;
        let slot = client.get_slot(None).await?;
        let recent_blockhash = Arc::new(RwLock::new(hash));

        debug!("creating ws client");
        let ws_client = WsClient::start(ws_url.clone())
            .await
            .context("couldn't start websocket service")?;

        let last_slot = {
            debug!("registering slot listener");
            *ws_client.last_slot.write().unwrap() = slot;
            ws_client.register_slot().await;
            Arc::clone(&ws_client.last_slot)
        };

        {
            debug!("registering blockhash listener");
            let client = Arc::clone(&client);
            let recent_blockhash = Arc::clone(&recent_blockhash);
            async_std::task::spawn_local(async move {
                loop {
                    if let Ok(hash) = client.get_recent_blockhash().await {
                        {
                            let mut old_hash = recent_blockhash.write().unwrap();
                            *old_hash = hash;
                        }

                        async_std::task::sleep(recent_blockhash_interval).await;
                    }
                }
            });
        }

        Ok(Self {
            ws_client,
            client,
            last_slot,
            recent_blockhash,
        })
    }
}

#[derive(Clone)]
pub struct SolanaClient<T: BasicClient + Send + Sync + 'static> {
    inner: Arc<SolanaClientInner<T>>,
}

impl<T: BasicClient + Send + Sync + 'static> SolanaClient<T> {
    pub async fn start(client: T, ws_url: Url) -> anyhow::Result<Self> {
        let inner = SolanaClientInner::<T>::new(client, ws_url, Duration::from_secs(5)).await?;

        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    pub fn recent_blockhash(&self) -> Hash {
        *self.inner.recent_blockhash.read().unwrap()
    }

    pub fn slot(&self) -> u64 {
        *self.inner.last_slot.read().unwrap()
    }

    /// Processes the full lifecycle of a transaction, starting from sending it to a cluster,
    /// to waiting for its confirmation.
    pub async fn process_transaction(&self, transaction: &Transaction) -> anyhow::Result<()> {
        info!("sending transaction - {}", transaction.signatures[0]);

        let signature = transaction.signatures[0];

        let notifier = self.inner.ws_client.register_signature(signature).await;

        self.inner
            .client
            .send_transaction(transaction)
            .await
            .context("couldn't send transaction")?;

        info!(
            "awaiting transaction confirmation - {}",
            transaction.signatures[0]
        );

        notifier.await.ok();
        info!("transaction confirmed - {}", transaction.signatures[0]);

        Ok(())
    }

    /// Processes an airdrop request transaction, up until confirmation.
    pub async fn request_airdrop(&self, target: &Pubkey, lamports: u64) -> anyhow::Result<()> {
        let signature = self
            .inner
            .client
            .request_airdrop(target, lamports, None)
            .await
            .context("couldn't request lamport airdrop")?;

        self.inner
            .ws_client
            .register_signature(signature)
            .await
            .await
            .ok();

        Ok(())
    }
}
