use std::os::unix::process::CommandExt;
use std::process::Command;

pub fn launch_new() -> ! {
    let err = Command::new("agy").exec();
    eprintln!("Error executing 'agy': {err}");
    eprintln!("Please ensure 'agy' is installed and available in your PATH.");
    std::process::exit(1);
}

pub fn launch_continue() -> ! {
    let err = Command::new("agy").arg("--continue").exec();
    eprintln!("Error executing 'agy --continue': {err}");
    eprintln!("Please ensure 'agy' is installed and available in your PATH.");
    std::process::exit(1);
}

pub fn launch_conversation(conversation_id: &str) -> ! {
    let err = Command::new("agy")
        .args(["--conversation", conversation_id])
        .exec();
    eprintln!("Error executing 'agy --conversation {conversation_id}': {err}");
    eprintln!("Please ensure 'agy' is installed and available in your PATH.");
    std::process::exit(1);
}
