use slint::StyledText;

pub use markdown::{MarkdownAnalysis, analyze, preview};

pub fn body_from_analysis(analysis: &MarkdownAnalysis) -> StyledText {
    if analysis.has_body {
        StyledText::from_markdown(&analysis.sanitized)
            .unwrap_or_else(|_| StyledText::from_plain_text(&analysis.plain))
    } else {
        StyledText::from_plain_text(&analysis.plain)
    }
}
