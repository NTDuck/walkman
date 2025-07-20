pub mod domain {
    // Public request type
    pub struct Request(pub String);

    // Public actor
    pub struct Actor;

    pub trait Accept<T> {
        fn accept(&self, input: T);
    }

    // Public API
    impl Accept<Request> for Actor {
        fn accept(&self, input: Request) {
            println!("Handling request: {}", input.0);
            internal::log(format!("Handled: {}", input.0));
        }
    }

    // Public diagnostic event
    pub struct Diagnostic(pub String);

    // Internal helper impl
    mod internal {
        use super::*;

        impl Accept<Diagnostic> for Actor {
            fn accept(&self, input: Diagnostic) {
                println!("[DIAG] {}", input.0);
            }
        }

        pub fn log(msg: String) {
            Actor.accept(Diagnostic(msg));
        }
    }

    pub fn new_actor() -> Actor {
        Actor
    }
}
