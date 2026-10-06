use crate::ui::theme::{BORDER, ELEVATED2, MUTED, TEXT, TEXT2};
use egui::{RichText, Stroke};

#[derive(Debug, Clone, PartialEq)]
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
    let mut refs = std::collections::HashMap::new();
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if let Some(bracket_end) = trimmed.find("]:") {
                let key = trimmed[1..bracket_end].trim().to_ascii_lowercase();
                let url = trimmed[bracket_end + 2..].trim();
                let clean_url = url.split_whitespace().next().unwrap_or(url);
                if !key.is_empty()
                    && (clean_url.starts_with("http://") || clean_url.starts_with("https://"))
                {
                    refs.insert(key, clean_url.to_string());
                }
            }
        }
    }

    let lines: Vec<&str> = markdown.lines().collect();
    let mut blocks = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim();
        if line.is_empty() || is_reference_line(line) || is_ignorable_html_line(line) {
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
                parse_inline_with_refs(line[level..].trim(), &refs),
            ));
            index += 1;
            continue;
        }
        if let Some(h) = parse_html_heading(line, &refs) {
            blocks.push(h);
            index += 1;
            continue;
        }
        if matches!(line, "---" | "***" | "___") || is_html_rule(line) {
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
            blocks.push(Block::Quote(parse_inline_with_refs(&quote, &refs)));
            continue;
        }
        if is_table_line(line) && index + 1 < lines.len() && is_table_separator(lines[index + 1]) {
            let mut rows = vec![parse_table_row(line, &refs)];
            index += 2;
            while index < lines.len() && is_table_line(lines[index].trim()) {
                rows.push(parse_table_row(lines[index], &refs));
                index += 1;
            }
            blocks.push(Block::Table(rows));
            continue;
        }
        if let Some((indent, item)) = list_item(lines[index]) {
            let mut items = vec![(indent, parse_inline_with_refs(item, &refs))];
            index += 1;
            while index < lines.len() {
                if let Some((indent, item)) = list_item(lines[index]) {
                    items.push((indent, parse_inline_with_refs(item, &refs)));
                    index += 1;
                } else {
                    break;
                }
            }
            blocks.push(Block::List(items));
            continue;
        }
        if is_image_line(line) {
            blocks.push(Block::Paragraph(parse_inline_with_refs(line, &refs)));
            index += 1;
            continue;
        }
        let mut paragraph = line.to_owned();
        index += 1;
        while index < lines.len() {
            let next = lines[index].trim();
            if next.is_empty()
                || is_reference_line(next)
                || is_ignorable_html_line(next)
                || is_image_line(next)
                || parse_html_heading(next, &refs).is_some()
                || is_html_rule(next)
                || matches!(next, "---" | "***" | "___")
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
        blocks.push(Block::Paragraph(parse_inline_with_refs(&paragraph, &refs)));
    }
    blocks
}

fn is_reference_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.starts_with('[') {
        if let Some(bracket_end) = trimmed.find("]:") {
            let url = trimmed[bracket_end + 2..].trim();
            let clean_url = url.split_whitespace().next().unwrap_or(url);
            return clean_url.starts_with("http://") || clean_url.starts_with("https://");
        }
    }
    false
}

fn decode_html_entities(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&#160;", " ")
        .replace("&ndash;", "-")
        .replace("&mdash;", "—")
        .replace("&bull;", "•")
        .replace("&copy;", "©")
        .replace("&reg;", "®")
        .replace("&trade;", "™")
        .replace("&#x2F;", "/")
        .replace("&#47;", "/")
        .replace("&#32;", " ")
}

fn strip_html_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    let mut in_quote = None;
    for ch in s.chars() {
        if in_tag {
            if let Some(q) = in_quote {
                if ch == q {
                    in_quote = None;
                }
            } else if ch == '"' || ch == '\'' {
                in_quote = Some(ch);
            } else if ch == '>' {
                in_tag = false;
            }
        } else if ch == '<' {
            in_tag = true;
        } else {
            out.push(ch);
        }
    }
    out
}

fn is_ignorable_html_line(line: &str) -> bool {
    let t = line.trim();
    if !t.starts_with('<') || !t.ends_with('>') {
        return false;
    }
    let stripped = strip_html_tags(t);
    if !stripped.trim().is_empty() {
        return false;
    }
    let inner = t[1..t.len() - 1].trim();
    let name = inner
        .trim_start_matches('/')
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        name.as_str(),
        "center"
            | "details"
            | "summary"
            | "div"
            | "p"
            | "span"
            | "font"
            | "section"
            | "header"
            | "footer"
            | "article"
            | "table"
            | "tbody"
            | "thead"
            | "tr"
            | "td"
            | "th"
            | "br"
            | "hr"
    )
}

