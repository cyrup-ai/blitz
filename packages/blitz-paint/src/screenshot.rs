// Stub for screenshot functionality

pub struct Screenshot {}
pub struct ScreenshotEngine {
    stats: ScreenshotStats,
}
pub struct ScreenshotConfig {}
pub struct ScreenshotConfigBuilder {}
pub struct ScreenshotRequest {}
pub struct ScreenshotStats {}

impl Screenshot {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Screenshot {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenshotEngine {
    pub fn new() -> Self {
        Self {
            stats: ScreenshotStats::new(),
        }
    }

    pub fn stats(&self) -> &ScreenshotStats {
        &self.stats
    }
}

impl Default for ScreenshotEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenshotConfig {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ScreenshotConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenshotConfigBuilder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn build(self) -> ScreenshotConfig {
        ScreenshotConfig::new()
    }
}

impl Default for ScreenshotConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenshotRequest {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ScreenshotRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenshotStats {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ScreenshotStats {
    fn default() -> Self {
        Self::new()
    }
}
