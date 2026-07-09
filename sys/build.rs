use std::{
    collections::BTreeMap,
    env,
    ffi::{OsStr, OsString},
    fs::{self, File},
    io::{self, Write},
    iter,
    path::{self, Path, PathBuf},
    process::Command,
};

use quote::{quote, ToTokens as _};
use syn::{parse_quote, visit::Visit, visit_mut::VisitMut};

/// Version of the used [TigerBeetle] release.
///
/// [TigerBeetle]: https://github.com/tigerbeetle/tigerbeetle
const TIGERBEETLE_RELEASE: &str = "0.17.1";

/// Commit hash of the [`TIGERBEETLE_RELEASE`].
const TIGERBEETLE_COMMIT: &str = "bbe61abcdc7dad53b62be50eaf584e868a3e609f";

fn target_to_lib_dir(target: &str) -> Option<&'static str> {
    match target {
        "aarch64-unknown-linux-gnu" => Some("aarch64-linux-gnu.2.27"),
        "aarch64-unknown-linux-musl" => Some("aarch64-linux-musl"),
        "aarch64-apple-darwin" => Some("aarch64-macos"),
        "x86_64-unknown-linux-gnu" => Some("x86_64-linux-gnu.2.27"),
        "x86_64-unknown-linux-musl" => Some("x86_64-linux-musl"),
        "x86_64-apple-darwin" => Some("x86_64-macos"),
        "x86_64-pc-windows-gnu" => Some("x86_64-windows"),
        _ => None,
    }
}

fn target_to_tigerbeetle_target(target: &str) -> Option<&'static str> {
    match target {
        "aarch64-unknown-linux-gnu" => Some("aarch64-linux"),
        "aarch64-unknown-linux-musl" => Some("aarch64-linux-musl"),
        "aarch64-apple-darwin" => Some("aarch64-macos"),
        "x86_64-unknown-linux-gnu" => Some("x86_64-linux"),
        "x86_64-unknown-linux-musl" => Some("x86_64-linux-musl"),
        "x86_64-apple-darwin" => Some("x86_64-macos"),
        "x86_64-pc-windows-gnu" => Some("x86_64-windows"),
        _ => None,
    }
}

