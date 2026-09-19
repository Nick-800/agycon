use std::os::unix::process::CommandExt;
use std::process::Command;

pub fn launch_new(skip_permissions: bool) -> ! {
    let mut cmd = Command::new("agy");
    if skip_permissions {
        cmd.arg("--dangerously-skip-permissions");
    }
    let err = cmd.exec();
    eprintln!("Error executing 'agy': {err}");
    eprintln!("Please ensure 'agy' is installed and available in your PATH.");
    std::process::exit(1);
}

pub fn launch_continue(skip_permissions: bool) -> ! {
    let mut cmd = Command::new("agy");
    cmd.arg("--continue");
    if skip_permissions {
        cmd.arg("--dangerously-skip-permissions");
    }
    let err = cmd.exec();
    eprintln!("Error executing 'agy --continue': {err}");
    eprintln!("Please ensure 'agy' is installed and available in your PATH.");
    std::process::exit(1);
}

pub fn launch_conversation(conversation_id: &str, skip_permissions: bool) -> ! {
    let mut cmd = Command::new("agy");
    cmd.args(["--conversation", conversation_id]);
    if skip_permissions {
        cmd.arg("--dangerously-skip-permissions");
    }
    let err = cmd.exec();
    eprintln!("Error executing 'agy --conversation {conversation_id}': {err}");
    eprintln!("Please ensure 'agy' is installed and available in your PATH.");
    std::process::exit(1);
}
