#![deny(rust_2018_idioms, unused, unused_crate_dependencies, unused_import_braces, unused_qualifications, warnings)]
#![forbid(unsafe_code)]

use {
    std::{
        collections::BTreeSet,
        env,
        fs::{
            self,
            File,
        },
        io::{
            self,
            prelude::*,
        },
        path::Path,
        time::Duration,
    },
    graphql_client::GraphQLQuery,
    itertools::Itertools as _,
    lazy_regex::regex_captures,
    quote::quote,
    reqwest::blocking::Client,
};

type GitObjectID = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../assets/graphql/github-schema.graphql",
    query_path = "../../assets/graphql/github-twemoji-head.graphql",
    response_derives = "Debug",
)]
struct TwemojiHeadQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../assets/graphql/github-schema.graphql",
    query_path = "../../assets/graphql/github-twemoji-list.graphql",
    response_derives = "Debug",
)]
struct TwemojiListQuery;

fn main() {
    println!("cargo:rerun-if-changed=nonexistent.foo"); // check a nonexistent file to make sure build script is always run (see https://github.com/rust-lang/cargo/issues/4213 and https://github.com/rust-lang/cargo/issues/3404)
    let version_path = Path::new(&env::var("OUT_DIR").unwrap()).join("twemoji-version.txt");
    let last_twemoji_version = match fs::read_to_string(&version_path) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => panic!("failed to read twemoji commit hash: {}", e),
    };
    let client = Client::builder()
        .user_agent(concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .http2_prior_knowledge()
        .use_rustls_tls()
        .https_only(true)
        .build().expect("failed to build HTTP client");
    let current_twemoji_version = client.post("https://api.github.com/graphql")
        .bearer_auth(env!("GITHUB_TOKEN"))
        .json(&TwemojiHeadQuery::build_query(twemoji_head_query::Variables {}))
        .send().expect("failed to send GitHub API request")
        .error_for_status().expect("error in GitHub API request")
        .json::<graphql_client::Response<twemoji_head_query::ResponseData>>().expect("failed to parse GitHub API response")
        .data.expect("missing data in GitHub API response")
        .repository.expect("twemoji repo not found")
        .default_branch_ref.expect("missing default branch on twemoji repo")
        .target.expect("no target for twemoji repo default branch")
        .oid;
    if last_twemoji_version.map_or(true, |last| last != current_twemoji_version) {
        let emoji = match client.post("https://api.github.com/graphql")
            .bearer_auth(env!("GITHUB_TOKEN"))
            .json(&TwemojiListQuery::build_query(twemoji_list_query::Variables {}))
            .send().expect("failed to send GitHub API request")
            .error_for_status().expect("error in GitHub API request")
            .json::<graphql_client::Response<twemoji_list_query::ResponseData>>().expect("failed to parse GitHub API response")
            .data.expect("missing data in GitHub API response")
            .repository.expect("twemoji repo not found")
            .object.expect("missing assets/svg tree in twemoji repo")
        {
            twemoji_list_query::TwemojiListQueryRepositoryObject::Tree(tree) => tree
                .entries.expect("missing tree entries in GitHub API response")
                .into_iter()
                .map(|entry| regex_captures!(r"^([0-9a-f]{1,6}(?:-[0-9a-f]{1,6})*)\.svg$", &entry.name).expect("unexpected twemoji file name").1
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
        writeln!(File::create(Path::new(&env::var("OUT_DIR").unwrap()).join("twemoji.rs")).expect("failed to write generated code"), "{}", quote! {
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
        }).expect("failed to write generated code");
        fs::write(version_path, current_twemoji_version).expect("failed to write twemoji commit hash");
    }
}
