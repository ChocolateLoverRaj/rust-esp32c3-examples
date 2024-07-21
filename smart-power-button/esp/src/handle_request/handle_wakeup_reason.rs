use std::ops::Deref;

use http_body_util::combinators::BoxBody;
use hyper::body::Bytes;
use hyper::{Method, Request, Response, StatusCode};
use postcard::to_allocvec;

use crate::hyper_util::{empty, full};
use crate::power_io::PowerIo;
use crate::Error;

pub async fn handle_wakeup_reason(
    req: Request<hyper::body::Incoming>,
    power_io: PowerIo,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, Error> {
    match *req.method() {
        Method::GET => {
            let response = Response::new(full(
                to_allocvec(power_io.wakeup_reason.lock().await.deref()).unwrap(),
            ));
            Ok(response)
        }
        Method::DELETE => {
            let response = Response::new(full(
                to_allocvec(&power_io.wakeup_reason.lock().await.take()).unwrap(),
            ));
            Ok(response)
        }
        _ => {
            let mut response = Response::new(empty());
            *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
            Ok(response)
        }
    }
}
