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

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(icmp: &[u8]) -> Vec<u8> {
        let mut packet = vec![0; IP_HEADER_LEN];
        packet.extend_from_slice(icmp);
        packet
    }

    #[test]
    fn checksum_matches_rfc1071_example() {
        let data = [0x00, 0x01, 0xf2, 0x03, 0xf4, 0xf5, 0xf6, 0xf7];
        assert_eq!(checksum(&data), !0xddf2);
    }

    #[test]
    fn checksum_pads_odd_length_input() {
        assert_eq!(checksum(&[0x12]), !0x1200);
    }

    #[test]
    fn echo_request_has_valid_checksum_and_sequence() {
        let packet = echo_request(0x1234);
        assert_eq!(packet[0], ECHO_REQUEST);
        assert_eq!(&packet[6..8], &[0x12, 0x34]);
        assert_eq!(checksum(&packet), 0);
    }

    #[test]
    fn accepts_only_echo_reply_with_our_sequence() {
        let matching = reply(&[ECHO_REPLY, 0, 0, 0, 0, 1, 0x12, 0x34]);
        let other_sequence = reply(&[ECHO_REPLY, 0, 0, 0, 0, 1, 0x12, 0x35]);
        let other_type = reply(&[ECHO_REQUEST, 0, 0, 0, 0, 1, 0x12, 0x34]);

        assert!(is_matching_reply(&matching, 0x1234));
        assert!(!is_matching_reply(&other_sequence, 0x1234));
        assert!(!is_matching_reply(&other_type, 0x1234));
    }

    #[test]
    fn short_packets_are_ignored() {
        assert!(!is_matching_reply(&[], 1));
        assert!(!is_matching_reply(&reply(&[ECHO_REPLY, 0, 0]), 1));
    }
}
