use http_body_util::BodyExt;
use http_body_util::combinators::BoxBody;
use hyper::body::Bytes;
use hyper::{Request, Response};
use log::error;
use crate::Error;
use crate::power_io::PowerIo;
use crate::serve_static::serve_static;
use crate::serve_websocket::serve_websocket;

pub async fn handle_request(mut req: Request<hyper::body::Incoming>, power_io: PowerIo) -> Result<Response<BoxBody<Bytes, hyper::Error>>, Error> {
    // Check if the request is a websocket upgrade request.
    if hyper_tungstenite::is_upgrade_request(&req) {
        let (response, websocket) = hyper_tungstenite::upgrade(&mut req, None)?;

        // Spawn a task to handle the websocket connection.
        tokio::spawn(async move {
            if let Err(e) = serve_websocket(websocket, power_io).await {
                error!("Error in websocket connection: {e}");
            }
        });

        // Return the response so the spawned future can continue.
        Ok(response.map(|a| a.map_err(|never| match never {}).boxed()))
    } else {
        serve_static(req).await
    }
}
