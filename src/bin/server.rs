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
const MSG_SIZE: usize = 500;

//allows loop to rest while it's not receiving messages
//will allow the thread to sleep for a moment, and we can call it passing the time duration
fn sleep() {
    thread::sleep(::std::time::Duration::from_millis(100));
}


fn flip_coin() -> &'static str {
    //  flip a coin: 50/50
    let mut rng = rand::thread_rng();
    if rng.gen_bool(0.5) { "heads" } else { "tails" }
}

// Helper: send buffer to all clients, removing any that fail
fn send_to_all(clients: &mut Vec<(TcpStream, String, String)>, buf: &[u8]) {
    let mut remove_idx: Vec<usize> = Vec::new();
    for (i, (client, _addr, _disp)) in clients.iter_mut().enumerate() {
        if client.write_all(buf).is_err() { remove_idx.push(i); }
    }
    for i in remove_idx.into_iter().rev() { clients.remove(i); }
}

// Helper: send buffer to all clients except the sender (by addr); remove failed clients
fn send_to_others(clients: &mut Vec<(TcpStream, String, String)>, sender: &str, buf: &[u8]) {
    let mut remove_idx: Vec<usize> = Vec::new();
    for (i, (client, addr, _disp)) in clients.iter_mut().enumerate() {
        if addr == sender { continue; }
        if client.write_all(buf).is_err() { remove_idx.push(i); }
    }
    for i in remove_idx.into_iter().rev() { clients.remove(i); }
}

// Helper: send buffer only to a single client (by addr). Does not remove other clients on failure.
fn send_to_client(clients: &mut Vec<(TcpStream, String, String)>, recipient: &str, buf: &[u8]) {
    for (client, addr, _disp) in clients.iter_mut() {
        if addr == recipient {
            let _ = client.write_all(buf);
            break;
        }
    }
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

                        // Command handling: keep :flip and :list server-side; other messages forwarded
                        match msg.as_str() {
                            ":flip" => {
                                let result = flip_coin();
                                println!("{} requested flip -> {}", addr, result);
                                // send framed message so main thread can map addr -> name
                                let framed = format!("[{}]::flipped: {}", addr, result);
                                tx.send(framed).expect("failed to send flip result to rx");
                            }
                            ":list" => {
                                // request the main loop to send the (multi-line) user list
                                let framed = format!("[{}]::{}", addr, msg);
                                tx.send(framed).expect("failed to send list request to rx");
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
                        handle_hangman_command(&mut clients, &mut name_rejected, sender, content, &mut hangman_active);
                    }

                    // list connected users (send only to requesting client)
                    if content == ":list" {
                        // build a multi-line list of display names (one per line)
                        let mut resp = String::from("connected:\n");
                        for (_, _, disp) in &clients {
                            resp.push_str(&format!("{}\n", disp));
                        }
                        let mut buf = resp.into_bytes();
                        buf.resize(MSG_SIZE, 0);
                        // write only to the requesting client (don't move the clients vec)
                        send_to_client(&mut clients, sender, &buf);
                        continue;
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
                        // broadcast to all; remove clients that fail
                        send_to_all(&mut clients, &buff);
                    } else {
                        // send to others only; keep sender always
                        send_to_others(&mut clients, sender, &buff);
                    }
                }
            } else {
                // not framed: broadcast raw
                let mut buff = recv_msg.into_bytes();
                buff.resize(MSG_SIZE, 0);
                send_to_all(&mut clients, &buff);
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
    game_active: &mut bool,
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
    content: &str,
) {
    let name = content[6..].to_string();
    println!("Registering name '{}' for {}", name, sender);

    // ---- PHASE 1: READ ONLY ----
    let name_taken = clients
        .iter()
        .any(|(_, addr, disp)| addr != sender && disp == &name);

    let previous_name = clients
        .iter()
        .find(|(_, addr, _)| addr == sender)
        .map(|(_, _, disp)| disp.clone());

    // ---- PHASE 2: MUTATE STATE ----
    if !name_taken {
        for (_stream, addr, disp) in clients.iter_mut() {
            if addr == sender {
                *disp = name.clone();
                break;
            }
        }
    }

    // ---- PHASE 3: SEND MESSAGES (no borrows alive) ----
    if name_taken {
        let reject = format!(
            "name_taken: {}\nchange the name with :name <new_name>",
            name
        );
        let mut buf = reject.into_bytes();
        buf.resize(MSG_SIZE, 0);

        send_to_client(clients, sender, &buf);
        name_rejected.insert(sender.to_string());
        return;
    }

    if name_rejected.remove(sender) {
        let confirm = format!("{} is unique and was appended to your client!", name);
        let mut buf = confirm.into_bytes();
        buf.resize(MSG_SIZE, 0);
        send_to_client(clients, sender, &buf);
    }

    let announce = match previous_name {
        Some(prev) if prev != sender && prev != name =>
            format!("{} changed the name to {}", prev, name),
        _ => format!("{} joined", name),
    };

    let mut buf = announce.into_bytes();
    buf.resize(MSG_SIZE, 0);
    send_to_others(clients, sender, &buf);
}

