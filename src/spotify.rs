use rspotify::{
    AuthCodePkceSpotify, prelude::*,
};

// 3. Dein gewünschtes Ziel-Struct
#[derive(Debug)]
pub struct Track {
    pub name: String,
    pub artists: Vec<String>,
    pub album: String,
    pub duration_ms: u32,
    pub current_ms: u32,
    pub explicit: bool,
    pub id: String,
    pub image_url: String,
    pub is_playing: bool,
    pub shuffle: bool,
    pub repeat: u8,
}

pub struct Spotify {
    pub auth: AuthCodePkceSpotify
}

impl Spotify {
    
    pub async fn playback_info(&self) -> Result<Option<Track>, String>{
        let playback = self.auth
            .current_playback(None, None::<Vec<_>>)
            .await
            .map_err(|error| error.to_string())?;

        let Some(playback) = playback else {
            return Ok(None);
        };

        let Some(item) = playback.item else {
            return Ok(None);
        };


        let track = match item {
            rspotify::model::PlayableItem::Track(track) => Track {
                name: track.name,
                artists: track.artists.into_iter().map(|artist| artist.name).collect(),
                album: track.album.name,
                duration_ms: u32::try_from(track.duration.num_milliseconds()).unwrap_or(0),
                current_ms: playback.progress.map(|p| u32::try_from(p.num_milliseconds()).unwrap_or(0)).unwrap_or(0),
                explicit: track.explicit,
                id: track.id.map(|id| id.id().to_owned()).unwrap_or_default(),
                image_url: track.album.images.get(1).map(|image| image.url.clone()).unwrap_or_default(),
                is_playing: playback.is_playing,
                shuffle: playback.shuffle_state,
                repeat: match playback.repeat_state {
                    rspotify::model::RepeatState::Off => 0,
                    rspotify::model::RepeatState::Context => 1,
                    rspotify::model::RepeatState::Track => 2,
                },
            },
            rspotify::model::PlayableItem::Unknown(value) => {
                let name = value.get("name").and_then(|v| v.as_str())
                    .ok_or_else(|| "Spotify-Antwort enthält keinen Titel".to_string())?
                    .to_string();
                let duration_ms = value.get("duration_ms").and_then(|v| v.as_u64())
                    .and_then(|duration| u32::try_from(duration).ok())
                    .unwrap_or(0);
                let artists = value.get("artists").and_then(|v| v.as_array())
                    .into_iter()
                    .flatten()
                    .filter_map(|artist| artist.get("name").and_then(|v| v.as_str()))
                    .map(str::to_owned)
                    .collect();
                let image_url = value.get("album").and_then(|album| album.get("images"))
                    .and_then(|images| images.as_array())
                    .and_then(|arr| arr.get(1))
                    .and_then(|img| img.get("url"))
                    .and_then(|url| url.as_str())
                    .unwrap_or_default()
                    .to_string();
                let album = value.get("album").and_then(|v| v.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let id = value.get("id").and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let explicit = value.get("explicit").and_then(|v| v.as_bool()).unwrap_or(false);
                Track {
                    name,
                    artists,
                    album,
                    duration_ms,
                    explicit,
                    id,
                    image_url,
                    current_ms: playback.progress.map(|p| u32::try_from(p.num_milliseconds()).unwrap_or(0)).unwrap_or(0),
                    is_playing: playback.is_playing,
                    shuffle: playback.shuffle_state,
                    repeat: match playback.repeat_state {
                        rspotify::model::RepeatState::Off => 0,
                        rspotify::model::RepeatState::Context => 2,
                        rspotify::model::RepeatState::Track => 1,
                    }
                }
            }
            rspotify::model::PlayableItem::Episode(episode) => Track {
                name: episode.name,
                artists: Vec::new(),
                album: episode.show.name,
                duration_ms: u32::try_from(episode.duration.num_milliseconds()).unwrap_or(0),
                explicit: false,
                id: episode.id.id().to_owned(),
                image_url: episode.images.get(1).map(|image| image.url.clone()).unwrap_or_default(),
                current_ms: playback.progress.map(|p| u32::try_from(p.num_milliseconds()).unwrap_or(0)).unwrap_or(0),
                is_playing: playback.is_playing,
                shuffle: playback.shuffle_state,
                    repeat: match playback.repeat_state {
                        rspotify::model::RepeatState::Off => 0,
                        rspotify::model::RepeatState::Context => 1,
                        rspotify::model::RepeatState::Track => 2,
                    }
            },
        };

        Ok(Some(track))
    }

    pub async fn pause(&self) -> Result<(), String> {
        self.auth.pause_playback(None).await.map_err(|error| error.to_string())?;
        Ok(())
    }
    pub async fn play(&self) -> Result<(), String> {
        self.auth.resume_playback(None, None).await.map_err(|error| error.to_string())?;
        Ok(())
    }
    pub async fn next(&self) -> Result<(), String> {
        self.auth.next_track(None).await.map_err(|error| error.to_string())?;
        Ok(())
    }
    pub async fn previous(&self) -> Result<(), String> {
        self.auth.previous_track(None).await.map_err(|error| error.to_string())?;
        Ok(())
    }
    pub async fn shuffle(&self, state: bool) -> Result<(), String> {
        self.auth.shuffle(state, None).await.map_err(|error| error.to_string())?;
        Ok(())
    }
    pub async fn repeat(&self, state: u8) -> Result<(), String> {
        let repeat = match state {
                        2 => rspotify::model::RepeatState::Context,
                        0 => rspotify::model::RepeatState::Off,
                        1 => rspotify::model::RepeatState::Track,
                        _ => rspotify::model::RepeatState::Off,
                    };
        self.auth.repeat(repeat, None).await.map_err(|error| error.to_string())?;
        Ok(())
    }


}