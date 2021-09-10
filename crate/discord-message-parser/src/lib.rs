use {
    std::{
        fmt,
        str::FromStr,
    },
    chrono::prelude::*,
    derivative::Derivative,
    once_cell::sync::Lazy,
    regex::Regex,
    ::serenity::model::prelude::*,
};

pub mod serenity;

static ANGLE_BRACKETS: Lazy<Regex> = Lazy::new(|| Regex::new("<.+?>").expect("failed to parse ANGLE_BRACKETS regex"));
static UNSTYLED_TIMESTAMP: Lazy<Regex> = Lazy::new(|| Regex::new("^<t:(-?[0-9]+)>$").expect("failed to parse UNSTYLED_TIMESTAMP regex"));
static STYLED_TIMESTAMP: Lazy<Regex> = Lazy::new(|| Regex::new("^<t:(-?[0-9]+):(.)>$").expect("failed to parse STYLED_TIMESTAMP regex"));

#[derive(Derivative, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derivative(Default)]
pub enum TimestampStyle {
    ShortTime,
    LongTime,
    ShortDate,
    LongDate,
    #[derivative(Default)]
    ShortDateTime,
    LongDateTime,
    RelativeTime,
}

impl TimestampStyle {
    pub fn fmt<Tz: TimeZone>(&self, timestamp: DateTime<Tz>) -> String
    where Tz::Offset: fmt::Display {
        match self {
            Self::ShortTime => timestamp.format("%H:%M").to_string(),
            Self::LongTime => timestamp.format("%H:%M:%S").to_string(),
            Self::ShortDate => timestamp.format("%Y-%m-%d").to_string(),
            Self::LongDate => timestamp.format("%Y-%m-%d").to_string(),
            Self::ShortDateTime => timestamp.format("%Y-%m-%d %H:%M").to_string(),
            Self::LongDateTime => timestamp.format("%A, %Y-%m-%d %H:%M").to_string(),
            Self::RelativeTime => timestamp.format("%Y-%m-%d %H:%M:%S").to_string(), //TODO
        }
    }
}

impl FromStr for TimestampStyle {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "t" => Ok(Self::ShortTime),
            "T" => Ok(Self::LongTime),
            "d" => Ok(Self::ShortDate),
            "D" => Ok(Self::LongDate),
            "f" => Ok(Self::ShortDateTime),
            "F" => Ok(Self::LongDateTime),
            "R" => Ok(Self::RelativeTime),
            _ => Err(()),
        }
    }
}

/// A parsed Discord message.
#[derive(Debug)]
pub enum MessagePart<'a> {
    Empty,
    Nested(Vec<MessagePart<'a>>),
    PlainText(&'a str),
    UserMention {
        user: UserId,
        nickname_mention: bool,
    },
    ChannelMention(ChannelId),
    RoleMention(RoleId),
    //UnicodeEmoji(), //TODO content type
    CustomEmoji(EmojiIdentifier),
    Timestamp {
        timestamp: DateTime<Utc>,
        style: Option<TimestampStyle>,
    },
}

impl<'a> From<&'a str> for MessagePart<'a> {
    fn from(s: &'a str) -> Self {
        let mut parts = Vec::default();
        let mut start = 0;
        for tag in ANGLE_BRACKETS.find_iter(s) {
            if let Ok(user) = tag.as_str().parse() {
                if tag.start() > start {
                    parts.push(MessagePart::PlainText(&s[start..tag.start()]));
                }
                parts.push(MessagePart::UserMention {
                    user,
                    nickname_mention: tag.as_str().starts_with("<@!"),
                });
                start = tag.end();
            } else if let Ok(channel) = tag.as_str().parse() {
                if tag.start() > start {
                    parts.push(MessagePart::PlainText(&s[start..tag.start()]));
                }
                parts.push(MessagePart::ChannelMention(channel));
                start = tag.end();
            } else if let Ok(role) = tag.as_str().parse() {
                if tag.start() > start {
                    parts.push(MessagePart::PlainText(&s[start..tag.start()]));
                }
                parts.push(MessagePart::RoleMention(role));
                start = tag.end();
            } else if let Ok(emoji) = tag.as_str().parse() {
                if tag.start() > start {
                    parts.push(MessagePart::PlainText(&s[start..tag.start()]));
                }
                parts.push(MessagePart::CustomEmoji(emoji));
                start = tag.end();
            } else if let Some(timestamp) = UNSTYLED_TIMESTAMP.captures(tag.as_str()).and_then(|captures| captures[1].parse().ok()) {
                if tag.start() > start {
                    parts.push(MessagePart::PlainText(&s[start..tag.start()]));
                }
                parts.push(MessagePart::Timestamp {
                    timestamp: Utc.timestamp(timestamp, 0),
                    style: None,
                });
                start = tag.end();
            } else if let Some((timestamp, style)) = STYLED_TIMESTAMP.captures(tag.as_str()).and_then(|captures| Some((captures[1].parse().ok()?, captures[2].parse().ok()?))) {
                if tag.start() > start {
                    parts.push(MessagePart::PlainText(&s[start..tag.start()]));
                }
                parts.push(MessagePart::Timestamp {
                    timestamp: Utc.timestamp(timestamp, 0),
                    style: Some(style),
                });
                start = tag.end();
            }
        }
        if s.len() > start {
            parts.push(MessagePart::PlainText(&s[start..]));
        }
        match parts.len() {
            0 => MessagePart::Empty,
            1 => parts.remove(0),
            _ => MessagePart::Nested(parts),
        }
    }
}
