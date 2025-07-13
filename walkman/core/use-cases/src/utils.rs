pub mod aliases {
    use std::{borrow::Cow, path::Path, pin::Pin};

    use futures_core::Stream;

    pub type MaybeOwnedString = Cow<'static, str>;
    pub type MaybeOwnedPath = Cow<'static, Path>;

    pub type BoxedStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;
}
