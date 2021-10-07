use log::LevelFilter;

use std::error::Error;
use std::env;

use kromer::services::Services;

use twilight_gateway::cluster::{Cluster, ShardScheme};
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

    // run migrations
    kromer::model::migrate(&db).await?;

    // throw up a cluster
    let (cluster, events) = Cluster::builder(token.clone(), Intents::GUILD_MESSAGES)
        .shard_scheme(ShardScheme::Auto)
        .build()
        .await?;

    // start up the cluster in the background
    let cluster_spawn = cluster.clone();

    tokio::spawn(async move {
        cluster_spawn.up().await;
    });

    // create and run our services
    Services::new()
        .add(kromer::services::Xp::new(db.clone()))
        .run(events)
        .await;

    Ok(())
}

