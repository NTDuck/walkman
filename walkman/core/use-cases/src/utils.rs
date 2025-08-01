pub mod aliases {
    pub type Fallible<T> = ::anyhow::Result<T>;

    pub type MaybeOwnedString = ::std::borrow::Cow<'static, str>;
    pub type MaybeOwnedPath = ::std::borrow::Cow<'static, ::std::path::Path>;
    pub type MaybeOwnedVec<T> = ::std::borrow::Cow<'static, [T]>;

    pub type BoxedStream<T> =
        ::std::pin::Pin<::std::boxed::Box<dyn ::futures::Stream<Item = T> + ::core::marker::Send>>;
}
