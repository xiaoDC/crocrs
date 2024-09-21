use super::{determine_pass, GlobalArgs};
use crate::{comm, croc};

pub(super) fn receive(global: &GlobalArgs) -> anyhow::Result<()> {
    {
        let mut lock = comm::SOCKS5_PROXY.write();
        *lock = global.socks5.clone();
    }
    {
        let mut lock = comm::HTTP_PROXY.write();
        *lock = global.connect.clone();
    }
    let mut opts = croc::Options {
        shared_secret: "".into(),
        is_sender: false,
        zip_folder: false,
        git_ignore: false,
        relay_address: global.relay.clone(),
        relay_address6: global.relay6.clone(),
        relay_password: determine_pass(&global.pass),
        disable_local: false,
        only_local: global.local,
        relay_ports: vec![],
        ip: global.ip.clone(),
    };
    let len = global.args.len();
    match len {
        1 => {
            opts.shared_secret = global.args.first().cloned().unwrap();
        },
        3 | 4 => {
            opts.shared_secret = global.args.clone().join("-");
        },
        _ => {},
    }
    // load options here
    // setDebugLevel(c)
    // let do_remember = global.remember;
    let mut cr = croc::new(opts)?;
    cr.receive()
}