fn main() {
    assert!(env!("CARGO_PKG_VERSION").ends_with(TIGERBEETLE_RELEASE));
    let out_dir: PathBuf = env::var("OUT_DIR").unwrap().into();
    let debug: bool = env::var("TB_CLIENT_DEBUG").map_or_else(
        |_| env::var("DEBUG").unwrap().parse().unwrap(),
        |s| s.parse().unwrap(),
    );
    let target = env::var("TARGET").unwrap();

    println!("cargo:rerun-if-env-changed=DOCS_RS");
    println!("cargo:rerun-if-env-changed=TB_CLIENT_DEBUG");
    println!("cargo:rerun-if-env-changed=ZIG_PATH");
    println!("cargo:rerun-if-changed=src/wrapper.h");

    let wrapper;
    if env::var("DOCS_RS").is_ok() {
        wrapper = "src/wrapper.h".into();
    } else {
        let target_lib_subdir = target_to_lib_dir(&target)
            .unwrap_or_else(|| panic!("target `{target:?}` is not supported"));

        let tigerbeetle_target = target_to_tigerbeetle_target(&target)
            .unwrap_or_else(|| panic!("target `{target:?}` is not supported"));

        let tigerbeetle_root = out_dir.join("tigerbeetle");
        fs::remove_dir_all(&tigerbeetle_root)
            .or_else(|e| {
                if let io::ErrorKind::NotFound = e.kind() {
                    Ok(())
                } else {
                    Err(e)
                }
            })
            .unwrap();
        create_mirror(
            "tigerbeetle".as_ref(),
            &tigerbeetle_root,
            &["src/clients/c/lib", "zig-cache", "zig-out", ".git"]
                .into_iter()
                .collect(),
        );

        // Check for `ZIG_PATH` env var to use system preinstalled Zig instead of downloading one.
        let zig_path: PathBuf = if let Some(zig_path) = env::var_os("ZIG_PATH") {
            let path = PathBuf::from(&zig_path);
            assert!(
                path.exists(),
                "`ZIG_PATH` is set to `{}` but the file does not exist",
                path.display(),
            );
            path
        } else {
            // Download ZIG using the bundled downloader script.
            let status = if cfg!(windows) {
                let mut cmd = Command::new("pwsh");
                _ = cmd.arg(tigerbeetle_root.join("zig/download.win.ps1"));
                cmd
            } else {
                // TODO: Use the original `zig/download.sh` once tigerbeetle/tigerbeetle@394d012c
                //       gets released.

                let orig_path = tigerbeetle_root.join("zig/download.sh");
                let patched_path = out_dir.join("zig_download.patched.sh");

                let download_script = fs::read_to_string(&orig_path)
                    .expect("failed to read `zig/download.sh`")
                    .replace(
                        "curl --silent --output",
                        "curl --location --silent --output",
                    );
                fs::copy(&orig_path, &patched_path).expect("failed to copy `zig/download.sh`");
                fs::write(&patched_path, download_script)
                    .expect("failed to patch `zig/download.sh`");

                Command::new(patched_path)
            }
            .current_dir(&tigerbeetle_root)
            .status()
            .expect("running `download` script");
            assert!(status.success(), "`download` script failed with {status:?}");

            tigerbeetle_root
                .join("zig/zig")
                .with_extension(env::consts::EXE_EXTENSION)
                .canonicalize()
                .unwrap()
        };

        let status = Command::new(&zig_path)
            .arg("build")
            .arg("clients:c")
            .args((!debug).then_some("-Drelease"))
            .arg(format!("-Dtarget={tigerbeetle_target}"))
            .arg(format!("-Dconfig-release={TIGERBEETLE_RELEASE}"))
            .arg(format!("-Dconfig-release-client-min={TIGERBEETLE_RELEASE}"))
            .arg(format!("-Dgit-commit={TIGERBEETLE_COMMIT}"))
            .current_dir(&tigerbeetle_root)
            .env_remove("CI")
            .status()
            .expect("running `zig build` subcommand");
        assert!(status.success(), "`zig build` failed with {status:?}");

        let c_dir = tigerbeetle_root.join("src/clients/c/");
        let lib_dir = tigerbeetle_root.join("src/clients/c/lib");
        let link_search = lib_dir.join(target_lib_subdir);
        println!(
            "cargo:rustc-link-search=native={}",
            link_search
                .to_str()
                .expect("link search directory path is not valid unicode"),
        );
        if target == "x86_64-pc-windows-gnu" {
            // `-gnu` toolchain looks for `lib<name>.a` file of a static library by default, but
            // `zig build` produces `<name>.lib` despite using MinGW under-the-hood.
            println!("cargo:rustc-link-lib=static:+verbatim=tb_client.lib");
            // As of Rust 1.87, its `std` doesn't link `advapi32` automatically anymore, however
            // the `tb_client` requires it.
            // See: https://github.com/rust-lang/rust/pull/138233
            //      https://github.com/rust-lang/rust/issues/139352
            println!("cargo:rustc-link-lib=advapi32");
        } else {
            println!("cargo:rustc-link-lib=static=tb_client");
        }

        wrapper = c_dir.join("wrapper.h");
        let generated_header = c_dir.join("tb_client.h");
        assert_eq!(
            fs::read_to_string(&generated_header).expect("reading generated `tb_client.h`"),
            fs::read_to_string("src/tb_client.h")
                .expect("reading pre-generated `tb_client.h`")
                .replace("\r\n", "\n"),
            "generated and pre-generated `tb_client.h` headers must be equal, \
             generated at: {generated_header:?}",
        );
        fs::copy("src/wrapper.h", &wrapper).expect("copying `wrapper.h`");
    };

    let bindings = bindgen::Builder::default()
        .header(
            wrapper
                .to_str()
                .expect("`wrapper.h` out path is not valid unicode"),
        )
        .default_enum_style(bindgen::EnumVariation::ModuleConsts)
        .parse_callbacks(Box::new(TigerbeetleCallbacks {
            inner: bindgen::CargoCallbacks::default(),
            out_dir: out_dir.clone(),
        }))
        .generate()
        .expect("generating `tb_client` bindings");

    let mut bindings = syn::parse_file(&bindings.to_string()).unwrap();
    LintSuppressionVisitor.visit_file_mut(&mut bindings);

    let bindings_path = out_dir.join("bindings.rs");
    fs::write(&bindings_path, bindings.to_token_stream().to_string())
        .expect("writing `tb_client` bindings");
    rustfmt(bindings_path);

    if env::var("CARGO_FEATURE_GENERATED_SAFE").is_ok() {
        let mut visitor = TigerbeetleVisitor::default();
        visitor.visit_file(&bindings);

        let generated_path = out_dir.join("generated.rs");
        let mut f = io::BufWriter::new(File::create(&generated_path).unwrap());
        write!(f, "{}", visitor.output).unwrap();
        drop(f);

        rustfmt(generated_path);
    }
}

