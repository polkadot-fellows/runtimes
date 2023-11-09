use dyn_clonable::*;

use std::io::Read;

#[clonable]
trait Difficult<R>: Clone
where R: Read {
    /* ... */
}

#[clonable]
trait MyTrait: Clone {
    fn recite(&self);
}

#[clonable]
trait MyTrait2: std::clone::Clone {
    fn recite2(&self);
}

impl MyTrait for String {
    fn recite(&self) {
        println!("{} ♫", self);
    }
}

#[derive(Clone)]
struct Container {
    trait_object: Box<dyn MyTrait>
}

fn main() {
    let line = "The slithy structs did gyre and gimble the namespace";

    // Build a trait object holding a String.
    // This requires String to implement MyTrait and std::clone::Clone.
    let x: Box<dyn MyTrait> = Box::new(String::from(line));

    x.recite();

    // The type of x2 is a Box<dyn MyTrait> cloned from x.
    let x2 = dyn_clone::clone_box(&*x);

    x2.recite();
}
