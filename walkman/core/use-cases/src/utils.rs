pub mod aliases {
    pub type Fallible<T> = ::anyhow::Result<T>;

    pub type MaybeOwnedString<'a> = ::std::borrow::Cow<'a, str>;
    pub type MaybeOwnedVec<'a, T> = ::std::borrow::Cow<'a, [T]>;

    pub type BoxedStream<T> =
        ::std::pin::Pin<::std::boxed::Box<dyn ::futures::Stream<Item = T> + ::core::marker::Send>>;
}
