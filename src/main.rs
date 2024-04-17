use config::{Case, Config, File};
use env_logger::WriteStyle;
use log::info;
use std::str::FromStr;
use std::{env, error::Error, net::ToSocketAddrs};
use tokio::sync::mpsc;

use tonic::{transport::Server, Request, Status};
use tonic_health::ServingStatus;

use crate::proto::pdf_rendering::pdf_rendering_service_server::PdfRenderingServiceServer;
use crate::renderer::start_renderer;
use crate::server::PDFServer;
use crate::types::InternalRequest;

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

    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::pdf_rendering::FILE_DESCRIPTOR_SET)
        .build()
        .unwrap();

    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();

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

    let server = Server::builder()
        .add_service(health_service)
        .add_service(reflection_service)
        .add_service(PdfRenderingServiceServer::with_interceptor(
            pdf_server, logging,
        ))
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

fn logging(req: Request<()>) -> Result<Request<()>, Status> {
    info!("Received request: {:?}", req);
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
