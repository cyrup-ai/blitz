use std::sync::Arc;

use goldylox::cache::traits::supporting_types::HashAlgorithm;
use goldylox::cache::traits::{CacheValue, CompressionHint};
use goldylox::CacheValueMetadata;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{FontLoadStatus, error::types::FontError};


/// Font cache key based on URL
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WebFontCacheKey {
    pub url: String,
    pub url_hash: u64,
}

impl WebFontCacheKey {
    pub fn new(url: &Url) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let url_str = url.to_string();
        let mut hasher = DefaultHasher::new();
        url_str.hash(&mut hasher);

        Self {
            url: url_str,
            url_hash: hasher.finish(),
        }
    }
}

/// Web font cache entry with loading status and data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFontEntry {
    pub url: String,
    pub status: FontLoadStatus,
    pub data: Option<Vec<u8>>,
    #[serde(skip, default = "std::time::Instant::now")]
    pub load_start: std::time::Instant,
    pub error: Option<String>,
    pub content_type: Option<String>,
    pub size: Option<u64>,
    #[serde(skip, default = "std::time::Instant::now")]
    pub last_accessed: std::time::Instant,
    pub access_count: u64,
}

impl Default for WebFontEntry {
    fn default() -> Self {
        let now = std::time::Instant::now();
        Self {
            url: String::new(),
            status: FontLoadStatus::default(),
            data: None,
            load_start: now,
            error: None,
            content_type: None,
            size: None,
            last_accessed: now,
            access_count: 0,
        }
    }
}

impl WebFontEntry {
    /// Create a new web font entry in NotStarted state
    pub fn new(url: &Url) -> Self {
        let now = std::time::Instant::now();
        Self {
            url: url.to_string(),
            status: FontLoadStatus::NotStarted,
            data: None,
            load_start: now,
            error: None,
            content_type: None,
            size: None,
            last_accessed: now,
            access_count: 0,
        }
    }

    /// Mark as loading
    pub fn mark_loading(&mut self) {
        self.status = FontLoadStatus::Loading;
        self.load_start = std::time::Instant::now();
        self.error = None;
    }

    /// Mark as loaded with data
    pub fn mark_loaded(&mut self, data: Vec<u8>, content_type: Option<String>) {
        self.size = Some(data.len() as u64);
        self.data = Some(data);
        self.content_type = content_type;
        self.status = FontLoadStatus::Loaded;
        self.access_count += 1;
        self.last_accessed = std::time::Instant::now();
    }

    /// Mark as failed with error
    pub fn mark_failed(&mut self, error: String) {
        self.error = Some(error);
        self.status = FontLoadStatus::Failed;
    }

    /// Mark as accessed (update access tracking)
    pub fn mark_accessed(&mut self) {
        self.access_count += 1;
        self.last_accessed = std::time::Instant::now();
    }

    /// Get loading duration
    pub fn loading_duration(&self) -> std::time::Duration {
        if self.status == FontLoadStatus::Loading {
            self.load_start.elapsed()
        } else {
            std::time::Duration::ZERO
        }
    }

    /// Get total time since creation
    pub fn total_age(&self) -> std::time::Duration {
        self.load_start.elapsed()
    }

    /// Get time since last access
    pub fn time_since_last_access(&self) -> std::time::Duration {
        self.last_accessed.elapsed()
    }

    /// Check if this entry is stale (not accessed recently)
    pub fn is_stale(&self, threshold: std::time::Duration) -> bool {
        self.time_since_last_access() > threshold
    }

    /// Check if this entry is actively used
    pub fn is_actively_used(
        &self,
        min_access_count: u64,
        max_idle_time: std::time::Duration,
    ) -> bool {
        self.access_count >= min_access_count && self.time_since_last_access() <= max_idle_time
    }

    /// Get cache efficiency score (access count per byte)
    pub fn efficiency_score(&self) -> f64 {
        if let Some(size) = self.size {
            if size > 0 {
                return self.access_count as f64 / size as f64 * 1024.0; // per KB
            }
        }
        0.0
    }

