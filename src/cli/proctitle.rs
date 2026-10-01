//! Mapping from parsed CLI arguments to an initial process title.
//!
//! This logic depends on the clap `Args`/`Command` types defined in `cli`, so
//! it lives in the CLI layer. The low-level title-setting primitives it uses
//! (`compact_process_title`, `session_name`, `set_title`) live in the
//! `process_title` core module.

use crate::cli::args::{AmbientCommand, Args, Command};
use crate::process_title::{compact_process_title, session_name, set_title};

pub(crate) fn initial_title(args: &Args) -> String {
    match &args.command {
        Some(Command::Serve { .. }) => "kraivcode:server".to_string(),
        Some(Command::Acp) => "kraivcode acp".to_string(),
        Some(Command::Server { .. }) => "kraivcode server".to_string(),
        Some(Command::Connect) => "kraivcode:client".to_string(),
        #[cfg(unix)]
        Some(Command::ApiBridge { .. }) => "kraivcode api-bridge".to_string(),
        Some(Command::Run { .. }) => "kraivcode run".to_string(),
        Some(Command::Login { .. }) => "kraivcode login".to_string(),
        Some(Command::Account { .. }) => "kraivcode account".to_string(),
        Some(Command::Repl) => "kraivcode repl".to_string(),
        Some(Command::Update) => "kraivcode update".to_string(),
        Some(Command::Version { .. }) => "kraivcode version".to_string(),
        Some(Command::Usage { .. }) => "kraivcode usage".to_string(),
        Some(Command::Telemetry(_)) => "kraivcode telemetry".to_string(),
        Some(Command::SelfDev { .. }) => "kraivcode:selfdev".to_string(),
        Some(Command::Debug { .. }) => "kraivcode debug".to_string(),
        Some(Command::Auth(_)) => "kraivcode auth".to_string(),
        Some(Command::Provider(_)) => "kraivcode provider".to_string(),
        Some(Command::Memory(_)) => "kraivcode memory".to_string(),
        Some(Command::Session(_)) => "kraivcode session".to_string(),
        Some(Command::Ambient(subcommand)) => match subcommand {
            AmbientCommand::RunVisible => "kraivcode ambient visible".to_string(),
            _ => "kraivcode ambient".to_string(),
        },
        Some(Command::Cloud(_)) => "kraivcode cloud".to_string(),
        Some(Command::Pair { .. }) => "kraivcode pair".to_string(),
        Some(Command::Permissions) => "kraivcode permissions".to_string(),
        Some(Command::Transcript { .. }) => "kraivcode transcript".to_string(),
        Some(Command::Dictate { .. }) => "kraivcode dictate".to_string(),
        Some(Command::SetupHotkey {
            listen_macos_hotkey,
            notify_cli_launch,
            listen_windows_hotkey,
            uninstall,
        }) => {
            if *listen_macos_hotkey || *listen_windows_hotkey {
                "kraivcode hotkey listener".to_string()
            } else if notify_cli_launch.is_some() {
                "kraivcode shortcut reminder".to_string()
            } else if *uninstall {
                "kraivcode hotkey uninstall".to_string()
            } else {
                "kraivcode hotkey setup".to_string()
            }
        }
        Some(Command::Browser { .. }) => "kraivcode browser".to_string(),
        Some(Command::Replay { .. }) => "kraivcode replay".to_string(),
        Some(Command::Model(_)) => "kraivcode model".to_string(),
        Some(Command::ProviderTestCoverage { .. }) => "kraivcode provider-test-coverage".to_string(),
        Some(Command::ProviderDoctor { .. }) => "kraivcode provider-doctor".to_string(),
        Some(Command::AuthTest { .. }) => "kraivcode auth-test".to_string(),
        Some(Command::Restart { .. }) => "kraivcode restart".to_string(),
        Some(Command::Menubar { .. }) => "kraivcode menubar".to_string(),
        Some(Command::SetupLauncher) => "kraivcode setup-launcher".to_string(),
        None => {
            if let Some(resume) = args.resume.as_deref().filter(|resume| !resume.is_empty()) {
                let prefix = if crate::cli::selfdev::client_selfdev_requested() {
                    "kraivcode:d:"
                } else {
                    "kraivcode:c:"
                };
                compact_process_title(prefix, Some(&session_name(resume)))
            } else if crate::cli::selfdev::client_selfdev_requested() {
                "kraivcode:selfdev".to_string()
            } else {
                "kraivcode:client".to_string()
            }
        }
    }
}

pub(crate) fn set_initial_title(args: &Args) {
    set_title(initial_title(args));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::lock_test_env;
    use clap::Parser;

    const SELFDEV_ENV: &str = jcode_selfdev_types::CLIENT_SELFDEV_ENV;

    fn with_selfdev_env_removed<T>(f: impl FnOnce() -> T) -> T {
        let _guard = lock_test_env();
        let previous = std::env::var_os(SELFDEV_ENV);
        crate::env::remove_var(SELFDEV_ENV);
        let result = f();
        if let Some(value) = previous {
            crate::env::set_var(SELFDEV_ENV, value);
        }
        result
    }

    #[test]
    fn initial_title_labels_server() {
        with_selfdev_env_removed(|| {
            let args = Args::parse_from(["jcode", "serve"]);
            assert_eq!(initial_title(&args), "kraivcode:server");
        });
    }

    #[test]
    fn initial_title_labels_resume_client_with_short_name() {
        with_selfdev_env_removed(|| {
            let args = Args::parse_from(["jcode", "--resume", "session_fox_123"]);
            assert_eq!(initial_title(&args), "kraivcode:c:fox");
        });
    }

    #[test]
    fn initial_title_labels_selfdev_command() {
        with_selfdev_env_removed(|| {
            let args = Args::parse_from(["jcode", "self-dev"]);
            assert_eq!(initial_title(&args), "kraivcode:selfdev");
        });
    }

    #[test]
    fn initial_title_labels_windows_hotkey_listener() {
        let args = Args::parse_from(["jcode", "setup-hotkey", "--listen-windows-hotkey"]);
        assert_eq!(initial_title(&args), "kraivcode hotkey listener");
    }

    #[test]
    fn initial_title_labels_hotkey_uninstall() {
        let args = Args::parse_from(["jcode", "setup-hotkey", "--uninstall"]);
        assert_eq!(initial_title(&args), "kraivcode hotkey uninstall");
    }
}
