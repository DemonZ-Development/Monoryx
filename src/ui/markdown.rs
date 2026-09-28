use crate::ui::theme::{BORDER, ELEVATED2, MUTED, TEXT, TEXT2};
use egui::{RichText, Stroke};

#[derive(Debug, Clone)]
pub enum Span {
    Text(String, bool, bool, bool),
    Link(String, String),
    Image(String, String),
}

#[derive(Debug, Clone)]
pub enum Block {
    Paragraph(Vec<Span>),
    Heading(u8, Vec<Span>),
    List(Vec<(usize, Vec<Span>)>),
    Quote(Vec<Span>),
    Code(String),
    Rule,
    Table(Vec<Vec<Vec<Span>>>),
}

pub fn parse(markdown: &str) -> Vec<Block> {
    let lines: Vec<&str> = markdown.lines().collect();
    let mut blocks = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim();
        if line.is_empty() {
            index += 1;
            continue;
        }
        if line.starts_with("```") || line.starts_with("~~~") {
            let fence = &line[..3];
            index += 1;
            let mut code = String::new();
            while index < lines.len() && !lines[index].trim().starts_with(fence) {
                if !code.is_empty() {
                    code.push('\n');
                }
                code.push_str(lines[index]);
                index += 1;
            }
            blocks.push(Block::Code(code));
            index += 1;
            continue;
        }
        let level = line.bytes().take_while(|byte| *byte == b'#').count();
        if (1..=6).contains(&level) && line.as_bytes().get(level) == Some(&b' ') {
            blocks.push(Block::Heading(
                level as u8,
                parse_inline(line[level..].trim()),
            ));
            index += 1;
            continue;
        }
        if matches!(line, "---" | "***" | "___") {
            blocks.push(Block::Rule);
            index += 1;
            continue;
        }
        if line.starts_with('>') {
            let mut quote = String::new();
            while index < lines.len() && lines[index].trim().starts_with('>') {
                if !quote.is_empty() {
                    quote.push(' ');
                }
                quote.push_str(lines[index].trim().trim_start_matches('>').trim());
                index += 1;
            }
            blocks.push(Block::Quote(parse_inline(&quote)));
            continue;
        }
        if is_table_line(line) && index + 1 < lines.len() && is_table_separator(lines[index + 1]) {
            let mut rows = vec![parse_table_row(line)];
            index += 2;
            while index < lines.len() && is_table_line(lines[index].trim()) {
                rows.push(parse_table_row(lines[index]));
                index += 1;
            }
            blocks.push(Block::Table(rows));
            continue;
        }
        if let Some((indent, item)) = list_item(lines[index]) {
            let mut items = vec![(indent, parse_inline(item))];
            index += 1;
            while index < lines.len() {
                if let Some((indent, item)) = list_item(lines[index]) {
                    items.push((indent, parse_inline(item)));
                    index += 1;
                } else {
                    break;
                }
            }
            blocks.push(Block::List(items));
            continue;
        }
        let mut paragraph = line.to_owned();
        index += 1;
        while index < lines.len() {
            let next = lines[index].trim();
            if next.is_empty()
                || next.starts_with('#')
                || next.starts_with('>')
                || next.starts_with("```")
                || next.starts_with("~~~")
                || list_item(lines[index]).is_some()
                || (is_table_line(next)
                    && index + 1 < lines.len()
                    && is_table_separator(lines[index + 1]))
            {
                break;
            }
            paragraph.push(' ');
            paragraph.push_str(next);
            index += 1;
        }
        blocks.push(Block::Paragraph(parse_inline(&paragraph)));
    }
    blocks
}

fn list_item(line: &str) -> Option<(usize, &str)> {
    let indent = line.len() - line.trim_start().len();
    let text = line.trim_start();
    for marker in ["- ", "* ", "+ "] {
        if let Some(item) = text.strip_prefix(marker) {
            return Some((indent, item));
        }
    }
    let digits = text.bytes().take_while(u8::is_ascii_digit).count();
    if digits > 0
        && text.as_bytes().get(digits) == Some(&b'.')
        && text.as_bytes().get(digits + 1) == Some(&b' ')
    {
        return Some((indent, &text[digits + 2..]));
    }
    None
}

fn is_table_line(line: &str) -> bool {
    line.contains('|')
}

fn is_table_separator(line: &str) -> bool {
    let trimmed = line.trim().trim_matches('|').trim();
    trimmed.contains('-')
        && trimmed
            .chars()
            .all(|ch| matches!(ch, '-' | ':' | '|' | ' '))
}

fn parse_table_row(line: &str) -> Vec<Vec<Span>> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| parse_inline(cell.trim()))
        .collect()
}

