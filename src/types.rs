use crate::proto::pdf_rendering::RenderData;
use std::error::Error;
use tokio::sync::mpsc;

pub struct InternalRequest {
    pub data: Vec<RenderData>,
    pub response: mpsc::Sender<InternalResponse>,
}

pub type InternalResponse = anyhow::Result<Vec<u8>, Box<dyn Error + Send + Sync>>;

pub struct RendererResponse {
    pub resp: InternalResponse,
    pub order: usize,
}

pub struct IDExtension {
    pub id: ulid::Ulid,
}