struct LintSuppressionVisitor;

impl VisitMut for LintSuppressionVisitor {
    fn visit_foreign_item_fn_mut(&mut self, i: &mut syn::ForeignItemFn) {
        // As of MSRV 1.78, `u128` is FFI-compatible:
        // https://blog.rust-lang.org/2024/03/30/i128-layout-update
        if i.sig.ident == "tb_client_init_parameters" {
            i.attrs.push(parse_quote! {
                #[allow(improper_ctypes)]
            });
        }

        syn::visit_mut::visit_foreign_item_fn_mut(self, i)
    }
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

            let Some((_, content)) = &i.content else {
                break 'process;
            };
            let mut type_exists = false;
            let mut variants = Vec::new();
            assert!(content.len() > 1);
            for item in content {
                match item {
                    syn::Item::Const(c) => {
                        let syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Int(i),
                            ..
                        }) = &*c.expr
                        else {
                            break 'process;
                        };
                        let i = i.base10_parse::<u32>().unwrap();
                        variants.push((c.ident.to_string(), c.ident.clone(), i));
                    }
                    syn::Item::Type(t) if t.ident == "Type" && !type_exists => type_exists = true,
                    _ => break 'process,
                }
            }

            'remove_variant_prefix: {
                while !variants.iter().all(|(n, _, _)| n.starts_with(prefix_enum)) {
                    match prefix_enum.rsplit_once('_') {
                        Some((n, _)) => prefix_enum = n,
                        None => break 'remove_variant_prefix,
                    }
                }

                variants.iter_mut().for_each(|(n, _, _)| {
                    *n = n
                        .strip_prefix(prefix_enum)
                        .and_then(|n| n.strip_prefix('_'))
                        .unwrap()
                        .into()
                });
            }

            variants.sort_unstable_by_key(|(_, _, i)| *i);

            let mut new_enum_name =
                screaming_snake_case_into_camel_case(enum_name.strip_prefix("TB_").unwrap());
            let mut new_enum_ident = syn::Ident::new(&new_enum_name, enum_ident.span());

            if enum_name.ends_with("_FLAGS") {
                let ty = syn::Ident::new(
                    match enum_name.as_str() {
                        "TB_ACCOUNT_FILTER_FLAGS" | "TB_QUERY_FILTER_FLAGS" => "u32",
                        "TB_ACCOUNT_FLAGS" | "TB_TRANSFER_FLAGS" => "u16",
                        other => panic!("unexpected flags type name: {other}"),
                    },
                    enum_ident.span(),
                );
                let variants = variants.iter().map(|(n, v, _)| {
                    let n = syn::Ident::new(n, v.span());
                    quote!(const #n = super:: #enum_ident :: #v as #ty;)
                });
                self.output.extend(quote! {
                    ::bitflags::bitflags! {
                        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
                        pub struct #new_enum_ident: #ty {
                            #(#variants)*
                        }
                    }
                })
            } else {
                variants.iter_mut().for_each(|(n, _, _)| {
                    *n = screaming_snake_case_into_camel_case(n);
                });

                let mut errorize = false;
                let mut repr_type = "u32";
                if let Some(n) = new_enum_name.strip_suffix("Result") {
                    new_enum_name = format!("{n}ErrorKind");
                    new_enum_ident = syn::Ident::new(&new_enum_name, new_enum_ident.span());
                    errorize = true;
                }
                match new_enum_name.as_str() {
                    "ClientStatus"
                    | "InitStatus"
                    | "PacketStatus"
                    | "PacketAcquireStatus"
                    | "RegisterLogCallbackStatus" => {
                        if new_enum_name == "PacketStatus" {
                            repr_type = "u8";
                        }
                        new_enum_name = format!("{new_enum_name}ErrorKind");
                        new_enum_ident = syn::Ident::new(&new_enum_name, new_enum_ident.span());
                        errorize = true;
                    }
                    "Operation" => {
                        repr_type = "u8";
                        new_enum_name = "OperationKind".into();
                        new_enum_ident = syn::Ident::new(&new_enum_name, new_enum_ident.span());
                    }
                    _ => (),
                }

                let repr_type = syn::Ident::new(repr_type, proc_macro2::Span::call_site());

                if errorize {
                    let first_variant = variants.first().unwrap();
                    assert!(
                        matches!(first_variant.0.as_str(), "Ok" | "Success"),
                        "variant name is {:?}",
                        first_variant.0,
                    );
                    assert_eq!(first_variant.2, 0);
                    variants.remove(0);
                }

                let all_codes = variants
                    .iter()
                    .filter_map(|(_, _, c)| (*c != 0).then_some(*c))
                    .collect::<Vec<_>>();
                let min_code = *all_codes.iter().min().unwrap();
                let max_code = *all_codes.iter().max().unwrap();
                let excluded_codes = (min_code..=max_code)
                    .filter(|c| !all_codes.contains(c))
                    .collect::<Vec<_>>();

                let minmax_prefix = enum_name
                    .strip_suffix("_RESULT")
                    .unwrap_or(&enum_name)
                    .strip_prefix("TB_")
                    .unwrap();
                let error = if errorize { "_ERROR" } else { "" };

                let min_name = syn::Ident::new(
                    &format!("MIN_{minmax_prefix}{error}_CODE"),
                    proc_macro2::Span::call_site(),
                );
                let max_name = syn::Ident::new(
                    &format!("MAX_{minmax_prefix}{error}_CODE"),
                    proc_macro2::Span::call_site(),
                );
                let min_code =
                    syn::LitInt::new(&min_code.to_string(), proc_macro2::Span::call_site());
                let max_code =
                    syn::LitInt::new(&max_code.to_string(), proc_macro2::Span::call_site());
                let excluded_const = (!excluded_codes.is_empty()).then(|| {
                    let excl_name = syn::Ident::new(
                        &format!("EXCLUDED_{minmax_prefix}{error}_CODES"),
                        proc_macro2::Span::call_site(),
                    );
                    let excl_codes = excluded_codes
                        .iter()
                        .map(|c| syn::LitInt::new(&c.to_string(), proc_macro2::Span::call_site()));
                    quote! {
                        pub const #excl_name: &[#repr_type] = &[#(#excl_codes),*];
                    }
                });
                let extra = quote! {
                    pub const #min_name: #repr_type = #min_code;
                    pub const #max_name: #repr_type = #max_code;
                    #excluded_const
                };

                let from_snake_case_str_branches = variants
                    .iter()
                    .map(|(s, v, _)| {
                        let n = syn::Ident::new(s, v.span());
                        let s = camel_case_into_snake_case(s);
                        quote!(#s => Some(Self:: #n))
                    })
                    .chain(iter::once(quote!(
                        _ => None
                    )));

                let into_snake_case_str_branches = variants
                    .iter()
                    .map(|(s, v, _)| {
                        let n = syn::Ident::new(s, v.span());
                        let s = camel_case_into_snake_case(s);
                        quote!(Self:: #n => #s)
                    })
                    .chain(iter::once(quote!(
                        Self::UnstableUncategorized => unimplemented!("variant is not supported yet")
                    )));

                let variants = variants
                    .iter()
                    .map(|(n, v, _)| {
                        let n = syn::Ident::new(n, v.span());
                        quote!(#n = super:: #enum_ident :: #v as #repr_type)
                    })
                    .chain(iter::once(quote!(
                        #[doc(hidden)]
                        UnstableUncategorized
                    )));

                let first_doc_str_from_snake_case_str =
                    format!("Try parsing [`{new_enum_name}`] from a string slice");
                let first_doc_str_into_snake_case_str = format!(
                    "Returns a static string slice according to [`{new_enum_name}`] variant's name but in snake_case"
                );

                self.output.extend(quote! {
                    #[derive(Debug, Clone, Copy)]
                    #[non_exhaustive]
                    #[repr( #repr_type )]
                    pub enum #new_enum_ident {
                        #(#variants),*
                    }

                    impl #new_enum_ident {
                        #[doc = #first_doc_str_from_snake_case_str]
                        #[doc = ""]
                        #[doc = "# Stability"]
                        #[doc = ""]
                        #[doc = "Might return `Some` instead of `None` after a minor version bump"]
                        pub fn from_snake_case_str(s: &str) -> Option<Self> {
                            match s {
                                #(#from_snake_case_str_branches),*
                            }
                        }

                        #[doc = #first_doc_str_into_snake_case_str]
                        pub fn into_snake_case_str(self) -> &'static str {
                            match self {
                                #(#into_snake_case_str_branches),*
                            }
                        }
                    }
                });
                self.output.extend(extra);
            }
        }

        syn::visit::visit_item_mod(self, i)
    }
}

