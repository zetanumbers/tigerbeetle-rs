use std::{
    env,
    io::Cursor,
    path::{Path, PathBuf},
    process::Command,
};

use curl::easy::Easy;
use quote::ToTokens;
use syn::fold::Fold;
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
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: true,
        })
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("generating tb_client bindings");

    let mut bindings = syn::parse_file(&bindings.to_string()).unwrap();
    let mut fold = TigerbeetleFold;
    bindings = fold.fold_file(bindings);
    std::fs::write(
        out_dir.join("bindings.rs"),
        bindings.into_token_stream().to_string(),
    )
    .expect("writing tb_client bindings");

    println!("OUT_DIR = {out_dir:?}");
}

struct TigerbeetleFold;

impl Fold for TigerbeetleFold {
    fn fold_item_enum(&mut self, mut i: syn::ItemEnum) -> syn::ItemEnum {
        let enum_name = i.ident.to_string();
        let mut prefix_enum = enum_name.as_str();

        if prefix_enum.starts_with("TB_") {
            let mut variant_names: Vec<_> =
                i.variants.iter().map(|v| v.ident.to_string()).collect();

            'remove_variant_prefix: {
                while !variant_names.iter().all(|n| n.starts_with(prefix_enum)) {
                    match prefix_enum.rsplit_once('_') {
                        Some((n, _)) => prefix_enum = n,
                        None => break 'remove_variant_prefix,
                    }
                }

                variant_names.iter_mut().for_each(|n| {
                    *n = n
                        .strip_prefix(prefix_enum)
                        .and_then(|n| n.strip_prefix('_'))
                        .unwrap()
                        .into()
                });
            }

            i.ident = syn::Ident::new(
                &screaming_snake_case_into_camel_case(&enum_name),
                i.ident.span(),
            );

            variant_names
                .iter_mut()
                .for_each(|n| *n = screaming_snake_case_into_camel_case(n));

            for (v, n) in i.variants.iter_mut().zip(&variant_names) {
                v.ident = syn::Ident::new(n, v.ident.span());
            }
        }

        syn::fold::fold_item_enum(self, i)
    }

    fn fold_path(&mut self, mut i: syn::Path) -> syn::Path {
        if let Some(segment) = i.segments.last_mut() {
            let ident = segment.ident.to_string();
            if ident.starts_with("TB_") {
                segment.ident = syn::Ident::new(
                    &screaming_snake_case_into_camel_case(&ident),
                    segment.ident.span(),
                );
            }
        }
        syn::fold::fold_path(self, i)
    }
}

fn screaming_snake_case_into_camel_case(src: &str) -> String {
    let mut dst = String::with_capacity(src.len());
    for word in src.split('_') {
        let mut chars = word.chars();
        let Some(ch) = chars.next() else { continue };
        assert!(ch.is_ascii_uppercase());
        dst.push(ch);
        dst.extend(chars.map(|c| c.to_ascii_lowercase()));
    }
    dst
}
