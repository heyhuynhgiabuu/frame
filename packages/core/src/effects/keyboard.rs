//! Keyboard event capture for shortcut display
//!
//! Uses macOS CGEventTap to capture system-wide keyboard events.
//! Requires Accessibility permissions.

use crate::effects::{Key, KeyboardEvent, Modifiers, NamedKey};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Maximum number of events to buffer
const MAX_EVENT_BUFFER: usize = 100;

/// Duration to keep events in buffer
const EVENT_RETENTION: Duration = Duration::from_secs(5);

/// Keyboard event buffer for processing
#[derive(Debug)]
pub struct KeyboardBuffer {
    events: VecDeque<KeyboardEvent>,
    /// Currently held modifiers
    modifiers: Modifiers,
    /// Current key being held (if any)
    held_key: Option<Key>,
    /// Time when held key was pressed
    held_key_time: Option<Duration>,
}

impl Default for KeyboardBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyboardBuffer {
    pub fn new() -> Self {
        Self {
            events: VecDeque::with_capacity(MAX_EVENT_BUFFER),
            modifiers: Modifiers::default(),
            held_key: None,
            held_key_time: None,
        }
    }

    /// Add an event to the buffer
    pub fn push(&mut self, event: KeyboardEvent) {
        // Update modifier state
        if event.pressed {
            self.modifiers = event.modifiers;
            if !matches!(event.key, Key::Named(NamedKey::Space)) {
                // Don't track space as "held" typically
                self.held_key = Some(event.key.clone());
                self.held_key_time = Some(event.timestamp);
            }
        } else {
            // Key released
            if self.held_key.as_ref() == Some(&event.key) {
                self.held_key = None;
                self.held_key_time = None;
            }
        }

        // Add to buffer
        if self.events.len() >= MAX_EVENT_BUFFER {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    /// Get events within a time window
    pub fn events_since(&self, since: Duration) -> impl Iterator<Item = &KeyboardEvent> {
        self.events.iter().filter(move |e| e.timestamp >= since)
    }

    /// Get the most recent key combination for display
    pub fn current_combo(&self, current_time: Duration) -> Option<String> {
        // Only show if modifiers are active or a key is being held recently
        if !self.modifiers.any_active() && self.held_key.is_none() {
            return None;
        }

        let mut combo = self.modifiers.display();

        if let Some(key) = &self.held_key {
            if let Some(held_time) = self.held_key_time {
                // Only show held key if pressed recently (within 2 seconds)
                if current_time.saturating_sub(held_time) < Duration::from_secs(2) {
                    combo.push_str(&key.display());
                }
            }
        }

        if combo.is_empty() {
            None
        } else {
            Some(combo)
        }
    }

    /// Clear old events and reset state
    pub fn cleanup(&mut self, current_time: Duration) {
        // Remove events older than retention period
        while self
            .events
            .front()
            .map(|e| current_time.saturating_sub(e.timestamp) > EVENT_RETENTION)
            .unwrap_or(false)
        {
            self.events.pop_front();
        }
    }

    /// Reset buffer state
    pub fn reset(&mut self) {
        self.events.clear();
        self.modifiers = Modifiers::default();
        self.held_key = None;
        self.held_key_time = None;
    }

    /// Get current modifiers
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }
}

/// Convert macOS key code to our Key type
pub fn keycode_to_key(keycode: u16) -> Option<Key> {
    // macOS virtual key codes
    // Reference: https://developer.apple.com/documentation/coregraphics/cgkeycode
    match keycode {
        // Letters (QWERTY layout)
        0 => Some(Key::Character('a')),
        1 => Some(Key::Character('s')),
        2 => Some(Key::Character('d')),
        3 => Some(Key::Character('f')),
        4 => Some(Key::Character('h')),
        5 => Some(Key::Character('g')),
        6 => Some(Key::Character('z')),
        7 => Some(Key::Character('x')),
        8 => Some(Key::Character('c')),
        9 => Some(Key::Character('v')),
        11 => Some(Key::Character('b')),
        12 => Some(Key::Character('q')),
        13 => Some(Key::Character('w')),
        14 => Some(Key::Character('e')),
        15 => Some(Key::Character('r')),
        16 => Some(Key::Character('y')),
        17 => Some(Key::Character('t')),
        31 => Some(Key::Character('o')),
        32 => Some(Key::Character('u')),
        34 => Some(Key::Character('i')),
        35 => Some(Key::Character('p')),
        37 => Some(Key::Character('l')),
        38 => Some(Key::Character('j')),
        40 => Some(Key::Character('k')),
        45 => Some(Key::Character('n')),
        46 => Some(Key::Character('m')),

        // Numbers
        18 => Some(Key::Character('1')),
        19 => Some(Key::Character('2')),
        20 => Some(Key::Character('3')),
        21 => Some(Key::Character('4')),
        22 => Some(Key::Character('6')),
        23 => Some(Key::Character('5')),
        25 => Some(Key::Character('9')),
        26 => Some(Key::Character('7')),
        28 => Some(Key::Character('8')),
        29 => Some(Key::Character('0')),

        // Special keys
        36 => Some(Key::Named(NamedKey::Return)),
        48 => Some(Key::Named(NamedKey::Tab)),
        49 => Some(Key::Named(NamedKey::Space)),
        51 => Some(Key::Named(NamedKey::Backspace)),
        53 => Some(Key::Named(NamedKey::Escape)),
        117 => Some(Key::Named(NamedKey::Delete)),
        115 => Some(Key::Named(NamedKey::Home)),
        119 => Some(Key::Named(NamedKey::End)),
        116 => Some(Key::Named(NamedKey::PageUp)),
        121 => Some(Key::Named(NamedKey::PageDown)),

        // Arrow keys
        123 => Some(Key::Named(NamedKey::ArrowLeft)),
        124 => Some(Key::Named(NamedKey::ArrowRight)),
        125 => Some(Key::Named(NamedKey::ArrowDown)),
        126 => Some(Key::Named(NamedKey::ArrowUp)),

        // Function keys
        122 => Some(Key::Named(NamedKey::F1)),
        120 => Some(Key::Named(NamedKey::F2)),
        99 => Some(Key::Named(NamedKey::F3)),
        118 => Some(Key::Named(NamedKey::F4)),
        96 => Some(Key::Named(NamedKey::F5)),
        97 => Some(Key::Named(NamedKey::F6)),
        98 => Some(Key::Named(NamedKey::F7)),
        100 => Some(Key::Named(NamedKey::F8)),
        101 => Some(Key::Named(NamedKey::F9)),
        109 => Some(Key::Named(NamedKey::F10)),
        103 => Some(Key::Named(NamedKey::F11)),
        111 => Some(Key::Named(NamedKey::F12)),

        _ => None,
    }
}

/// Convert macOS modifier flags to our Modifiers type
pub fn flags_to_modifiers(flags: u64) -> Modifiers {
    // CGEventFlags bit masks
    const COMMAND: u64 = 0x100000; // kCGEventFlagMaskCommand
    const SHIFT: u64 = 0x20000; // kCGEventFlagMaskShift
    const OPTION: u64 = 0x80000; // kCGEventFlagMaskAlternate
    const CONTROL: u64 = 0x40000; // kCGEventFlagMaskControl

    Modifiers {
        command: (flags & COMMAND) != 0,
        shift: (flags & SHIFT) != 0,
        option: (flags & OPTION) != 0,
        control: (flags & CONTROL) != 0,
    }
}

/// Platform-specific keyboard capture
///
/// On macOS, this uses CGEventTap which requires Accessibility permissions.
/// The actual CGEventTap implementation is in the desktop app (apps/desktop)
/// since it requires platform-specific code and permissions handling.
///
/// This module provides the buffer and conversion utilities.
#[derive(Debug)]
pub struct KeyboardCapture {
    buffer: KeyboardBuffer,
    enabled: Arc<AtomicBool>,
}

impl Default for KeyboardCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyboardCapture {
    pub fn new() -> Self {
        Self {
            buffer: KeyboardBuffer::new(),
            enabled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if capture is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Enable/disable capture
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Get enabled flag for sharing with event handler
    pub fn enabled_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.enabled)
    }

