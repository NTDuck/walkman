pub mod aliases {
    use std::{borrow::Cow, path::Path};

    pub type MaybeOwnedPath = Cow<'static, Path>;

}
