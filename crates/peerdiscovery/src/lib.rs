use std::time::Duration;

mod internal;
mod listener;
mod peerdiscovery;

pub use self::peerdiscovery::*;

// IPVersion specifies the version of the Internet Protocol to be used.
#[derive(Debug, Clone, PartialEq)]
pub enum IPVersion {
    V4,
    V6,
}

// PeerDiscovery is the object that can do the discovery for finding LAN peers.
pub struct PeerDiscovery {
    pub settings: Settings,
    // received map[string]*PeerState
    // sync.RWMutex
    // exit bool
}

// Discovered is the structure of the discovered peers,
// which holds their local address (port removed) and
// a payload if there is one.
pub struct Discovered {
    // Address is the local address of a discovered peer.
    pub address: String,
    // Payload is the associated payload from discovered peer.
    pub payload: Vec<u8>,
    // Metadata *Metadata
}

// Settings are the settings that can be specified for
// doing peer discovery.
#[derive(Clone)]
pub struct Settings {
    // Port is the port to broadcast on (the peers must also broadcast using the same port).
    // The default port is 9999.
    pub port: u16,
    // MulticastAddress specifies the multicast address.
    // You should be able to use any of 224.0.0.0/4 or ff00::/8.
    // By default it uses the Simple Service Discovery Protocol
    // address (239.255.255.250 for IPv4 or ff02::c for IPv6).
    pub multicast_address: String,
    // Payload is the bytes that are sent out with each broadcast. Must be short.
    pub payload: Vec<u8>,
    // Delay is the amount of time between broadcasts. The default delay is 1 second.
    pub delay: Duration,
    // TimeLimit is the amount of time to spend discovering, if the limit is not reached.
    // A negative limit indiciates scanning until the limit was reached or, if an
    // unlimited scanning was requested, no timeout.
    // The default time limit is 10 seconds.
    pub time_limit: i64,
    // DisableBroadcast will not allow sending out a broadcast
    pub disable_broadcast: bool,
    // IPVersion specifies the version of the Internet Protocol (default IPv4)
    pub ip_version: IPVersion,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            multicast_address: "".into(),
            port: 9999,
            payload: vec![],
            delay: Duration::from_secs(1),
            time_limit: 10,
            disable_broadcast: false,
            ip_version: IPVersion::V4,
        }
    }
}
