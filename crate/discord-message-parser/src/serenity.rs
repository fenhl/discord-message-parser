use {
    dep_serenity::model::channel::Message,
    crate::MessagePart,
};

pub trait MessageExt {
    fn parse<'a>(&'a self) -> MessagePart<'a>;
}

impl MessageExt for Message {
    fn parse<'a>(&'a self) -> MessagePart<'a> {
        MessagePart::from(&*self.content)
    }
}
