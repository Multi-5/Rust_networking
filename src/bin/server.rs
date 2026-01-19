use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::env;
use rand::Rng;
use std::sync::mpsc;
use std::collections::HashSet;
use std::thread;
use chatproject::shared::hangman::*;

// Default bind address. Can be overridden with the SERVER_ADDR env var, e.g.
// SERVER_ADDR=127.0.0.1 :9090 cargo run --bin server
// Using 127.0.0.1 lets other machines connect to this host.
const DEFAULT_LOCAL: &str = "127.0.0.1:9090";
const MSG_SIZE: usize = 120;


fn sleep() {
    thread::sleep(::std::time::Duration::from_millis(100));
}




fn flip_coin() -> &'static str {
    //  flip a coin: 50/50
    let mut rng = rand::thread_rng();
    if rng.gen_bool(0.5) { "heads" } else { "tails" }
}

fn main() {
    let mut hangman_active: bool = false;

    // Allow overriding the listening address via SERVER_ADDR environment variable.
    let local = env::var("SERVER_ADDR").unwrap_or_else(|_| DEFAULT_LOCAL.to_string());
    println!("Binding server to {}", local);
    let server = TcpListener::bind(&local).expect("Listener failed to bind");
    server.set_nonblocking(true).expect("failed to initialize non-blocking");

    // clients: Vec of (stream, peer_addr_string, display_name)
    let mut clients: Vec<(TcpStream, String, String)> = vec![];
    // track clients who recently received a name_taken so we can confirm when they later pick a unique name
    let mut name_rejected: HashSet<String> = HashSet::new();
    let (tx, rx) = mpsc::channel::<String>();
    loop {
        if let Ok((mut socket, addr)) = server.accept() {
            println!("Client {} connected", addr);

            let tx = tx.clone();
            // store (stream, addr, display_name) - display_name defaults to addr
            clients.push((socket.try_clone().expect("failed to clone client"), addr.to_string(), addr.to_string()));

            thread::spawn(move || loop {
                let mut buff = vec![0; MSG_SIZE];

                match socket.read_exact(&mut buff) {
                    Ok(_) => {
                        let msg = buff.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
                        let msg = String::from_utf8(msg).expect("Invalid utf8 message");

                        // Command handling: keep :flip server-side; other messages forwarded
                        match msg.as_str() {
                            ":flip" => {
                                let result = flip_coin();
                                println!("{} requested flip -> {}", addr, result);
                                // send framed message so main thread can map addr -> name
                                let framed = format!("[{}]::flipped: {}", addr, result);
                                tx.send(framed).expect("failed to send flip result to rx");
                            }
                            _ => {
                                // Prefix with sender addr so main thread can identify sender
                                let framed = format!("[{}]::{}", addr, msg);
                                tx.send(framed).expect("failed to send msg to rx");
                            }
                        }
                    },
                    Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
                    Err(_) => {
                        println!("closing connection with: {}", addr);
                        break;
                    }
                }

                sleep();
            });
        }

        if let Ok(recv_msg) = rx.try_recv() {
            // Messages arrive framed as "[<addr>]::<content>" from per-client threads.
            if recv_msg.starts_with('[') {
                if let Some(pos) = recv_msg.find("]::") {
                    let sender = &recv_msg[1..pos];
                    let content = &recv_msg[pos + 3..];

                    if content.starts_with(":name ") {
                        try_client_name_change(&mut clients, &mut name_rejected, sender, content);

                        continue;
                    } else if content.starts_with(":h") {
                        handle_hangman_command(&mut clients, &mut name_rejected, sender, content, hangman_active)
                    }

                    // Normal message: find display name for sender (fallback to sender addr)
                    let sender_name = clients.iter().find(|(_, addr, _)| addr == sender).map(|(_, _, disp)| disp.clone()).unwrap_or_else(|| sender.to_string());
                    let to_send_str = format!("{}: {}", sender_name, content);

                    // server log using the sender name
                    println!("{}", to_send_str);

                    let mut buff = to_send_str.into_bytes();
                    buff.resize(MSG_SIZE, 0);
                    // If this is a coin-flip result (content starts with "flipped:"), send to everyone including sender.
                    // Otherwise, avoid sending the message back to the originating client to prevent duplicate echo.
                    if content.starts_with("flipped:") {
                        clients = clients.into_iter().filter_map(|(mut client, addr, disp)| {
                            client.write_all(&buff).map(|_| (client, addr, disp)).ok()
                        }).collect();
                    } else {
                        clients = clients.into_iter().filter_map(|(mut client, addr, disp)| {
                            if addr == sender {
                                // keep the sender in the clients list but don't write the message back
                                Some((client, addr, disp))
                            } else {
                                client.write_all(&buff).map(|_| (client, addr, disp)).ok()
                            }
                        }).collect();
                    }
                }
            } else {
                // not framed: broadcast raw
                let mut buff = recv_msg.into_bytes();
                buff.resize(MSG_SIZE, 0);
                clients = clients.into_iter().filter_map(|(mut client, addr, disp)| {
                    client.write_all(&buff).map(|_| (client, addr, disp)).ok()
                }).collect();
            }
        }

        sleep();
    }
}

