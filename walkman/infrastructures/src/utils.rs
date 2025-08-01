pub mod aliases {
    pub type Fallible<T> = ::anyhow::Result<T>;

    pub type MaybeOwnedString = ::std::borrow::Cow<'static, str>;
    pub type MaybeOwnedPath = ::std::borrow::Cow<'static, ::std::path::Path>;
    pub type MaybeOwnedVec<T> = ::std::borrow::Cow<'static, [T]>;

    pub type BoxedStream<T> =
        ::std::pin::Pin<::std::boxed::Box<dyn ::futures::Stream<Item = T> + ::core::marker::Send>>;
}

pub mod extensions {
    use ::async_trait::async_trait;

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
                    Err(::anyhow::anyhow!(
                        "called `OptionExt::some()` on a `None` value at {}:{}:{}",
                        location.file(),
                        location.line(),
                        location.column()
                    ))
                },
            }
        }
    }

    #[async_trait]
    pub trait EntryExt<'a, K, V> {
        async fn or_insert_with_future<Fut, F>(self, default: F) -> &'a mut V
        where
            F: FnOnce() -> Fut + ::core::marker::Send,
            Fut: ::std::future::Future<Output = V> + ::core::marker::Send,
            Self: Sized + ::core::marker::Send;
    }

    #[async_trait]
    impl<'a, K, V, Entry> EntryExt<'a, K, V> for Entry
    where
        Entry: Into<::std::collections::hash_map::Entry<'a, K, V>>,
        K: 'a + ::core::marker::Send,
        V: 'a + ::core::marker::Send,
    {
        async fn or_insert_with_future<Fut, F>(self, default: F) -> &'a mut V
        where
            F: FnOnce() -> Fut + ::core::marker::Send,
            Fut: ::std::future::Future<Output = V> + ::core::marker::Send,
            Self: Sized + ::core::marker::Send,
        {
            match self.into() {
                ::std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
                ::std::collections::hash_map::Entry::Vacant(entry) => entry.insert(default().await),
            }
        }
    }
}
