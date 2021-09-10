#![deny(missing_docs, rust_2018_idioms, unused, unused_crate_dependencies, unused_import_braces, unused_qualifications, warnings)]
#![forbid(unsafe_code)]

//! This library parses [Discord](https://discord.com/) messages. It currently recognizes [message formatting tags](https://discord.com/developers/docs/reference#message-formatting) but not Discord's Markdown-like formatting.
//!
//! The main entry point is [`MessagePart`]'s `From<&str>` implementation.

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

/// The formatting information in a timestamp message formatting tag.
///
/// See also [Discord's documentation](https://discord.com/developers/docs/reference#message-formatting-timestamp-styles).
#[derive(Derivative, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derivative(Default)]
pub enum TimestampStyle {
    /// `t`, e.g. `16:20`.
    ShortTime,
    /// `T`, e.g. `16:20:30`.
    LongTime,
    /// `d`, e.g. `20/04/2021`.
    ShortDate,
    /// `D`, e.g. `20 April 2021`.
    LongDate,
    /// `f`, e.g. `20 April 2021 16:20`. This is the default.
    #[derivative(Default)]
    ShortDateTime,
    /// `F`, e.g. `Tuesday, 20 April 2021 16:20`.
    LongDateTime,
    /// `R`, e.g. `2 months ago`.
    RelativeTime,
}

fn timestamp_relative<Tz: TimeZone>(time: DateTime<Tz>, base: DateTime<Tz>) -> String {
    let mut years_diff = time.year() - base.year();
    if years_diff > 0 && (time.month() < base.month() || time.month() == base.month() && (time.day() < base.day() || time.day() == base.day() && time.time() < base.time())) { years_diff -= 1 }
    if years_diff < 0 && (time.month() > base.month() || time.month() == base.month() && (time.day() > base.day() || time.day() == base.day() && time.time() > base.time())) { years_diff += 1 }
    match years_diff {
        2.. => return format!("in {} years", years_diff),
        1 => return format!("in 1 year"),
        0 => {}
        -1 => return format!("1 year ago"),
        _ => return format!("{} years ago", -years_diff),
    }
    let mut months_diff = time.month() as i32 - base.month() as i32;
    if months_diff > 0 && (time.day() < base.day() || time.day() == base.day() && time.time() < base.time()) { months_diff -= 1 }
    if months_diff < 0 && (time.day() > base.day() || time.day() == base.day() && time.time() > base.time()) { months_diff += 1 }
    months_diff += 12 * (time.year() - base.year());
    match months_diff {
        2.. => return format!("in {} months", months_diff),
        1 => return format!("in 1 month"),
        0 => {}
        -1 => return format!("1 month ago"),
        _ => return format!("{} months ago", -months_diff),
    }
    let delta = time - base;
    match delta.num_weeks() {
        2.. => return format!("in {} weeks", delta.num_weeks()),
        1 => return format!("in 1 week"),
        0 => {}
        -1 => return format!("1 week ago"),
        _ => return format!("{} weeks ago", -delta.num_weeks()),
    }
    match delta.num_days() {
        2.. => return format!("in {} days", delta.num_days()),
        1 => return format!("in 1 day"),
        0 => {}
        -1 => return format!("1 day ago"),
        _ => return format!("{} days ago", -delta.num_days()),
    }
    match delta.num_hours() {
        2.. => return format!("in {} hours", delta.num_hours()),
        1 => return format!("in 1 hour"),
        0 => {}
        -1 => return format!("1 hour ago"),
        _ => return format!("{} hours ago", -delta.num_hours()),
    }
    match delta.num_minutes() {
        2.. => return format!("in {} minutes", delta.num_minutes()),
        1 => return format!("in 1 minute"),
        0 => {}
        -1 => return format!("1 minute ago"),
        _ => return format!("{} minutes ago", -delta.num_minutes()),
    }
    match delta.num_seconds() {
        2.. => format!("in {} seconds", delta.num_seconds()),
        1 => format!("in 1 second"),
        0 => format!("now"),
        -1 => format!("1 second ago"),
        _ => format!("{} seconds ago", -delta.num_seconds()),
    }
}

impl TimestampStyle {
    /// Formats the given timestamp with this style.
    ///
    /// The formatting is not locale-aware.
    pub fn fmt<Tz: TimeZone>(&self, timestamp: DateTime<Tz>) -> String
    where Tz::Offset: fmt::Display {
        match self {
            Self::ShortTime => timestamp.format("%H:%M").to_string(),
            Self::LongTime => timestamp.format("%H:%M:%S").to_string(),
            Self::ShortDate => timestamp.format("%Y-%m-%d").to_string(),
            Self::LongDate => timestamp.format("%Y-%m-%d").to_string(),
            Self::ShortDateTime => timestamp.format("%Y-%m-%d %H:%M").to_string(),
            Self::LongDateTime => timestamp.format("%A, %Y-%m-%d %H:%M").to_string(),
            Self::RelativeTime => {
                let now = Utc::now().with_timezone(&timestamp.timezone());
                timestamp_relative(timestamp, now)
            }
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
    /// An empty message.
    Empty,
    /// A message that consists of multiple parts.
    Nested(Vec<MessagePart<'a>>),
    /// A plain text message without any recognized formatting.
    PlainText(&'a str),
    /// A tag referring to a user.
    UserMention {
        /// The user in question.
        user: UserId,
        /// Whether the user was “nickname-mentioned”, i.e. `<@!...>` rather than `<@...>`.
        ///
        /// This was originally an implementation detail of the iOS app but is now the default behavior for all Discord clients, and behaves the same as a non-nickname mention.
        nickname_mention: bool,
    },
    /// A tag referring to a channel or channel group.
    ChannelMention(ChannelId),
    /// A tag referring to a role.
    RoleMention(RoleId),
    //UnicodeEmoji(), //TODO content type
    /// A custom (i.e. non-Unicode) emoji.
    CustomEmoji(EmojiIdentifier),
    /// A timestamp tag. See [Discord's docs](https://discord.com/developers/docs/reference#message-formatting) for details.
    Timestamp {
        /// The given UNIX timestamp.
        timestamp: DateTime<Utc>,
        /// `None` if omitted, which should behave like `Some(ShortDateTime)`.
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
