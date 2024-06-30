use esp_idf_svc::ipv4::IpInfo;
use hyper::Request;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use log::info;
use tokio::net::TcpListener;
use crate::handle_request::handle_request;
use crate::power_io::PowerIo;

pub async fn run_server(ip_info: IpInfo, power_io: PowerIo) -> anyhow::Result<()> {
    let addr = "0.0.0.0:80";

    info!("Binding to {addr}...");
    let listener = TcpListener::bind(&addr).await?;

    let ip = ip_info.ip;
    info!("Server is listening at http://{ip}");

    loop {
        info!("Waiting for new connection on socket: {listener:?}");
        let (stream, _) = listener.accept().await?;

        let io = TokioIo::new(stream);

        let power_io = power_io.clone();
        tokio::spawn({
            async move {
                info!("Spawned handler!");
                if let Err(err) = hyper::server::conn::http1::Builder::new()
                    .keep_alive(true)
                    // `service_fn` converts our function in a `Service`
                    .serve_connection(io, service_fn({
                        move |req: Request<hyper::body::Incoming>| {
                            handle_request(req, power_io.clone())
                        }
                    }))
                    .with_upgrades()
                    .await
                {
                    eprintln!("Error serving connection: {:?}", err);
                }
            }
        });
    }
}