    /// Process a keyboard event from the platform layer
    pub fn process_event(&mut self, event: KeyboardEvent) {
        if self.is_enabled() {
            self.buffer.push(event);
        }
    }

    /// Get current key combination for display
    pub fn current_combo(&self, current_time: Duration) -> Option<String> {
        self.buffer.current_combo(current_time)
    }

    /// Perform cleanup
    pub fn cleanup(&mut self, current_time: Duration) {
        self.buffer.cleanup(current_time);
    }

    /// Reset capture state
    pub fn reset(&mut self) {
        self.buffer.reset();
    }

    /// Get reference to buffer for advanced queries
    pub fn buffer(&self) -> &KeyboardBuffer {
        &self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyboard_buffer() {
        let mut buffer = KeyboardBuffer::new();

        // Simulate Cmd+S
        buffer.push(KeyboardEvent {
            key: Key::Character('s'),
            modifiers: Modifiers {
                command: true,
                ..Default::default()
            },
            timestamp: Duration::from_millis(100),
            pressed: true,
        });

        let combo = buffer.current_combo(Duration::from_millis(200));
        assert_eq!(combo, Some("⌘S".to_string()));
    }

    #[test]
    fn test_keycode_conversion() {
        assert_eq!(keycode_to_key(0), Some(Key::Character('a')));
        assert_eq!(keycode_to_key(36), Some(Key::Named(NamedKey::Return)));
        assert_eq!(keycode_to_key(126), Some(Key::Named(NamedKey::ArrowUp)));
    }

    #[test]
    fn test_modifier_flags() {
        // Command + Shift
        let mods = flags_to_modifiers(0x100000 | 0x20000);
        assert!(mods.command);
        assert!(mods.shift);
        assert!(!mods.option);
        assert!(!mods.control);
    }

    #[test]
    fn test_capture_enable() {
        let capture = KeyboardCapture::new();
        assert!(!capture.is_enabled());

        capture.set_enabled(true);
        assert!(capture.is_enabled());
    }
}
