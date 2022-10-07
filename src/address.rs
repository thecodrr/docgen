use port_scanner::*;
use std::net::SocketAddr;

pub fn get_safe_addr(ip: &str, port: u16) -> Option<SocketAddr> {
    if local_port_available(port) {
        format!("{}:{}", ip, port).parse::<SocketAddr>().ok()
    } else if let Some(available_port) = request_open_port() {
        format!("{}:{}", ip, available_port)
            .parse::<SocketAddr>()
            .ok()
    } else {
        None
    }
}
