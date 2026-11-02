use std::net::TcpStream;
use std::time::{Duration, Instant};


pub fn tcp_ping(address : &str) -> std::io::Result<(Duration)> {
    let start_time = Instant::now();

    let _stream = TcpStream::connect(address)?;

    let elapsed = start_time.elapsed();

    println!("Successfully Connected to {} in {:?}",address,elapsed);
    println!("Time taken in milliseconds: {} ms", elapsed.as_millis());
    Ok(elapsed)
}