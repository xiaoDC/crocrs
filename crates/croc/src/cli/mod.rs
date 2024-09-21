use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use tracing::error;

use super::{model, utils};

mod receive;
mod send;

/// crors(croc - easily and securely transfer stuff from one computer to another) rewrite by rust
#[derive(Parser, Debug)]
#[command(version, bin_name = "crocrs", name = "crocrs")]
pub struct App {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    pub command: Option<CrocCommand>,
}

#[derive(Args, Debug)]
pub struct GlobalArgs {
    #[arg(
        long,
        help = "toggle between the classic mode (insecure due to local attack vector) and new mode (secure) (default: false)",
        default_value_t = false
    )]
    pub classic: bool,

    #[arg(
        long,
        help = "save these settings to reuse next time",
        default_value_t = false
    )]
    pub remember: bool,

    #[arg(
        long,
        global = true,
        help = "address of the relay",
        env = "CROC_RELAY",
        default_value = model::DEFAULT_RELAY,
    )]
    pub relay: String,

    #[arg(
        long,
        global = true,
        help = "ipv6 address of the relay",
        env = "CROC_RELAY6",
        default_value = model::DEFAULT_RELAY6,
    )]
    pub relay6: String,

    #[arg(
        long,
        global = true,
        help = "password for the relay",
        env = "CROC_PASS",
        default_value = model::DEFAULT_PASSPHRASE,
    )]
    pub pass: String,

    #[arg(
        long,
        global = true,
        help = "force to use only local connections",
        default_value_t = false
    )]
    pub local: bool,

    #[arg(
        long,
        help = "set sender ip if known e.g. 10.0.0.1:9009, [::1]:9009",
        default_value = ""
    )]
    pub ip: String,

    #[arg(
        long,
        help = "add a socks5 proxy",
        env = "SOCKS5_PROXY",
        default_value = ""
    )]
    pub socks5: String,

    #[arg(
        long,
        help = "add a http proxy",
        env = "HTTP_PROXY",
        default_value = ""
    )]
    pub connect: String,

    // #[arg(long, action = clap::ArgAction::Append)]
    pub args: Vec<String>,
}

#[derive(Subcommand, Debug)]
pub enum CrocCommand {
    /// crors send - send file(s), or folder
    #[command(name = "send", verbatim_doc_comment)]
    Send(SendArgs),

    #[command(name = "relay", about = "start your own relay (optional)")]
    Relay,
}

#[derive(Args, Debug)]
pub struct SendArgs {
    pub fnames: Vec<String>,

    #[arg(
        long = "zip",
        help = "zip folder before sending",
        default_value_t = false
    )]
    zip: bool,

    #[arg(
        long = "hash",
        help = "hash algorithm (xxhash, imohash, md5)",
        default_value = "xxhash"
    )]
    hash: String,

    #[arg(long = "text", short = 't', help = "send some text")]
    text: Option<String>,

    #[arg(long = "no-local", help = "disable local relay when sending")]
    no_local: Option<bool>,

    #[arg(long = "no-multi", help = "disable multiplexing")]
    no_multi: Option<bool>,

    #[arg(
        long = "git",
        help = "enable .gitignore respect / don't send ignored files",
        default_value_t = false
    )]
    git: bool,

    #[arg(
        long = "port",
        help = "base port for the relay",
        default_value_t = 9009
    )]
    port: u16,

    #[arg(
        long = "transfers",
        help = "number of ports to use for transfers",
        default_value_t = 4
    )]
    transfers: usize,

    #[arg(
        long,
        short = 'c',
        help = "codephrase used to connect to relay",
        env = "CROC_SECRET",
        default_value = ""
    )]
    pub code: String,
}

impl App {
    pub fn run(&self) -> anyhow::Result<()> {
        if let Some(subcommand) = &self.command {
            match subcommand {
                CrocCommand::Send(args) => {
                    send::send(args, &self.global)?;
                    return Ok(());
                },
                CrocCommand::Relay => {
                    dbg!("Relay");
                },
            }
        }

        // check if "classic" is set
        let classic_file = get_classic_config_file(true);
        dbg!(&classic_file);
        let classic_insecure_mode = classic_file.exists();
        if self.global.classic {
            dbg!(&classic_insecure_mode);
            if classic_insecure_mode {
                // classic mode not enabled
                println!(
                    r#"Classic mode is currently ENABLED.

Disabling this mode will prevent the shared secret from being visible
on the host's process list when passed via the command line. On a
multi-user system, this will help ensure that other local users cannot
access the shared secret and receive the files instead of the intended
recipient.

Do you wish to continue to DISABLE the classic mode? (y/N) "#
                );
                let choice = utils::get_input(b"")?.to_lowercase();
                if matches!(choice.as_str(), "y" | "yes") {
                    std::fs::remove_file(&classic_file)?;
                    print!("\nClassic mode DISABLED.\n\n");
                    print!(
                        r#"To send and receive, export the CROC_SECRET variable with the code phrase:

  Send:    CROC_SECRET=*** croc send file.txt

  Receive: CROC_SECRET=*** croc

"#
                    );
                } else {
                    print!("\nClassic mode ENABLED.\n")
                }
            } else {
                // enable classic mode
                // touch the file
                print!(
                    r##"Classic mode is currently DISABLED.

Please note that enabling this mode will make the shared secret visible
on the host's process list when passed via the command line. On a
multi-user system, this could allow other local users to access the
shared secret and receive the files instead of the intended recipient.

Do you wish to continue to enable the classic mode? (y/N) "##
                );
                let choice = utils::get_input(b"")?.to_lowercase();
                if matches!(choice.as_str(), "y" | "yes") {
                    print!("\nClassic mode ENABLED.\n\n");
                    std::fs::write(&classic_file, b"enabled")?;
                    let perms = std::fs::Permissions::from_mode(0o644);
                    std::fs::set_permissions(&classic_file, perms)?;
                    print!(
                        r#"To send and receive, use the code phrase:

Send:    croc send --code *** file.txt

Receive: croc ***

"#
                    );
                } else {
                    print!("\nClassic mode DISABLED.\n");
                }
            }
            return Ok(());
        }

        // if trying to send but forgot send, let the user know
        // @fri3nd TODO
        receive::receive(&self.global)
    }
}

fn determine_pass(pass: &str) -> String {
    let mut rst = pass.into();
    if let Ok(txt) = std::fs::read_to_string(pass) {
        rst = txt
    }
    rst
}

fn get_classic_config_file(require: bool) -> PathBuf {
    match utils::get_config_dir(require) {
        Err(e) => {
            error!(error = ?e);
            PathBuf::new()
        },
        Ok(cfg) => {
            let mut path = PathBuf::new();
            path.push(cfg);
            path.push("classic_enabled");
            path
        },
    }
}
