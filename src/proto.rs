pub mod attribute {
    tonic::include_proto!("io.restorecommerce.attribute");
}
pub mod status {
    tonic::include_proto!("io.restorecommerce.status");
}
pub mod auth {
    tonic::include_proto!("io.restorecommerce.auth");
}
pub mod meta {
    tonic::include_proto!("io.restorecommerce.meta");
}
pub mod user {
    tonic::include_proto!("io.restorecommerce.user");
}
pub mod image {
    tonic::include_proto!("io.restorecommerce.image");
}
pub mod role {
    tonic::include_proto!("io.restorecommerce.role");
}
pub mod resourcebase {
    tonic::include_proto!("io.restorecommerce.resourcebase");
}
pub mod filter {
    tonic::include_proto!("io.restorecommerce.filter");
}

pub mod pdf_rendering {
    tonic::include_proto!("io.restorecommerce.pdf_rendering");

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("proto_fd");
}
