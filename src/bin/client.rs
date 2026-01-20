
// Simple synchronous TCP client for the chat server. The client uses a
// small thread to concurrently read from the server while the main thread
// reads user input and sends messages. Fixed-size framing (MSG_SIZE) is used
// to match the server's framing policy.
use std::io::{self, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::env;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::Duration;

// Server address used when connecting. This can be changed to a machine
// reachable on the local network when testing with other hosts.
const LOCAL: &str = "127.0.0.1:9090";
//const LOCAL: &str = "172.20.10.3:9090";

// Message framing size in bytes. Must match the server's MSG_SIZE.
const MSG_SIZE: usize = 500;

fn main() {
    // Connect to the server and mark the socket as non-blocking. Non-blocking
    // reads paired with a short sleep keep the client responsive without
    // dedicating a blocking read loop to the main thread.
    let mut client = TcpStream::connect(LOCAL).expect("Stream failed to connect");
    client.set_nonblocking(true).expect("failed to initiate non-blocking");

    // Channel used to send user-entered messages from the main thread to the
    // network writer in the reader thread. This keeps all network writes in
    // a single place to avoid concurrent writes to the same TcpStream.
    let (tx, rx) = mpsc::channel::<String>();

    // Reader thread: reads fixed-size frames from the server and prints
    // received messages to stdout. It also receives outgoing messages from
    // the main thread through `rx` and writes them to the server.
    thread::spawn(move || loop {
        // Read from server
        let mut buff = vec![0; MSG_SIZE];
        match client.read_exact(&mut buff) {
            Ok(_) => {
                // Trim trailing zeros and convert to UTF-8 for printing.
                let msg = buff.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
                match String::from_utf8(msg) {
                    Ok(s) => println!("{}", s),
                    Err(e) => println!("message recv (invalid utf8): {:?}", e.into_bytes()),
                }
            },
            // No data available yet on non-blocking socket; continue the loop.
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
            // Read error indicates the server closed the connection.
            Err(_) => {
                println!("connection with server was severed");
                std::process::exit(0);
            }
        }

        // Check for outbound messages from the main thread and send them.
        match rx.try_recv() {
            Ok(msg) => {
                let mut buff = msg.clone().into_bytes();
                buff.resize(MSG_SIZE, 0);
                if let Err(_) = client.write_all(&buff) {
                    println!("connection with server was severed");
                    std::process::exit(0);
                }
            }, 
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => break
        }

        // Yield a small amount of time to avoid busy-waiting.
        thread::sleep(Duration::from_millis(100));
    });

    // If a name was supplied on the command line, send a registration message
    // to the server using the :name command. The code accepts either
    // `client <name>` or `client :name <name>` for convenience.
    let mut args = env::args().skip(1);
    if let Some(first) = args.next() {
        if first == ":name" {
            if let Some(name) = args.next() {
                let _ = tx.send(format!(":name {}", name));
            }
        } else {
            // treat first arg as the name directly
            let _ = tx.send(format!(":name {}", first));
        }
    }

    // Main input loop: read user input and forward it to the reader/writer
    // thread via the channel. Sending :quit will break the loop and exit.
    println!("Write a Message:");
    loop {
        let mut buff = String::new();
        io::stdin().read_line(&mut buff).expect("reading from stdin failed");
        let msg = buff.trim().to_string();
        if msg == ":quit" || tx.send(msg).is_err() {break}
    }
    println!("bye bye!");

}

/*  
To run this program you need to open 2 terminals. One for the client and one for the server. 
In the server run `cargo run`. 
Then do the same in the client. And this time you should see a message, `write a message`. 
Type something and then you should see that in the server. 
If you type ':quit' then the program will quit. 
 */