#[cfg(feature = "serenity")] pub mod serenity;

/// A parsed Discord message.
#[derive(Debug)]
#[non_exhaustive] // for future Discord features
pub enum MessagePart<'a> {
    PlainText(&'a str),
}

impl<'a> From<&'a str> for MessagePart<'a> {
    fn from(s: &'a str) -> MessagePart<'a> {
        MessagePart::PlainText(s) //TODO
    }
}
