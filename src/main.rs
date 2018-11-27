use std::io::Write;
use rand::seq::SliceRandom;
use std::borrow::{Borrow, BorrowMut};

use std::rc::Rc;
use std::cell::RefCell;

use stdweb::web::{document, INode, IEventTarget, Element, IElement, Document};
use stdweb::web::event::ClickEvent;
use stdweb::web::error::InvalidCharacterError;

struct Reactive<T> {
    inner: T,
    listeners: Vec<Box<FnMut(&T)>>,
}

impl<T> Borrow<T> for Reactive<T> {
    fn borrow(&self) -> &T {
        &self.inner
    }
}
impl<T> BorrowMut<T> for Reactive<T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

struct ReactiveMut<'a, T> {
    reactive: &'a mut Reactive<T>
}

impl<'a, T> Drop for ReactiveMut<'a, T> {
    fn drop(&mut self) {
        for f in self.reactive.listeners.iter_mut() {
            f(&self.reactive.inner);
        }
    }
}

impl<'a, T> std::ops::Deref for ReactiveMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.reactive.inner
    }
}
impl<'a, T> std::ops::DerefMut for ReactiveMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.reactive.inner
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Reactive<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "Reactive({:?})", self.inner)
    }
}
impl<T: std::fmt::Display> std::fmt::Display for Reactive<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.inner.fmt(fmt)
    }
}
impl<T: PartialEq> PartialEq for Reactive<T> {
    //type Rhs = Reactive<T::Rhs>;
    fn eq(&self, rhs: &Self) -> bool {
        self.inner.eq(&rhs.inner)
    }
}
impl<T: Eq> Eq for Reactive<T> {
}

impl<T> Reactive<T> {
    fn new(inner: T) -> Reactive<T> {
        Reactive {
            inner,
            listeners: vec![],
        }
    }

    fn register<F: FnMut(&T) + 'static>(&mut self, mut f: F) {
        f(&self.inner);
        self.listeners.push(Box::new(f));
    }

    fn lock<'a>(&'a mut self) -> ReactiveMut<'a, T> {
        ReactiveMut {
            reactive: self
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
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

fn light_widget(doc: &Document, game: Rc<RefCell<Game>>, row: usize, col: usize) -> Result<Element, InvalidCharacterError> {
    let button = doc.create_element("div")?;
    let button_clone = button.clone();

    let game_clone = game.clone();
    button.add_event_listener(move |_: ClickEvent| {
        RefCell::borrow_mut(&game_clone).make_move(row, col);
    });

    RefCell::borrow_mut(&game).lights[row * 5 + col].register(move |light| {
        while let Some(child) = button.first_child() {
            button.remove_child(&child).unwrap();
        }

        const STYLE_ON:  &str = "width: 3em; height: 3em; background: #ff9;";
        const STYLE_OFF: &str = "width: 3em; height: 3em; background: black;";
        let style = if light.status { STYLE_ON } else { STYLE_OFF };

        button.set_attribute("style", style).unwrap();
    });

    Ok(button_clone)
}

fn moves_widget(doc: &Document, game: Rc<RefCell<Game>>) -> Result<Element, InvalidCharacterError> {
    let span = doc.create_element("span")?;
    let span_clone = span.clone();
    span.append_child(&doc.create_text_node(&RefCell::borrow(&game).moves.to_string()));
    RefCell::borrow_mut(&game).moves.register(move |new_moves| {
        while let Some(child) = span.first_child() {
            span.remove_child(&child).unwrap();
        }

        span.append_child(&document().create_text_node(&new_moves.to_string()));
    });
    Ok(span_clone)
}

#[derive(Debug, PartialEq, Eq)]
struct Game {
    lights: [Reactive<Light>; 25],
    moves: Reactive<usize>,
}

impl Game {
    fn new_empty() -> Game {
        Game {
            lights: unsafe {
                let mut lights: [Reactive<Light>; 25] =  std::mem::uninitialized();
                for element in lights.iter_mut() {
                    std::ptr::write(element, Reactive::new(Light::new()));
                }
                lights
            },
            moves: Reactive::new(0),
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

        self.lights[i].lock().toggle();

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

        self.lights[row * 5 + col].lock().toggle();
    }

    fn check_rc(&self, row: usize, col: usize) -> bool {
        assert!(row < 5);
        assert!(col < 5);

        self.lights[row * 5 + col].inner.status
    }

    fn all_off(&self) -> bool {
        for i in 0..25 {
            if self.lights[i].inner.status {
                return false;
            }
        }
        true
    }

    // like toggle_rc, but increments the move counter
    fn make_move(&mut self, row: usize, col: usize) {
        self.toggle(row * 5 + col);
        *self.moves.lock() += 1;
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

/*
fn main() -> Result<(), std::io::Error> {
    let mut game = Game::new_random(4, &mut rand::thread_rng());

    game.moves.register(|moves| {
        println!("New number of moves: {}", moves);
    });

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
*/

fn main() -> Result<(), Box<dyn std::error::Error>> {
    /*
    let mut game = Game::new_random(4, &mut rand::thread_rng());
     */
    let game = Rc::new(RefCell::new(Game::new_empty()));
    let doc = document();

    let body = match doc.body() {
        None => return Err(From::from("Could not find body element")),
        Some(body) => body,
    };

    let table = doc.create_element("table")?;
    body.append_child(&table);
    for row in 0..5 {
        let tr = doc.create_element("tr")?;
        table.append_child(&tr);
        for col in 0..5 {
            let td = doc.create_element("td")?;
            tr.append_child(&td);
            td.append_child(&light_widget(&doc, game.clone(), row, col)?);
        }
    }

    let p = doc.create_element("p")?;
    body.append_child(&p);
    p.append_child(&doc.create_text_node("Total moves: "));
    p.append_child(&moves_widget(&doc, game.clone())?);

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
