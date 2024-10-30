use flate2::read::GzDecoder;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tar::Archive;

// npm view @restorecommerce/protos dist.tarball
const PROTO_URL: &str = "https://registry.npmjs.org/@restorecommerce/protos/-/protos-6.8.0.tgz";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("protos");
    let relative_path = dest_path.strip_prefix(env::current_dir()?)?;

    let resp = reqwest::blocking::get(PROTO_URL).expect("request failed");
    let tar = GzDecoder::new(resp);
    let mut archive = Archive::new(tar);
    archive.unpack(relative_path)?;

    let proto_path = Path::new(&relative_path).join("package/io/restorecommerce/");
    fs::copy(
        Path::new(&env::current_dir()?).join("pdf_rendering.proto"),
        Path::new(&proto_path).join("pdf_rendering.proto").clone(),
    )?;

    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_well_known_types(true)
        .extern_path(
            ".google.protobuf.BytesValue",
            "::prost::alloc::vec::Vec<u8>",
        )
        .extern_path(
            ".google.protobuf.StringValue",
            "::prost::alloc::string::String",
        )
        .extern_path(".google.protobuf", "::prost_wkt_types")
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .file_descriptor_set_path(Path::new(&out_dir).join("proto_fd.bin"))
        .compile_protos(
            &[Path::new(&proto_path).join("pdf_rendering.proto").clone()],
            &[Path::new(&relative_path).join("package/")],
        )?;

    let paths: Vec<PathBuf> = fs::read_dir(proto_path.clone())
        .expect("")
        .filter(|p| {
            p.as_ref()
                .unwrap()
                .file_name()
                .to_str()
                .unwrap()
                .to_string()
                .contains(".proto")
        })
        .filter(|p| {
            !p.as_ref()
                .unwrap()
                .file_name()
                .to_str()
                .unwrap()
                .to_string()
                .eq("pdf_rendering.proto")
        })
        .map(|p| p.unwrap().path())
        .collect();

    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_well_known_types(true)
        .extern_path(
            ".google.protobuf.BytesValue",
            "::prost::alloc::vec::Vec<u8>",
        )
        .extern_path(
            ".google.protobuf.StringValue",
            "::prost::alloc::string::String",
        )
        .extern_path(".google.protobuf", "::prost_wkt_types")
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(&paths, &[Path::new(&relative_path).join("package/")])?;

    Ok(())
}