fn is_html_rule(line: &str) -> bool {
    let t = line.trim();
    t.eq_ignore_ascii_case("<hr>")
        || t.eq_ignore_ascii_case("<hr/>")
        || t.eq_ignore_ascii_case("<hr />")
}

fn parse_html_heading(
    line: &str,
    refs: &std::collections::HashMap<String, String>,
) -> Option<Block> {
    let t = line.trim();
    if !(t.starts_with("<h") || t.starts_with("<H")) {
        return None;
    }
    let bytes = t.as_bytes();
    if bytes.len() < 4 {
        return None;
    }
    let level_char = bytes[2];
    if !(b'1'..=b'6').contains(&level_char) {
        return None;
    }
    let level = level_char - b'0';
    let gt = t.find('>')?;
    let close_tag = format!("</h{}>", level as char);
    let close_tag_upper = format!("</H{}>", level as char);
    let inner = if let Some(close_pos) = t.find(&close_tag).or_else(|| t.find(&close_tag_upper)) {
        &t[gt + 1..close_pos]
    } else {
        &t[gt + 1..]
    };
    let clean = strip_html_tags(inner);
    Some(Block::Heading(
        level,
        parse_inline_with_refs(clean.trim(), refs),
    ))
}

fn is_image_line(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("![")
        || t.starts_with("<img")
        || t.starts_with("<IMG")
        || (t.starts_with('[') && (t.contains("<img") || t.contains("<IMG")))
        || ((t.starts_with("<a ") || t.starts_with("<A "))
            && (t.contains("<img") || t.contains("<IMG")))
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

fn parse_table_row(line: &str, refs: &std::collections::HashMap<String, String>) -> Vec<Vec<Span>> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| parse_inline_with_refs(cell.trim(), refs))
        .collect()
}

fn extract_attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    for quote in ["\"", "'"] {
        let pattern = format!("{name}={quote}");
        if let Some(pos) = tag.find(&pattern) {
            let start = pos + pattern.len();
            if let Some(len) = tag[start..].find(quote) {
                return Some(&tag[start..start + len]);
            }
        }
    }
    None
}

fn label_from_badge_or_url(img_src: &str, target_url: &str) -> Option<String> {
    for query_key in ["label=", "logo=", "message="] {
        if let Some(pos) = img_src.find(query_key) {
            let val = &img_src[pos + query_key.len()..];
            let end = val.find('&').unwrap_or(val.len());
            let raw = &val[..end];
            let clean = raw.replace(['_', '-'], " ");
            if !clean.trim().is_empty() {
                return Some(clean.trim().to_string());
            }
        }
    }
    let lower_src = img_src.to_ascii_lowercase();
    let lower_target = target_url.to_ascii_lowercase();
    if lower_src.contains("discord") || lower_target.contains("discord") {
        return Some("Discord".to_string());
    }
    if lower_src.contains("kofi") || lower_src.contains("ko-fi") || lower_target.contains("ko-fi") {
        return Some("Ko-fi".to_string());
    }
    if lower_src.contains("patreon") || lower_target.contains("patreon") {
        return Some("Patreon".to_string());
    }
    if lower_src.contains("license") || lower_target.contains("license") {
        return Some("License".to_string());
    }
    if lower_target.contains("curseforge.com") {
        return Some("CurseForge".to_string());
    }
    if lower_target.contains("modrinth.com") {
        return Some("Modrinth".to_string());
    }
    if lower_target.contains("bisecthosting.com") {
        return Some("BisectHosting".to_string());
    }
    if lower_target.contains("github.com") {
        if lower_target.contains("/issues") {
            return Some("Issues".to_string());
        }
        if lower_target.contains("/wiki") {
            return Some("Wiki".to_string());
        }
        return Some("GitHub".to_string());
    }
    None
}

