use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone)]
pub struct AttachmentRef {
    pub hash: Vec<u8>,
    pub blake3: bool,
    pub name: String,
    pub is_image: bool,
}

#[derive(Debug, Clone)]
pub struct MarkdownAnalysis {
    pub sanitized: String,
    pub plain: String,
    pub has_body: bool,
    pub attachments: Vec<AttachmentRef>,
}

#[derive(Clone, Copy)]
enum CaptureKind {
    AttachmentImage,
    AttachmentLink,
    OtherImage,
}

struct Capture {
    kind: CaptureKind,
    hash: Option<Vec<u8>>,
    blake3: bool,
    start: usize,
    text: String,
}

struct Edit {
    start: usize,
    end: usize,
    replacement: String,
}

pub fn analyze(markdown: &str) -> MarkdownAnalysis {
    let parser = Parser::new_ext(markdown, Options::ENABLE_STRIKETHROUGH);
    let mut capture: Option<Capture> = None;
    let mut edits = Vec::new();
    let mut attachments = Vec::new();
    let mut plain = String::new();

    for (event, range) in parser.into_offset_iter() {
        if let Some(current) = capture.as_mut() {
            match event {
                Event::Text(text) => {
                    current.text.push_str(&text);
                    plain.push_str(&text);
                }
                Event::Code(text) => {
                    current.text.push_str(&text);
                    plain.push_str(&text);
                }
                Event::SoftBreak | Event::HardBreak => {
                    current.text.push(' ');
                    plain.push('\n');
                }
                Event::End(TagEnd::Image)
                    if matches!(
                        current.kind,
                        CaptureKind::AttachmentImage | CaptureKind::OtherImage
                    ) =>
                {
                    let finished = capture.take().expect("capture disappeared");
                    let replacement = match finished.kind {
                        CaptureKind::AttachmentImage => String::new(),
                        CaptureKind::OtherImage => {
                            let alt = finished.text.trim();
                            if alt.is_empty() {
                                "图片".to_string()
                            } else {
                                alt.to_string()
                            }
                        }
                        CaptureKind::AttachmentLink => unreachable!(),
                    };
                    edits.push(Edit {
                        start: finished.start,
                        end: range.end,
                        replacement,
                    });
                    if matches!(finished.kind, CaptureKind::AttachmentImage)
                        && let Some(hash) = finished.hash
                    {
                        attachments.push(AttachmentRef {
                            hash,
                            blake3: finished.blake3,
                            name: attachment_name(&finished.text, "图片"),
                            is_image: true,
                        });
                    }
                }
                Event::End(TagEnd::Link) if matches!(current.kind, CaptureKind::AttachmentLink) => {
                    let finished = capture.take().expect("capture disappeared");
                    edits.push(Edit {
                        start: finished.start,
                        end: range.end,
                        replacement: String::new(),
                    });
                    if let Some(hash) = finished.hash {
                        attachments.push(AttachmentRef {
                            hash,
                            blake3: finished.blake3,
                            name: attachment_name(&finished.text, "附件"),
                            is_image: false,
                        });
                    }
                }
                _ => {}
            }
            continue;
        }

        match event {
            Event::Start(Tag::Image { dest_url, .. }) => {
                let attachment = attachment_id(&dest_url);
                let kind = if attachment.is_some() {
                    CaptureKind::AttachmentImage
                } else {
                    CaptureKind::OtherImage
                };
                capture = Some(Capture {
                    kind,
                    hash: attachment.as_ref().map(|(hash, _)| hash.clone()),
                    blake3: attachment.is_some_and(|(_, blake3)| blake3),
                    start: range.start,
                    text: String::new(),
                });
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                if let Some((hash, blake3)) = attachment_id(&dest_url) {
                    capture = Some(Capture {
                        kind: CaptureKind::AttachmentLink,
                        hash: Some(hash),
                        blake3,
                        start: range.start,
                        text: String::new(),
                    });
                }
            }
            Event::Text(text) => plain.push_str(&text),
            Event::Code(text) => plain.push_str(&text),
            Event::SoftBreak | Event::HardBreak => plain.push('\n'),
            Event::End(
                TagEnd::Paragraph
                | TagEnd::Heading(_)
                | TagEnd::BlockQuote(_)
                | TagEnd::CodeBlock
                | TagEnd::List(_)
                | TagEnd::Item
                | TagEnd::Table
                | TagEnd::TableHead
                | TagEnd::TableRow
                | TagEnd::TableCell,
            ) => plain.push('\n'),
            _ => {}
        }
    }

    let sanitized = cleanup(&apply_edits(markdown, edits));
    let has_body = !sanitized.trim().is_empty();
    MarkdownAnalysis {
        sanitized,
        plain,
        has_body,
        attachments,
    }
}

