mod cli;

use hyper::{Request, Response, Method, StatusCode, body};
use hyper::server::conn::http1::Builder;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use bytes::Bytes;
use http_body_util::{Full, BodyExt};
use std::sync::Arc;
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use std::fs;
use std::path::PathBuf;
use tokio::net::TcpListener;
use clap::Parser;
use cli::Cli;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

struct ServerState {
    auth_version: u32,
    data_dir: PathBuf,
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody {
    Full::new(chunk.into()).map_err(|never| match never {}).boxed()
}

async fn handle_request(req: Request<body::Incoming>, state: Arc<ServerState>) -> Result<Response<BoxBody>, GenericError> {
    let (parts, body) = req.into_parts();
    match (parts.method, parts.uri.path()) {
        (Method::GET, "/isalive") => {
            let response = Response::new(full(format!("{{ authVersion: {} }}", state.auth_version)));
            Ok(response)
        }
        (Method::GET, path) if path.starts_with("/files/") => {
            // Update the length of the string slice below if you update the path prefix above
            let file_name = &path[7..];
            let file_path = state.data_dir.join(file_name);
            let data = fs::read(file_path)?;
            let response = Response::new(full(data));
            Ok(response)
        }
        (Method::PUT, path) if path.starts_with("/files/") => {
            // Update the length of the string slice below if you update the path prefix above
            let file_name = &path[7..];
            let file_path = state.data_dir.join(file_name);
            let data = body.collect().await?.to_bytes();
            fs::write(file_path, &data)?;
            // let builder = Response::builder().status(StatusCode::OK);
            // builder.body(())
            let response = Response::new(full(Bytes::new()));
            Ok(response)
        }
        _ => {
            let builder = Response::builder().status(StatusCode::NOT_FOUND);
            let response = builder.body(full("Page not found"))?;
            Ok(response)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Cli::parse();

    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), args.port);
    let listener = TcpListener::bind(socket).await?;

    fs::create_dir_all(&args.folder)?;

    let server_state = Arc::new(ServerState {
        auth_version: 0,
        data_dir: args.folder,
    });

    loop {
        let (stream, _) = listener.accept().await?;
        let server_state_clone = server_state.clone();
        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            if let Err(error) = Builder::new()
                .serve_connection(io, service_fn(|request| handle_request(request, server_state_clone.clone())))
                .await
            {
                eprintln!("failed to serve connection: {}", error);
            }
        });
    }
}