fn extract_link_label(inner: &str, target_url: &str) -> String {
    let trimmed = inner.trim();
    if let Some(start) = trimmed.find("<img") {
        if let Some(end) = trimmed[start..].find('>') {
            let tag = &trimmed[start..start + end + 1];
            if let Some(alt) = extract_attr(tag, "alt") {
                if !alt.trim().is_empty() {
                    return decode_html_entities(alt.trim());
                }
            }
            if let Some(title) = extract_attr(tag, "title") {
                if !title.trim().is_empty() {
                    return decode_html_entities(title.trim());
                }
            }
            let before = &trimmed[..start];
            let after = &trimmed[start + end + 1..];
            let surrounding = format!("{} {}", before.trim(), after.trim());
            let clean = strip_html_tags(&surrounding);
            let decoded = decode_html_entities(&clean);
            if !decoded.trim().is_empty() {
                return decoded.trim().to_string();
            }
            if let Some(src) = extract_attr(tag, "src") {
                if let Some(label) = label_from_badge_or_url(src, target_url) {
                    return label;
                }
            }
        }
    }
    if let Some(after) = trimmed.strip_prefix("![") {
        if let Some(close) = after.find(']') {
            let alt = &after[..close];
            if !alt.trim().is_empty() {
                return decode_html_entities(alt.trim());
            }
        }
    }
    let stripped = strip_html_tags(trimmed);
    let decoded = decode_html_entities(&stripped);
    let clean = decoded.trim();
    if !clean.is_empty() {
        return clean.to_string();
    }
    if let Some(label) = label_from_badge_or_url("", target_url) {
        return label;
    }
    if let Ok(parsed) = url::Url::parse(target_url) {
        if let Some(host) = parsed.host_str() {
            let host_clean = host.strip_prefix("www.").unwrap_or(host);
            return host_clean.to_string();
        }
    }
    "Link".to_string()
}

fn find_matching_close(s: &str, open: char, close: char) -> Option<usize> {
    let mut depth = 0;
    for (idx, ch) in s.char_indices() {
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                return Some(idx);
            }
        }
    }
    None
}

#[cfg(test)]
pub fn parse_inline(input: &str) -> Vec<Span> {
    let refs = std::collections::HashMap::new();
    parse_inline_with_refs(input, &refs)
}

