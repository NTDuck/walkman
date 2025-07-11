pub mod aliases {
    use std::{borrow::Cow, pin::Pin};

    use futures_core::Stream;

    pub type MaybeOwnedString = Cow<'static, str>;

    pub type BoxedStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;
}
