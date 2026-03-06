use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::mem::MaybeUninit;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU16, Ordering};

static SEQ: AtomicU16 = AtomicU16::new(1);
fn calculate_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i + 1 < data.len() {
        sum += u16::from_be_bytes([data[i], data[i + 1]]) as u32;
        i += 2;
    }
    if i < data.len() {
        sum += (data[i] as u32) << 8;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

pub struct PingResult {
    pub ip: String,
    pub rtt_ms: u128,
    pub success: bool,
    pub message: String,
}

pub fn ping(ip: &str) -> PingResult {
    let result = do_ping(ip);
    match result {
        Ok((rtt_ms, from)) => PingResult {
            ip: from,
            rtt_ms,
            success: true,
            message: "OK".to_string(),
        },
        Err(e) => PingResult {
            ip: ip.to_string(),
            rtt_ms: 0,
            success: false,
            message: e.to_string(),
        },
    }
}

fn do_ping(ip: &str) -> Result<(u128, String), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    let socket = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))?;
    #[cfg(not(target_os = "windows"))]
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::ICMPV4))?;

    socket.set_read_timeout(Some(Duration::from_secs(2)))?;

    // ✅ Unique sequence per request
    let sequence: u16 = SEQ.fetch_add(1, Ordering::Relaxed);

    let mut packet: Vec<u8> = vec![
        8, 0, 0, 0, 0, 1,
        (sequence >> 8) as u8, (sequence & 0xff) as u8,
        b'h', b'e', b'l', b'l', b'o',
    ];
    let checksum = calculate_checksum(&packet);
    packet[2] = (checksum >> 8) as u8;
    packet[3] = (checksum & 0xff) as u8;

    let dest: SockAddr = format!("{}:0", ip).parse::<SocketAddr>()?.into();

    let start = Instant::now();
    socket.send_to(&packet, &dest)?;

    let mut buffer = [MaybeUninit::<u8>::uninit(); 128];

    // ✅ Loop until we get the reply matching OUR sequence
    loop {
        let (size, from_addr) = socket.recv_from(&mut buffer)?;

        let buf: Vec<u8> = buffer[..size]
            .iter()
            .map(|b| unsafe { b.assume_init() })
            .collect();

        #[cfg(any(target_os = "macos", target_os = "windows"))]
        const IP_HEADER_LEN: usize = 20;
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        const IP_HEADER_LEN: usize = 0;

        let data = &buf[IP_HEADER_LEN..];
        if data.len() < 8 {
            continue;
        }

        // ✅ Validate ICMP type and sequence number
        let reply_seq = u16::from_be_bytes([data[6], data[7]]);
        if data[0] == 0 && reply_seq == sequence {
            let rtt = start.elapsed().as_millis();
            let ip_str = from_addr.as_socket().unwrap().ip().to_string();
            return Ok((rtt, ip_str));
        }
        // Otherwise loop and wait for the correct reply
    }
}
