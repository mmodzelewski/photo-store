//! Media-type model and the media-type-driven variant contract.
//!
//! The server can never inspect a file's contents (everything is E2E
//! encrypted), so the media type is supplied by the client at metadata-upload
//! time and trusted as cosmetic metadata. The variant contract below decides
//! which thumbnail variants a file must provide before its upload can be
//! marked complete;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::thumbnails::ThumbnailVariant;

/// The kind of media a file holds. Drives the required-variant contract and
/// client-side rendering (e.g. a play badge / duration label for video).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumString)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum MediaType {
    Image,
    Video,
}

/// Poster variants shared by every media type: a 512px cover crop and a
/// 1920px contained preview. For video these are decoded-keyframe posters; the
/// upload-completion contract stays uniform across media types.
const POSTER_VARIANTS: &[ThumbnailVariant] = &[
    ThumbnailVariant::small_cover(),
    ThumbnailVariant::big_contain(),
];

/// The thumbnail variants a file of the given media type must provide before
/// its upload can be completed. Thumbnails are always stored whole (never
/// segmented); only the `original` variant is segment-encrypted.
pub fn required_variants(media_type: MediaType) -> &'static [ThumbnailVariant] {
    match media_type {
        MediaType::Image | MediaType::Video => POSTER_VARIANTS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn media_type_serde_is_lowercase() {
        assert_eq!(
            serde_json::to_string(&MediaType::Video).unwrap(),
            "\"video\""
        );
        assert_eq!(
            serde_json::from_str::<MediaType>("\"image\"").unwrap(),
            MediaType::Image
        );
    }

    #[test]
    fn media_type_string_roundtrip() {
        assert_eq!(MediaType::Video.to_string(), "video");
        assert_eq!(MediaType::from_str("image").unwrap(), MediaType::Image);
    }

    #[test]
    fn required_variants_are_the_poster_set() {
        for media_type in [MediaType::Image, MediaType::Video] {
            let names: Vec<String> = required_variants(media_type)
                .iter()
                .map(|v| v.to_string())
                .collect();
            assert_eq!(names, vec!["512-cover", "1920-contain"]);
        }
    }
}
