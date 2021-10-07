use log::LevelFilter;

use std::env;

use twilight_model::application::command::{
    BaseCommandOptionData, CommandOption,
};
use twilight_model::id::{ApplicationId, GuildId};
use twilight_http::client::ClientBuilder;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "kromer-interactions", about = "Migrates interactions.")]
struct Opt {
    /// The guild to apply the commands to.
    #[structopt(long)]
    guild: u64,
    /// The id of the application.
    #[structopt(short = "a")]
    application_id: u64,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv::dotenv()?;

    env_logger::Builder::new()
        .filter(None, LevelFilter::Info)
        .parse_env("KROMER_LOG")
        .init();

    // get options
    let opt = Opt::from_args();

    // create a client
    let client = ClientBuilder::new()
        .token(env::var("DISCORD_TOKEN")?)
        .application_id(ApplicationId(opt.application_id))
        .build();

    client
        .new_create_guild_command(GuildId(opt.guild), "rank")?
        .chat_input("Gets the level and amount of experience a user has accumulated.")?
        .command_options(&[
            CommandOption::User(BaseCommandOptionData {
                name: String::from("user"),
                description: String::from("The user to check. If omitted, gets the user who executed the command."),
                required: false,
            })
        ])?
        .exec()
        .await?;

    Ok(())
}