#[derive(Debug)]
struct TigerbeetleCallbacks {
    inner: bindgen::CargoCallbacks,
    out_dir: PathBuf,
}

impl bindgen::callbacks::ParseCallbacks for TigerbeetleCallbacks {
    fn add_derives(&self, info: &bindgen::callbacks::DeriveInfo<'_>) -> Vec<String> {
        let mut out = Vec::new();
        if let bindgen::callbacks::DeriveInfo {
            kind: bindgen::callbacks::TypeKind::Struct,
            name:
                "tb_account_t"
                | "tb_account_balance_t"
                | "tb_account_filter_t"
                | "tb_create_accounts_result_t"
                | "tb_create_transfers_result_t"
                | "tb_query_filter_t"
                | "tb_transfer_t",
            ..
        } = info
        {
            out.extend(["::bytemuck::Pod".into(), "::bytemuck::Zeroable".into()]);
        };
        if let bindgen::callbacks::DeriveInfo {
            kind: bindgen::callbacks::TypeKind::Struct,
            name: "tb_init_parameters_t",
            ..
        } = info
        {
            out.extend(["::bytemuck::Zeroable".into()]);
        };
        out.append(&mut self.inner.add_derives(info));
        out
    }

    fn will_parse_macro(&self, name: &str) -> bindgen::callbacks::MacroParsingBehavior {
        self.inner.will_parse_macro(name)
    }