fn parse_inline(input: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut rest = input;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("![") {
            if let Some((label, url, consumed)) = parse_link(after) {
                spans.push(Span::Image(label.to_owned(), url.to_owned()));
                rest = &after[consumed..];
                continue;
            }
        }
        if let Some(after) = rest.strip_prefix('[') {
            if let Some((label, url, consumed)) = parse_link(after) {
                spans.push(Span::Link(label.to_owned(), url.to_owned()));
                rest = &after[consumed..];
                continue;
            }
        }
        let mut matched = false;
        for (marker, bold, italic, code) in [
            ("**", true, false, false),
            ("__", true, false, false),
            ("*", false, true, false),
            ("_", false, true, false),
            ("`", false, false, true),
        ] {
            if let Some(after) = rest.strip_prefix(marker) {
                if let Some(end) = after.find(marker) {
                    if end > 0 {
                        spans.push(Span::Text(after[..end].to_owned(), bold, italic, code));
                        rest = &after[end + marker.len()..];
                        matched = true;
                        break;
                    }
                }
            }
        }
        if matched {
            continue;
        }
        let next = rest
            .char_indices()
            .skip(1)
            .find(|(_, ch)| matches!(ch, '!' | '[' | '*' | '_' | '`'))
            .map(|(i, _)| i)
            .unwrap_or(rest.len());
        spans.push(Span::Text(rest[..next].to_owned(), false, false, false));
        rest = &rest[next..];
    }
    spans
}

fn parse_link(after_open: &str) -> Option<(&str, &str, usize)> {
    let close = after_open.find("](")?;
    let url_start = close + 2;
    let url_end = after_open[url_start..].find(')')? + url_start;
    Some((
        &after_open[..close],
        &after_open[url_start..url_end],
        url_end + 1,
    ))
}

pub fn show(ui: &mut egui::Ui, blocks: &[Block]) -> Option<String> {
    let mut selected_image = None;
    for block in blocks {
        match block {
            Block::Paragraph(spans) => draw_inline(ui, spans, 12.5, false, &mut selected_image),
            Block::Heading(level, spans) => {
                let size = match level {
                    1 => 21.0,
                    2 => 18.0,
                    3 => 15.5,
                    _ => 13.5,
                };
                ui.add_space(if *level <= 2 { 10.0 } else { 5.0 });
                draw_inline(ui, spans, size, true, &mut selected_image);
            }
            Block::List(items) => {
                for (indent, spans) in items {
                    ui.horizontal_wrapped(|ui| {
                        ui.add_space((*indent as f32).min(36.0));
                        ui.label(RichText::new("•").color(TEXT2));
                        draw_spans(ui, spans, 12.5, false, &mut selected_image);
                    });
                }
            }
            Block::Quote(spans) => {
                egui::Frame::new()
                    .fill(ELEVATED2)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        draw_inline(ui, spans, 12.5, false, &mut selected_image);
                    });
            }
            Block::Code(code) => {
                egui::Frame::new()
                    .fill(ELEVATED2)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .inner_margin(egui::Margin::symmetric(12, 9))
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(code).monospace().size(12.0).color(TEXT2),
                            )
                            .selectable(true),
                        );
                    });
            }
            Block::Rule => {
                ui.separator();
            }
            Block::Table(rows) => {
                egui::Grid::new(ui.next_auto_id())
                    .striped(true)
                    .min_col_width(90.0)
                    .show(ui, |ui| {
                        for (row_index, row) in rows.iter().enumerate() {
                            for cell in row {
                                draw_inline(ui, cell, 12.0, row_index == 0, &mut selected_image);
                            }
                            ui.end_row();
                        }
                    });
            }
        }
        ui.add_space(5.0);
    }
    selected_image
}

fn draw_inline(
    ui: &mut egui::Ui,
    spans: &[Span],
    size: f32,
    strong: bool,
    selected_image: &mut Option<String>,
) {
    ui.horizontal_wrapped(|ui| draw_spans(ui, spans, size, strong, selected_image));
}

fn draw_spans(
    ui: &mut egui::Ui,
    spans: &[Span],
    size: f32,
    strong: bool,
    selected_image: &mut Option<String>,
) {
    for span in spans {
        match span {
            Span::Text(value, bold, italic, code) => {
                let mut text =
                    RichText::new(value)
                        .size(size)
                        .color(if strong { TEXT } else { TEXT2 });
                if strong || *bold {
                    text = text.strong();
                }
                if *italic {
                    text = text.italics();
                }
                if *code {
                    text = text.monospace().background_color(ELEVATED2);
                }
                ui.add(egui::Label::new(text).selectable(true));
            }
            Span::Link(label, url) => {
                if url.starts_with("https://") || url.starts_with("http://") {
                    ui.hyperlink_to(label, url);
                } else {
                    ui.label(RichText::new(label).size(size).color(MUTED));
                }
            }
            Span::Image(label, url) => {
                if url.starts_with("https://") && ui.link(format!("View image: {label}")).clicked()
                {
                    *selected_image = Some(url.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_project_sections_and_images() {
        let blocks = parse("# Title\n\nSome **bold** text and [site](https://example.com).\n\n- First\n- Second\n\n![Preview](https://example.com/image.png)");
        assert!(matches!(blocks[0], Block::Heading(1, _)));
        assert!(matches!(blocks[2], Block::List(_)));
        assert!(matches!(blocks[3], Block::Paragraph(_)));
        assert!(matches!(
            parse_inline("![Preview](https://example.com/image.png)")[0],
            Span::Image(_, _)
        ));
    }
}
