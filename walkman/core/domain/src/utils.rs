pub mod aliases {
    pub type MaybeOwnedString<'a> = ::std::borrow::Cow<'a, str>;
    pub type MaybeOwnedPath<'a> = ::std::borrow::Cow<'a, ::std::path::Path>;
    pub type MaybeOwnedVec<'a, T> = ::std::borrow::Cow<'a, [T]>;
}
