use serde::{Serialize, Deserialize};

pub const HANGMAN_STRINGS: [&'static str; 10] = [
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

#[derive(Serialize, Deserialize)]
struct GameState {
    ongoing: bool,
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

    println!("Incorrect guesses: {}", incorrect_guesses);

    if incorrect_guesses < HANGMAN_STRINGS.len() - 1 {
        println!("{}", HANGMAN_STRINGS[incorrect_guesses]);
        println!("\nhangman can still be saved - guess wisely!")
    } else {
        println!("{}", HANGMAN_STRINGS[HANGMAN_STRINGS.len()-1]);
        print!("\nGame Over! :/")
    }
}


fn check_letter(input: &str, game_state: &mut GameState) -> Result<bool, String> {
    if !game_state.ongoing {
        return Err(String::from("This match is already over, cannot check new letters for it!"));
    }
    if input.chars().count() != 1 {
        return Err(String::from("Please enter exactly one letter"));
    }


    let letter = input.chars().next().unwrap().to_lowercase().next().unwrap();

    if game_state.guessed_letters.contains(&letter) {
        return Err(String::from("You already guessed this letter"));
    }

    game_state.guessed_letters.push(letter);
    let letter_in_word = game_state.secret_word
        .to_lowercase()
        .chars()
        .any(|c| c == letter);

    Ok(letter_in_word)
}


fn create_hangman_match(pl_creator: &str, word: &str, pl_guesser: &str) -> GameState {
    let mut game = GameState {
        ongoing: true,
        secret_word: String::from(word),
        guessed_letters: Vec::new(),
        guesser_name: String::from(pl_guesser),
        word_suggester_name: String::from(pl_creator),
    };
    game
}

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