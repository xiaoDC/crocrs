use super::{determine_pass, GlobalArgs, SendArgs};
use crate::croc;

pub(super) fn send(
    args: &SendArgs,
    global: &GlobalArgs,
) -> anyhow::Result<()> {
    let port_param: u16 = if args.port == 0 { 9009 } else { args.port };
    let transfers_param: usize = if args.transfers == 0 { 4 } else { args.transfers };
    let mut ports = Vec::with_capacity(transfers_param + 1);
    for i in 0..(transfers_param + 1) {
        ports.push((port_param + i as u16).to_string());
    }
    let opts = croc::Options {
        // @fri3nd TODO
        // todo!()
        shared_secret: "8971-djkasjd".into(),
        // @fri3nd TODO
        // todo!()
        is_sender: true,
        zip_folder: args.zip,
        git_ignore: args.git,
        relay_address: global.relay.clone(),
        relay_address6: global.relay6.clone(),
        relay_password: determine_pass(&global.pass),
        disable_local: args.no_local.unwrap_or(false),
        only_local: global.local,
        relay_ports: ports,
        ip: "".into(),
    };
    // xxxxxxxxxxxx
    // xxxxxxxxxxxx
    // xxxxxxxxxxxx
    // xxxxxxxxxxxx
    let (minimal_file_infos, empty_folders_to_transfer, total_number_folders) =
        croc::get_files_info(&args.fnames, opts.zip_folder, opts.git_ignore)?;
    let mut cr = croc::new(opts)?;

    // save the config
    // saveConfig(c, crocOptions)

    cr.send(minimal_file_infos, empty_folders_to_transfer, total_number_folders)
}
