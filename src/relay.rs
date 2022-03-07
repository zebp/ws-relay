use std::sync::Arc;

use futures::{lock::Mutex, StreamExt};

use crate::*;

#[durable_object]
pub struct Relay {
    inner: Arc<Mutex<InnerRelay>>,
}

struct InnerRelay {
    env: Env,
    sockets: Vec<WebSocket>,
    connections: usize,
}

#[durable_object]
impl DurableObject for Relay {
    fn new(state: State, env: Env) -> Self {
        let inner = InnerRelay {
            env,
            sockets: Vec::new(),
            connections: 0,
        };

        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    async fn fetch(&mut self, req: Request) -> Result<Response> {
        let env: Env = {
            let inner = self.inner.lock().await;
            inner.env.clone().into()
        };

        Router::with_data(self.inner.clone())
            .get_async("/subscriber", subscriber)
            .get_async("/publisher", publisher)
            .run(req, env)
            .await
    }
}

async fn subscriber(_: Request, ctx: RouteContext<Arc<Mutex<InnerRelay>>>) -> Result<Response> {
    let WebSocketPair { client, server } = WebSocketPair::new()?;

    server.accept()?;

    let mut inner = ctx.data.lock().await;
    inner.sockets.push(server);
    inner.connections += 1;

    Response::from_websocket(client)
}

async fn publisher(_: Request, ctx: RouteContext<Arc<Mutex<InnerRelay>>>) -> Result<Response> {
    let WebSocketPair { client, server } = WebSocketPair::new()?;
    let inner_lock = ctx.data;

    server.accept()?;

    wasm_bindgen_futures::spawn_local(async move {
        let mut stream = server.events().unwrap();

        while let Some(event) = stream.next().await {
            let event = event.unwrap();

            let mut inner = inner_lock.lock().await;
            let sockets = &mut inner.sockets;

            match event {
                WebsocketEvent::Message(msg) => {
                    if let Some(text) = msg.text() {
                        for socket in sockets {
                            socket.send_with_str(&text).unwrap();
                        }
                    }
                }
                WebsocketEvent::Close(_) => {
                    for socket in sockets.drain(0..) {
                        socket.close::<String>(Some(1001), None).unwrap();
                    }

                    inner.connections = 0;
                }
            }
        }
    });

    Response::from_websocket(client)
}
