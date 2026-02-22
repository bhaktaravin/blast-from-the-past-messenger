use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};

use std::thread;
use std::time::Duration;

fn main() -> io::Result<()> {
    // Create a TCP listener that listens on port 8080
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    listener.set_nonblocking(false)?;

    println!("Listening for incoming connections on 127.0.0.1:8080...");

    // Accept incoming connections and spawn a new thread to handle each one
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let addr = stream.peer_addr()?;
                println!("New connection from {}", addr);

                // Spawn a new thread to handle the connection
                thread::spawn(move || {
                    if let Err(e) = handle_client(stream) {
                        eprintln!("Error handling client {}: {}", addr, e);
                    }
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Non-blocking operation would block - continue
                thread::sleep(Duration::from_millis(100));
                continue;
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
                continue;
            }
        }
    }

    Ok(())
}

fn handle_client(mut stream: TcpStream) -> io::Result<()> {
    let mut buffer = [0; 1024];

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                // Connection closed by client
                println!("Client disconnected: {}", stream.peer_addr()?);
                break;
            }
            Ok(n) => {
                let data = &buffer[..n];
                println!("Received {} bytes: {}", n, String::from_utf8_lossy(data));

                // Echo back for testing
                stream.write_all(data)?;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Non-blocking operation would block - continue
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    Ok(())
}