pub fn attachment_refs(markdown: &str) -> Vec<AttachmentRef> {
    analyze(markdown).attachments
}

pub fn preview(markdown: &str) -> String {
    let parser = Parser::new_ext(markdown, Options::ENABLE_STRIKETHROUGH);
    let mut text = String::new();
    let mut image_alt = String::new();
    let mut link_text = String::new();
    let mut in_image = false;
    let mut in_attachment_link = false;

    for event in parser {
        match event {
            Event::Start(Tag::Image { .. }) => {
                in_image = true;
                image_alt.clear();
            }
            Event::Start(Tag::Link { dest_url, .. }) if attachment_hash(&dest_url).is_some() => {
                in_attachment_link = true;
                link_text.clear();
            }
            Event::End(TagEnd::Image) => {
                in_image = false;
                push_space(&mut text);
                text.push_str(if image_alt.is_empty() {
                    "图片"
                } else {
                    &image_alt
                });
            }
            Event::End(TagEnd::Link) if in_attachment_link => {
                in_attachment_link = false;
                push_space(&mut text);
                text.push_str(if link_text.is_empty() {
                    "附件"
                } else {
                    &link_text
                });
            }
            Event::End(
                TagEnd::Paragraph
                | TagEnd::Heading(_)
                | TagEnd::BlockQuote(_)
                | TagEnd::CodeBlock
                | TagEnd::List(_)
                | TagEnd::Item
                | TagEnd::Table
                | TagEnd::TableHead
                | TagEnd::TableRow
                | TagEnd::TableCell,
            ) => push_space(&mut text),
            Event::Text(value) if in_image => image_alt.push_str(&value),
            Event::Text(value) if in_attachment_link => link_text.push_str(&value),
            Event::Text(value) => text.push_str(&value),
            Event::Code(value) => text.push_str(&value),
            Event::SoftBreak | Event::HardBreak => push_space(&mut text),
            _ => {}
        }
    }

    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn attachment_hash(value: &str) -> Option<Vec<u8>> {
    attachment_id(value)
        .filter(|(_, blake3)| !blake3)
        .map(|(hash, _)| hash)
}

fn attachment_id(value: &str) -> Option<(Vec<u8>, bool)> {
    let (value, blake3) = if let Some(value) = value.strip_prefix("attachment://") {
        (value, false)
    } else if let Some(value) = value.strip_prefix("blob://") {
        (value, true)
    } else {
        return None;
    };
    let bytes = hex::decode(value).ok()?;
    (bytes.len() == 32).then_some((bytes, blake3))
}

fn attachment_name(value: &str, fallback: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

fn apply_edits(markdown: &str, mut edits: Vec<Edit>) -> String {
    edits.sort_by_key(|edit| edit.start);
    let mut result = String::with_capacity(markdown.len());
    let mut cursor = 0;
    for edit in edits {
        if edit.start < cursor {
            continue;
        }
        result.push_str(&markdown[cursor..edit.start]);
        result.push_str(&edit.replacement);
        cursor = edit.end;
    }
    result.push_str(&markdown[cursor..]);
    result
}

fn cleanup(markdown: &str) -> String {
    markdown
        .lines()
        .filter(|line| !is_attachment_definition(line) && !is_block_marker_only(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_attachment_definition(line: &str) -> bool {
    let line = line.trim_start();
    let Some(close) = line.find("]:") else {
        return false;
    };
    let target = line[close + 2..].trim_start();
    line.starts_with('[') && (target.starts_with("attachment://") || target.starts_with("blob://"))
}

fn is_block_marker_only(line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() {
        return false;
    }
    if line.chars().all(|character| character == '#') {
        return true;
    }
    if matches!(line, "-" | "*" | "+" | ">") {
        return true;
    }
    let digits = line
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .count();
    digits > 0 && matches!(&line[digits..], "." | ")")
}

fn push_space(text: &mut String) {
    if !text.is_empty() && !text.ends_with(' ') {
        text.push(' ');
    }
}
