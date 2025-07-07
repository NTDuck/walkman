pub mod aliases {
    use std::borrow::Cow;

    pub type MaybeOwnedStr = Cow<'static, str>;
}
