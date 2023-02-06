use hyper::{
    body::Bytes,
    client::{Client, HttpConnector},
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT},
    Body, Method, Request, StatusCode,
};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};

use serde::de;
use serde::de::DeserializeOwned;
use serde::de::{Deserializer, Error, SeqAccess, Unexpected, Visitor};
use serde::Deserialize;

use crate::error::{ApiErrorResponse, OsuApiError};
use std::fmt;
use std::fmt::Write;
use std::str::FromStr;
use std::string::ToString;

use chrono::{DateTime, NaiveDateTime, Utc};

use bitflags::bitflags;

type ApiResult<T> = Result<T, OsuApiError>;

pub fn deserialize_utc_datetime<'de, D>(d: D) -> Result<DateTime<Utc>, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct LocalDateTimeVisitor;

    impl<'de> de::Visitor<'de> for LocalDateTimeVisitor {
        type Value = DateTime<Utc>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(formatter, "a datetime string")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%SZ") {
                Ok(ndt) => Ok(DateTime::from_utc(ndt, Utc)),
                Err(e) => Err(E::custom(format!("Parse error {e} for {value}"))),
            }
        }
    }

    d.deserialize_str(LocalDateTimeVisitor)
}

pub fn cut(mut source: &str, n: usize) -> impl Iterator<Item = &str> {
    std::iter::from_fn(move || {
        if source.is_empty() {
            None
        } else {
            let end_idx = source
                .char_indices()
                .nth(n - 1)
                .map_or_else(|| source.len(), |(idx, c)| idx + c.len_utf8());

            let (split, rest) = source.split_at(end_idx);

            source = rest;

            Some(split)
        }
    })
}

bitflags! {
    #[derive(Default)]
    pub struct OsuMods: u32 {
        const NOMOD = 0;
        const NOFAIL = 1;
        const EASY = 2;
        const TOUCHDEVICE = 4;
        const HIDDEN = 8;
        const HARDROCK = 16;
        const SUDDENDEATH = 32;
        const DOUBLETIME = 64;
        const RELAX = 128;
        const HALFTIME = 256;
        const NIGHTCORE = 512 | Self::DOUBLETIME.bits;
        const FLASHLIGHT = 1024;
        const SPUNOUT = 4096;
        const PERFECT = 16_384 | Self::SUDDENDEATH.bits;
        const FADEIN = 1_048_576;
        const SCOREV2 = 536_870_912;
        const MIRROR = 1_073_741_824;
    }
}

impl ToString for OsuMods {
    fn to_string(&self) -> String {
        let mut res = String::new();

        if self.is_empty() {
            res.push_str("NM");
            return res;
        }

        if self.contains(OsuMods::NOFAIL) {
            res.push_str("NF")
        }
        if self.contains(OsuMods::EASY) {
            res.push_str("EZ")
        }
        if self.contains(OsuMods::TOUCHDEVICE) {
            res.push_str("TD")
        }
        if self.contains(OsuMods::HIDDEN) {
            res.push_str("HD")
        }
        if self.contains(OsuMods::DOUBLETIME) {
            if self.contains(OsuMods::NIGHTCORE) {
                res.push_str("NC")
            } else {
                res.push_str("DT")
            }
        }
        if self.contains(OsuMods::HALFTIME) {
            res.push_str("HT")
        }
        if self.contains(OsuMods::FLASHLIGHT) {
            res.push_str("FL")
        }
        if self.contains(OsuMods::HARDROCK) {
            res.push_str("HR")
        }
        if self.contains(OsuMods::SUDDENDEATH) {
            res.push_str("SD")
        }
        if self.contains(OsuMods::SPUNOUT) {
            res.push_str("SO")
        }
        if self.contains(OsuMods::PERFECT) {
            res.push_str("PF")
        }
        if self.contains(OsuMods::MIRROR) {
            res.push_str("MR")
        }

        res
    }
}

impl FromStr for OsuMods {
    type Err = OsuApiError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_uppercase();
        let mut flags = OsuMods::empty();

