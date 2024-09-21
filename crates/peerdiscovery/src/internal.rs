use pnet::datalink;

use crate::{PeerDiscovery, Settings};

// initialize returns a new peerDiscovery object which can be used to discover peers.
// The settings are optional. If any setting is not supplied, then defaults are used.
// See the Settings for more information.
pub(crate) fn initialize(_settings: &Settings) -> anyhow::Result<PeerDiscovery> {
    // @fri3nd TODO
    todo!()
}

// filterInterfaces returns a list of valid network interfaces
pub(crate) fn filter_interfaces(use_ipv4: bool) -> Vec<datalink::NetworkInterface> {
    let interfaces = datalink::interfaces();
    interfaces
        .into_iter()
        // Interface must be up and either support multicast or be a loopback interface.
        .filter(|x| x.is_up() || x.is_loopback() || x.is_multicast())
        .filter(|x| x.ips.iter().any(|y| y.is_ipv4() == use_ipv4))
        // .collect::<Vec<datalink::NetworkInterface>>()
        .collect()
}
