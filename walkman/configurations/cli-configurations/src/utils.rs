pub mod aliases {
    use std::{borrow::Cow, path::Path};

    pub type MaybeOwnedString = Cow<'static, str>;
    pub type MaybeOwnedPath = Cow<'static, Path>;
}
