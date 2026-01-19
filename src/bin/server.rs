use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use serde::{Serialize, Deserialize};

const LOCAL: &str = "127.0.0.1:9090";
const MSG_SIZE: usize = 32;


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



fn main() {
    let server = TcpListener::bind(LOCAL).expect("Listener failed to bind");
    server.set_nonblocking(true).expect("failed to initialize non-blocking");

    let mut clients: Vec<std::net::TcpStream> = vec![];
    let (tx, rx) = mpsc::channel::<String>();
    loop {
        if let Ok((mut socket, addr)) = server.accept() {
            println!("Client {} connected", addr);

            let tx = tx.clone();
            clients.push(socket.try_clone().expect("failed to clone client"));

            thread::spawn(move || loop {
                let mut buff = vec![0; MSG_SIZE];

                match socket.read_exact(&mut buff) {
                    Ok(_) => {
                        let msg = buff.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
                        let msg = String::from_utf8(msg).expect("Invalid utf8 message");

                        println!("{}: {:?}", addr, msg);
                        tx.send(msg).expect("failed to send msg to rx");
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

        if let Ok(msg) = rx.try_recv() {
            clients = clients.into_iter().filter_map(|mut client| {
                let mut buff = msg.clone().into_bytes();
                buff.resize(MSG_SIZE, 0);

                client.write_all(&buff).map(|_| client).ok()
            }).collect::<Vec<_>>();
        }

        sleep();
    }
}