    /// Get access frequency (accesses per second since last access)
    pub fn access_frequency(&self) -> f64 {
        let age_seconds = self.time_since_last_access().as_secs_f64();
        if age_seconds > 0.0 {
            self.access_count as f64 / age_seconds
        } else {
            self.access_count as f64
        }
    }

    /// Get a priority score for cache retention (higher = keep longer)
    pub fn retention_priority(&self) -> f64 {
        let mut score = 0.0;

        // Base score from access count
        score += self.access_count as f64 * 0.1;

        // Bonus for recent access
        let hours_since_access = self.time_since_last_access().as_secs_f64() / 3600.0;
        score += (24.0 - hours_since_access.min(24.0)) * 0.1; // Up to 2.4 bonus for recent access

        // Bonus for efficiency
        score += self.efficiency_score() * 0.001; // Small bonus for efficiency

        // Penalty for large size (prefer keeping smaller fonts)
        if let Some(size) = self.size {
            let size_penalty = (size as f64 / (1024.0 * 1024.0)).min(10.0) * 0.1; // Up to 1.0 penalty for 10MB+
            score -= size_penalty;
        }

        score.max(0.0)
    }

    /// Create a status report for this entry
    pub fn status_report(&self) -> Result<WebFontStatusReport, FontError> {
        let url = Url::parse(&self.url)
            .or_else(|_| {
                // Safe fallback to data URL - guaranteed to parse successfully
                Url::parse("data:text/plain;charset=utf-8,invalid-url")
            })
            .map_err(FontError::from)?;

        Ok(WebFontStatusReport {
            url,
            status: self.status.clone(),
            size: self.size,
            access_count: self.access_count,
            total_age: self.total_age(),
            time_since_last_access: self.time_since_last_access(),
            loading_duration: self.loading_duration(),
            error: self.error.clone(),
            content_type: self.content_type.clone(),
            efficiency_score: self.efficiency_score(),
            retention_priority: self.retention_priority(),
        })
    }

    /// Create a status report with backward compatibility (non-panicking)
    pub fn status_report_safe(&self) -> (WebFontStatusReport, Option<FontError>) {
        let (url, url_error) = match Url::parse(&self.url) {
            Ok(url) => (url, None),
            Err(e) => {
                let fallback_url = Url::parse("data:text/plain;charset=utf-8,invalid-url")
                    .expect("Data URL should always be valid");
                (fallback_url, Some(FontError::from(e)))
            }
        };

        let report = WebFontStatusReport {
            url,
            status: self.status.clone(),
            size: self.size,
            access_count: self.access_count,
            total_age: self.total_age(),
            time_since_last_access: self.time_since_last_access(),
            loading_duration: self.loading_duration(),
            error: self.error.clone(),
            content_type: self.content_type.clone(),
            efficiency_score: self.efficiency_score(),
            retention_priority: self.retention_priority(),
        };

        (report, url_error)
    }
}

/// Status report for a web font entry
#[derive(Debug, Clone)]
pub struct WebFontStatusReport {
    pub url: Url,
    pub status: FontLoadStatus,
    pub size: Option<u64>,
    pub access_count: u64,
    pub total_age: std::time::Duration,
    pub time_since_last_access: std::time::Duration,
    pub loading_duration: std::time::Duration,
    pub error: Option<String>,
    pub content_type: Option<String>,
    pub efficiency_score: f64,
    pub retention_priority: f64,
}

impl std::fmt::Display for WebFontStatusReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Web Font: {}", self.url)?;
        writeln!(f, "  Status: {}", self.status)?;

        if let Some(size) = self.size {
            writeln!(f, "  Size: {} bytes ({:.1} KB)", size, size as f64 / 1024.0)?;
        }

        writeln!(f, "  Access Count: {}", self.access_count)?;
        writeln!(f, "  Total Age: {:?}", self.total_age)?;
        writeln!(f, "  Last Access: {:?} ago", self.time_since_last_access)?;

        if self.loading_duration > std::time::Duration::ZERO {
            writeln!(f, "  Loading Time: {:?}", self.loading_duration)?;
        }

        if let Some(ref content_type) = self.content_type {
            writeln!(f, "  Content Type: {}", content_type)?;
        }

        if let Some(ref error) = self.error {
            writeln!(f, "  Error: {}", error)?;
        }

        writeln!(f, "  Efficiency: {:.3} accesses/KB", self.efficiency_score)?;
        writeln!(f, "  Retention Priority: {:.2}", self.retention_priority)?;

        Ok(())
    }
}

