#![deny(rust_2018_idioms, unused, unused_crate_dependencies, unused_import_braces, unused_qualifications, warnings)]
#![forbid(unsafe_code)]

use {
    std::{
        collections::BTreeSet,
        time::Duration,
    },
    graphql_client::GraphQLQuery,
    itertools::Itertools as _,
    once_cell::sync::Lazy,
    proc_macro::TokenStream,
    quote::quote,
    regex::Regex,
    reqwest::blocking::Client,
};

static FILENAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("^([0-9a-f]{1,6}(?:-[0-9a-f]{1,6})*)\\.svg$").expect("failed to compile twemoji filename regex"));

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../assets/graphql/github-schema.graphql",
    query_path = "../../assets/graphql/github-twemoji-list.graphql",
    response_derives = "Debug",
)]
struct TwemojiListQuery;

#[proc_macro]
pub fn parse_unicode(input: TokenStream) -> TokenStream {
    if !input.is_empty() { return quote!(compile_error!("discord_message_parser_derive::parse_unicode! takes no arguments");).into() }
    let client = Client::builder()
        .user_agent(concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .http2_prior_knowledge()
        .use_rustls_tls()
        .https_only(true)
        .build().expect("failed to build HTTP client");
    let emoji = match client.post("https://api.github.com/graphql")
        .bearer_auth(env!("GITHUB_TOKEN"))
        .json(&TwemojiListQuery::build_query(twemoji_list_query::Variables {}))
        .send().expect("failed to send GitHub API request")
        .error_for_status().expect("error in GitHub API request")
        .json::<graphql_client::Response<twemoji_list_query::ResponseData>>().expect("failed to parse GitHub API response")
        .data.expect("missing data in GitHub API response")
        .repository.expect("missing repo in GitHub API response")
        .object.expect("missing assets/svg tree in GitHub API response")
    {
        twemoji_list_query::TwemojiListQueryRepositoryObject::Tree(tree) => tree
            .entries.expect("missing tree entries in GitHub API response")
            .into_iter()
            .map(|entry| FILENAME_REGEX.captures(&entry.name).expect("unexpected twemoji file name")[1]
                .split('-')
                .map(|hex| std::char::from_u32(u32::from_str_radix(hex, 16).expect("invalid hex in twemoji file name")).expect("invalid codepoint in twemoji file name"))
                .collect::<String>()
            )
            .collect::<BTreeSet<_>>(),
        on => panic!("unexpected GraphQL interface: {:?}", on),
    };
    let emoji_first_char_groups = emoji.into_iter().rev().into_group_map_by(|emoji| emoji.chars().next().expect("emoty emoji"));
    let char_match_arms = emoji_first_char_groups.into_iter()
        .map(|(first_char, group)| {
            let emoji_checks = group.into_iter().map(|emoji| quote!(if s[index..].starts_with(#emoji) {
                if index > start { parts.push(MessagePart::PlainText(&s[start..index])) }
                parts.push(MessagePart::UnicodeEmoji(#emoji));
                start = index + #emoji.len();
                continue
            }));
            quote!(#first_char => { #(#emoji_checks)* })
        });
    TokenStream::from(quote! {
        fn parse_unicode<'a>(parts: &mut Vec<MessagePart<'a>>, s: &'a str) {
            let mut start = 0; // start of plain text segment
            for (index, c) in s.char_indices() {
                if index >= start { // skip chars that have already been parsed as part of an emoji
                    match c {
                        #(#char_match_arms,)*
                        _ => {}
                    }
                }
            }
            if s.len() > start { parts.push(MessagePart::PlainText(&s[start..])) }
        }
    })
}
