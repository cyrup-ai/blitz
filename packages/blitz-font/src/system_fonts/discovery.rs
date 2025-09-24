use std::path::{Path, PathBuf};

use crate::{FontError, SystemFont};

/// Configuration for system font discovery
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    pub max_fonts: usize,
    pub recursive_depth: u8,
    pub include_hidden: bool,
    pub custom_directories: Vec<PathBuf>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            max_fonts: crate::constants::MAX_FONT_CACHE_SIZE,
            recursive_depth: 3,
            include_hidden: false,
            custom_directories: Vec::new(),
        }
    }
}

/// System font discovery utility
pub struct SystemFontDiscovery {
    config: DiscoveryConfig,
}

impl SystemFontDiscovery {
    pub fn new(config: DiscoveryConfig) -> Self {
        Self { config }
    }

    /// Discover all system fonts across all platforms
    pub fn discover_all_fonts(&self) -> Result<Vec<SystemFont>, FontError> {
        let mut fonts = Vec::new();

        #[cfg(target_os = "windows")]
        self.discover_windows_fonts(&mut fonts)?;

        #[cfg(target_os = "macos")]
        self.discover_macos_fonts(&mut fonts)?;

        #[cfg(target_os = "linux")]
        self.discover_linux_fonts(&mut fonts)?;

        // Scan custom directories
        for dir in &self.config.custom_directories {
            if dir.exists() {
                self.scan_font_directory_sync(dir, &mut fonts, 0)?;
            }
        }

        // Limit total fonts to prevent memory issues
        fonts.truncate(self.config.max_fonts);

        Ok(fonts)
    }

    /// Windows font discovery
    #[cfg(target_os = "windows")]
    fn discover_windows_fonts(&self, fonts: &mut Vec<SystemFont>) -> Result<(), FontError> {
        let windows_fonts_dir = PathBuf::from(r"C:\Windows\Fonts");
        let user_fonts_dir = dirs::font_dir().unwrap_or_else(|| {
            PathBuf::from(r"C:\Users\Default\AppData\Local\Microsoft\Windows\Fonts")
        });

        for fonts_dir in &[windows_fonts_dir, user_fonts_dir] {
            if fonts_dir.exists() && fonts.len() < self.config.max_fonts {
                self.scan_font_directory_sync(fonts_dir, fonts, 0)?;
            }
        }

        Ok(())
    }

    /// macOS font discovery
    #[cfg(target_os = "macos")]
    fn discover_macos_fonts(&self, fonts: &mut Vec<SystemFont>) -> Result<(), FontError> {
        let font_dirs = vec![
            PathBuf::from("/System/Library/Fonts"),
            PathBuf::from("/Library/Fonts"),
            PathBuf::from(format!(
                "{}/Library/Fonts",
                std::env::var("HOME").unwrap_or_default()
            )),
        ];

        for font_dir in font_dirs {
            if font_dir.exists() && fonts.len() < self.config.max_fonts {
                self.scan_font_directory_sync(&font_dir, fonts, 0)?;
            }
        }

        Ok(())
    }

    /// Linux font discovery
    #[cfg(target_os = "linux")]
    fn discover_linux_fonts(&self, fonts: &mut Vec<SystemFont>) -> Result<(), FontError> {
        let font_dirs = vec![
            PathBuf::from("/usr/share/fonts"),
            PathBuf::from("/usr/local/share/fonts"),
            PathBuf::from(format!(
                "{}/.fonts",
                std::env::var("HOME").unwrap_or_default()
            )),
            PathBuf::from(format!(
                "{}/.local/share/fonts",
                std::env::var("HOME").unwrap_or_default()
            )),
        ];

        for font_dir in font_dirs {
            if font_dir.exists() && fonts.len() < self.config.max_fonts {
                self.scan_font_directory_sync(&font_dir, fonts, 0)?;
            }
        }

        Ok(())
    }

    /// Synchronous font directory scanning with depth control
    fn scan_font_directory_sync(
        &self,
        dir: &Path,
        fonts: &mut Vec<SystemFont>,
        depth: u8,
    ) -> Result<(), FontError> {
        if depth > self.config.recursive_depth || fonts.len() >= self.config.max_fonts {
            return Ok(());
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return Ok(()), // Skip inaccessible directories
        };

        for entry in entries {
            if fonts.len() >= self.config.max_fonts {
                break;
            }

            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue, // Skip problematic entries
            };

            let path = entry.path();

            // Skip hidden files unless configured otherwise
            if !self.config.include_hidden {
                if let Some(filename) = path.file_name() {
                    if filename.to_string_lossy().starts_with('.') {
                        continue;
                    }
                }
            }

            if path.is_file() {
                if self.is_font_file(&path) {
                    match crate::system_fonts::parsing::FontParser::parse_font_file(&path) {
                        Ok(Some(system_font)) => fonts.push(system_font),
                        Ok(None) => {} // File parsed but no font found
                        Err(_) => {}   // Skip files that can't be parsed
                    }
                }
            } else if path.is_dir() {
                // Recursively scan subdirectories with depth tracking
                self.scan_font_directory_sync(&path, fonts, depth + 1)?;
            }
        }

        Ok(())
    }

    /// Check if file is a supported font file
    fn is_font_file(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            matches!(
                ext.as_str(),
                "ttf" | "otf" | "woff" | "woff2" | "ttc" | "otc"
            )
        } else {
            false
        }
    }

    /// Get platform-specific default font directories
    pub fn get_default_directories() -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        #[cfg(target_os = "windows")]
        {
            dirs.push(PathBuf::from(r"C:\Windows\Fonts"));
            if let Some(user_fonts) = dirs::font_dir() {
                dirs.push(user_fonts);
            }
        }

        #[cfg(target_os = "macos")]
        {
            dirs.push(PathBuf::from("/System/Library/Fonts"));
            dirs.push(PathBuf::from("/Library/Fonts"));
            if let Ok(home) = std::env::var("HOME") {
                dirs.push(PathBuf::from(format!("{}/Library/Fonts", home)));
            }
        }

        #[cfg(target_os = "linux")]
        {
            dirs.push(PathBuf::from("/usr/share/fonts"));
            dirs.push(PathBuf::from("/usr/local/share/fonts"));
            if let Ok(home) = std::env::var("HOME") {
                dirs.push(PathBuf::from(format!("{}/.fonts", home)));
                dirs.push(PathBuf::from(format!("{}/.local/share/fonts", home)));
            }
        }

        dirs
    }

    /// Create discovery config with common presets
    pub fn config_minimal() -> DiscoveryConfig {
        DiscoveryConfig {
            max_fonts: 50,
            recursive_depth: 1,
            include_hidden: false,
            custom_directories: Vec::new(),
        }
    }

    pub fn config_comprehensive() -> DiscoveryConfig {
        DiscoveryConfig {
            max_fonts: 2048,
            recursive_depth: 5,
            include_hidden: true,
            custom_directories: Vec::new(),
        }
    }
}

impl Default for SystemFontDiscovery {
    fn default() -> Self {
        Self::new(DiscoveryConfig::default())
    }
}
