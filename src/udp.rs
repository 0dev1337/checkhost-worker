
use std::net::UdpSocket;
use std::time::{Duration, Instant};
use std::io;

fn hex_to_bytes(hex: &str) -> io::Result<Vec<u8>> {
    let hex = hex.strip_prefix("0x")
        .or_else(|| hex.strip_prefix("0X"))
        .unwrap_or(hex);

    if hex.len() % 2 != 0 || !hex.is_ascii() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid HEX string"
        ));
    }

    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| io::Error::new(
                    io::ErrorKind::InvalidInput, e
                ))
        })
        .collect()
}

pub fn udp_ping(address: &str) -> io::Result<Duration> {

    let socket = UdpSocket::bind("0.0.0.0:0")?;

    socket.set_read_timeout(Some(Duration::from_secs(5)))?;
    // dns palyoad
    let payload = hex_to_bytes(
        "0x241a010000010000000000000377777706676f6f676c6503636f6d0000010001"
    )?;

    let start_time = Instant::now();

    socket.send_to(&payload, address)?;

    let mut buf = [0; 1024];

    let (size, source) = socket.recv_from(&mut buf)?;

    let elapsed = start_time.elapsed();

    println!("UDP Reply from: {}", source);
    println!("Response size: {} bytes", size);
    println!("UDP Reply received in: {:?}", elapsed);

    Ok(elapsed)
}
