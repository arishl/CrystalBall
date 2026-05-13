use std::path::PathBuf;

pub(crate) enum TerminalCommand {
    Clear,
    ChangeDirectory(PathBuf),
    Shell(String),
}

pub(crate) fn parse_terminal_command(command: &str) -> TerminalCommand {
    let command = command.trim();

    if command == "clear" {
        return TerminalCommand::Clear;
    }

    if command == "cd" {
        return TerminalCommand::ChangeDirectory(home_dir());
    }

    if let Some(path) = command.strip_prefix("cd ") {
        return TerminalCommand::ChangeDirectory(expand_home(path.trim()));
    }

    TerminalCommand::Shell(command.to_owned())
}

pub(crate) fn default_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned())
}

fn expand_home(path: &str) -> PathBuf {
    if path == "~" {
        return home_dir();
    }

    if let Some(rest) = path.strip_prefix("~/") {
        return home_dir().join(rest);
    }

    PathBuf::from(path)
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}
