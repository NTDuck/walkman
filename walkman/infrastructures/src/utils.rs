pub mod aliases {
    pub type Fallible<T> = ::anyhow::Result<T>;

    pub type MaybeOwnedString = ::std::borrow::Cow<'static, str>;
    pub type MaybeOwnedPath = ::std::borrow::Cow<'static, ::std::path::Path>;
    pub type MaybeOwnedVec<T> = ::std::borrow::Cow<'static, [T]>;

    pub type BoxedStream<T> =
        ::std::pin::Pin<::std::boxed::Box<dyn ::futures::Stream<Item = T> + ::core::marker::Send>>;
}

pub mod extensions {
    use crate::utils::aliases::Fallible;

    pub trait OptionExt<T> {
        fn ok(self) -> Fallible<T>;
    }

    impl<T> OptionExt<T> for Option<T> {
        #[track_caller]
        fn ok(self) -> Fallible<T> {
            match self {
                Some(val) => Ok(val),
                None => {
                    let location = ::std::panic::Location::caller();
                    Err(::anyhow::anyhow!("called `OptionExt::some()` on a `None` value at {}:{}:{}", location.file(), location.line(), location.column()))
                }
            }
        }
    }
}
