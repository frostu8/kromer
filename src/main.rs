#[macro_use] extern crate log;

use std::env;

use kromer::services::Services;

use twilight_model::id::{ApplicationId, GuildId};
use twilight_model::gateway::Intents;
use twilight_model::application::command::{
    BaseCommandOptionData, CommandOption,
};
use twilight_gateway::cluster::{Cluster, ShardScheme};
use twilight_http::client::ClientBuilder;

use log::LevelFilter;
use anyhow::{Result, anyhow};
use tokio::runtime::Runtime;

use ansi_term::{Style, Color};

use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
    name = "kromer",
    about = "discord bot",
)]
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
#[structopt(
    name = "run",
    about = "runs the discord bot in the foreground",
)]
struct Run {}

#[derive(StructOpt)]
#[structopt(
    name = "migrate",
    about = "runs discord command migrations",
)]
struct Migrate {
    #[structopt(short, long)]
    /// the id of the guild to run discord migrations on
    guild: u64,
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
        Command::Run(run) => Runtime::new()
            .unwrap()
            .block_on(main_run(opt.options, run)),
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
    let application_id = get_application_id()?;
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
    let client = ClientBuilder::new()
        .token(token.clone())
        .application_id(ApplicationId(application_id))
        .build();

    info!("starting discord gateway...");

    // throw up a cluster
    let cluster = Cluster::builder(token, Intents::GUILD_MESSAGES | Intents::GUILD_MESSAGE_REACTIONS)
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

    // create and run our services
    Services::new()
        .add(kromer::services::xp::Xp::new(db.clone()))
        .add(kromer::services::xp::RankCommand::new(db.clone(), client.clone()))
        .add(kromer::services::xp::TopCommand::new(db.clone(), client.clone()))
        .add(kromer::services::roles::reaction::ReactionRoles::new(db.clone(), client.clone()))
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
    let application_id = get_application_id()?;

    let guild_id = GuildId(migrate.guild);

    info!("running discord command migrations...");

    // create a client
    let client = ClientBuilder::new()
        .token(token)
        .application_id(ApplicationId(application_id))
        .build();

    info!("migrating {}...", highlight.paint("/rank"));

    client
        .new_create_guild_command(guild_id, "rank")?
        .chat_input("returns your or another user's level and KR balance")?
        .command_options(&[
            CommandOption::User(BaseCommandOptionData {
                name: String::from("user"),
                description: String::from("the user to check"),
                required: false,
            })
        ])?
        .exec()
        .await?;

    info!("migrating {}...", highlight.paint("/top"));

    client
        .new_create_guild_command(guild_id, "top")?
        .chat_input("returns the leading 10 members of the guild in KR balance")?
        .exec()
        .await?;

    info!("migrations complete!");

    Ok(())
}

fn get_database_url() -> Result<String> {
    env::var("DATABASE_URL")
        .map_err(|_| anyhow!("postgres database url not provided! provide DATABASE_URL in environment or .env file."))
}

fn get_discord_token() -> Result<String> {
    env::var("DISCORD_TOKEN")
        .map_err(|_| anyhow!("discord token missing! provide DISCORD_TOKEN in environment or .env file."))
}

fn get_application_id() -> Result<u64> {
    env::var("DISCORD_APPLICATION_ID")
        .map_err(|_| anyhow!("discord application id not provided! provide DISCORD_APPLICATION_ID in environment or .env file."))?
        .parse::<u64>()
        .map_err(|err| anyhow!("discord application id is invalid: {}", err))
}

