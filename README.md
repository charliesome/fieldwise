# Fieldwise

Functional lenses for Rust.

```rust
struct Crab {
    name: String,
}

let crab = Crab { name: "Ferris".to_owned() };

let name = path!(Crab.name);

println!("Hello {}", name.get(&crab).unwrap());
```
