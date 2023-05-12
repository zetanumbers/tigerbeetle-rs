use std::{
    env,
    io::Cursor,
    path::{Path, PathBuf},
    process::Command,
};

use curl::easy::Easy;
use zip::ZipArchive;

const TIGERBEETLE_ZIP_URL: &str =
    "https://github.com/tigerbeetledb/tigerbeetle/archive/refs/tags/0.13.3.zip";

fn target_to_lib_dir(target: &str) -> Option<&'static str> {
    match target {
        "aarch64-unknown-linux-gnu" => Some("aarch64-linux-gnu"),
        "aarch64-unknown-linux-musl" => Some("aarch64-linux-musl"),
        "aarch64-apple-darwin" => Some("aarch64-macos"),
        "x86_64-unknown-linux-gnu" => Some("x86_64-linux-gnu"),
        "x86_64-unknown-linux-musl" => Some("x86_64-linux-musl"),
        "x86_64-apple-darwin" => Some("x86_64-macos"),
        "x86_64-pc-windows-msvc" => Some("x86_64-windows"),
        _ => None,
    }
}

#[cfg(unix)]
const SCRIPT_EXTENSION: &str = "sh";
#[cfg(windows)]
const SCRIPT_EXTENSION: &str = "bat";

fn main() {
    let out_dir: PathBuf = env::var("OUT_DIR").unwrap().into();
    let debug: bool = env::var("DEBUG").unwrap().parse().unwrap();
    let target = env::var("TARGET").unwrap();
    let target_lib_subdir =
        target_to_lib_dir(&target).unwrap_or_else(|| panic!("target {target:?} is not supported"));

    let mut zip = Vec::new();

    {
        // fetching data into `zip`
        let mut curl = Easy::new();
        curl.url(TIGERBEETLE_ZIP_URL).unwrap();
        curl.follow_location(true).unwrap();
        let mut transfer = curl.transfer();
        transfer
            .write_function(|data| {
                zip.extend_from_slice(data);
                Ok(data.len())
            })
            .unwrap();
        transfer.perform().expect("fetching tigerbeetle code");
    }

    let tigerbeetle_root = {
        // extracting `zip` into a directory
        let mut zip = ZipArchive::new(Cursor::new(zip))
            .expect("creating zip archive handle from fetched data");

        let mut root_files = zip
            .file_names()
            .map(Path::new)
            .filter(|p| p.iter().nth(1).is_none());
        let root = out_dir.join(root_files.next().expect("zip archive is empty"));
        assert_eq!(
            root_files.next(),
            None,
            "zip archive has multiple files at its root"
        );
        drop(root_files);

        zip.extract(&out_dir)
            .expect("extracting fetched tigerbeetle zip archive");

        root
    };

    let status = Command::new(
        tigerbeetle_root
            .join("scripts/install_zig")
            .with_extension(SCRIPT_EXTENSION),
    )
    .current_dir(&tigerbeetle_root)
    .status()
    .expect("running install script");
    assert!(status.success(), "install script failed with {status:?}");

    let status = Command::new(
        tigerbeetle_root
            .join("scripts/build")
            .with_extension(SCRIPT_EXTENSION),
    )
    .current_dir(&tigerbeetle_root)
    .arg("c_client")
    .args((!debug).then_some("-Drelease-safe"))
    .status()
    .expect("running build script");
    assert!(status.success(), "install script failed with {status:?}");

    let lib_dir = tigerbeetle_root.join("src/clients/c/lib");
    let link_search = lib_dir.join(target_lib_subdir);
    println!(
        "cargo:rustc-link-search=native={}",
        link_search
            .to_str()
            .expect("link search directory path is not valid unicode")
    );
    println!("cargo:rustc-link-lib=static=tb_client");

    println!("cargo:rerun-if-changed=src/wrapper.h");
    let wrapper = lib_dir.join("include/wrapper.h");
    std::fs::copy("src/wrapper.h", &wrapper).expect("copying wrapper.h");

    let bindings = bindgen::Builder::default()
        .header(
            wrapper
                .to_str()
                .expect("wrapper.h out path is not valid unicode"),
        )
        .bitfield_enum("TB_ACCOUNT_FLAGS")
        .bitfield_enum("TB_TRANSFER_FLAGS")
        .rustified_enum("TB_CREATE_ACCOUNT_RESULT")
        .rustified_enum("TB_CREATE_TRANSFER_RESULT")
        .rustified_enum("TB_OPERATION")
        .rustified_enum("TB_PACKET_STATUS")
        .rustified_enum("TB_STATUS")
        .parse_callbacks(Box::new(TbCallbacks))
        .generate()
        .expect("generating tb_client bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("writing tb_client bindings");

    println!("OUT_DIR = {out_dir:?}");
}

#[derive(Debug)]
struct TbCallbacks;

impl bindgen::callbacks::ParseCallbacks for TbCallbacks {
    fn enum_variant_name(
        &self,
        enum_name: Option<&str>,
        original_variant_name: &str,
        _variant_value: bindgen::callbacks::EnumVariantValue,
    ) -> Option<String> {
        let mut enum_name = enum_name?.strip_prefix("enum ")?;

        if !enum_name.starts_with("TB_") {
            return None;
        }

        loop {
            if let Some(new_variant_name) = original_variant_name
                .strip_prefix(enum_name)
                .and_then(|v| v.strip_prefix('_'))
            {
                return Some(new_variant_name.into());
            }
            (enum_name, _) = enum_name.rsplit_once('_')?;
        }
    }

    fn include_file(&self, filename: &str) {
        bindgen::CargoCallbacks.include_file(filename)
    }

    fn read_env_var(&self, key: &str) {
        bindgen::CargoCallbacks.read_env_var(key)
    }
}
