use std::{
    env,
    path::Path,
    process::{Command, Stdio},
};

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
    let output = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .arg("locate-project")
        .arg("--message-format=plain")
        .stderr(Stdio::inherit())
        .output()
        .expect("Running `cargo locate-project` command");
    assert!(
        output.status.success(),
        "`cargo locate-project` command failed"
    );
    let workspace_manifest =
        String::from_utf8(output.stdout).expect("Workspace manifest path is not UTF-8");
    let workspace_manifest = Path::new(workspace_manifest.trim());
    let sys_root = workspace_manifest.parent().unwrap().join("sys");
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

    eprintln!("Running zig build command...");
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

    eprintln!("Copying the generated header...");
    std::fs::copy(
        tb_root.join("src/clients/c/lib/include/tb_client.h"),
        sys_root.join("src/tb_client.h"),
    )
    .expect("Copying generated tb_client.h into sys/src dir");

    eprintln!("Done!")
}

#[cfg(windows)]
const SHELL_SCRIPT_EXT: &str = "bat";
#[cfg(unix)]
const SHELL_SCRIPT_EXT: &str = "sh";