        for abbrev in cut(&s, 2) {
            flags = match abbrev {
                "NM" => flags | OsuMods::NOMOD,
                "NF" => flags | OsuMods::NOFAIL,
                "EZ" => flags | OsuMods::EASY,
                "TD" => flags | OsuMods::TOUCHDEVICE,
                "HD" => flags | OsuMods::HIDDEN,
                "HR" => flags | OsuMods::HARDROCK,
                "SD" => flags | OsuMods::SUDDENDEATH,
                "DT" => flags | OsuMods::DOUBLETIME,
                "RX" => flags | OsuMods::RELAX,
                "HT" => flags | OsuMods::HALFTIME,
                "NC" => flags | OsuMods::NIGHTCORE,
                "FL" => flags | OsuMods::FLASHLIGHT,
                "SO" => flags | OsuMods::SPUNOUT,
                "PF" => flags | OsuMods::PERFECT,
                "FD" => flags | OsuMods::FADEIN,
                _ => flags,
            };
        }

        Ok(flags)
    }
}

struct OsuModsVisitor;

impl<'de> Visitor<'de> for OsuModsVisitor {
    type Value = OsuMods;

    #[inline]
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut mods = OsuMods::default();

        while let Some(next) = seq.next_element()? {
            mods |= next;
        }

        Ok(mods)
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        let mods = match v {
            "NM" => OsuMods::NOMOD,
            "NF" => OsuMods::NOFAIL,
            "EZ" => OsuMods::EASY,
            "TD" => OsuMods::TOUCHDEVICE,
            "HD" => OsuMods::HIDDEN,
            "HR" => OsuMods::HARDROCK,
            "SD" => OsuMods::SUDDENDEATH,
            "DT" => OsuMods::DOUBLETIME,
            "RX" => OsuMods::RELAX,
            "HT" => OsuMods::HALFTIME,
            "NC" => OsuMods::NIGHTCORE,
            "FL" => OsuMods::FLASHLIGHT,
            "SO" => OsuMods::SPUNOUT,
            "PF" => OsuMods::PERFECT,
            "FD" => OsuMods::FADEIN,
            _ => {
                return Err(Error::invalid_value(
                    Unexpected::Str(v),
                    &r#"valid mods acronym"#,
                ))
            }
        };

        Ok(mods)
    }
}

impl<'de> Deserialize<'de> for OsuMods {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(OsuModsVisitor)
    }
}

#[derive(Debug, Deserialize)]
pub struct BeatmapCompact {
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct BeatmapSetCompact {
    pub artist: String,
    pub artist_unicode: String,
    pub creator: String,
    pub source: String,
    pub title: String,
    pub title_unicode: String,
}

#[derive(Debug, Deserialize)]
pub struct Score {
    pub id: i64,
    pub best_id: i64,
    pub user_id: i64,
    pub accuracy: f32,
    pub mods: OsuMods,
    pub score: i64,
    pub pp: f32,
    #[serde(deserialize_with = "deserialize_utc_datetime")]
    pub created_at: DateTime<Utc>,
    pub replay: bool,
    pub beatmapset: BeatmapSetCompact,
    pub beatmap: BeatmapCompact,
}

#[derive(Debug, Deserialize)]
pub struct UserCompact {
    pub id: i64,
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct UserStatistics {
    pub pp: f32,
    pub global_rank: i32,
    pub user: UserCompact,
}

#[derive(Debug, Deserialize)]
pub struct RankingResponse {
    pub ranking: Vec<UserStatistics>,
    pub total: i32,
}

#[derive(Debug, Deserialize)]
pub struct OauthResponse {
    pub token_type: String,
    pub expires_in: i32,
    pub access_token: String,
}

pub struct OsuApi {
    client: Client<HttpsConnector<HttpConnector>, Body>,
    client_id: i32,
    client_secret: String,
    token: Option<String>,
}

pub enum RankingType {
    Country { code: String }, // Replace with cow
    Global,
}

impl OsuApi {
    pub async fn new(client_id: i32, client_secret: &str) -> ApiResult<Self> {
        let https = HttpsConnectorBuilder::new()
            .with_native_roots()
            .https_only()
            .enable_http1()
            .build();

        let client = Client::builder().build(https);

        let mut api = Self {
            client,
            client_id,
            client_secret: client_secret.to_string(),
            token: None,
        };

        api.token = Some(api.request_oauth().await?);

        Ok(api)
    }

