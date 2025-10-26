use uuid::Uuid;
use std::collections::HashMap;

use linkwithmentor_common::AppError;
use crate::{
    models::{StreamingManifest, StreamingFormat, StreamingQuality, SubtitleTrack, ThumbnailSprite},
    storage::StorageService,
    config::StreamingConfig,
};

#[derive(Clone)]
pub struct StreamingService {
    storage_service: StorageService,
    config: StreamingConfig,
}

impl StreamingService {
    pub fn new(storage_service: StorageService, config: &StreamingConfig) -> Self {
        Self {
            storage_service,
            config: config.clone(),
        }
    }

    pub async fn get_streaming_manifest(
        &self,
        lecture_id: Uuid,
        format: StreamingFormat,
    ) -> Result<StreamingManifest, AppError> {
        let base_url = format!("{}/videos/{}", self.config.cdn_base_url, lecture_id);

        let qualities = vec![
            StreamingQuality {
                name: "720p".to_string(),
                resolution: "1280x720".to_string(),
                bitrate: "2500k".to_string(),
                url: format!("{}/720p/playlist.m3u8", base_url),
            },
            StreamingQuality {
                name: "480p".to_string(),
                resolution: "854x480".to_string(),
                bitrate: "1000k".to_string(),
                url: format!("{}/480p/playlist.m3u8", base_url),
            },
            StreamingQuality {
                name: "360p".to_string(),
                resolution: "640x360".to_string(),
                bitrate: "500k".to_string(),
                url: format!("{}/360p/playlist.m3u8", base_url),
            },
        ];

        let subtitles = vec![
            SubtitleTrack {
                language: "en".to_string(),
                label: "English".to_string(),
                url: format!("{}/subtitles/en.vtt", base_url),
                is_default: true,
            },
        ];

        let thumbnails = vec![
            ThumbnailSprite {
                url: format!("{}/thumbnails/sprite.jpg", base_url),
                width: 160,
                height: 90,
                columns: 10,
                rows: 10,
                interval_seconds: 10,
            },
        ];

        let manifest_url = match format {
            StreamingFormat::HLS => format!("{}/master.m3u8", base_url),
            StreamingFormat::DASH => format!("{}/manifest.mpd", base_url),
            StreamingFormat::Progressive => format!("{}/720p/video.mp4", base_url),
        };

        Ok(StreamingManifest {
            lecture_id,
            format,
            manifest_url,
            qualities,
            subtitles,
            thumbnails,
        })
    }

    pub async fn generate_hls_manifest(&self, lecture_id: Uuid) -> Result<String, AppError> {
        // Generate HLS master playlist
        let manifest = r#"#EXTM3U
#EXT-X-VERSION:3
#EXT-X-STREAM-INF:BANDWIDTH=2500000,RESOLUTION=1280x720
720p/playlist.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=1000000,RESOLUTION=854x480
480p/playlist.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=500000,RESOLUTION=640x360
360p/playlist.m3u8"#;

        Ok(manifest.to_string())
    }

    pub async fn generate_dash_manifest(&self, lecture_id: Uuid) -> Result<String, AppError> {
        // Generate DASH manifest (simplified)
        let manifest = r#"<?xml version="1.0" encoding="UTF-8"?>
<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" type="static" mediaPresentationDuration="PT0H30M0S">
  <Period>
    <AdaptationSet mimeType="video/mp4">
      <Representation id="720p" bandwidth="2500000" width="1280" height="720">
        <BaseURL>720p/</BaseURL>
        <SegmentTemplate media="segment_$Number$.m4s" initialization="init.mp4" startNumber="1"/>
      </Representation>
    </AdaptationSet>
  </Period>
</MPD>"#;

        Ok(manifest.to_string())
    }
}