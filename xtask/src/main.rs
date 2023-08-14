use std::{env, process::Command};

fn print_help() {
    eprintln!(
        r#"Execute xtask.
Subcommands:
    help - Print this help message.
    regenerate-header - Regenerate `sys/src/tb_client.h` header.
"#
    )
}

fn main() -> std::process::ExitCode {
    match env::args().nth(1).as_deref() {
        Some("regenerate-header") => regenerate_header(),
        Some("help") => print_help(),
        Some(other) => {
            eprintln!("Unknown xtask subcommand `{other}`!");
            print_help();
            return std::process::ExitCode::FAILURE;
        }
        None => {
            eprintln!("No xtask subcommand was specified!");
            print_help();
            return std::process::ExitCode::FAILURE;
        }
    }
    std::process::ExitCode::SUCCESS
}

fn regenerate_header() {
    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .expect("Running cargo metadata command");
    let sys_pkg = metadata
        .packages
        .iter()
        .filter(|p| metadata.workspace_members.contains(&p.id))
        .find(|p| p.name == "tigerbeetle-unofficial-sys")
        .expect("Could not find `tigerbeetle-unofficial-sys` package");
    let sys_root = sys_pkg.manifest_path.parent().unwrap();
    let tb_root = sys_root.join("tigerbeetle");

    let status = Command::new(
        tb_root
            .join("scripts/install_zig")
            .with_extension(SHELL_SCRIPT_EXT),
    )
    .current_dir(&tb_root)
    .status()
    .expect("Executing install_zig script");
    assert!(status.success(), "install_zig script failed");

    let status = Command::new(
        tb_root
            .join("zig/zig")
            .with_extension(env::consts::EXE_EXTENSION),
    )
    .arg("build")
    .arg("c_client")
    .current_dir(&tb_root)
    .status()
    .expect("Execution C client build");
    assert!(status.success(), "C client build command have failed");

    std::fs::copy(
        tb_root.join("src/clients/c/lib/include/tb_client.h"),
        sys_root.join("src/tb_client.h"),
    )
    .expect("Copying generated tb_client.h into sys/src dir");
}

#[cfg(windows)]
const SHELL_SCRIPT_EXT: &str = "bat";
#[cfg(unix)]
const SHELL_SCRIPT_EXT: &str = "sh";
