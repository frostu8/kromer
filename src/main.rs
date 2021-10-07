use log::{error, info, LevelFilter};

use std::error::Error;
use std::env;

use tokio_stream::StreamExt;

use twilight_gateway::{cluster::{Cluster, ShardScheme}, Event};
use twilight_model::gateway::Intents;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    dotenv::dotenv()?;

    env_logger::Builder::new()
        .filter(None, LevelFilter::Info)
        // sqlx query logs are annoying and large
        .filter(Some("sqlx::query"), LevelFilter::Warn)
        .parse_env("KROMER_LOG")
        .init();

    let token = env::var("DISCORD_TOKEN")?;
    let database = env::var("DATABASE_URL")?;

    // connect to the database
    let db = sqlx::SqlitePool::connect(&database).await?;

    // throw up a cluster
    let (cluster, mut events) = Cluster::builder(token.clone(), Intents::GUILD_MESSAGES)
        .shard_scheme(ShardScheme::Auto)
        .build()
        .await?;

    // start up the cluster in the background
    let cluster_spawn = cluster.clone();

    tokio::spawn(async move {
        cluster_spawn.up().await;
    });

    // create our services
    let xp = kromer::services::Xp::new(db.clone());

    // process each event
    while let Some((shard_id, ev)) = events.next().await {
        let xp = xp.clone();

        tokio::spawn(async move {
            match ev {
                Event::MessageCreate(msg) => {
                    if let Err(err) = xp.handle_message(&msg).await {
                        error!("error while handling xp: {}", err);
                    }
                }
                Event::ShardConnected(_) => {
                    info!("shard #{} connected", shard_id);
                }
                _ => ()
            }
        });
    }

    Ok(())
}