/// Cache statistics for web fonts
#[derive(Debug, Default, Clone)]
pub struct WebFontCacheStats {
    pub total_entries: usize,
    pub loaded_count: usize,
    pub loading_count: usize,
    pub failed_count: usize,
    pub total_size: u64,
    pub total_access_count: u64,
}

impl WebFontCacheStats {
    /// Get cache hit rate (loaded / total)
    pub fn hit_rate(&self) -> f64 {
        if self.total_entries == 0 {
            0.0
        } else {
            self.loaded_count as f64 / self.total_entries as f64
        }
    }

    /// Get failure rate (failed / total)
    pub fn failure_rate(&self) -> f64 {
        if self.total_entries == 0 {
            0.0
        } else {
            self.failed_count as f64 / self.total_entries as f64
        }
    }

    /// Get average font size
    pub fn average_size(&self) -> f64 {
        if self.loaded_count == 0 {
            0.0
        } else {
            self.total_size as f64 / self.loaded_count as f64
        }
    }

    /// Get average access count per font
    pub fn average_access_count(&self) -> f64 {
        if self.total_entries == 0 {
            0.0
        } else {
            self.total_access_count as f64 / self.total_entries as f64
        }
    }

    /// Get total size in human-readable format
    pub fn total_size_human(&self) -> String {
        let size = self.total_size as f64;
        if size < 1024.0 {
            format!("{} B", size)
        } else if size < 1024.0 * 1024.0 {
            format!("{:.1} KB", size / 1024.0)
        } else if size < 1024.0 * 1024.0 * 1024.0 {
            format!("{:.1} MB", size / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", size / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

impl std::fmt::Display for WebFontCacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Web Font Cache Statistics:")?;
        writeln!(f, "  Total Entries: {}", self.total_entries)?;
        writeln!(
            f,
            "  Loaded: {} ({:.1}%)",
            self.loaded_count,
            self.hit_rate() * 100.0
        )?;
        writeln!(f, "  Loading: {}", self.loading_count)?;
        writeln!(
            f,
            "  Failed: {} ({:.1}%)",
            self.failed_count,
            self.failure_rate() * 100.0
        )?;
        writeln!(f, "  Total Size: {}", self.total_size_human())?;
        writeln!(f, "  Average Size: {:.1} KB", self.average_size() / 1024.0)?;
        writeln!(f, "  Total Accesses: {}", self.total_access_count)?;
        write!(
            f,
            "  Average Accesses/Font: {:.1}",
            self.average_access_count()
        )?;
        Ok(())
    }
}

// WebFontCacheKey is no longer needed - goldylox uses String keys directly

/// CacheValue implementation for WebFontEntry
impl CacheValue for WebFontEntry {
    type Metadata = CacheValueMetadata;

    fn estimated_size(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.url.len()
            + self.data.as_ref().map(|d| d.len()).unwrap_or(0)
            + self.error.as_ref().map(|e| e.len()).unwrap_or(0)
            + self.content_type.as_ref().map(|c| c.len()).unwrap_or(0)
    }

    fn is_expensive(&self) -> bool {
        // Large fonts or fonts with loading errors are expensive
        self.size.unwrap_or(0) > 100_000 || self.error.is_some()
    }

    fn compression_hint(&self) -> CompressionHint {
        // Large fonts benefit from compression, woff2 already compressed
        if let Some(size) = self.size {
            if size > 50_000 && !self.url.contains(".woff2") {
                CompressionHint::Force
            } else {
                CompressionHint::Disable
            }
        } else {
            CompressionHint::Auto
        }
    }

    fn metadata(&self) -> Self::Metadata {
        CacheValueMetadata::from_cache_value(self)
    }
}
