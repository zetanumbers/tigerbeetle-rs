use std::{
    env,
    fs::File,
    io::{Cursor, Write},
    path::{Path, PathBuf},
    process::Command,
};

use curl::easy::Easy;
use quote::quote;
use syn::visit::Visit;
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
        .default_enum_style(bindgen::EnumVariation::ModuleConsts)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("generating tb_client bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("writing tb_client bindings");

    let bindings = syn::parse_file(&bindings.to_string()).unwrap();
    let mut visitor = TigerbeetleVisitor::default();
    visitor.visit_file(&bindings);
    let mut f = std::io::BufWriter::new(File::create(out_dir.join("generated.rs")).unwrap());
    write!(f, "{}", visitor.output).unwrap();

    eprintln!("OUT_DIR = {out_dir:?}");
}

#[derive(Default)]
struct TigerbeetleVisitor {
    output: proc_macro2::TokenStream,
}

impl Visit<'_> for TigerbeetleVisitor {
    fn visit_item_mod(&mut self, i: &syn::ItemMod) {
        let enum_ident = i.ident.clone();
        let enum_name = enum_ident.to_string();
        let mut prefix_enum = enum_name.as_str();

        'process: {
            if !prefix_enum.starts_with("TB_") {
                break 'process;
            }

            let Some((_, content)) = &i.content else { break 'process };
            let mut type_exists = false;
            let mut variants = Vec::new();
            for item in content {
                match item {
                    syn::Item::Const(c) => variants.push((c.ident.to_string(), c.ident.clone())),
                    syn::Item::Type(t) if t.ident == "Type" && !type_exists => type_exists = true,
                    _ => break 'process,
                }
            }

            'remove_variant_prefix: {
                while !variants.iter().all(|(n, _)| n.starts_with(prefix_enum)) {
                    match prefix_enum.rsplit_once('_') {
                        Some((n, _)) => prefix_enum = n,
                        None => break 'remove_variant_prefix,
                    }
                }

                variants.iter_mut().for_each(|(n, _)| {
                    *n = n
                        .strip_prefix(prefix_enum)
                        .and_then(|n| n.strip_prefix('_'))
                        .unwrap()
                        .into()
                });
            }

            variants.iter_mut().for_each(|(n, _)| {
                *n = screaming_snake_case_into_camel_case(n);
            });

            let variants = variants.iter().map(|(n, v)| {
                let n = syn::Ident::new(n, v.span());
                quote!(#n = super:: #enum_ident :: #v)
            });
            let enum_ident = syn::Ident::new(
                &screaming_snake_case_into_camel_case(&enum_name),
                enum_ident.span(),
            );
            self.output
                .extend(quote!(#[repr(u32)] pub enum #enum_ident { #(#variants),* }))
        }

        syn::visit::visit_item_mod(self, i)
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
