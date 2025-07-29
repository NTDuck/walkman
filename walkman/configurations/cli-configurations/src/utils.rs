pub mod aliases {
    pub type Fallible<T> = ::anyhow::Result<T>;
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
}
