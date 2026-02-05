//! Error dialog component for displaying errors with recovery options

use frame_core::{ErrorContext, ErrorSeverity, FrameError, RecoveryAction};
use iced::{
    widget::{button, column, container, row, text, Space},
    Alignment, Element, Length,
};

/// Message type for error dialog interactions
#[derive(Debug, Clone)]
pub enum ErrorDialogMessage {
    /// Close the error dialog
    Close,
    /// Retry the failed operation
    Retry,
    /// Open system settings
    OpenSettings,
    /// Request permissions again
    RequestPermissions,
    /// Ignore the error and continue
    Ignore,
}

/// Error dialog for displaying errors with recovery options
#[derive(Debug, Clone, Default)]
pub struct ErrorDialog {
    pub is_open: bool,
    pub context: Option<ErrorContext>,
}

impl ErrorDialog {
    /// Create a new error dialog
    pub fn new() -> Self {
        Self {
            is_open: false,
            context: None,
        }
    }

    /// Open the dialog with an error
    pub fn open(&mut self, error: FrameError) {
        self.is_open = true;
        self.context = Some(error.into_context());
    }

    /// Close the dialog
    pub fn close(&mut self) {
        self.is_open = false;
        self.context = None;
    }

    /// Check if the dialog is open
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Get the current error context
    pub fn context(&self) -> Option<&ErrorContext> {
        self.context.as_ref()
    }

    /// Get the title based on severity
    fn title(&self) -> &str {
        if let Some(ctx) = &self.context {
            match ctx.severity {
                ErrorSeverity::Info => "Information",
                ErrorSeverity::Warning => "Warning",
                ErrorSeverity::Error => "Error",
                ErrorSeverity::Critical => "Critical Error",
            }
        } else {
            "Error"
        }
    }

    /// Get the icon/color based on severity
    fn severity_color(&self) -> iced::Color {
        if let Some(ctx) = &self.context {
            match ctx.severity {
                ErrorSeverity::Info => iced::Color::from_rgb(0.2, 0.6, 1.0),
                ErrorSeverity::Warning => iced::Color::from_rgb(1.0, 0.8, 0.2),
                ErrorSeverity::Error => iced::Color::from_rgb(1.0, 0.3, 0.3),
                ErrorSeverity::Critical => iced::Color::from_rgb(0.9, 0.1, 0.1),
            }
        } else {
            iced::Color::from_rgb(1.0, 0.3, 0.3)
        }
    }

    /// Update the dialog state
    pub fn update(&mut self, message: ErrorDialogMessage) -> Option<RecoveryAction> {
        match message {
            ErrorDialogMessage::Close => {
                self.close();
                None
            }
            ErrorDialogMessage::Retry => {
                self.close();
                self.context.as_ref().and_then(|ctx| ctx.recovery.clone())
            }
            ErrorDialogMessage::OpenSettings => {
                self.close();
                Some(RecoveryAction::OpenSettings)
            }
            ErrorDialogMessage::RequestPermissions => {
                self.close();
                Some(RecoveryAction::RequestPermissions)
            }
            ErrorDialogMessage::Ignore => {
                self.close();
                Some(RecoveryAction::Ignore)
            }
        }
    }

    /// Render the error dialog
    pub fn view(&self) -> Element<ErrorDialogMessage> {
        if !self.is_open {
            return Space::new(Length::Shrink, Length::Shrink).into();
        }

        let ctx = match &self.context {
            Some(ctx) => ctx,
            None => return Space::new(Length::Shrink, Length::Shrink).into(),
        };

        // Error message
        let error_message = text(&format!("{}", ctx.error)).size(16);

        // Severity indicator - simple colored bar
        let severity_indicator = container(Space::new(Length::Fixed(4.0), Length::Fill));

        // Title
        let title = text(self.title()).size(20);

        // Suggested action
        let action_text = ctx
            .action
            .as_ref()
            .map(|action| text(action).size(14))
            .map(Element::from)
            .unwrap_or_else(|| Space::new(Length::Shrink, Length::Shrink).into());

        // Action buttons based on recovery option
        let mut buttons = row![];

        if let Some(recovery) = &ctx.recovery {
            match recovery {
                RecoveryAction::Retry => {
                    buttons = buttons.push(
                        button("Retry")
                            .on_press(ErrorDialogMessage::Retry)
                            .style(iced::theme::Button::Primary),
                    );
                }
                RecoveryAction::RequestPermissions => {
                    buttons = buttons.push(
                        button("Grant Permissions")
                            .on_press(ErrorDialogMessage::RequestPermissions)
                            .style(iced::theme::Button::Primary),
                    );
                }
                RecoveryAction::OpenSettings => {
                    buttons = buttons.push(
                        button("Open Settings")
                            .on_press(ErrorDialogMessage::OpenSettings)
                            .style(iced::theme::Button::Primary),
                    );
                }
                _ => {}
            }
        }

        // Always add close and ignore buttons
        buttons = buttons.push(button("Close").on_press(ErrorDialogMessage::Close));

        if ctx.severity != ErrorSeverity::Critical {
            buttons = buttons.push(
                button("Ignore")
                    .on_press(ErrorDialogMessage::Ignore)
                    .style(iced::theme::Button::Secondary),
            );
        }

        let content = column![
            row![severity_indicator, title,].spacing(10),
            error_message,
            action_text,
            Space::new(Length::Fill, Length::Fixed(20.0)),
            buttons.spacing(10),
        ]
        .spacing(15)
        .align_items(Alignment::Start);

        container(content)
            .width(Length::Fixed(500.0))
            .padding(20)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_dialog() {
        let mut dialog = ErrorDialog::new();
        assert!(!dialog.is_open());

        dialog.open(FrameError::Cancelled);
        assert!(dialog.is_open());
        assert_eq!(dialog.title(), "Information");

        dialog.close();
        assert!(!dialog.is_open());
    }

    #[test]
    fn test_severity_colors() {
        let mut dialog = ErrorDialog::new();

        dialog.open(FrameError::Cancelled);
        assert_eq!(dialog.title(), "Information");

        dialog.open(FrameError::Io("test".to_string()));
        assert_eq!(dialog.title(), "Error");
    }
}
