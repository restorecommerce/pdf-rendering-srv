use crate::proto::attribute::Attribute;
use crate::proto::auth::Subject;
use crate::proto::meta;
use crate::proto::pdf_rendering::UploadOptions;
use crate::proto::user::user_service_client::UserServiceClient;
use crate::proto::user::FindByTokenRequest;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::put_object::{PutObjectError, PutObjectOutput};
use aws_sdk_s3::primitives::ByteStream;
use aws_smithy_runtime_api::client::orchestrator::HttpResponse;
use config::Config;
use serde::Serialize;
use serde_json::json;
use std::time::SystemTime;

pub async fn upload_to_s3(
    config: Config,
    upload_opt: UploadOptions,
    data: Vec<u8>,
    subject: Option<Subject>,
) -> Result<PutObjectOutput, SdkError<PutObjectError, HttpResponse>> {
    let endpoint = config.get_string("s3.client.endpoint").unwrap();
    let region = config.get_string("s3.client.region").unwrap();
    let access_key = config.get_string("s3.client.access_key").unwrap();
    let secret_key = config.get_string("s3.client.secret_key").unwrap();

    let s3_config = aws_sdk_s3::Config::builder()
        .behavior_version_latest()
        .endpoint_url(endpoint)
        .region(aws_sdk_config::config::Region::new(region))
        .credentials_provider(Credentials::new(
            access_key, secret_key, None, None, "Static",
        ))
        .force_path_style(
            config
                .get_bool("s3.client.s3_force_path_style")
                .unwrap_or(false),
        )
        .build();

    let client = aws_sdk_s3::Client::from_conf(s3_config);

    let bucket_name = upload_opt.bucket.unwrap();
    let key = upload_opt.key.unwrap();
    let meta = create_metadata(config.clone(), subject.clone());
    let mut subject_value = "{}".to_owned();

    if subject.clone().is_some() && subject.clone().unwrap().token.is_some() {
        let mut ids_client =
            UserServiceClient::connect(config.get_string("client.user.address").unwrap())
                .await
                .unwrap();

        let response = ids_client
            .find_by_token(tonic::Request::new(FindByTokenRequest {
                token: subject.clone().unwrap().token,
            }))
            .await
            .unwrap();

        match response.get_ref().clone().payload {
            None => {}
            Some(user) => {
                subject_value = json!({
                  "id": user.id.unwrap()
                })
                .to_string();
            }
        }
    }

    client
        .put_object()
        .bucket(bucket_name.clone())
        .key(key.clone())
        .body(ByteStream::from(data.clone()))
        .content_type("application/pdf")
        .set_content_disposition(upload_opt.content_disposition)
        .metadata("Data", "{}")
        .metadata("Key", key.clone())
        .metadata(
            "Meta",
            serde_json::to_string(&meta).expect("failed json serialization"),
        )
        .metadata("Subject", subject_value)
        .send()
        .await
}

#[derive(Debug, Default, Serialize)]
struct Resource {
    id: String,
    key: String,
    bucket: String,
    meta: meta::Meta,
}

fn create_metadata(config: Config, subject: Option<Subject>) -> meta::Meta {
    let mut out = meta::Meta::default();

    let now: prost_wkt_types::Timestamp = SystemTime::now().into();
    out.created = Some(now.clone());
    out.modified = Some(now.clone());

    out.created_by = subject.clone().and_then(|s| s.id);
    out.modified_by = subject.clone().and_then(|s| s.id);

    match subject.clone() {
        None => {}
        Some(s) => match s.scope {
            None => {}
            Some(target_scope) => out.owners.push(Attribute {
                id: Some(
                    config
                        .get_string("authorization.urns.ownerIndicatoryEntity")
                        .unwrap(),
                ),
                value: Some(
                    config
                        .get_string("authorization.urns.organization")
                        .unwrap(),
                ),
                attributes: vec![Attribute {
                    id: Some(
                        config
                            .get_string("authorization.urns.ownerInstance")
                            .unwrap(),
                    ),
                    value: Some(target_scope),
                    attributes: Vec::new(),
                }],
            }),
        },
    }

    out
}
