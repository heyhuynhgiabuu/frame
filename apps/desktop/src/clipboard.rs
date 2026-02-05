//! Clipboard integration for Frame desktop app
//!
//! Provides macOS pasteboard integration for copying:
//! - File references (MP4 files as file:// URLs)
//! - Image data (GIF frames as bitmap data)

use std::path::Path;
use std::sync::Mutex;

use arboard::{Clipboard, ImageData};
use frame_core::error::{FrameError, FrameResult};
use tracing::{debug, warn};

/// Manages clipboard operations with arboard
///
/// Thread-safe wrapper around the arboard clipboard that provides
/// Frame-specific operations for copying files and images.
#[allow(dead_code)]
pub struct ClipboardManager {
    /// The underlying clipboard (wrapped in Mutex for thread safety)
    clipboard: Mutex<Clipboard>,
}

#[allow(dead_code)]
impl ClipboardManager {
    /// Creates a new ClipboardManager instance
    ///
    /// # Errors
    /// Returns an error if the clipboard cannot be initialized
    ///
    /// # Example
    /// ```
    /// use frame_desktop::clipboard::ClipboardManager;
    ///
    /// match ClipboardManager::new() {
    ///     Ok(manager) => println!("Clipboard ready"),
    ///     Err(e) => eprintln!("Failed to initialize clipboard: {}", e),
    /// }
    /// ```
    pub fn new() -> FrameResult<Self> {
        let clipboard = Clipboard::new().map_err(|e| {
            FrameError::PlatformError(format!("Failed to initialize clipboard: {}", e))
        })?;

        debug!("ClipboardManager initialized successfully");
        Ok(Self {
            clipboard: Mutex::new(clipboard),
        })
    }

    /// Checks if the clipboard is available and functional
    ///
    /// This can be used to check clipboard access before attempting
    /// copy operations, especially useful for handling permission scenarios.
    ///
    /// # Returns
    /// `true` if clipboard is available and can be used, `false` otherwise
    pub fn is_available() -> bool {
        match Clipboard::new() {
            Ok(mut clipboard) => {
                // Try a simple operation to verify functionality
                match clipboard.get_text() {
                    Ok(_) | Err(arboard::Error::ContentNotAvailable) => {
                        debug!("Clipboard is available");
                        true
                    }
                    Err(e) => {
                        warn!("Clipboard available but test read failed: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                warn!("Clipboard not available: {}", e);
                false
            }
        }
    }

    /// Copies a file reference to the clipboard as a file:// URL
    ///
    /// On macOS, this copies the file path in a format that allows
    /// pasting into Finder or other applications as a file reference.
    ///
    /// # Arguments
    /// * `path` - The path to the file to copy
    ///
    /// # Errors
    /// Returns an error if:
    /// - The path doesn't exist
    /// - The clipboard cannot be accessed
    /// - The file URL cannot be created
    ///
    /// # Example
    /// ```
    /// use std::path::Path;
    /// use frame_desktop::clipboard::ClipboardManager;
    ///
    /// let manager = ClipboardManager::new().unwrap();
    /// manager.copy_file_reference(Path::new("/path/to/video.mp4")).unwrap();
    /// ```
    pub fn copy_file_reference(&self, path: &Path) -> FrameResult<()> {
        if !path.exists() {
            return Err(FrameError::Io(format!(
                "File does not exist: {}",
                path.display()
            )));
        }

        // Convert to absolute path and create file:// URL
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|e| FrameError::Io(format!("Failed to get current directory: {}", e)))?
                .join(path)
        };

        let file_url = format!("file://{}", absolute_path.display());

        let mut clipboard = self
            .clipboard
            .lock()
            .map_err(|e| FrameError::PlatformError(format!("Failed to lock clipboard: {}", e)))?;

        clipboard.set_text(file_url.clone()).map_err(|e| {
            FrameError::PlatformError(format!("Failed to copy file reference: {}", e))
        })?;

        debug!(
            file_path = %absolute_path.display(),
            "Copied file reference to clipboard"
        );

        Ok(())
    }

    /// Copies raw image data to the clipboard
    ///
    /// Converts the provided image data into a format suitable for
    /// pasting into image editors or chat applications.
    ///
    /// # Arguments
    /// * `data` - Raw RGBA pixel data (4 bytes per pixel)
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    ///
    /// # Errors
    /// Returns an error if:
    /// - The data length doesn't match width × height × 4
    /// - The clipboard cannot be accessed
    /// - The image data is too large for the clipboard
    ///
    /// # Example
    /// ```
    /// use frame_desktop::clipboard::ClipboardManager;
    ///
    /// let manager = ClipboardManager::new().unwrap();
    /// // Create a 100x100 red image
    /// let mut data = vec![255u8; 100 * 100 * 4];
    /// for pixel in data.chunks_exact_mut(4) {
    ///     pixel[0] = 255; // R
    ///     pixel[1] = 0;   // G
    ///     pixel[2] = 0;   // B
    ///     pixel[3] = 255; // A
    /// }
    /// manager.copy_image_data(&data, 100, 100).unwrap();
    /// ```
    pub fn copy_image_data(&self, data: &[u8], width: u32, height: u32) -> FrameResult<()> {
        let expected_len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|p| p.checked_mul(4))
            .ok_or_else(|| {
                FrameError::PlatformError("Image dimensions overflow (too large)".to_string())
            })?;

        if data.len() != expected_len {
            return Err(FrameError::PlatformError(format!(
                "Image data length mismatch: expected {} bytes for {}x{} RGBA, got {}",
                expected_len,
                width,
                height,
                data.len()
            )));
        }

        // Validate dimensions are reasonable
        if width == 0 || height == 0 {
            return Err(FrameError::PlatformError(
                "Image dimensions must be greater than 0".to_string(),
            ));
        }

        if width > 16384 || height > 16384 {
            return Err(FrameError::PlatformError(format!(
                "Image dimensions too large: {}x{} (max 16384x16384)",
                width, height
            )));
        }

        let image_data = ImageData {
            width: width as usize,
            height: height as usize,
            bytes: std::borrow::Cow::Borrowed(data),
        };

        let mut clipboard = self
            .clipboard
            .lock()
            .map_err(|e| FrameError::PlatformError(format!("Failed to lock clipboard: {}", e)))?;

        clipboard.set_image(image_data).map_err(|e| {
            FrameError::PlatformError(format!("Failed to copy image to clipboard: {}", e))
        })?;

        debug!(
            width = width,
            height = height,
            bytes = data.len(),
            "Copied image data to clipboard"
        );

        Ok(())
    }

