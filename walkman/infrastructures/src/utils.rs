pub mod aliases {
    pub type Fallible<T> = ::anyhow::Result<T>;

    pub type MaybeOwnedString = ::std::borrow::Cow<'static, str>;
    pub type MaybeOwnedPath = ::std::borrow::Cow<'static, ::std::path::Path>;

    pub type BoxedStream<T> =
        ::std::pin::Pin<::std::boxed::Box<dyn ::futures::Stream<Item = T> + ::core::marker::Send>>;
}

pub mod extensions {
    use crate::utils::aliases::Fallible;

    pub trait OptionExt<T> {
        fn some(self) -> Fallible<T>;
    }

    impl<T> OptionExt<T> for Option<T> {
        #[track_caller]
        fn some(self) -> Fallible<T> {
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

pub mod macros {
    #[macro_export]
    macro_rules! event {
        ($payload:expr) => {
            ::use_cases::models::events::Event {
                metadata: ::use_cases::models::events::EventMetadata {
                    worker_id: worker_id.clone(),
                    correlation_id: correlation_id.clone(),
                    timestamp: ::std::time::SystemTime::now(),
                },
                payload: $payload,
            }
        };
    }
    
    #[macro_export]
    macro_rules! lazy_progress_style {
        ($template:expr) => {
            ::once_cell::sync::Lazy::new(|| ::indicatif::ProgressStyle::with_template($template).unwrap())
        };
    }

    #[macro_export]
    macro_rules! lazy_color {
        ($color:expr) => {
            ::once_cell::sync::Lazy::new(|| {
                use ::colored::Colorize as _;

                $color
            })
        };
    }
}
