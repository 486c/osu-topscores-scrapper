mod osu_api;
mod error;

use crate::osu_api::OsuApi;
use clap::Parser;

use chrono::{DateTime, NaiveDate, Utc, NaiveDateTime, NaiveTime};
use serde::Deserialize;

macro_rules! str_to_datetime {
    ($s:expr) => {{ 
        let naivedate = NaiveDate::parse_from_str(
            $s, "%d-%m-%Y"
        ).unwrap(); // TODO remove unwrap

        let naivetime = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // TODO remove unwrap
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
    #[arg(short, long, default_value_t=200)]
    pub amount: i32
}

#[derive(Debug, Deserialize)]
struct Output {
    username: String,
    pp: f32,
    date: String,
    replay: bool,
    map: String,
    diff: String,
    mods: String,
    country_rank: i32,
    global_rank: i32,
    total_pp: f32
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let from: DateTime<Utc> = str_to_datetime!(&args.from);
    let to: DateTime<Utc> = str_to_datetime!(&args.to);
    let amount = args.amount;

    // TODO move to env
    let client_id = -1;
    let client_secret = "changeme";

    let api = OsuApi::new(
        client_id,
        client_secret
    ).await.unwrap();

    let users = api.get_country_ranking("by")
        .await.unwrap()
        .ranking;

    let mut output: Vec<Output> = Vec::with_capacity(amount as usize);
    
    for (index, user_stats) in users.iter().enumerate().take(amount as usize) {
        let user = &user_stats.user;

        println!("Processing user {}", user.username);

        // Getting scores
        let scores = api.get_user_best_scores(user.id)
            .await
            .unwrap();
        
        for score in scores {
            if score.created_at > from && score.created_at < to {
                output.push(Output {
                        username: user.username.clone(),
                        pp: score.pp,
                        date: score.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                        replay: score.replay,
                        map: format!(
                            "{} - {}", 
                            score.beatmapset.artist,
                            score.beatmapset.title),
                        diff: score.beatmap.version,
                        mods: score.mods.to_string(),
                        country_rank: index as i32,
                        global_rank: user_stats.global_rank,
                        total_pp: user_stats.pp
                })
            }
        }
    }

    dbg!(output);
}