    /// Convenience method to copy a GIF file's first frame to clipboard
    ///
    /// This is a placeholder for future GIF frame extraction functionality.
    /// Currently just copies the file reference.
    ///
    /// # Arguments
    /// * `path` - Path to the GIF file
    ///
    /// # Errors
    /// Returns an error if the file doesn't exist or clipboard access fails
    pub fn copy_gif_file(&self, path: &Path) -> FrameResult<()> {
        if !path.exists() {
            return Err(FrameError::Io(format!(
                "GIF file does not exist: {}",
                path.display()
            )));
        }

        // For now, copy as file reference
        // TODO: Implement actual GIF frame extraction
        self.copy_file_reference(path)?;

        debug!(gif_path = %path.display(), "Copied GIF file reference to clipboard");

        Ok(())
    }
}

impl Default for ClipboardManager {
    fn default() -> Self {
        // This will panic if clipboard initialization fails
        // Use `ClipboardManager::new()` for fallible initialization
        Self::new().expect("Failed to initialize default ClipboardManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_clipboard_is_available() {
        // Just check that it doesn't panic
        let _available = ClipboardManager::is_available();
    }

    #[test]
    fn test_clipboard_manager_new() {
        // Should succeed on macOS with proper permissions
        let result = ClipboardManager::new();
        // We don't assert success here since clipboard access may be restricted in CI
        assert!(result.is_ok() || !ClipboardManager::is_available());
    }

    #[test]
    fn test_copy_file_reference_nonexistent() {
        let manager = ClipboardManager::new();
        if manager.is_err() {
            // Skip test if clipboard unavailable
            return;
        }
        let manager = manager.unwrap();

        let result = manager.copy_file_reference(Path::new("/nonexistent/path/file.mp4"));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("does not exist"));
    }

    #[test]
    fn test_copy_file_reference_success() {
        let manager = ClipboardManager::new();
        if manager.is_err() {
            // Skip test if clipboard unavailable
            return;
        }
        let manager = manager.unwrap();

        // Create a temporary file
        let temp_file = tempfile::NamedTempFile::with_suffix(".mp4").unwrap();
        let path = temp_file.path();

        let result = manager.copy_file_reference(path);
        assert!(
            result.is_ok(),
            "Failed to copy file reference: {:?}",
            result
        );
    }

    #[test]
    fn test_copy_image_data_invalid_dimensions() {
        let manager = ClipboardManager::new();
        if manager.is_err() {
            // Skip test if clipboard unavailable
            return;
        }
        let manager = manager.unwrap();

        // Test zero dimensions
        let result = manager.copy_image_data(&[], 0, 100);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("must be greater than 0"));

        // Test mismatched data length
        let result = manager.copy_image_data(&[0u8; 100], 100, 100);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("length mismatch"));
    }

    #[test]
    fn test_copy_image_data_too_large() {
        let manager = ClipboardManager::new();
        if manager.is_err() {
            // Skip test if clipboard unavailable
            return;
        }
        let manager = manager.unwrap();

        // Test dimensions exceeding limit
        let result = manager.copy_image_data(&[], 20000, 100);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("too large"));
    }

    #[test]
    fn test_copy_image_data_success() {
        let manager = ClipboardManager::new();
        if manager.is_err() {
            // Skip test if clipboard unavailable
            return;
        }
        let manager = manager.unwrap();

        // Create a simple 10x10 RGBA image
        let data = vec![255u8; 10 * 10 * 4];
        let result = manager.copy_image_data(&data, 10, 10);
        assert!(result.is_ok(), "Failed to copy image data: {:?}", result);
    }

    #[test]
    fn test_copy_gif_file_nonexistent() {
        let manager = ClipboardManager::new();
        if manager.is_err() {
            // Skip test if clipboard unavailable
            return;
        }
        let manager = manager.unwrap();

        let result = manager.copy_gif_file(Path::new("/nonexistent/path/file.gif"));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("does not exist"));
    }

    #[test]
    fn test_copy_gif_file_success() {
        let manager = ClipboardManager::new();
        if manager.is_err() {
            // Skip test if clipboard unavailable
            return;
        }
        let manager = manager.unwrap();

        // Create a temporary GIF file
        let mut temp_file = tempfile::NamedTempFile::with_suffix(".gif").unwrap();
        // Write a minimal GIF header
        temp_file.write_all(b"GIF89a").unwrap();
        let path = temp_file.path();

        let result = manager.copy_gif_file(path);
        assert!(result.is_ok(), "Failed to copy GIF file: {:?}", result);
    }
}
