mod error;
mod osu_api;

use crate::osu_api::OsuApi;
use clap::Parser;

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::Serialize;
use std::fs::File;

use dotenv::dotenv;
use std::env;

use eyre::Result;

macro_rules! str_to_datetime {
    ($s:expr) => {{
        let naivedate = NaiveDate::parse_from_str($s, "%d-%m-%Y")?;

        // Should never fails so using unwrap
        let naivetime = NaiveTime::from_hms_opt(0, 0, 0).unwrap();

        let ndt = NaiveDateTime::new(naivedate, naivetime);
        DateTime::from_utc(ndt, Utc)
    }};
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Start date (%d-%m-%Y) e.g. 01-05-2023
    #[arg(short, long)]
    pub from: String,

    /// End date (%d-%m-%Y) e.g. 01-05-2023
    #[arg(short, long)]
    pub to: String,

    /// Amount of users to process
    #[arg(short, long, default_value_t = 200)]
    pub amount: i32,
}

#[derive(Debug, Serialize)]
struct Output {
    username: String,
    pp: f32,
    date: String,
    replay: bool,
    map: String,
    diff: String,
    score_link: String,
    mods: String,
    country_rank: i32,
    global_rank: i32,
    total_pp: f32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    dotenv()?;

    let from: DateTime<Utc> = str_to_datetime!(&args.from);
    let to: DateTime<Utc> = str_to_datetime!(&args.to);
    let amount = args.amount;

    let api = OsuApi::new(
        env::var("CLIENT_ID")?.parse()?,
        env::var("CLIENT_SECRET")?.as_str(),
    )
    .await?;

    let users = api.get_country_ranking("by").await?.ranking;

    let mut output: Vec<Output> = Vec::with_capacity(amount as usize);

    for (index, user_stats) in users.iter().enumerate().take(amount as usize) {
        let user = &user_stats.user;

        println!("Processing user {}", user.username);

        // Getting scores
        let scores = api.get_user_best_scores(user.id).await?;

        for score in scores
            .iter()
            .filter(|&x| x.created_at > from && x.created_at < to)
        {
            output.push(Output {
                username: user.username.clone(),
                pp: score.pp,
                date: score.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                replay: score.replay,
                map: format!("{} - {}", score.beatmapset.artist, score.beatmapset.title),
                diff: score.beatmap.version.clone(),
                score_link: format!("https://osu.ppy.sh/scores/osu/{}", score.id),
                mods: score.mods.to_string(),
                country_rank: index as i32,
                global_rank: user_stats.global_rank,
                total_pp: user_stats.pp,
            })
        }
    }

    let file = File::create("output.csv")?;
    let mut wtr = csv::Writer::from_writer(file);

    for o in output {
        wtr.serialize(o)?;
    }

    wtr.flush()?;

    Ok(())
}
