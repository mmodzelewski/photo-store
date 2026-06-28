//! Pure layout math for segment-encrypted files.
//!
//! An `original` is sealed as a sequence of fixed-size plaintext segments, each
//! independently AES-256-GCM encrypted (see [`crate::crypto`]). Every segment's
//! ciphertext is its plaintext bytes followed by a 16-byte GCM tag, and the
//! segments are concatenated in order into a single object. Because each
//! segment is sealed independently, a client can fetch and decrypt an arbitrary
//! sub-range of segments — this is what makes video seek possible.
//!
//! This module owns the cipher-free arithmetic that maps between plaintext
//! offsets, segment indices, and ciphertext byte offsets. It is shared so that
//! clients computing seek ranges and the backend (which mostly just stores the
//! scalars) agree on the layout.

use std::ops::RangeInclusive;

/// Default plaintext bytes per segment (1 MiB). Tag overhead is ~0.0015%, and
/// it gives ~1 MiB seek granularity. Persisted per file so it can evolve.
pub const DEFAULT_SEGMENT_SIZE: u32 = 1024 * 1024;

/// Bytes appended to each segment's ciphertext by AES-256-GCM (the auth tag).
pub const GCM_TAG_SIZE: u64 = 16;

/// Describes how a file's plaintext is split into encrypted segments. Built
/// from the two scalars persisted on the file (`segment_size`, `plaintext_size`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentLayout {
    segment_size: u32,
    plaintext_size: u64,
}

/// The segments and ciphertext byte range that cover a requested plaintext
/// range — the result of mapping a seek request onto the stored object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentSpan {
    /// Inclusive range of segment indices that must be fetched and decrypted.
    pub segments: RangeInclusive<u64>,
    /// Start offset (inclusive) into the ciphertext object.
    pub ciphertext_start: u64,
    /// End offset (exclusive) into the ciphertext object.
    pub ciphertext_end: u64,
}

impl SegmentSpan {
    /// HTTP `Range` header value for this span. The ciphertext end is
    /// exclusive here but HTTP byte ranges are inclusive, so the upper bound is
    /// `ciphertext_end - 1`.
    pub fn http_range_header(&self) -> String {
        format!(
            "bytes={}-{}",
            self.ciphertext_start,
            self.ciphertext_end - 1
        )
    }
}

impl SegmentLayout {
    /// Construct a layout. `segment_size` must be non-zero; returns `None`
    /// otherwise (callers validate bounds before persisting).
    pub fn new(segment_size: u32, plaintext_size: u64) -> Option<Self> {
        if segment_size == 0 {
            return None;
        }
        Some(Self {
            segment_size,
            plaintext_size,
        })
    }

    pub fn segment_size(&self) -> u32 {
        self.segment_size
    }

    pub fn plaintext_size(&self) -> u64 {
        self.plaintext_size
    }

    /// Number of segments. Zero-length plaintext yields zero segments.
    pub fn segment_count(&self) -> u64 {
        if self.plaintext_size == 0 {
            0
        } else {
            self.plaintext_size.div_ceil(self.segment_size as u64)
        }
    }

    /// Plaintext length of segment `i` (the last segment may be shorter).
    /// Returns 0 for out-of-range indices.
    pub fn plaintext_len(&self, i: u64) -> u64 {
        let start = i.saturating_mul(self.segment_size as u64);
        if start >= self.plaintext_size {
            0
        } else {
            (self.segment_size as u64).min(self.plaintext_size - start)
        }
    }

    /// Byte offset of segment `i` within the ciphertext object. Every preceding
    /// segment is exactly `segment_size + tag`, so this is correct for all `i`.
    pub fn ciphertext_offset(&self, i: u64) -> u64 {
        i.saturating_mul(self.segment_size as u64 + GCM_TAG_SIZE)
    }

    /// Ciphertext length of segment `i` (plaintext length plus the GCM tag).
    /// Returns 0 for out-of-range indices.
    pub fn ciphertext_len(&self, i: u64) -> u64 {
        if i >= self.segment_count() {
            0
        } else {
            self.plaintext_len(i) + GCM_TAG_SIZE
        }
    }

    /// Total size of the ciphertext object (`plaintext_size + count * tag`).
    pub fn ciphertext_size(&self) -> u64 {
        self.plaintext_size + self.segment_count() * GCM_TAG_SIZE
    }

