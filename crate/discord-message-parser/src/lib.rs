use {
    std::str::FromStr,
    chrono::prelude::*,
    once_cell::sync::Lazy,
    regex::Regex,
    ::serenity::model::prelude::*,
};

pub mod serenity;

static ANGLE_BRACKETS: Lazy<Regex> = Lazy::new(|| Regex::new("<.+?>").expect("failed to parse ANGLE_BRACKETS regex"));
static UNSTYLED_TIMESTAMP: Lazy<Regex> = Lazy::new(|| Regex::new("^<t:(-?[0-9]+)>$").expect("failed to parse UNSTYLED_TIMESTAMP regex"));
static STYLED_TIMESTAMP: Lazy<Regex> = Lazy::new(|| Regex::new("^<t:(-?[0-9]+):(.)>$").expect("failed to parse STYLED_TIMESTAMP regex"));

#[derive(Debug)]
pub enum TimestampStyle {
    ShortTime,
    LongTime,
    ShortDate,
    LongDate,
    ShortDateTime,
    LongDateTime,
    RelativeTime,
}

impl FromStr for TimestampStyle {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "t" => Ok(TimestampStyle::ShortTime),
            "T" => Ok(TimestampStyle::LongTime),
            "d" => Ok(TimestampStyle::ShortDate),
            "D" => Ok(TimestampStyle::LongDate),
            "f" => Ok(TimestampStyle::ShortDateTime),
            "F" => Ok(TimestampStyle::LongDateTime),
            "R" => Ok(TimestampStyle::RelativeTime),
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