    pub async fn get_user_best_scores(&self, user_id: i64) -> ApiResult<Vec<Score>> {
        let mut link = format!(
            "https://osu.ppy.sh/api/v2/users/{}/scores/{}",
            user_id, "best"
        );
        let _ = write!(link, "?mode=osu");
        let _ = write!(link, "&limit=100");

        self.make_request(Method::GET, &link).await
    }

    pub async fn get_ranking(
        &self, 
        ranking: RankingType,
        pages: i32
    ) -> ApiResult<Vec<UserStatistics>> {

        let mut buff = Vec::with_capacity(pages as usize * 50);

        for page in 1..=pages {
            let mut link = format!(
                "https://osu.ppy.sh/api/v2/rankings/{}/{}",
                "osu", "performance"
            );

            match &ranking {
                RankingType::Country { code } => {
                    let _ = write!(
                        link, 
                        "?country={code}&cursor[page]={page}"
                    );
                },
                RankingType::Global => {
                    let _ = write!(
                        link,
                        "?cursor[page]={page}"
                    );
                }
            }

            let r: RankingResponse = self.make_request(Method::GET, &link).await?;

            buff.extend(r.ranking);
        }

        Ok(buff)
    }

    // Make request with corresponding token (that we requested earlier
    async fn make_request<T: DeserializeOwned>(&self, method: Method, link: &str) -> ApiResult<T> {
        let token = match &self.token {
            Some(t) => t.as_str(),
            None => return Err(OsuApiError::NoToken),
        };

        let req = Request::builder()
            .method(method)
            .uri(link)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .header(USER_AGENT, "vasteri-bebrik")
            .body(Body::empty())?;

        let mut resp = self.client.request(req).await?;
        let bytes = self.handle_error(&mut resp).await?;

        self.parse_bytes(&bytes).await
    }

    async fn handle_error(&self, res: &mut hyper::Response<Body>) -> ApiResult<Bytes> {
        let bytes = hyper::body::to_bytes(res.body_mut()).await?;
        match res.status() {
            StatusCode::OK => return Ok(bytes),
            StatusCode::BAD_REQUEST => return Err(OsuApiError::BadRequest),
            StatusCode::TOO_MANY_REQUESTS => return Err(OsuApiError::RateLimited),
            StatusCode::SERVICE_UNAVAILABLE => return Err(OsuApiError::ServiceUnavailable),
            _ => (),
        };

        let err: ApiErrorResponse =
            serde_json::from_slice(&bytes).map_err(|e| OsuApiError::ParsingError {
                inner: e,
                body: bytes.clone(),
            })?;

        Err(OsuApiError::ApiError { inner: err })
    }

    async fn parse_bytes<T: DeserializeOwned>(&self, bytes: &Bytes) -> ApiResult<T> {
        serde_json::from_slice(bytes).map_err(|e| OsuApiError::ParsingError {
            inner: e,
            body: bytes.clone(),
        })
    }

    async fn request_oauth(&self) -> ApiResult<String> {
        let data = format!(
            r#"{{
            "client_id":"{}",
            "client_secret":"{}",
            "grant_type":"client_credentials",
            "scope":"public" 
        }}"#,
            &self.client_id, &self.client_secret
        );

        let req = Request::builder()
            .method(Method::POST)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .uri("https://osu.ppy.sh/oauth/token")
            .body(Body::from(data))?;

        let mut response = self.client.request(req).await?;

        let bytes = self.handle_error(&mut response).await?;

        let r: OauthResponse = self.parse_bytes(&bytes).await?;

        Ok(r.access_token)
    }
}

#[cfg(test)]
mod tests {
    use crate::osu_api::{OsuApi, RankingType};
    use std::env;
    use eyre::Result;
    use dotenv::dotenv;

    #[tokio::test]
    async fn test_limit() -> Result<()> {
        dotenv()?;

        let api = OsuApi::new(
            env::var("CLIENT_ID")?.parse()?,
            env::var("CLIENT_SECRET")?.as_str(),
        )
        .await?;
        let ranking = RankingType::Country{ code: "by".to_owned() };

        let lb = api.get_ranking(ranking, 2).await?;

        assert_eq!(lb.len(), 100);

        Ok(())
    }
}