fn handle_hangman_command(
    clients: &mut Vec<(TcpStream, String, String)>, 
    name_rejected: &mut HashSet<String>, 
    sender: &str, 
    content: &str,
    game_active: &bool
) {
    if let Some(rest) = content.strip_prefix(":start ") {
    if *game_active {
        // TODO: send to all
        return;
    }
    // rest = arguments after :start
} else if content == ":end" {
    // end game
}
}

fn try_client_name_change(
    clients: &mut Vec<(TcpStream, String, String)>, 
    name_rejected: &mut HashSet<String>, 
    sender: &str, 
    content: &str
) {
    // registration: try to update the stored display name for this client
    let name = content[6..].to_string();
    println!("Registering name '{}' for {}", name, sender);

    // If the name is already taken by another client (different addr), inform the registering client only
    let name_taken = clients.iter().any(|(_, addr, disp)| addr != sender && disp == &name);
    if name_taken {
        let reject = format!("name_taken: {}\nchange the name with :name <new_name>", name);
        let mut buf = reject.into_bytes();
        buf.resize(MSG_SIZE, 0);
        let old_clients = std::mem::take(clients);

        *clients = old_clients
        .into_iter()
        .map(|(mut client, addr, disp)| {
            if addr == sender {
                let _ = client.write_all(&buf);
                name_rejected.insert(addr.clone());
            }
            (client, addr, disp)
        })
        .collect();
    } else {
        // accept the registration and update the stored display name
        
        let old_clients = std::mem::take(clients);

        *clients = old_clients
            .into_iter()
            .map(|(stream, addr, disp)| {
                if addr == sender {
                    (stream, addr, name.clone())
                } else {
                    (stream, addr, disp)
                }
            })
            .collect();
        // If this sender was previously rejected, send a one-off confirmation to them
        if name_rejected.remove(sender) {
            let confirm = format!("{} is unique and was appended to your client!", name);
            let mut confirm_buf = confirm.as_bytes().to_vec();
            confirm_buf.resize(MSG_SIZE, 0);
            let old_clients = std::mem::take(clients);

            *clients = old_clients
            .into_iter()
            .map(|(mut client, addr, disp)| {
                if addr == sender {
                    let _ = client.write_all(&confirm_buf);
                }
                (client, addr, disp)
            })
            .collect();
        }

        // announce join to others (don't send the join announcement back to the registering client)
        let announce = format!("{} joined", name);
        println!("Announcing: {}", announce);
        let mut to_send = announce.into_bytes();
        to_send.resize(MSG_SIZE, 0);
        let old_clients = std::mem::take(clients);

        *clients = old_clients
        .into_iter()
        .filter_map(|(mut client, addr, disp)| {
            if addr == sender {
                Some((client, addr, disp))
            } else {
                client.write_all(&to_send).map(|_| (client, addr, disp)).ok()
            }
        })
        .collect();

    }
}