    fn generated_name_override(
        &self,
        item_info: bindgen::callbacks::ItemInfo<'_>,
    ) -> Option<String> {
        self.inner.generated_name_override(item_info)
    }

    fn generated_link_name_override(
        &self,
        item_info: bindgen::callbacks::ItemInfo<'_>,
    ) -> Option<String> {
        self.inner.generated_link_name_override(item_info)
    }

    fn int_macro(&self, name: &str, value: i64) -> Option<bindgen::callbacks::IntKind> {
        self.inner.int_macro(name, value)
    }

    fn str_macro(&self, name: &str, value: &[u8]) {
        self.inner.str_macro(name, value)
    }

    fn func_macro(&self, name: &str, value: &[&[u8]]) {
        self.inner.func_macro(name, value)
    }

    fn enum_variant_behavior(
        &self,
        enum_name: Option<&str>,
        original_variant_name: &str,
        variant_value: bindgen::callbacks::EnumVariantValue,
    ) -> Option<bindgen::callbacks::EnumVariantCustomBehavior> {
        self.inner
            .enum_variant_behavior(enum_name, original_variant_name, variant_value)
    }

    fn enum_variant_name(
        &self,
        enum_name: Option<&str>,
        original_variant_name: &str,
        variant_value: bindgen::callbacks::EnumVariantValue,
    ) -> Option<String> {
        self.inner
            .enum_variant_name(enum_name, original_variant_name, variant_value)
    }

    fn item_name(&self, original_item: bindgen::callbacks::ItemInfo<'_>) -> Option<String> {
        self.inner.item_name(original_item)
    }

    fn include_file(&self, filename: &str) {
        if !Path::new(filename).starts_with(&self.out_dir) {
            self.inner.include_file(filename)
        }
    }

    fn read_env_var(&self, key: &str) {
        self.inner.read_env_var(key)
    }

    fn blocklisted_type_implements_trait(
        &self,
        name: &str,
        derive_trait: bindgen::callbacks::DeriveTrait,
    ) -> Option<bindgen::callbacks::ImplementsTrait> {
        self.inner
            .blocklisted_type_implements_trait(name, derive_trait)
    }

    fn process_comment(&self, comment: &str) -> Option<String> {
        self.inner.process_comment(comment)
    }
}

fn screaming_snake_case_into_camel_case(src: &str) -> String {
    let mut dst = String::with_capacity(src.len());
    for word in src.split('_') {
        let mut chars = word.chars();
        let Some(ch) = chars.next() else { continue };
        assert!(ch.is_ascii_uppercase() || ch.is_ascii_digit());
        dst.push(ch);
        dst.extend(chars.map(|c| c.to_ascii_lowercase()));
    }
    dst
}

fn camel_case_into_snake_case(src: &str) -> String {
    let mut chars = src.chars();
    let Some(ch) = chars.next() else {
        return String::new();
    };
    assert!(ch.is_ascii_uppercase());

    let mut dst = String::with_capacity(src.len() * 2);
    dst.push(ch.to_ascii_lowercase());

    dst.extend(chars.flat_map(|c| {
        if c.is_ascii_uppercase() {
            Some('_')
                .into_iter()
                .chain(iter::once(c.to_ascii_lowercase()))
        } else {
            None.into_iter().chain(iter::once(c))
        }
    }));
    dst
}

