#[macro_use]
extern crate log;

use std::env;

use kromer::bot;
use kromer::service::Services;

use twilight_gateway::cluster::{Cluster, ShardScheme};
use twilight_http::Client;
use twilight_model::application::command::{
    BaseCommandOptionData, 
    OptionsCommandOptionData,
    permissions::{
        CommandPermissions,
        CommandPermissionsType,
    },
    CommandOption,
};
use twilight_model::gateway::Intents;
use twilight_model::id::GuildId;

use twilight_standby::Standby;

use anyhow::{anyhow, Result};
use log::LevelFilter;
use tokio::runtime::Runtime;

use ansi_term::{Color, Style};

use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "kromer", about = "discord bot")]
struct Kromer {
    #[structopt(flatten)]
    options: Opt,
    #[structopt(subcommand)]
    command: Option<Command>,
}

#[derive(StructOpt)]
struct Opt {
    #[structopt(long)]
    /// do not print with pretty colors
    no_color: bool,
}

#[derive(StructOpt)]
enum Command {
    Run(Run),
    Migrate(Migrate),
}

impl Default for Command {
    fn default() -> Command {
        Command::Run(Run {})
    }
}

#[derive(StructOpt)]
#[structopt(name = "run", about = "runs the discord bot in the foreground")]
struct Run {}

#[derive(StructOpt)]
#[structopt(name = "migrate", about = "runs discord command migrations")]
struct Migrate {
    #[structopt(short, long, required_unless = "global")]
    /// the id of the guild to run discord migrations on
    guild: Option<u64>,
    #[structopt(long)]
    #[allow(dead_code)]
    /// whether to apply the migrations globally or not
    global: bool,
}

fn main() {
    // initialize logging and environment
    dotenv::dotenv().ok();
    env_logger::Builder::new()
        .filter(None, LevelFilter::Info)
        // sqlx query logs are annoying and large
        .filter(Some("sqlx::query"), LevelFilter::Warn)
        .parse_env("KROMER_LOG")
        .init();

    let opt = Kromer::from_args();

    let res = match opt.command.unwrap_or_default() {
        Command::Run(run) => Runtime::new().unwrap().block_on(main_run(opt.options, run)),
        Command::Migrate(migrate) => Runtime::new()
            .unwrap()
            .block_on(main_migrate(opt.options, migrate)),
    };

    if let Err(err) = res {
        error!("{}", err);
        std::process::exit(1);
    }
}

async fn main_run(_options: Opt, _run: Run) -> Result<()> {
    // get config
    let token = get_discord_token()?;
    let database = get_database_url()?;

    info!("initiating connection to database...");

    // connect to the database
    let db = match sqlx::PgPool::connect(&database).await {
        Ok(db) => db,
        Err(err) => {
            error!("failed to initiate connection with database");
            error!("make sure the url provided in DATABASE_URL is correct");

            return Err(err.into());
        }
    };

    info!("running database migrations...");

    // run migrations
    if let Err(err) = kromer::model::migrate(&db).await {
        error!("failed to run migrations for database");

        return Err(err.into());
    }

    // get an http client
    let client = create_client(&token).await?;

    info!("starting discord gateway...");

    // throw up a cluster
    let cluster = Cluster::builder(
        token,
        Intents::GUILD_MESSAGES | Intents::GUILD_MESSAGE_REACTIONS,
    )
    .shard_scheme(ShardScheme::Auto)
    .build()
    .await;

    let (cluster, events) = match cluster {
        Ok(cluster) => cluster,
        Err(err) => {
            error!("failed to start discord gateway");
            error!("make sure the token provided in DISCORD_TOKEN is correct");
            error!("make sure the id provided in DISCORD_APPLICATION_ID is correct");

            return Err(err.into());
        }
    };

    // start up the cluster in the background
    let cluster_spawn = cluster.clone();

    tokio::spawn(async move {
        cluster_spawn.up().await;
    });

    // create standby ref
    let standby = Standby::new();

    // create and run our services
    Services::new(standby.clone())
        .add(bot::xp::Xp::new(db.clone()))
        .add(bot::xp::RankCommand::new(db.clone(), client.clone()))
        .add(bot::xp::TopCommand::new(db.clone(), client.clone()))
        .add(bot::roles::reaction::ReactionRoles::new(
            db.clone(),
            client.clone(),
        ))
        .add(bot::roles::reaction::CreateReactionRole::new(
            db.clone(),
            client.clone(),
            standby.clone(),
        ))
        .add(bot::info::InfoCommand::new(client.clone()))
        .run(events)
        .await;

    Ok(())
}

