use chrono::{DateTime, Utc};
use http_body_util::combinators::BoxBody;
use hyper::body::Bytes;
use hyper::header::HeaderValue;
use hyper::{Method, Request, Response, StatusCode};
use include_dir::{Dir, DirEntry};

use crate::http_content_type::EXTENSION_MAP;
use crate::hyper_util::{empty, full};
use crate::Error;

#[cfg(feature = "static-files")]
const ASSETS: Dir = include_dir::include_dir!("$CARGO_MANIFEST_DIR/../web/dist");
#[cfg(not(feature = "static-files"))]
const ASSETS: Dir = Dir::new("", &[]);

pub async fn serve_static(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, Error> {
    let url_without_leading_slash = &req.uri().path()[1..];
    match ASSETS.get_entry(match url_without_leading_slash {
        "" => "index.html",
        normal => normal,
    }) {
        Some(entry) => match req.method() {
            &Method::GET => match entry {
                DirEntry::File(file) => {
                    // Cache cuz it takes incredibly long to serve
                    let last_modified = file.metadata().unwrap().modified();
                    let last_modified = DateTime::<Utc>::from(last_modified);
                    // println!("Last modified: {last_modified}");
                    let not_modified =
                        req.headers()
                            .get("If-Modified-Since")
                            .map_or(false, |if_modified_since| {
                                let if_modified_since = if_modified_since.to_str().unwrap();
                                let if_modified_since =
                                    DateTime::parse_from_rfc2822(if_modified_since).unwrap();
                                // println!("If modified since: {if_modified_since}");
                                if_modified_since >= last_modified
                            });
                    if not_modified {
                        let mut response = Response::new(empty());
                        *response.status_mut() = StatusCode::NOT_MODIFIED;
                        Ok(response)
                    } else {
                        let mut response = Response::new(full(file.contents()));
                        let content_type = EXTENSION_MAP
                            .get(
                                file.path()
                                    .extension()
                                    .expect("No extension")
                                    .to_str()
                                    .expect("Couldn't convert extension to string"),
                            )
                            .expect("Unknown Content-Type");
                        response
                            .headers_mut()
                            .insert("Content-Type", HeaderValue::from_str(content_type)?);
                        // 1 year cache
                        response.headers_mut().insert(
                            "Cache-Control",
                            HeaderValue::from_str("public, max-age=31536000")?,
                        );
                        response.headers_mut().insert(
                            "Last-Modified",
                            HeaderValue::from_str(&last_modified.to_rfc2822())?,
                        );
                        Ok(response)
                    }
                }
                DirEntry::Dir(_dir) => {
                    let mut not_implemented = Response::new(empty());
                    *not_implemented.status_mut() = StatusCode::NOT_IMPLEMENTED;
                    Ok(not_implemented)
                }
            },
            _ => {
                let mut method_not_allowed = Response::new(empty());
                *method_not_allowed.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
                Ok(method_not_allowed)
            }
        },
        None => {
            let mut not_found = Response::new(empty());
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}
