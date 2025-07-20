use sandbox_infrastructures::domain::{self, Accept as _, Diagnostic, Request};

fn main() {
    let actor = domain::new_actor();
    actor.accept(Request("Hello".into())); // ✅ OK

    actor.accept(Diagnostic("Nope".into())); // ❌ Won't compile: `Diagnostic` is public, but `impl Accept<Diagnostic>` is not visible
}