async fn main_migrate(options: Opt, migrate: Migrate) -> Result<()> {
    let highlight = if options.no_color {
        Style::default()
    } else {
        Style::new().fg(Color::Green).bold()
    };

    let token = get_discord_token()?;

    let guild_id = migrate.guild.map(GuildId);

    info!("running discord command migrations...");

    if guild_id.is_none() {
        warn!("applying migrations globally! this could take a while to be reflected");
    }

    // get an http client
    let client = create_client(&token).await?;

    info!("migrating {}...", highlight.paint("/rank"));

    if let Some(guild_id) = guild_id {
        client
            .new_create_guild_command(guild_id, "rank")?
            .chat_input("returns your or another user's level and KR balance")?
            .command_options(&[CommandOption::User(BaseCommandOptionData {
                name: String::from("user"),
                description: String::from("the user to check"),
                required: false,
            })])?
            .exec()
            .await?;
    } else {
        client
            .new_create_global_command("rank")?
            .chat_input("returns your or another user's level and KR balance")?
            .command_options(&[CommandOption::User(BaseCommandOptionData {
                name: String::from("user"),
                description: String::from("the user to check"),
                required: false,
            })])?
            .exec()
            .await?;
    }

    info!("migrating {}...", highlight.paint("/top"));

    if let Some(guild_id) = guild_id {
        client
            .new_create_guild_command(guild_id, "top")?
            .chat_input("returns the leading 10 members of the guild in KR balance")?
            .exec()
            .await?;
    } else {
        client
            .new_create_global_command("top")?
            .chat_input("returns the leading 10 members of the guild in KR balance")?
            .exec()
            .await?;
    }

    info!("migrating {}...", highlight.paint("/info"));

    if let Some(guild_id) = guild_id {
        client
            .new_create_guild_command(guild_id, "info")?
            .chat_input("returns info about the bot currently running")?
            .exec()
            .await?;
    } else {
        client
            .new_create_global_command("info")?
            .chat_input("returns info about the bot currently running")?
            .exec()
            .await?;
    }

    info!("migrating {}...", highlight.paint("/reactionroles"));

    if let Some(guild_id) = guild_id {
        client
            .new_create_guild_command(guild_id, "reactionroles")?
            .chat_input("configure reaction roles")?
            .default_permission(false)
            .command_options(&[CommandOption::SubCommand(OptionsCommandOptionData {
                name: String::from("add"),
                description: String::from("creates a new reaction role"),
                options: vec![CommandOption::Role(BaseCommandOptionData {
                    name: String::from("role"),
                    description: String::from("the role to set the reaction role as"),
                    required: true,
                })],
                required: false,
            })])?
            .exec()
            .await?;
    } else {
        error!("todo");
    }

    if let Some(guild_id) = guild_id {
        info!("setting up permissions");

        let commands = client.get_guild_commands(guild_id)?
            .exec().await?
            .model().await?;

        let reactionroles_cmd = commands
            .iter()
            .find(|cmd| cmd.name == "reactionroles")
            .unwrap();

        client
            .set_command_permissions(
                guild_id,
                &[
                    (reactionroles_cmd.id.unwrap(), CommandPermissions {
                        id: CommandPermissionsType::User(155785208556290048.into()),
                        permission: true,
                    })
                ],
            )?
            .exec()
            .await?;
    }

    info!("migrations complete!");

    Ok(())
}

fn get_database_url() -> Result<String> {
    env::var("DATABASE_URL").map_err(|_| {
        anyhow!(
            "postgres database url not provided! provide DATABASE_URL in environment or .env file."
        )
    })
}

fn get_discord_token() -> Result<String> {
    env::var("DISCORD_TOKEN").map_err(|_| {
        anyhow!("discord token missing! provide DISCORD_TOKEN in environment or .env file.")
    })
}

async fn create_client(token: impl Into<String>) -> Result<Client> {
    let client = Client::new(token.into());
    let app_id = kromer::bot::fetch_application_id(&client).await?;

    client.set_application_id(app_id);

    Ok(client)
}

