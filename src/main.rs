extern crate rand;

use std::io::Write;
use rand::seq::SliceRandom;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Light {
    status: bool,
}

impl Light {
    fn new() -> Light {
        Light {
            status: false,
        }
    }

    fn toggle(&mut self) {
        self.status = !self.status;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Game {
    lights: [Light; 25],
    moves: usize,
}

impl Game {
    fn new_empty() -> Game {
        Game {
            lights: unsafe {
                let mut lights: [Light; 25] =  std::mem::uninitialized();
                for element in lights.iter_mut() {
                    std::ptr::write(element, Light::new());
                }
                lights
            },
            moves: 0,
        }
    }

    fn new_random<R: rand::Rng>(difficulty: usize, rng: &mut R) -> Game {
        assert!(difficulty <= 25);
        let mut game = Game::new_empty();

        let mut toggles: [bool; 25] = [false; 25];
        for i in 0..difficulty {
            toggles[i] = true;
        }
        toggles.shuffle(rng);

        for i in 0..25 {
            if toggles[i] {
                game.toggle(i);
            }
        }
        game
    }

    fn toggle(&mut self, i: usize) {
        assert!(i < 25);

        let row = i / 5;
        let col = i % 5;

        self.lights[i].toggle();

        if row > 0 {
            self.toggle_rc(row - 1, col);
        }
        if row < 4 {
            self.toggle_rc(row + 1, col);
        }
        if col > 0 {
            self.toggle_rc(row, col - 1);
        }
        if col < 4 {
            self.toggle_rc(row, col + 1);
        }
    }

    fn toggle_rc(&mut self, row: usize, col: usize) {
        assert!(row < 5);
        assert!(col < 5);

        self.lights[row * 5 + col].toggle();
    }

    fn check_rc(&self, row: usize, col: usize) -> bool {
        assert!(row < 5);
        assert!(col < 5);

        self.lights[row * 5 + col].status
    }

    fn all_off(&self) -> bool {
        for i in 0..25 {
            if self.lights[i].status {
                return false;
            }
        }
        true
    }

    // like toggle_rc, but increments the move counter
    fn make_move(&mut self, row: usize, col: usize) {
        self.toggle(row * 5 + col);
        self.moves += 1;
    }
}

impl std::fmt::Display for Game {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, " 01234")?;

        for row in 0..5 {
            write!(fmt, "\n{}", row)?;
            for col in 0..5 {
                let c = if self.check_rc(row, col) {
                    '!'
                } else {
                    ' '
                };
                write!(fmt, "{}", c)?;
            }
        }

        write!(fmt, "\n\nTotal moves: {}\n", self.moves)?;

        Ok(())
    }
}

fn read_usize(buffer: &mut String, stdin: &std::io::Stdin, stdout: &std::io::Stdout, label: &str) -> Result<usize, std::io::Error> {
    loop {
        print!("{}", label);
        let mut stdout_lock = stdout.lock();
        stdout_lock.flush()?;
        buffer.clear();
        stdin.read_line(buffer)?;
        let trimmed = buffer.trim();
        match trimmed.parse::<usize>() {
            Ok(x) => {
                if x < 5 {
                    return Ok(x);
                } else {
                    println!("You must enter a number between 0 and 4");
                }
            }
            Err(e) => {
                println!("Invalid input {:?}: {}", trimmed, e);
            }
        }
    }
}

fn main() -> Result<(), std::io::Error> {
    let mut game = Game::new_random(4, &mut rand::thread_rng());
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut buffer = String::new();
    let mut read_usize = move |label| {
        read_usize(&mut buffer, &stdin, &stdout, label)
    };
    while !game.all_off() {
        println!("{}", game);
        let row = read_usize("Row   : ")?;
        let col = read_usize("Column: ")?;
        game.make_move(row, col);
    }

    println!("You win!");
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_toggle_twice_is_noop() {
        let game1 = Game::new_random(4, &mut rand::thread_rng());
        let mut game2 = game1.clone();
        assert_eq!(game1, game2);
        for i in 0..25 {
            game2.toggle(i);
            assert_ne!(game1, game2);
            game2.toggle(i);
            assert_eq!(game1, game2);
        }
    }

    #[test]
    fn test_toggle_corner() {
        let mut game = Game::new_empty();

        assert_eq!(false, game.check_rc(0, 0));
        assert_eq!(false, game.check_rc(1, 0));
        assert_eq!(false, game.check_rc(0, 1));
        assert_eq!(false, game.check_rc(1, 1));

        game.toggle(0);

        assert_eq!(true, game.check_rc(0, 0));
        assert_eq!(true, game.check_rc(1, 0));
        assert_eq!(true, game.check_rc(0, 1));
        assert_eq!(false, game.check_rc(1, 1));
    }

    #[test]
    fn test_all_off() {
        let mut game = Game::new_empty();
        assert_eq!(true, game.all_off());
        game.toggle(0);
        assert_eq!(false, game.all_off());
    }

    #[test]
    fn test_actually_random( ){
        // odds of this happening are infinitesmally small
        assert_eq!(false, Game::new_random(25, &mut rand::thread_rng()).all_off());
    }
}
