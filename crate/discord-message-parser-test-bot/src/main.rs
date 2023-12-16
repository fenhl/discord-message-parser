//! A Discord bot that replies to every non-bot message it can read with its parsed representation.

use {
    std::{
        sync::Arc,
        time::Duration,
    },
    serenity::{
        all::CreateBotAuthParameters,
        async_trait,
        framework::standard::StandardFramework,
        model::prelude::*,
        prelude::*,
        utils::MessageBuilder,
    },
    serenity_utils::ShardManagerContainer,
    tokio::time::sleep,
    discord_message_parser::serenity::MessageExt as _,
};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, data_about_bot: Ready) {
        println!("Ready");
        if data_about_bot.guilds.is_empty() {
            println!("No guilds found, use following URL to invite the bot:");
            let invite_url = CreateBotAuthParameters::new()
                .permissions(Permissions::empty())
                .scopes(&[Scope::Bot])
                .auto_client_id(&ctx).await.expect("failed to generate invite URL")
                .build();
            println!("{invite_url}");
            serenity_utils::shut_down(&ctx).await;
        }
    }

    async fn message(&self, ctx: Context, msg: Message) { //TODO move to normal_message in the framework?
        if msg.author.bot { return; } // ignore bots to prevent message loops
        let parsed_message = msg.parse();
        println!("{:?} -> {:#?}", msg.content, parsed_message);
        msg.reply(ctx, MessageBuilder::default().push_codeblock_safe(format!("{:#?}", parsed_message), Some("rust")).build()).await.expect("failed to send reply");
    }
}

#[tokio::main]
async fn main() -> serenity::Result<()> {
    let mut client = Client::builder(include_str!("../../../assets/test-bot-token"), GatewayIntents::GUILD_MESSAGES)
        .event_handler(Handler)
        .framework(StandardFramework::default()) // some dependency seems to have enabled serenity's `framework` feature
        .await?;
    client.data.write().await.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
    client.start_autosharded().await?;
    sleep(Duration::from_secs(1)).await; // wait to make sure websockets can be closed cleanly
    Ok(())
}
