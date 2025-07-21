pub mod aliases {
    pub type Fallible<T> = ::anyhow::Result<T>;

    pub type MaybeOwnedString = ::std::borrow::Cow<'static, str>;
    pub type MaybeOwnedPath = ::std::borrow::Cow<'static, ::std::path::Path>;

    pub type BoxedStream<T> =
        ::std::pin::Pin<::std::boxed::Box<dyn ::futures::Stream<Item = T> + ::core::marker::Send>>;
}

pub mod macros {
    #[macro_export]
    macro_rules! progress_style {
        ($template:expr) => {
            ::once_cell::sync::Lazy::new(|| ::indicatif::ProgressStyle::with_template($template).unwrap())
        };
    }

    #[macro_export]
    macro_rules! regex {
        ($pattern:expr) => {
            ::once_cell::sync::Lazy::new(|| ::regex::Regex::new($pattern).unwrap())
        };
    }
}
