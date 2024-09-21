use std::net::{SocketAddr, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;
use std::{fs, path::PathBuf};

use anyhow::Context;

pub fn find_open_ports(
    host: &str,
    port_num_start: u16,
    num_ports: usize,
) -> Vec<u16> {
    let mut open_ports = vec![];
    let mut port = port_num_start;
    while port - port_num_start < 200 {
        if format!("{}:{}", host, port)
            .parse()
            .context("addr is not a SocketAddr")
            .and_then(|addr| {
                TcpStream::connect_timeout(&addr, Duration::from_millis(100))
                    .context("TcpStream connect error")
            })
            .is_err()
        {
            open_ports.push(port);
        }
        if open_ports.len() >= num_ports {
            break;
        }
        port += 1;
    }

    open_ports
}

// Get or create home directory
pub fn get_config_dir(require: bool) -> anyhow::Result<String> {
    let mut homedir = PathBuf::new();
    if let Ok(x) = std::env::var("CROC_CONFIG_DIR") {
        homedir.push(x);
    } else if let Ok(x) = std::env::var("XDG_CONFIG_HOME") {
        homedir.push(x);
        homedir.push("croc");
    } else {
        match dirs::home_dir() {
            None => {
                if !require {
                    homedir = PathBuf::new();
                }
                return Ok(homedir.display().to_string());
            },
            Some(mut pf) => {
                pf.push(".config");
                pf.push("croc");
                homedir = pf;
            },
        }
    }

    if require && !homedir.exists() {
        let perms = fs::Permissions::from_mode(0o700);
        let fp = homedir.as_path();
        fs::create_dir_all(fp)?;
        fs::set_permissions(fp, perms)?;
    }

    Ok(homedir.display().to_string())
}

// GetInput returns the input with a given prompt
pub fn get_input(prompt: &[u8]) -> anyhow::Result<String> {
    // let mut stderr = tokio::io::stderr();
    // stderr.write_all(prompt)?;
    // stderr.flush()?;
    // let mut bytes = vec![];
    // let mut buffer = BufReader::new(tokio::io::stdin());
    // let _ = buffer.read_until(b'\n', &mut bytes)?;
    // let input = std::str::from_utf8(&bytes)?;
    // Ok(input.trim().to_string())
    // @fri3nd TODO
    todo!()
}
