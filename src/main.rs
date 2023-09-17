mod error;
mod osu_api;

use crate::osu_api::{ OsuApi, RankingType };
use clap::Parser;

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use osu_api::UserStatistics;
use serde::Serialize;
use std::{fs::File, sync::{Arc, }};

use tokio::sync::mpsc::{Sender, channel};

use dotenv::dotenv;
use std::env;

use eyre::Result;

macro_rules! str_to_datetime {
    ($s:expr) => {{
        let naivedate = NaiveDate::parse_from_str($s, "%d-%m-%Y")?;

        // Should never fails so using unwrap
        let naivetime = NaiveTime::from_hms_opt(0, 0, 0).unwrap();

        let ndt = NaiveDateTime::new(naivedate, naivetime);
        let r: DateTime<Utc> = DateTime::from_naive_utc_and_offset(ndt, Utc);

        r
    }};
}

#[derive(Debug, Clone)]
pub struct Period {
    from: DateTime<Utc>,
    to: DateTime<Utc>
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

    /// Fetch global leaderboard? If set to true overrides --country flag
    #[arg(short, long)]
    pub global: bool,

    /// Country code e.g. BY, US, UK, BE, JP
    #[arg(short, long, required_unless_present("global"))]
    pub country: Option<String>,


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
    score_link: String,
    map: String,
    diff: String,
    mods: String,
    country_rank: i32,
    global_rank: i32,
    total_pp: f32,
}

async fn fetch_thread(
    api: Arc<OsuApi>,
    tx: Sender<Output>,
    users: Vec<UserStatistics>,
    amount: usize,
    period: Period
) {
    for (index, user_stats) in users
        .iter()
        .enumerate()
        .take(amount) 
    {
        let stats = user_stats.clone();
        let tx = tx.clone();
        let api = Arc::clone(&api);
        let period = period.clone();

        tokio::spawn(async move {
            let _ = process_score(
                Arc::clone(&api),
                tx,
                stats,
                index,
                period
            ).await;
        });
    }
}

async fn process_score(
    api: Arc<OsuApi>, 
    tx: Sender<Output>,
    user_stats: UserStatistics,
    index: usize,
    period: Period
) -> Result<()> {
    let user = &user_stats.user;

    println!("Processing user {}", user.username);

    // Getting scores
    let scores = api.get_user_best_scores(user.id).await?;

    for score in scores
        .iter()
        .filter(|&x| x.created_at > period.from && x.created_at < period.to)
        {
            let _ = tx.send(Output {
                username: user.username.clone(),
                pp: score.pp,
                date: score.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                replay: score.replay,
                map: format!("{} - {}", score.beatmapset.artist, score.beatmapset.title),
                diff: score.beatmap.version.clone(),
                score_link: format!("https://osu.ppy.sh/scores/osu/{}", score.id),
                mods: score.mods.to_string(),
                country_rank: index as i32 + 1,
                global_rank: user_stats.global_rank,
                total_pp: user_stats.pp,
            }).await;
        }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    dotenv()?;

    let from: DateTime<Utc> = str_to_datetime!(&args.from);
    let to: DateTime<Utc> = str_to_datetime!(&args.to);

    let period = Period{
        from,
        to
    };

    let amount = args.amount;

    let ranking = match args.global {
        true => RankingType::Global,
        false => RankingType::Country{ code: args.country.unwrap() },
    };

    let api = Arc::new(OsuApi::new(
        env::var("CLIENT_ID")?.parse()?,
        env::var("CLIENT_SECRET")?.as_str(),
        ).await?
    );
    
    println!("Getting leaderboard...");
    let users = api.get_ranking(
        ranking,
        (amount as f32 / 50.0).ceil() as i32
    ).await?;

    let mut output: Vec<Output> = Vec::with_capacity(amount as usize);

    let (tx, mut rx) = channel(amount as usize);

    tokio::spawn(fetch_thread(
        Arc::clone(&api),
        tx,
        users,
        amount as usize,
        period
    ));
    
    while let Some(i) = rx.recv().await {
        output.push(i);
    }

    println!("Found {} scores!", output.len());

    let file = File::create("output.csv")?;

    let mut wtr = csv::Writer::from_writer(file);

    for o in output {
        wtr.serialize(o)?;
    }

    wtr.flush()?;

    Ok(())
}
