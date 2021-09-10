//! Provides integration with [`serenity`](::serenity) for convenience.

use {
    ::serenity::model::channel::Message,
    crate::MessagePart,
};

/// An extension trait for serenity's [`Message`] type.
pub trait MessageExt {
    /// Parses the contents of this message.
    fn parse<'a>(&'a self) -> MessagePart<'a>;
}

impl MessageExt for Message {
    fn parse<'a>(&'a self) -> MessagePart<'a> {
        MessagePart::from(&*self.content)
    }
}
