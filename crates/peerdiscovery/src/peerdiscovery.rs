use std::net::{
    Ipv4Addr,
    Ipv6Addr,
    // IpAddr, ,
    SocketAddr,
    UdpSocket,
};
use std::str::FromStr;
use std::time::{Duration, Instant};

use crate::{internal, Discovered, IPVersion, PeerDiscovery, Settings};

// Discover will use the created settings to scan for LAN peers. It will return
// an array of the discovered peers and their associate payloads. It will not
// return broadcasts sent to itself.
pub fn discover(settings: &[Settings]) -> anyhow::Result<Vec<Discovered>> {
    let (_, discoveries) = new_peer_discovery(settings)?;
    Ok(discoveries)
}

fn new_peer_discovery(settings: &[Settings]) -> anyhow::Result<(PeerDiscovery, Vec<Discovered>)> {
    let s = if settings.is_empty() {
        Settings::default()
    } else {
        settings.first().cloned().unwrap()
    };
    let p = internal::initialize(&s)?;
    // p.RLock()
    let address: String =
        SocketAddr::new(p.settings.multicast_address.parse()?, p.settings.port).to_string();
    // p.RUnlock()

    let ifaces = internal::filter_interfaces(p.settings.ip_version == IPVersion::V4);
    if ifaces.is_empty() {
        anyhow::bail!("no multicast interface found")
    }
    let socket = socket2::Socket::from(UdpSocket::bind(&address)?);
    if p.settings.ip_version == IPVersion::V4 {
        let group = Ipv4Addr::from_str(&p.settings.multicast_address)?;
        for iface in &ifaces {
            for ip in &iface.ips {
                if let ipnetwork::IpNetwork::V4(x) = ip {
                    socket.join_multicast_v4(group, x.network())?;
                }
            }
        }
    } else {
        let group = Ipv6Addr::from_str(&p.settings.multicast_address)?;
        for iface in &ifaces {
            socket.join_multicast_v6(&group, iface.index)?;
        }
    }

    // @fri3nd TODO
    // go p.listen(c)
    // @fri3nd TODO
    let start = Instant::now();
    let mut interval = tokio::time::interval(p.settings.delay);
    loop {
        if !s.disable_broadcast {
            // let payload = p.settings.payload;
            // write to multicast
            // for _iface in &ifaces {
            if socket.set_multicast_loop_v4(true).is_ok() {
                let _ = socket.set_ttl(2);
                let _ = socket.send_to(&p.settings.payload, &address);
            }
            // }
        }

        // let _ = interval.tick();
        if p.settings.time_limit > 0
            && start.elapsed() > Duration::from_secs(p.settings.time_limit as u64)
        {
            break;
        }
    }

    // Open up a connection
    // @fri3nd TODO
    todo!()
}
