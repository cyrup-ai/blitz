use std::sync::Arc;

use crate::{FontError, FontKey, FontMetrics, FontSource};

/// A loaded font with associated metadata
#[derive(Debug, Clone)]
pub struct LoadedFont {
    pub key: FontKey,
    pub source: FontSource,
    pub data: Arc<[u8]>,
    pub face_index: u32,
    pub metrics: FontMetrics,
    pub load_time: std::time::Instant,
    pub usage_count: Arc<std::sync::atomic::AtomicU64>,
    pub font_id: Option<blitz_text::fontdb::ID>,
}

impl LoadedFont {
    /// Create a new LoadedFont instance
    pub fn new(
        key: FontKey,
        source: FontSource,
        data: Arc<[u8]>,
        face_index: u32,
        metrics: FontMetrics,
    ) -> Self {
        Self {
            key,
            source,
            data,
            face_index,
            metrics,
            load_time: std::time::Instant::now(),
            usage_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            font_id: None,
        }
    }

    /// Increment usage count
    pub fn increment_usage(&self) {
        self.usage_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get current usage count
    pub fn get_usage_count(&self) -> u64 {
        self.usage_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get time since font was loaded
    pub fn age(&self) -> std::time::Duration {
        self.load_time.elapsed()
    }

    /// Get the size of the font data in bytes
    pub fn data_size(&self) -> usize {
        self.data.len()
    }

    /// Get font format based on data signature
    pub fn get_format(&self) -> FontFormat {
        if self.data.len() < 4 {
            return FontFormat::Unknown;
        }

        let signature = &self.data[0..4];
        match signature {
            b"OTTO" => FontFormat::OpenType,
            [0x00, 0x01, 0x00, 0x00] => FontFormat::TrueType,
            [0x74, 0x72, 0x75, 0x65] => FontFormat::TrueType,
            b"wOFF" => FontFormat::WOFF,
            b"wOF2" => FontFormat::WOFF2,
            _ => FontFormat::Unknown,
        }
    }

    /// Check if this font has been recently used
    pub fn is_recently_used(&self, threshold: std::time::Duration) -> bool {
        self.age() < threshold
    }

    /// Check if this font is actively used (high usage count)
    pub fn is_actively_used(&self, usage_threshold: u64) -> bool {
        self.get_usage_count() >= usage_threshold
    }

    /// Get memory usage estimate in bytes
    pub fn memory_usage(&self) -> usize {
        self.data.len() + std::mem::size_of::<Self>()
    }

    /// Create a usage report for this font
    pub fn usage_report(&self) -> FontUsageReport {
        FontUsageReport {
            family: self.key.family.clone(),
            source: self.source.display_name(),
            usage_count: self.get_usage_count(),
            age: self.age(),
            data_size: self.data_size(),
            format: self.get_format(),
        }
    }

    /// Convert LoadedFont to fontdb::Source for cosmyc-text integration
    pub fn to_fontdb_source(&self) -> glyphon::cosmyc_text::fontdb::Source {
        // Convert Arc<[u8]> to Arc<dyn AsRef<[u8]> + Send + Sync> by creating a Vec wrapper
        let data_vec = self.data.to_vec();
        let data: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(data_vec);
        glyphon::cosmyc_text::fontdb::Source::Binary(data)
    }

    /// Register this font with cosmyc-text FontSystem and return assigned font ID
    pub fn register_with_font_system(
        &mut self,
        font_system: &mut glyphon::cosmyc_text::FontSystem,
    ) -> Result<glyphon::cosmyc_text::fontdb::ID, FontError> {
        let source = self.to_fontdb_source();
        let font_ids = font_system.db_mut().load_font_source(source);

        // Take the first font ID from the returned collection
        let font_id = font_ids.into_iter().next().ok_or_else(|| {
            FontError::FontSystemError("No font ID returned from load_font_source".to_string())
        })?;

        self.font_id = Some(font_id);
        Ok(font_id)
    }
}

/// Font format enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFormat {
    TrueType,
    OpenType,
    WOFF,
    WOFF2,
    TrueTypeCollection,
    OpenTypeCollection,
    Unknown,
}

impl std::fmt::Display for FontFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FontFormat::TrueType => write!(f, "TrueType"),
            FontFormat::OpenType => write!(f, "OpenType"),
            FontFormat::WOFF => write!(f, "WOFF"),
            FontFormat::WOFF2 => write!(f, "WOFF2"),
            FontFormat::TrueTypeCollection => write!(f, "TrueType Collection"),
            FontFormat::OpenTypeCollection => write!(f, "OpenType Collection"),
            FontFormat::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Font usage report for analytics and optimization
#[derive(Debug, Clone)]
pub struct FontUsageReport {
    pub family: String,
    pub source: String,
    pub usage_count: u64,
    pub age: std::time::Duration,
    pub data_size: usize,
    pub format: FontFormat,
}

impl FontUsageReport {
    /// Get usage frequency (uses per second since load)
    pub fn usage_frequency(&self) -> f64 {
        if self.age.as_secs() == 0 {
            self.usage_count as f64
        } else {
            self.usage_count as f64 / self.age.as_secs_f64()
        }
    }

    /// Get efficiency score (usage per byte)
    pub fn efficiency_score(&self) -> f64 {
        if self.data_size == 0 {
            0.0
        } else {
            self.usage_count as f64 / self.data_size as f64 * 1024.0 // per KB
        }
    }
}

impl std::fmt::Display for FontUsageReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Font: {} ({})\n  Usage: {} times ({:.2}/s)\n  Size: {} bytes\n  Age: {:?}\n  Format: {}",
            self.family,
            self.source,
            self.usage_count,
            self.usage_frequency(),
            self.data_size,
            self.age,
            self.format
        )
    }
}