fn parse_inline_with_refs(
    input: &str,
    refs: &std::collections::HashMap<String, String>,
) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut rest = input;
    while !rest.is_empty() {
        if rest.starts_with('\\') && rest.len() > 1 {
            let next_byte = rest.as_bytes()[1];
            if matches!(next_byte, b'<' | b'[' | b'!' | b'*' | b'_' | b'`' | b'#') {
                rest = &rest[1..];
            }
        }
        if rest.starts_with("<img") || rest.starts_with("<IMG") {
            if let Some(gt) = rest.find('>') {
                let tag = &rest[..gt + 1];
                if let Some(src) = extract_attr(tag, "src") {
                    let raw_alt = extract_attr(tag, "alt")
                        .or_else(|| extract_attr(tag, "title"))
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("image"));
                    let alt_label = if let Some(a) = raw_alt {
                        decode_html_entities(a)
                    } else if let Some(badge) = label_from_badge_or_url(src, "") {
                        badge
                    } else {
                        "Image".to_string()
                    };
                    spans.push(Span::Image(alt_label, src.trim().to_string()));
                    rest = &rest[gt + 1..];
                    continue;
                }
            }
        }
        if rest.starts_with("<a ") || rest.starts_with("<A ") {
            if let Some(gt) = rest.find('>') {
                let tag = &rest[..gt + 1];
                if let Some(href) = extract_attr(tag, "href") {
                    let close_opt = rest[gt + 1..]
                        .find("</a>")
                        .or_else(|| rest[gt + 1..].find("</A>"));
                    if let Some(close_a) = close_opt {
                        let inner = &rest[gt + 1..gt + 1 + close_a];
                        let label = extract_link_label(inner, href.trim());
                        spans.push(Span::Link(label, href.trim().to_string()));
                        rest = &rest[gt + 1 + close_a + 4..];
                        continue;
                    }
                }
            }
        }
        if rest.starts_with("<summary>") || rest.starts_with("<SUMMARY>") {
            if let Some(close) = rest.find("</summary>").or_else(|| rest.find("</SUMMARY>")) {
                let title = &rest[9..close];
                spans.push(Span::Text(
                    decode_html_entities(title.trim()),
                    true,
                    false,
                    false,
                ));
                rest = &rest[close + 10..];
                continue;
            }
        }
        if rest.starts_with('<') {
            if let Some(gt) = rest.find('>') {
                let tag_body = rest[1..gt].trim();
                if tag_body.eq_ignore_ascii_case("br")
                    || tag_body.eq_ignore_ascii_case("br/")
                    || tag_body.eq_ignore_ascii_case("br /")
                {
                    spans.push(Span::Text("\n".to_string(), false, false, false));
                }
                rest = &rest[gt + 1..];
                continue;
            }
        }
        if rest.starts_with("![") {
            let after_bang = &rest[1..];
            if let Some(close) = find_matching_close(after_bang, '[', ']') {
                let alt = &after_bang[1..close];
                if after_bang[close..].starts_with("](") {
                    let url_start = close + 2;
                    if let Some(paren_close) = after_bang[url_start..].find(')') {
                        let url = &after_bang[url_start..url_start + paren_close];
                        let clean_alt = decode_html_entities(alt.trim());
                        spans.push(Span::Image(clean_alt, url.trim().to_string()));
                        rest = &after_bang[url_start + paren_close + 1..];
                        continue;
                    }
                }
            }
        }
        if rest.starts_with('[') {
            if let Some(close) = find_matching_close(rest, '[', ']') {
                let inner = &rest[1..close];
                if rest[close..].starts_with("](") {
                    let url_start = close + 2;
                    if let Some(paren_close) = rest[url_start..].find(')') {
                        let url = &rest[url_start..url_start + paren_close];
                        let label = extract_link_label(inner, url.trim());
                        spans.push(Span::Link(label, url.trim().to_string()));
                        rest = &rest[url_start + paren_close + 1..];
                        continue;
                    }
                } else if rest[close..].starts_with("][") {
                    let ref_start = close + 2;
                    if let Some(ref_close) = rest[ref_start..].find(']') {
                        let ref_key = rest[ref_start..ref_start + ref_close]
                            .trim()
                            .to_ascii_lowercase();
                        if let Some(url) = refs.get(&ref_key) {
                            let label = extract_link_label(inner, url);
                            spans.push(Span::Link(label, url.clone()));
                            rest = &rest[ref_start + ref_close + 1..];
                            continue;
                        }
                    }
                } else {
                    let key = inner.trim().to_ascii_lowercase();
                    if let Some(url) = refs.get(&key) {
                        let label = extract_link_label(inner, url);
                        spans.push(Span::Link(label, url.clone()));
                        rest = &rest[close + 1..];
                        continue;
                    }
                }
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
                        let text = if code {
                            after[..end].to_owned()
                        } else {
                            decode_html_entities(&after[..end])
                        };
                        spans.push(Span::Text(text, bold, italic, code));
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
            .find(|(_, ch)| matches!(ch, '<' | '!' | '[' | '*' | '_' | '`' | '\\'))
            .map(|(i, _)| i)
            .unwrap_or(rest.len());
        let text = decode_html_entities(&rest[..next]);
        spans.push(Span::Text(text, false, false, false));
        rest = &rest[next..];
    }
    spans
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
                if url.starts_with("https://") || url.starts_with("http://") {
                    let clean = decode_html_entities(&strip_html_tags(label));
                    let clean_label = clean.trim();
                    let display =
                        if clean_label.is_empty() || clean_label.eq_ignore_ascii_case("image") {
                            "🖼 Image".to_string()
                        } else {
                            format!("🖼 {clean_label}")
                        };
                    if ui
                        .link(display)
                        .on_hover_text("Click to view full image")
                        .clicked()
                    {
                        *selected_image = Some(url.clone());
                    }
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

    #[test]
    fn parses_html_and_reference_links() {
        let doc = "<center>\n[<img src=\"https://example.com/i.png\" alt=\"Logo\"/>](https://example.com)\n</center>\n\nSee [terms][1].\n\n[1]: https://example.com/terms";
        let blocks = parse(doc);
        assert!(!blocks.is_empty());
    }

    #[test]
    fn decodes_html_entities_and_strips_nested_tags() {
        let spans =
            parse_inline("[<span style=\"font-size:24px\">here</span>](https://example.com)");
        assert_eq!(spans.len(), 1);
        if let Span::Link(label, url) = &spans[0] {
            assert_eq!(label, "here");
            assert_eq!(url, "https://example.com");
        } else {
            panic!("expected link span");
        }

        let badge = parse_inline("[<img src=\"https://img.shields.io/discord/12345\" width=\"285\" height=\"40\">](https://discord.gg/test)");
        assert_eq!(badge.len(), 1);
        if let Span::Link(label, _) = &badge[0] {
            assert_eq!(label, "Discord");
        } else {
            panic!("expected link span");
        }

        let text = parse_inline("Installation&nbsp;&nbsp;and&nbsp;&ndash;&nbsp;Setup");
        assert_eq!(
            text[0],
            Span::Text("Installation  and - Setup".to_string(), false, false, false)
        );
    }

    #[test]
    fn parses_html_headings_and_image_blocks() {
        let doc = "<h2 align=\"center\">Installation</h2>\n![Banner](https://example.com/banner.png)\nSome description";
        let blocks = parse(doc);
        assert_eq!(blocks.len(), 3);
        assert!(matches!(blocks[0], Block::Heading(2, _)));
        assert!(matches!(blocks[1], Block::Paragraph(_)));
        assert!(matches!(blocks[2], Block::Paragraph(_)));
    }
}
