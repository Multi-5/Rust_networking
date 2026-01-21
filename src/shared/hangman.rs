use serde::{Serialize, Deserialize};
use unicode_normalization::UnicodeNormalization;


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
pub struct GameState {
    ongoing: bool,
    secret_word: String,
    guessed_letters: Vec<char>,
    word_suggester_name: String,
}

pub fn render_hangman_state(state: &GameState) -> String {
    let displayed_word: String = state.secret_word
        .chars()
        .map(|letter| {
            let normalized_letter = normalize_char(letter);
            if state.guessed_letters
                .iter()
                .any(|&guess| guess == normalized_letter)
            {
                letter  // keep original accent for display
            } else {
                '_'
            }
        })
        .collect();


    let normalized_word: Vec<char> = state.secret_word
        .chars()
        .map(normalize_char)
        .collect();

    let incorrect_guesses = state.guessed_letters
        .iter()
        .filter(|&&letter| !normalized_word.contains(&letter))
        .count();


    let mut out = String::new();
    out.push_str("\n");
    out.push_str(" ---------------- \n");

    out.push_str(&format!("Word: {}\n", displayed_word));

    if state.guessed_letters.is_empty() {
        out.push_str("Start with your guesses!\n");
    } else {
        out.push_str(&format!(
            "Guessed letters: {}\n",
            state.guessed_letters.iter().collect::<String>()
        ));
    }

    out.push_str(&format!(
        "Incorrect guesses: {}\n",
        incorrect_guesses
    ));

    if is_word_solved(state) && incorrect_guesses < HANGMAN_STRINGS.len() - 1 {
        out.push_str("\nSuccess! You guessed the word - hangman is safe.");
    } else if incorrect_guesses < HANGMAN_STRINGS.len() - 1 {
        out.push_str(HANGMAN_STRINGS[incorrect_guesses]);
        out.push_str("\nHangman can still be saved - guess wisely!");
    } else {
        out.push_str(HANGMAN_STRINGS.last().unwrap());
        out.push_str("\nGame Over!");
    }
    out.push_str("\n ---------------- ");
    out.push_str("\n");

    out
}


pub fn is_word_solved(state: &GameState) -> bool {
    state.secret_word
        .chars()
        .filter(|c| c.is_alphabetic())
        .map(normalize_char)
        .all(|c| state.guessed_letters.contains(&c))
}



fn normalize_char(c: char) -> char {
    if c.is_alphabetic() {
        c.nfd().next().unwrap().to_lowercase().next().unwrap()
    } else {
        c
    }
}


pub fn check_letter(input: &str, game_state: &mut GameState) -> Result<bool, String> {
    if !game_state.ongoing {
        return Err(String::from("This match is already over, cannot check new letters for it!"));
    }
    if input.chars().count() != 1 {
        return Err(String::from("Please enter exactly one letter"));
    }


    let letter = normalize_char(input.chars().next().unwrap());

    if game_state.guessed_letters.contains(&letter) {
        return Err(String::from("You already guessed this letter"));
    }

    game_state.guessed_letters.push(letter);

    let letter_in_word = game_state.secret_word
        .chars()
        .map(normalize_char)
        .any(|c| c == letter);


    if is_word_solved(game_state) {
        game_state.ongoing = false;
    }

    Ok(letter_in_word)
}


pub fn create_hangman_match(pl_creator: &str, word: &str) -> GameState {
    let game = GameState {
        ongoing: true,
        secret_word: String::from(word),
        guessed_letters: Vec::new(),
        word_suggester_name: String::from(pl_creator),
    };
    game
}