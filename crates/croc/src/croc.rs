use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::ops::Not;
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tracing::debug;

use super::{model, tcp, utils};

// Options specifies user specific options
#[derive(Debug)]
pub struct Options {
    pub is_sender: bool,
    pub shared_secret: String,
    pub relay_address: String,
    pub relay_address6: String,
    pub relay_ports: Vec<String>,
    pub relay_password: String,
    pub zip_folder: bool,
    pub git_ignore: bool,
    pub disable_local: bool,
    pub only_local: bool,
    pub ip: String,
}

// FileInfo registers the information about the file
pub struct FileInfo {
    pub _name: String,
    pub _size: u64,
}

// Client holds the state of the croc transfer
pub struct Client {
    options: Options,
    // steps involved in forming relationship
    step1_channel_secured: bool,
    files_has_finished: BTreeSet<usize>,
}

// New establishes a new connection for transferring files between two instances.
pub fn new(ops: Options) -> anyhow::Result<Client> {
    // @fri3nd TODO
    let clt = Client {
        options: ops,
        step1_channel_secured: false,
        files_has_finished: BTreeSet::new(),
    };
    Ok(clt)
}

impl Client {
    pub fn send(
        &mut self,
        files_info: Vec<FileInfo>,
        _empty_folders_to_transfer: Vec<FileInfo>,
        _total_number_folders: usize,
    ) -> anyhow::Result<()> {
        // c.EmptyFoldersToTransfer = emptyFoldersToTransfer
        // c.TotalNumberFolders = totalNumberFolders
        // c.TotalNumberOfContents = len(filesInfo)

        self.send_collect_files(&files_info)?;
        let mut flags = String::new();
        if self.options.relay_address != model::DEFAULT_RELAY && !self.options.only_local {
            flags += "--relay ";
            flags += &self.options.relay_address;
            flags += " ";
        }
        if self.options.relay_password != model::DEFAULT_PASSPHRASE {
            flags += "--pass ";
            flags += &self.options.relay_password;
            flags += " ";
        }
        let tips = format!(
            r##"Code is: {}

On the other computer run:
(For Windows)
    croc {}{}
(For Linux/OSX)
    CROC_SECRET={:?} croc {}
"##,
            self.options.shared_secret,
            flags,
            self.options.shared_secret,
            self.options.shared_secret,
            flags,
        );
        std::io::copy(&mut tips.as_bytes(), &mut std::io::stderr())?;
        // xxxxxxxxxxxxxxxxxxxxxxxxx
        // if c.Options.Ask {
        //     machid, _ := machineid.ID()
        //     fmt.Fprintf(os.Stderr, "\rYour machine ID is '%s'\n", machid)
        // }
        // xxxxxxxxxxxxxxxxxxxxxxxxx

        // let (_err_tx, _err_rx) = async_channel::bounded::<anyhow::Error>(2);
        if !self.options.disable_local {
            // add two things to the error channel
            self.setup_local_relay()?;

            std::thread::scope(|s| {
                // broadcast on ipv4
                let only_local = self.options.only_local;
                let first_reply_port = self.options.relay_ports[0].clone();

                s.spawn(move || {
                    broadcast_on_local_network(only_local, &first_reply_port, false);
                });

                // broadcast on ipv6
                // let only_local = self.options.only_local;
                let first_reply_port = self.options.relay_ports[0].clone();
                s.spawn(move || {
                    broadcast_on_local_network(only_local, &first_reply_port, true);
                });
            });

            // go c.broadcastOnLocalNetwork(true)
            // go c.transferOverLocalRelay(errchan)
        }

        // let err = err_rx.recv();

        // @fri3nd TODO
        Ok(())
    }

    pub fn receive(&mut self) -> anyhow::Result<()> {
        let mut stderr = std::io::stderr();
        stderr.write_all(b"connecting...")?;
        stderr.flush()?;
        // recipient will look for peers first
        // and continue if it doesn't find any within 100 ms

        let using_local = false;
        let mut is_ipset = false;
        if self.options.only_local || !self.options.ip.is_empty() {
            self.options.relay_address = "".into();
            self.options.relay_address6 = "".into();
        }
        if !self.options.ip.is_empty() {
            // check ip version
            if self.options.ip.matches(":").count() >= 2 {
                debug!("assume ipv6");
                self.options.relay_address6 = self.options.ip.clone();
            }
            if self.options.ip.contains(".") {
                debug!("assume ipv4");
                self.options.relay_address = self.options.ip.clone();
            }
            is_ipset = false;
        }

        if !(self.options.disable_local || is_ipset) {
            debug!("attempt to discover peers");
        }
        // // @fri3nd TODO
        // // @fri3nd TODO
        // // @fri3nd TODO
        Ok(())
    }

    fn send_collect_files(
        &self,
        _files_info: &[FileInfo],
    ) -> anyhow::Result<()> {
        // let mut total_files_size: u64 = 0;
        // total_files_size += 1;
        // @fri3nd TODO
        Ok(())
    }

    fn setup_local_relay(&mut self) -> anyhow::Result<()> {
        // setup the relay locally
        let first_port: u16 = self.options.relay_ports[0].parse()?;
        let open_ports =
            utils::find_open_ports("127.0.0.1", first_port, self.options.relay_ports.len());
        if open_ports.len() < self.options.relay_ports.len() {
            anyhow::bail!("not enough open ports to run local relay")
        }
        self.options.relay_ports.clear();
        for port in open_ports {
            self.options.relay_ports.push(port.to_string());
        }

        std::thread::scope(|s| {
            for it in &self.options.relay_ports {
                let port = it.clone();
                let password = self.options.relay_password.clone();
                let banner: String = self.options.relay_ports[1..].join(",");

                s.spawn(move || {
                    if let Err(e) = tcp::run("127.0.0.1", port, password, banner) {
                        panic!("{:?}", e);
                    }
                });
            }
        });

        Ok(())
    }
}

// This function retrieves the important file information
// for every file that will be transferred
pub fn get_files_info(
    fnames: &[String],
    _zip_folder: bool,
    ignore_git: bool,
) -> anyhow::Result<(Vec<FileInfo>, Vec<FileInfo>, usize)> {
    // fnames: the relative/absolute paths of files/folders that will be transferred
    let total_number_folders: usize = 0;
    let mut paths: Vec<String> = vec![];
    let empty_folders: Vec<FileInfo> = vec![];
    let files_info: Vec<FileInfo> = vec![];
    for fname in fnames {
        // Support wildcard
        paths.push(fname.clone());
    }
    let _ignored_paths: BTreeMap<String, bool> = BTreeMap::new();
    // xxxxxxxxxxxxxx
    // if ignore_git {}
    // @fri3nd TODO
    // todo!()
    Ok((files_info, empty_folders, total_number_folders))
}

fn broadcast_on_local_network(
    only_local: bool,
    _first_reply_port: &str,
    _useipv6: bool,
) {
    // if we don't use an external relay, the broadcast messages need to be sent continuously
    let _time_limit = only_local.not().then(|| Duration::from_secs(30));
    // look for peers first
    // @fri3nd TODO
    // @fri3nd TODO
    // @fri3nd TODO
    // todo!()
}
