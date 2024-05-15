use crate::proto::pdf_rendering::pdf_options::PaperFormat;
use crate::proto::pdf_rendering::render_source::Content;
use crate::proto::pdf_rendering::{RenderData, RenderOptions};
use crate::types::{InternalRequest, RendererResponse};
use anyhow::{anyhow, Result};
use headless_chrome::browser::default_executable;
use headless_chrome::types::PrintToPdfOptions;
use headless_chrome::{Browser, LaunchOptionsBuilder, Tab};
use std::error::Error;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;

impl PaperFormat {
    pub fn width(&self) -> f32 {
        match self {
            PaperFormat::A0 => 33.1,
            PaperFormat::A1 => 23.4,
            PaperFormat::A2 => 16.5,
            PaperFormat::A3 => 11.7,
            PaperFormat::A4 => 8.27,
            PaperFormat::A5 => 5.83,
            PaperFormat::A6 => 4.13,
            PaperFormat::A7 => 2.91,
            PaperFormat::Letter => 8.5,
            PaperFormat::Legal => 8.5,
            PaperFormat::Tabloid => 11.0,
        }
    }

    pub fn height(&self) -> f32 {
        match self {
            PaperFormat::A0 => 46.8,
            PaperFormat::A1 => 33.1,
            PaperFormat::A2 => 23.4,
            PaperFormat::A3 => 16.5,
            PaperFormat::A4 => 11.69,
            PaperFormat::A5 => 8.27,
            PaperFormat::A6 => 5.83,
            PaperFormat::A7 => 4.13,
            PaperFormat::Letter => 11.0,
            PaperFormat::Legal => 14.0,
            PaperFormat::Tabloid => 17.0,
        }
    }
}

pub fn content_to_pdf(
    tab: Arc<Tab>,
    content: Content,
    options: Option<RenderOptions>,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    let mut landscape = None;
    let mut display_header_footer = None;
    let mut print_background = Some(true);
    let mut format = Some(PaperFormat::A4);
    let mut scale = None;
    let mut paper_width = None;
    let mut paper_height = None;
    let mut margin_top = Some(0.0);
    let mut margin_bottom = Some(0.0);
    let mut margin_left = Some(0.0);
    let mut margin_right = Some(0.0);
    let mut page_ranges = None;
    let mut ignore_invalid_page_ranges = None;
    let mut header_template = None;
    let mut footer_template = None;
    let mut prefer_css_page_size = Some(true);

    match options {
        None => {}
        Some(opt) => match opt.puppeteer_options {
            None => {}
            Some(puppeteer) => match puppeteer.pdf_options {
                None => {}
                Some(pdf) => {
                    paper_width = pdf.paper_width.or(paper_width);
                    paper_height = pdf.paper_height.or(paper_height);

                    landscape = pdf.landscape.or(landscape);
                    display_header_footer = pdf.display_header_footer.or(display_header_footer);
                    print_background = pdf.print_background.or(print_background);
                    format = pdf
                        .format
                        .map(|t| PaperFormat::try_from(t).unwrap())
                        .or(format);
                    scale = pdf.scale.or(scale);
                    paper_width = pdf.paper_width.or(paper_width);
                    paper_height = pdf.paper_height.or(paper_height);
                    margin_top = pdf.margin_top.or(margin_top);
                    margin_bottom = pdf.margin_bottom.or(margin_bottom);
                    margin_left = pdf.margin_left.or(margin_left);
                    margin_right = pdf.margin_right.or(margin_right);
                    page_ranges = pdf.page_ranges.or(page_ranges);
                    ignore_invalid_page_ranges = pdf
                        .ignore_invalid_page_ranges
                        .or(ignore_invalid_page_ranges);
                    header_template = pdf.header_template.or(header_template);
                    footer_template = pdf.footer_template.or(footer_template);
                    prefer_css_page_size = pdf.prefer_css_page_size.or(prefer_css_page_size);
                }
            },
        },
    }

    paper_width = paper_width.or(Some(format.unwrap().width()));
    paper_height = paper_height.or(Some(format.unwrap().height()));

    let pdf_options = PrintToPdfOptions {
        paper_width: paper_width.map(|t| t as f64),
        paper_height: paper_height.map(|t| t as f64),
        margin_top: margin_top.map(|t| t as f64),
        margin_right: margin_right.map(|t| t as f64),
        margin_bottom: margin_bottom.map(|t| t as f64),
        margin_left: margin_left.map(|t| t as f64),
        scale: scale.map(|t| t as f64),
        landscape,
        prefer_css_page_size,
        print_background,
        page_ranges,
        ignore_invalid_page_ranges,
        header_template,
        display_header_footer,
        footer_template,
        ..Default::default()
    };

    let pdf = match content {
        Content::Url(url) => tab
            .navigate_to(url.as_str())?
            .wait_until_navigated()?
            .print_to_pdf(Some(pdf_options))?,
        Content::Html(data) => {
            let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").unwrap());

            let response = tiny_http::Response::new(
                200.into(),
                vec![
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap(),
                ],
                io::Cursor::new(data.clone()),
                Some(data.clone().len()),
                None,
            );

            let srv = server.clone();
            std::thread::spawn(move || {
                let request = match srv.recv() {
                    Ok(rq) => rq,
                    Err(e) => {
                        panic!("error: {}", e);
                    }
                };

                let _ = request.respond(response);

                drop(srv)
            });

            tab.navigate_to(
                format!(
                    "http://127.0.0.1:{}",
                    server.server_addr().to_ip().unwrap().port()
                )
                .as_str(),
            )?
            .wait_until_navigated()?
            .print_to_pdf(Some(pdf_options))?
        }
    };

    tab.close(true)?;

    Ok(pdf)
}

pub async fn start_renderer(mut rx: Receiver<InternalRequest>) -> Result<(), Box<dyn Error>> {
    let options = LaunchOptionsBuilder::default()
        .path(Some(default_executable().map_err(|e| anyhow!(e))?))
        .sandbox(false)
        .idle_browser_timeout(Duration::MAX)
        .build()
        .unwrap();

    tokio::spawn(async move {
        let browser = Arc::new(Browser::new(options).expect("failed instantiating browser"));

        while let Some(cmd) = rx.recv().await {
            handle_cmd(browser.clone(), cmd);
        }
    });

    Ok(())
}

pub fn handle_cmd(browser: Arc<Browser>, cmd: InternalRequest) {
    tokio::spawn(async move {
        let (tx, mut rx) = mpsc::channel::<RendererResponse>(32);

        let data = cmd.data;
        for (i, req) in data.iter().enumerate() {
            let tab = browser.new_tab().expect("failed opening new browser tab");
            handle_req(tab, req, tx.clone(), i);
        }

        let mut rendered = Vec::with_capacity(data.clone().len());

        for _ in data.clone().iter() {
            rendered.push(rx.recv().await.unwrap());
        }

        rendered.sort_by_key(|r| r.order);

        for out in rendered {
            let _ = cmd.response.send(out.resp).await;
        }
    });
}

pub fn handle_req(
    tab: Arc<Tab>,
    req: &RenderData,
    tx: mpsc::Sender<RendererResponse>,
    order: usize,
) {
    let req2 = req.clone();
    tokio::spawn(async move {
        let content = req2
            .clone()
            .source
            .expect("no source")
            .content
            .expect("no content");
        let options = req2.clone().options;
        let out = content_to_pdf(tab, content, options.clone());
        let _ = tx.clone().send(RendererResponse { resp: out, order }).await;
    });
}
