use console::Term;
use std::io;

pub fn wait_for_enter(prompt: &str) {
    println!();
    println!("{}", prompt);
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
}

pub fn clear_screen() -> io::Result<()> {
    Term::stdout().clear_screen()
}