    /// Map a half-open plaintext byte range `[start, end_exclusive)` to the
    /// segments and ciphertext byte range that cover it. `end_exclusive` is
    /// clamped to the plaintext size. Returns `None` for an empty file or an
    /// empty/out-of-bounds request.
    pub fn range_for_plaintext(&self, start: u64, end_exclusive: u64) -> Option<SegmentSpan> {
        if self.plaintext_size == 0 || start >= self.plaintext_size {
            return None;
        }
        let end = end_exclusive.min(self.plaintext_size);
        if start >= end {
            return None;
        }

        let ss = self.segment_size as u64;
        let first = start / ss;
        let last = (end - 1) / ss;

        Some(SegmentSpan {
            segments: first..=last,
            ciphertext_start: self.ciphertext_offset(first),
            ciphertext_end: self.ciphertext_offset(last) + self.ciphertext_len(last),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// segment_size=10, plaintext=25 → segments of 10, 10, 5 bytes.
    fn small() -> SegmentLayout {
        SegmentLayout::new(10, 25).unwrap()
    }

    #[test]
    fn rejects_zero_segment_size() {
        assert!(SegmentLayout::new(0, 100).is_none());
    }

    #[test]
    fn counts_and_lengths() {
        let l = small();
        assert_eq!(l.segment_count(), 3);
        assert_eq!(l.plaintext_len(0), 10);
        assert_eq!(l.plaintext_len(1), 10);
        assert_eq!(l.plaintext_len(2), 5);
        assert_eq!(l.plaintext_len(3), 0); // out of range
    }

    #[test]
    fn ciphertext_offsets_and_lengths() {
        let l = small();
        // each full segment is 10 + 16 = 26 ciphertext bytes
        assert_eq!(l.ciphertext_offset(0), 0);
        assert_eq!(l.ciphertext_offset(1), 26);
        assert_eq!(l.ciphertext_offset(2), 52);
        assert_eq!(l.ciphertext_len(0), 26);
        assert_eq!(l.ciphertext_len(2), 21); // 5 + 16
        assert_eq!(l.ciphertext_len(3), 0); // out of range
        // total: 25 plaintext + 3 tags = 73; matches offset(last)+len(last)
        assert_eq!(l.ciphertext_size(), 73);
        assert_eq!(l.ciphertext_offset(2) + l.ciphertext_len(2), 73);
    }

    #[test]
    fn single_segment_file() {
        let l = SegmentLayout::new(DEFAULT_SEGMENT_SIZE, 100).unwrap();
        assert_eq!(l.segment_count(), 1);
        assert_eq!(l.ciphertext_offset(0), 0);
        assert_eq!(l.ciphertext_len(0), 100 + GCM_TAG_SIZE);
        assert_eq!(l.ciphertext_size(), 116);
    }

    #[test]
    fn exact_multiple_has_full_last_segment() {
        let l = SegmentLayout::new(10, 20).unwrap();
        assert_eq!(l.segment_count(), 2);
        assert_eq!(l.plaintext_len(1), 10);
        assert_eq!(l.ciphertext_size(), 20 + 2 * GCM_TAG_SIZE);
    }

    #[test]
    fn zero_length_file() {
        let l = SegmentLayout::new(10, 0).unwrap();
        assert_eq!(l.segment_count(), 0);
        assert_eq!(l.ciphertext_size(), 0);
        assert!(l.range_for_plaintext(0, 10).is_none());
    }

    #[test]
    fn range_spanning_two_segments() {
        let l = small();
        let span = l.range_for_plaintext(5, 15).unwrap();
        assert_eq!(span.segments, 0..=1);
        assert_eq!(span.ciphertext_start, 0);
        assert_eq!(span.ciphertext_end, 52); // offset(1)+len(1) = 26+26
    }

    #[test]
    fn range_covering_whole_file() {
        let l = small();
        let span = l.range_for_plaintext(0, 25).unwrap();
        assert_eq!(span.segments, 0..=2);
        assert_eq!(span.ciphertext_start, 0);
        assert_eq!(span.ciphertext_end, l.ciphertext_size());
    }

    #[test]
    fn range_into_last_segment_only() {
        let l = small();
        let span = l.range_for_plaintext(20, 25).unwrap();
        assert_eq!(span.segments, 2..=2);
        assert_eq!(span.ciphertext_start, 52);
        assert_eq!(span.ciphertext_end, 73);
    }

    #[test]
    fn range_end_is_clamped_to_plaintext_size() {
        let l = small();
        let span = l.range_for_plaintext(24, 9999).unwrap();
        assert_eq!(span.segments, 2..=2);
        assert_eq!(span.ciphertext_end, 73);
    }

    #[test]
    fn range_out_of_bounds_is_none() {
        let l = small();
        assert!(l.range_for_plaintext(25, 30).is_none());
        assert!(l.range_for_plaintext(10, 10).is_none());
    }

    #[test]
    fn http_range_header_is_inclusive() {
        let l = small();
        let span = l.range_for_plaintext(0, 25).unwrap();
        // ciphertext [0, 73) -> inclusive bytes=0-72
        assert_eq!(span.http_range_header(), "bytes=0-72");
    }
}
