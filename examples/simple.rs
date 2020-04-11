#![feature(proc_macro_hygiene)]

use fieldwise::{Fieldwise, Path};
use fieldwise_derive::{Fieldwise, path};

#[derive(Fieldwise, Debug)]
pub struct Person {
    name: String,
    age: usize,
}

fn main() {
    let mut person = Person {
        name: "Ferris".to_owned(),
        age: 7,
    };

    let name = path!(Person.name);
    let age = path!(Person.age);

    println!("Hello {}", name.get(&person).unwrap());

    // Ferris has aged one year
    *age.get_mut(&mut person).unwrap() += 1;

    println!("{} is {} years old",
        name.get(&person).unwrap(),
        age.get(&person).unwrap());
}

