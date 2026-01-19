use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::env;
use rand::Rng;
use std::sync::mpsc;
use std::collections::HashSet;
use std::thread;
use serde::{Serialize, Deserialize};


// Default bind address. Can be overridden with the SERVER_ADDR env var, e.g.
// SERVER_ADDR=127.0.0.1 :9090 cargo run --bin server
// Using 127.0.0.1 lets other machines connect to this host.
const DEFAULT_LOCAL: &str = "127.0.0.1:9090";
const MSG_SIZE: usize = 120;


const HANGMAN_STRINGS: [&'static str; 10] = [
r#"
 
 
 
 
n∩"#,
r"
 |
 |
 |
 |
n∩",
r" ____
 |
 |
 |
 |
n∩",
r" ____
 |  !
 |
 |
 |
n∩",
r" ____
 |  !
 |  o
 |
 |
n∩",
r"____
 |  !
 |  o
 |  |
 |
n∩",
r"____
 |  !
 | \o
 |  |
 |
n∩",
r"____
 |  !
 | \o
 |  |\
 |
n∩",
r"____
 |  !
 | \o
 |  |\
 |   \
n∩",
r" ____
 |  !
 | \o
 |  |\
 | / \
n∩"
];


fn sleep() {
    thread::sleep(::std::time::Duration::from_millis(100));
}

#[derive(Serialize, Deserialize)]
struct GameState {
    secret_word: String,
    guessed_letters: Vec<char>,
    guesser_name: String,
    word_suggester_name: String,
}


fn display_hangman_state(state: &GameState) {
    let displayed_word: String = state.secret_word
        .chars()
        .map(|letter| {
            if state.guessed_letters.contains(&letter.to_lowercase().next().unwrap()) {
                letter
            } else {
                '_'
            }
        })
        .collect();

    println!("Word: {}", displayed_word);

    // Display previous guesses
    if state.guessed_letters.is_empty() {
        println!("Start with your guesses!");
    } else {
        println!("Guessed letters: {}", 
            state.guessed_letters.iter().collect::<String>()
        );
    }

    let incorrect_guesses = state.guessed_letters
        .iter()
        .filter(|&letter| 
            !state.secret_word.to_lowercase().contains(letter.to_lowercase().to_string().as_str())
        )
        .count();

    if incorrect_guesses > 0 {
        println!("Incorrect guesses: {}", incorrect_guesses);
        println!("{}", HANGMAN_STRINGS[incorrect_guesses]);
    }
}


fn check_letter(input: &str, game_state: &mut GameState) -> Result<bool, String> {
    // Check if input is exactly one letter
    if input.chars().count() != 1 {
        return Err(String::from("Please enter exactly one letter"));
    }

    // Convert input to lowercase
    let letter = input.chars().next().unwrap().to_lowercase().next().unwrap();

    // Check if letter was already guessed
    if game_state.guessed_letters.contains(&letter) {
        return Err(String::from("You already guessed this letter"));
    }

    // Add letter to guessed letters
    game_state.guessed_letters.push(letter);

    // Check if letter is in the secret word
    let letter_in_word = game_state.secret_word
        .to_lowercase()
        .chars()
        .any(|c| c == letter);

    Ok(letter_in_word)
}


fn create_hangman_match(pl_creator: &str, word: &str, pl_guesser: &str) -> GameState {
    let mut game = GameState {
        secret_word: String::from(word),
        guessed_letters: Vec::new(),
        guesser_name: String::from(pl_guesser),
        word_suggester_name: String::from(pl_creator),
    };
    game
}
// Example usage
fn check_examples() {

    let mut game: GameState = create_hangman_match("a", "Hangman", "player");

    // Successful guess
    match check_letter("h", &mut game) {
        Ok(true) => println!("Letter is in the word!"),
        Ok(false) => println!("Letter is not in the word."),
        Err(e) => println!("Error: {}", e),
    }

    // Duplicate guess
    match check_letter("h", &mut game) {
        Ok(true) => println!("Letter is in the word!"),
        Ok(false) => println!("Letter is not in the word."),
        Err(e) => println!("Error: {}", e),
    }
}



fn flip_coin() -> &'static str {
    //  flip a coin: 50/50
    let mut rng = rand::thread_rng();
    if rng.gen_bool(0.5) { "heads" } else { "tails" }
}

fn main() {
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
                        // registration: try to update the stored display name for this client
                        let name = content[6..].to_string();
                        println!("Registering name '{}' for {}", name, sender);

                        // If the name is already taken by another client (different addr), inform the registering client only
                        let name_taken = clients.iter().any(|(_, addr, disp)| addr != sender && disp == &name);
                        if name_taken {
                            let reject = format!("name_taken: {}\nchange the name with :name <new_name>", name);
                            let mut buf = reject.into_bytes();
                            buf.resize(MSG_SIZE, 0);
                            clients = clients.into_iter().map(|(mut client, addr, disp)| {
                                if addr == sender {
                                    // notify the registering client that the name was taken
                                    let _ = client.write_all(&buf);
                                    // record that this sender was rejected so a later successful registration can be confirmed
                                    name_rejected.insert(addr.clone());
                                }
                                (client, addr, disp)
                            }).collect();
                        } else {
                            // accept the registration and update the stored display name
                            clients = clients.into_iter().map(|(stream, addr, _disp)| {
                                if addr == sender {
                                    (stream, addr.clone(), name.clone())
                                } else {
                                    (stream, addr, _disp)
                                }
                            }).collect();

                            // If this sender was previously rejected, send a one-off confirmation to them
                            if name_rejected.remove(sender) {
                                let confirm = format!("{} is unique and was appended to your client!", name);
                                let mut confirm_buf = confirm.as_bytes().to_vec();
                                confirm_buf.resize(MSG_SIZE, 0);
                                clients = clients.into_iter().map(|(mut client, addr, disp)| {
                                    if addr == sender {
                                        let _ = client.write_all(&confirm_buf);
                                    }
                                    (client, addr, disp)
                                }).collect();
                            }

                            // announce join to others (don't send the join announcement back to the registering client)
                            let announce = format!("{} joined", name);
                            println!("Announcing: {}", announce);
                            let mut to_send = announce.into_bytes();
                            to_send.resize(MSG_SIZE, 0);
                            clients = clients.into_iter().filter_map(|(mut client, addr, disp)| {
                                if addr == sender {
                                    // keep the registering client but don't send the announce back to it
                                    Some((client, addr, disp))
                                } else {
                                    client.write_all(&to_send).map(|_| (client, addr, disp)).ok()
                                }
                            }).collect();
                        }

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

