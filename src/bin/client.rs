
use std::io::{self, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::env;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::Duration;

const LOCAL: &str = "127.0.0.1:9090";
const MSG_SIZE: usize = 500;

fn main() {
    let mut client = TcpStream::connect(LOCAL).expect("Stream failed to connect");
    client.set_nonblocking(true).expect("failed to initiate non-blocking");

    let (tx, rx) = mpsc::channel::<String>();

    thread::spawn(move || loop {
        let mut buff = vec![0; MSG_SIZE];
        match client.read_exact(&mut buff) {
            Ok(_) => {
                let msg = buff.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
                match String::from_utf8(msg) {
                    Ok(s) => println!("{}", s),
                    Err(e) => println!("message recv (invalid utf8): {:?}", e.into_bytes()),
                }
            },
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
            Err(_) => {
                println!("connection with server was severed");
                std::process::exit(0);
            }
        }

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

        thread::sleep(Duration::from_millis(100));
    });

    // If a name was supplied as CLI args, send registration to server.
    // Accept either: `client <name>` or `client :name <name>` for convenience.
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