# Improv (WIP)
### An improvised Actor Model implementation in Rust

This is not meant to be a "true" Actor Model implementation. The sole 
objective of this crate is to provide the concurrency benefits of the 
Actor Model in Rust.

This crate is comprised of traits for custom implementations of 
threading, with a little concrete glue, as well as pre-made 
implementations: Tokio, Rayon, and a rudimentary std impl using 
`thread::spawn`. Focus will be on getting Tokio working.

Using this crate should be as simple as these few steps:  
1) Initialize an `ActorSystem` with the Driver implementation
of your choice.  
2) Implement the `Actor` trait.  
3) Run `ActorSystem::register` to start your `Actor`.  
4) Use the returned `ActorRef<YourActor>` to send messages to
your `YourActor`.

## Planned features
- Monitors  
- Async traits (blocked)  


## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
