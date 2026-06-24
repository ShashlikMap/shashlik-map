use std::borrow::Cow;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct StyleId(pub Cow<'static, str>);

impl StyleId {
    pub fn new<T>(id: T) -> Self
    where
        T: Into<Cow<'static, str>>
    {
        StyleId(id.into())
    }
}