fn create_mirror(original: &Path, mirror: &Path, ignores: &IgnoreNode) {
    if ignores.ignored() {
        return;
    }

    assert!(!mirror.exists(), "mirror path is occupied already");
    let mirror_parent = mirror
        .parent()
        .expect("mirror should have parent directory");
    assert!(mirror_parent.is_dir(), "mirror's parent is not a directory");

    if ignores.inner_is_empty() {
        let original = original
            .canonicalize()
            .expect("Could not canonicalize original path");

        let common_root = original
            .iter()
            .zip(mirror.iter())
            .take_while(|(a, b)| a == b)
            .map(|(a, _)| a)
            .collect::<PathBuf>();

        let mirror_from_common = mirror.strip_prefix(&common_root).unwrap();
        let original_from_common = original.strip_prefix(&common_root).unwrap();
        let link_original: PathBuf = (0..mirror_from_common.iter().count() - 1)
            .map(|_| Path::new(".."))
            .chain(iter::once(original_from_common))
            .collect();

        return symlink(link_original, mirror).expect("Symlinking the mirror fragment");
    }

    let original_traversal = original
        .read_dir()
        .expect("Initiating traversal of original directory");
    std::fs::create_dir(mirror).expect("Creating mirror dir");
    for entry in original_traversal {
        let entry = entry.expect("Reading original directory");
        let entry_name = entry.file_name();
        create_mirror(
            &original.join(&entry_name),
            &mirror.join(&entry_name),
            ignores.get(&entry_name),
        );
    }
}

#[derive(Default)]
struct IgnoreNode {
    inner: BTreeMap<OsString, IgnoreNode>,
    ignored: bool,
}

impl IgnoreNode {
    const fn new() -> Self {
        IgnoreNode {
            inner: BTreeMap::new(),
            ignored: false,
        }
    }

    fn get(&self, path_component: &OsStr) -> &IgnoreNode {
        static EMPTY: IgnoreNode = IgnoreNode::new();
        self.inner.get(path_component).unwrap_or(&EMPTY)
    }

    fn ignored(&self) -> bool {
        self.ignored
    }

    fn inner_is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn insert(&mut self, path: &Path) {
        path.components().for_each(|c| {
            assert!(
                matches!(c, path::Component::Normal(_)),
                "path component {c:?} must be `Normal(_)` instead"
            )
        });

        fn impl_(node: &mut IgnoreNode, path: &Path) {
            let mut iter = path.iter();
            let Some(component) = iter.next() else {
                panic!("path is empty")
            };
            let v = node.inner.entry(component.to_owned()).or_default();
            let path = iter.as_path();
            if path == Path::new("") {
                v.ignored = true;
                return;
            }
            impl_(v, path)
        }

        impl_(self, path)
    }
}

impl<A: AsRef<Path>> Extend<A> for IgnoreNode {
    fn extend<T: IntoIterator<Item = A>>(&mut self, iter: T) {
        for path in iter {
            self.insert(path.as_ref())
        }
    }
}

impl<A: AsRef<Path>> FromIterator<A> for IgnoreNode {
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
        let mut out = Self::new();
        out.extend(iter);
        out
    }
}

fn symlink<P, Q>(original: P, link: Q) -> io::Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    #[cfg(unix)]
    return std::os::unix::fs::symlink(original, link);
    #[cfg(windows)]
    return {
        let meta = link
            .as_ref()
            .parent()
            .ok_or(io::ErrorKind::NotFound)?
            .join(original.as_ref())
            .metadata()?;
        if meta.is_file() {
            std::os::windows::fs::symlink_file(original, link)
        } else if meta.is_dir() {
            std::os::windows::fs::symlink_dir(original, link)
        } else {
            Err(io::ErrorKind::NotFound.into())
        }
    };
    #[cfg(not(any(unix, windows)))]
    unimplemented!("symlink on current platform is not supported")
}

/// Runs `rustfmt` over the provided `file`.
///
/// # Panics
///
/// If `rustfmt` fails to run.
fn rustfmt(file: impl AsRef<Path>) {
    let file = file.as_ref();

    Command::new(env::var("RUSTFMT").unwrap_or_else(|_| "rustfmt".into()))
        .arg(file)
        .status()
        .unwrap_or_else(|e| panic!("failed to run `rustfmt` on {}: {e}", file.display()));
}
