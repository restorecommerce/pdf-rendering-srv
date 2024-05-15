use crate::pdf_utils::{add_pdf_metadata, merge_pdfs};
use crate::proto::auth::Subject;
use crate::proto::pdf_rendering::info_response::ChromeVersion;
use crate::proto::pdf_rendering::pdf_rendering_service_server::PdfRenderingService;
use crate::proto::pdf_rendering::render_request::Type;
use crate::proto::pdf_rendering::{
    rendering_response, response_payload, CombinedRequest, IndividualRequest, IndividualResponse,
    InfoResponse, OutputOptions, RenderRequest, RenderingResponse, ResponsePayload,
    ResponsePayloadWithStatus, ResponsePdf, ResponseS3Upload,
};
use crate::proto::status;
use crate::proto::status::OperationStatus;
use crate::s3::upload_to_s3;
use crate::types::{IDExtension, InternalRequest, InternalResponse};
use config::Config;
use log::{debug, error, info};
use lopdf::Document;
use prost_wkt_types::Empty;
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

pub struct PDFServer {
    pub config: Config,
    pub renderer: mpsc::Sender<InternalRequest>,
}

#[tonic::async_trait]
impl PdfRenderingService for PDFServer {
    async fn render(
        &self,
        request: Request<RenderRequest>,
    ) -> Result<Response<RenderingResponse>, Status> {
        let (tx, mut rx) = mpsc::channel::<InternalResponse>(32);

        let id = request.extensions().get::<IDExtension>().unwrap();

        debug!("[{}] Rendering request: {:?}", id.id, request.get_ref());

        let data = match request.get_ref().clone().r#type.unwrap() {
            Type::Individual(req) => req.data.iter().map(|x| x.clone().data.unwrap()).collect(),
            Type::Combined(req) => req.data,
        };

        match self
            .renderer
            .send(InternalRequest {
                response: tx,
                data: data.clone(),
            })
            .await
        {
            Ok(_) => {}
            Err(err) => {
                error!("error sending rendering request: {}", err);
            }
        }

        let mut rendered = Vec::with_capacity(data.len());

        for _ in data.iter() {
            rendered.push(rx.recv().await);
        }

        info!("[{}] Rendering success", id.id);

        let output = match request.get_ref().clone().r#type.unwrap() {
            Type::Individual(req) => Ok(Self::individual_response(
                req,
                self.config.clone(),
                rendered,
                request.get_ref().clone().subject,
            )
            .await),
            Type::Combined(req) => Ok(Self::combined_response(
                req,
                self.config.clone(),
                rendered,
                request.get_ref().clone().subject,
            )
            .await),
        };

        return output;
    }

    async fn info(&self, _: Request<Empty>) -> Result<Response<InfoResponse>, Status> {
        let version = headless_chrome::Browser::default()
            .expect("failed opening browser")
            .get_version()
            .expect("failed fetching version");
        Ok(Response::new(InfoResponse {
            chrome: Some(ChromeVersion {
                js_version: version.js_version,
                product: version.product,
                protocol_version: version.protocol_version,
                revision: version.revision,
                user_agent: version.user_agent,
            }),
        }))
    }
}

impl PDFServer {
    async fn individual_response(
        req: IndividualRequest,
        config: Config,
        rendered: Vec<Option<InternalResponse>>,
        subject: Option<Subject>,
    ) -> Response<RenderingResponse> {
        let mut out = Vec::with_capacity(rendered.len());

        for (i, opt) in rendered.iter().enumerate() {
            match opt {
                None => {
                    out.push(ResponsePayloadWithStatus {
                        status: Some(status::Status {
                            id: None,
                            code: Some(500),
                            message: Some("unknown error".to_string()),
                        }),
                        payload: None,
                    });
                }
                Some(response) => match response {
                    Err(err) => {
                        out.push(ResponsePayloadWithStatus {
                            status: Some(status::Status {
                                id: None,
                                code: Some(400),
                                message: Some(format!("render failed: {}", err)),
                            }),
                            payload: None,
                        });
                    }
                    Ok(data) => {
                        let output = req.data[i].output.clone();

                        let mut out_data = data.clone();
                        if output.is_some() {
                            out_data = add_pdf_metadata(
                                out_data.clone(),
                                output.clone().unwrap().meta_data,
                            )
                            .expect("failed adding meta");
                        }

                        out.push(
                            Self::construct_response(
                                config.clone(),
                                out_data.clone(),
                                output,
                                subject.clone(),
                            )
                            .await,
                        )
                    }
                },
            }
        }

        Response::new(RenderingResponse {
            operation_status: Some(OperationStatus {
                code: Some(0),
                message: Some("success".to_string()),
            }),
            response: Some(rendering_response::Response::Individual(
                IndividualResponse {
                    rendering_response: out,
                },
            )),
        })
    }

    async fn combined_response(
        req: CombinedRequest,
        config: Config,
        rendered: Vec<Option<InternalResponse>>,
        subject: Option<Subject>,
    ) -> Response<RenderingResponse> {
        let mut merged = merge_pdfs(
            rendered
                .iter()
                .map(|x| match x {
                    None => {
                        panic!("missing pdf")
                    }
                    Some(r) => match r {
                        Err(err) => {
                            panic!("missing pdf: {}", err)
                        }
                        Ok(response) => Document::load_mem(response).expect("failed parsing pdf"),
                    },
                })
                .collect(),
        )
        .expect("render failed");

        if req.output.is_some() {
            merged = add_pdf_metadata(merged, req.output.clone().unwrap().meta_data)
                .expect("failed adding meta");
        }

        Response::new(RenderingResponse {
            operation_status: Some(OperationStatus {
                code: Some(0),
                message: Some("success".to_string()),
            }),
            response: Some(rendering_response::Response::Combined(
                Self::construct_response(
                    config.clone(),
                    merged.clone(),
                    req.output.clone(),
                    subject,
                )
                .await,
            )),
        })
    }

    async fn construct_response(
        config: Config,
        data: Vec<u8>,
        output: Option<OutputOptions>,
        subject: Option<Subject>,
    ) -> ResponsePayloadWithStatus {
        if output.clone().is_some() && output.clone().unwrap().upload_options.is_some() {
            match upload_to_s3(
                config.clone(),
                output.unwrap().upload_options.unwrap(),
                data.clone(),
                subject,
            )
            .await
            {
                Ok(_) => ResponsePayloadWithStatus {
                    status: Some(status::Status {
                        id: None,
                        code: Some(0),
                        message: Some("success".to_string()),
                    }),
                    payload: Some(ResponsePayload {
                        response: Some(response_payload::Response::UploadResult(
                            ResponseS3Upload {
                                length: data.len() as i32,
                                url: "n/a".to_string(),
                            },
                        )),
                    }),
                },
                Err(err) => ResponsePayloadWithStatus {
                    status: Some(status::Status {
                        id: None,
                        code: Some(400),
                        message: Some(format!("render failed: {}", err)),
                    }),
                    payload: None,
                },
            }
        } else {
            ResponsePayloadWithStatus {
                status: Some(status::Status {
                    id: None,
                    code: Some(0),
                    message: Some("success".to_string()),
                }),
                payload: Some(ResponsePayload {
                    response: Some(response_payload::Response::Pdf(ResponsePdf {
                        data: data.clone(),
                    })),
                }),
            }
        }
    }
}
