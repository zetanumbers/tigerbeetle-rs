use std::{
    collections::BTreeMap,
    env,
    ffi::{OsStr, OsString},
    fs::File,
    io::{self, Write},
    iter,
    path::{self, Path, PathBuf},
    process::Command,
};

use quote::quote;
use syn::visit::Visit;

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

    println!("cargo:rerun-if-env-changed=DOCS_RS");
    println!("cargo:rerun-if-changed=src/wrapper.h");

    let wrapper;
    if std::env::var("DOCS_RS").is_ok() {
        wrapper = "src/wrapper.h".into();
    } else {
        let target_lib_subdir = target_to_lib_dir(&target)
            .unwrap_or_else(|| panic!("target {target:?} is not supported"));

        let tigerbeetle_root = out_dir.join("tigerbeetle");
        std::fs::remove_dir_all(&tigerbeetle_root)
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
            &["src/clients/c/lib", "zig-cache", "zig-out", "zig"]
                .into_iter()
                .collect(),
        );

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
                .join("zig/zig")
                .with_extension(env::consts::EXE_EXTENSION)
                .canonicalize()
                .unwrap(),
        )
        .arg("build")
        .arg("c_client")
        .args((!debug).then_some("-Drelease-safe"))
        .arg(format!("-Dtarget={target_lib_subdir}"))
        .current_dir(&tigerbeetle_root)
        .status()
        .expect("running zig build subcommand");
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

        wrapper = lib_dir.join("include/wrapper.h");
        std::fs::copy("src/wrapper.h", &wrapper).expect("copying wrapper.h");
    };

    let bindings = bindgen::Builder::default()
        .header(
            wrapper
                .to_str()
                .expect("wrapper.h out path is not valid unicode"),
        )
        .default_enum_style(bindgen::EnumVariation::ModuleConsts)
        .parse_callbacks(Box::new(TigerbeetleCallbacks {
            out_dir: out_dir.clone(),
        }))
        .generate()
        .expect("generating tb_client bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("writing tb_client bindings");

    if std::env::var("CARGO_FEATURE_GENERATED_SAFE").is_ok() {
        let bindings = syn::parse_file(&bindings.to_string()).unwrap();

        let mut visitor = TigerbeetleVisitor::default();
        visitor.visit_file(&bindings);

        let generated_path = out_dir.join("generated.rs");
        let mut f = io::BufWriter::new(File::create(&generated_path).unwrap());
        write!(f, "{}", visitor.output).unwrap();
        drop(f);

        Command::new(std::env::var("RUSTFMT").unwrap_or_else(|_| "rustfmt".into()))
            .arg(&generated_path)
            .status()
            .unwrap();
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

            let mut new_enum_name =
                screaming_snake_case_into_camel_case(enum_name.strip_prefix("TB_").unwrap());
            let mut new_enum_ident = syn::Ident::new(&new_enum_name, enum_ident.span());

            if enum_name.ends_with("_FLAGS") {
                let variants = variants.iter().map(|(n, v, _)| {
                    let n = syn::Ident::new(n, v.span());
                    quote!(const #n = super:: #enum_ident :: #v as u16;)
                });
                self.output.extend(quote! {
                    ::bitflags::bitflags! {
                        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
                        pub struct #new_enum_ident: u16 {
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
                    "Status" => {
                        new_enum_name = "StatusErrorKind".to_string();
                        new_enum_ident = syn::Ident::new(&new_enum_name, new_enum_ident.span());
                        errorize = true;
                    }
                    "PacketStatus" => {
                        new_enum_name = "PacketStatusErrorKind".to_string();
                        new_enum_ident = syn::Ident::new(&new_enum_name, new_enum_ident.span());
                        repr_type = "u8";
                        errorize = true;
                    }
                    "PacketAcquireStatus" => {
                        new_enum_name = "PacketAcquireStatusErrorKind".to_string();
                        new_enum_ident = syn::Ident::new(&new_enum_name, new_enum_ident.span());
                        errorize = true;
                    }
                    "Operation" => {
                        new_enum_name = "OperationKind".to_string();
                        new_enum_ident = syn::Ident::new(&new_enum_name, new_enum_ident.span());
                        repr_type = "u8"
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

                let mut variants_iter = variants.iter();
                let mut j = variants_iter.next().unwrap().2;
                for (_, _, i) in variants_iter {
                    j += 1;
                    assert_eq!(*i, j);
                }

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
                let j = syn::LitInt::new(&j.to_string(), proc_macro2::Span::call_site());
                let extra = quote! {
                    pub const #min_name: #repr_type = 1;
                    pub const #max_name: #repr_type = #j;
                };

                let from_snake_case_str_branches = variants
                    .iter()
                    .map(|(s, v, _)| {
                        let n = syn::Ident::new(s, v.span());
                        let s = camel_case_into_snake_case(s);
                        quote!(#s => Some(Self:: #n))
                    })
                    .chain(std::iter::once(quote!(
                        _ => None
                    )));

                let into_snake_case_str_branches = variants
                    .iter()
                    .map(|(s, v, _)| {
                        let n = syn::Ident::new(s, v.span());
                        let s = camel_case_into_snake_case(s);
                        quote!(Self:: #n => #s)
                    })
                    .chain(std::iter::once(quote!(
                        Self::UnstableUncategorized => unimplemented!("variant is not supported yet")
                    )));

                let variants = variants
                    .iter()
                    .map(|(n, v, _)| {
                        let n = syn::Ident::new(n, v.span());
                        quote!(#n = super:: #enum_ident :: #v as #repr_type)
                    })
                    .chain(std::iter::once(quote!(
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
    out_dir: PathBuf,
}

impl bindgen::callbacks::ParseCallbacks for TigerbeetleCallbacks {
    fn add_derives(&self, info: &bindgen::callbacks::DeriveInfo<'_>) -> Vec<String> {
        let mut out = Vec::new();
        if let bindgen::callbacks::DeriveInfo {
            kind: bindgen::callbacks::TypeKind::Struct,
            name:
                "tb_account_t"
                | "tb_create_accounts_result_t"
                | "tb_transfer_t"
                | "tb_create_transfers_result_t",
            ..
        } = info
        {
            out.extend(["::bytemuck::Pod".into(), "::bytemuck::Zeroable".into()]);
        };
        out.append(&mut bindgen::CargoCallbacks.add_derives(info));
        out
    }

    fn will_parse_macro(&self, name: &str) -> bindgen::callbacks::MacroParsingBehavior {
        bindgen::CargoCallbacks.will_parse_macro(name)
    }

    fn generated_name_override(
        &self,
        item_info: bindgen::callbacks::ItemInfo<'_>,
    ) -> Option<String> {
        bindgen::CargoCallbacks.generated_name_override(item_info)
    }

    fn generated_link_name_override(
        &self,
        item_info: bindgen::callbacks::ItemInfo<'_>,
    ) -> Option<String> {
        bindgen::CargoCallbacks.generated_link_name_override(item_info)
    }

    fn int_macro(&self, name: &str, value: i64) -> Option<bindgen::callbacks::IntKind> {
        bindgen::CargoCallbacks.int_macro(name, value)
    }

    fn str_macro(&self, name: &str, value: &[u8]) {
        bindgen::CargoCallbacks.str_macro(name, value)
    }

    fn func_macro(&self, name: &str, value: &[&[u8]]) {
        bindgen::CargoCallbacks.func_macro(name, value)
    }

    fn enum_variant_behavior(
        &self,
        enum_name: Option<&str>,
        original_variant_name: &str,
        variant_value: bindgen::callbacks::EnumVariantValue,
    ) -> Option<bindgen::callbacks::EnumVariantCustomBehavior> {
        bindgen::CargoCallbacks.enum_variant_behavior(
            enum_name,
            original_variant_name,
            variant_value,
        )
    }

    fn enum_variant_name(
        &self,
        enum_name: Option<&str>,
        original_variant_name: &str,
        variant_value: bindgen::callbacks::EnumVariantValue,
    ) -> Option<String> {
        bindgen::CargoCallbacks.enum_variant_name(enum_name, original_variant_name, variant_value)
    }

    fn item_name(&self, original_item_name: &str) -> Option<String> {
        bindgen::CargoCallbacks.item_name(original_item_name)
    }

    fn include_file(&self, filename: &str) {
        if !Path::new(filename).starts_with(&self.out_dir) {
            bindgen::CargoCallbacks.include_file(filename)
        }
    }

    fn read_env_var(&self, key: &str) {
        bindgen::CargoCallbacks.read_env_var(key)
    }

    fn blocklisted_type_implements_trait(
        &self,
        name: &str,
        derive_trait: bindgen::callbacks::DeriveTrait,
    ) -> Option<bindgen::callbacks::ImplementsTrait> {
        bindgen::CargoCallbacks.blocklisted_type_implements_trait(name, derive_trait)
    }

    fn process_comment(&self, comment: &str) -> Option<String> {
        bindgen::CargoCallbacks.process_comment(comment)
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
