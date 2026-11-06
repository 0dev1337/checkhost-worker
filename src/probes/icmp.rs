use std::{
    io,
    mem::MaybeUninit,
    net::SocketAddr,
    sync::atomic::{AtomicU16, Ordering},
    time::{Duration, Instant},
};

use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use tracing::trace;

const REPLY_TIMEOUT: Duration = Duration::from_secs(2);
const ECHO_REQUEST: u8 = 8;
const ECHO_REPLY: u8 = 0;
const ICMP_HEADER_LEN: usize = 8;

// Replies on these platforms still carry the IPv4 header.
#[cfg(any(target_os = "macos", target_os = "windows"))]
const IP_HEADER_LEN: usize = 20;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const IP_HEADER_LEN: usize = 0;

#[cfg(target_os = "windows")]
const SOCKET_TYPE: Type = Type::RAW;
#[cfg(not(target_os = "windows"))]
const SOCKET_TYPE: Type = Type::DGRAM;

static NEXT_SEQUENCE: AtomicU16 = AtomicU16::new(1);

pub struct Reply {
    pub from: String,
    pub rtt: Duration,
}

/// Sends an ICMP echo request to the IPv4 address `ip` and waits for the reply.
pub async fn ping(ip: String) -> io::Result<Reply> {
    tokio::task::spawn_blocking(move || ping_blocking(&ip))
        .await
        .map_err(io::Error::other)?
}

fn ping_blocking(ip: &str) -> io::Result<Reply> {
    let socket = Socket::new(Domain::IPV4, SOCKET_TYPE, Some(Protocol::ICMPV4))?;
    socket.set_read_timeout(Some(REPLY_TIMEOUT))?;

    let destination: SockAddr = format!("{ip}:0")
        .parse::<SocketAddr>()
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?
        .into();

    let sequence = NEXT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let packet = echo_request(sequence);

    let start = Instant::now();
    socket.send_to(&packet, &destination)?;

    let mut buffer = [MaybeUninit::<u8>::uninit(); 128];
    loop {
        let (size, from) = socket.recv_from(&mut buffer)?;
        let received: Vec<u8> = buffer[..size]
            .iter()
            // SAFETY: `recv_from` initialized the first `size` bytes.
            .map(|byte| unsafe { byte.assume_init() })
            .collect();

        if is_matching_reply(&received, sequence) {
            let rtt = start.elapsed();
            let from = from
                .as_socket()
                .ok_or_else(|| io::Error::other("reply from a non-IP address"))?
                .ip()
                .to_string();
            return Ok(Reply { from, rtt });
        }

        trace!(size, "ignoring unrelated ICMP packet");
    }
}

fn echo_request(sequence: u16) -> Vec<u8> {
    let [sequence_high, sequence_low] = sequence.to_be_bytes();
    let mut packet = vec![
        ECHO_REQUEST,
        0, // code
        0,
        0, // checksum, filled in below
        0,
        1, // identifier
        sequence_high,
        sequence_low,
    ];
    packet.extend_from_slice(b"hello");

    let checksum = checksum(&packet).to_be_bytes();
    packet[2..4].copy_from_slice(&checksum);
    packet
}

fn is_matching_reply(received: &[u8], sequence: u16) -> bool {
    match received.get(IP_HEADER_LEN..) {
        Some(icmp) if icmp.len() >= ICMP_HEADER_LEN => {
            icmp[0] == ECHO_REPLY && u16::from_be_bytes([icmp[6], icmp[7]]) == sequence
        }
        _ => false,
    }
}

/// RFC 1071 Internet checksum.
fn checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = data
        .chunks(2)
        .map(|chunk| match *chunk {
            [high, low] => u32::from(u16::from_be_bytes([high, low])),
            [high] => u32::from(high) << 8,
            _ => unreachable!("chunks(2) yields one or two bytes"),
        })
        .sum();

    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}
