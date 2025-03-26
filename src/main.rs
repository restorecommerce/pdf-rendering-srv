use config::{Case, Config, File};
use env_logger::WriteStyle;
use log::info;
use std::str::FromStr;
use std::{env, error::Error, net::ToSocketAddrs};
use tokio::sync::mpsc;

use tonic::codegen::InterceptedService;
use tonic::{transport::Server, Request, Status};
use tonic_health::ServingStatus;

use crate::proto::pdf_rendering::pdf_rendering_service_server::PdfRenderingServiceServer;
use crate::renderer::start_renderer;
use crate::server::PDFServer;
use crate::types::{IDExtension, InternalRequest};

mod pdf_utils;
mod proto;
mod renderer;
mod s3;
mod server;
mod types;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = load_config();

    let level = log::LevelFilter::from_str(
        config
            .get_string("logger.console.level")
            .unwrap_or("info".to_string())
            .as_str(),
    );

    env_logger::builder()
        .filter_level(level.expect("invalid level"))
        .write_style(WriteStyle::Always)
        .init();

    let metrics = tokio::runtime::Handle::current().metrics();
    info!("worker count: {}", metrics.num_workers());

    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::pdf_rendering::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

    let (health_reporter, health_service) = tonic_health::server::health_reporter();

    health_reporter
        .set_serving::<PdfRenderingServiceServer<PDFServer>>()
        .await;

    health_reporter
        .set_service_status("readiness", ServingStatus::Serving)
        .await;

    let (tx, rx) = mpsc::channel::<InternalRequest>(32);

    let pdf_server = PDFServer {
        config: config.clone(),
        renderer: tx,
    };

    start_renderer(rx).await?;

    let pdf_service =
        InterceptedService::new(
            PdfRenderingServiceServer::new(pdf_server)
                .max_decoding_message_size(
                    config.get_int("server.message_size_limit").unwrap() as usize
                )
                .max_encoding_message_size(
                    config.get_int("server.message_size_limit").unwrap() as usize
                ),
            logging,
        );

    let server = Server::builder()
        .add_service(health_service)
        .add_service(reflection_service)
        .add_service(pdf_service)
        .serve(
            (format!(
                "{}:{}",
                config.get_string("server.host").unwrap(),
                config.get_int("server.port").unwrap()
            ))
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap(),
        );

    info!(
        "Serving gRPC on port {}.",
        config.get_int("server.port").unwrap()
    );

    server.await?;

    Ok(())
}

fn logging(mut req: Request<()>) -> Result<Request<()>, Status> {
    let id = ulid::Ulid::new();

    req.extensions_mut().insert(IDExtension { id });

    info!("[{}] Received request: {:?}", id, req);
    Ok(req)
}

fn load_config() -> Config {
    let run_mode = env::var("NODE_ENV").unwrap_or_else(|_| "development".into());

    println!("Running in mode: {}", run_mode);

    Config::builder()
        .add_source(File::with_name("cfg/config"))
        .add_source(File::with_name(&format!("cfg/config_{}", run_mode)))
        .add_source(config::Environment::with_convert_case(Case::Lower).separator("__"))
        .build()
        .unwrap()
}
