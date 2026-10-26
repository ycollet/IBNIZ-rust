use std::ops::Range;

use eframe::egui::{self, Color32, FontId, Stroke, TextFormat, text::LayoutJob};
use ibniz::CompileError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Token {
    Plain,
    Comment,
    Number,
    DataMode,
    Operator,
    Control,
    Stack,
    Media,
    Invalid,
}

fn tokens(source: &str) -> Vec<(Range<usize>, Token)> {
    let mut tokens: Vec<(Range<usize>, Token)> = Vec::new();
    let mut comment = false;
    let mut data = false;
    for (offset, ch) in source.char_indices() {
        let token = if comment {
            comment = ch != '\n';
            Token::Comment
        } else if ch == '\\' {
            comment = true;
            Token::Comment
        } else if ch.is_whitespace() || ch == ',' {
            Token::Plain
        } else if data {
            match ch {
                'b' | 'q' | 'o' | 'h' => Token::DataMode,
                '0'..='9' | 'A'..='F' => Token::Number,
                _ => Token::Invalid,
            }
        } else {
            match ch {
                '$' => {
                    data = true;
                    Token::DataMode
                }
                '0'..='9' | 'A'..='F' | '.' => Token::Number,
                '+' | '-' | '*' | '/' | '%' | 'q' | '&' | '|' | '^' | 'r' | 'l' | '~' | 's'
                | 'a' | '<' | '>' | '=' => Token::Operator,
                '?' | ':' | ';' | 'i' | 'j' | 'X' | '[' | 'L' | ']' | 'J' | '{' | '}' | 'V' => {
                    Token::Control
                }
                'd' | 'p' | 'x' | 'v' | ')' | '(' | 'z' | 'R' | 'P' => Token::Stack,
                'M' | 'w' | 'T' | '@' | '!' | 'U' | 'G' => Token::Media,
                _ => Token::Invalid,
            }
        };
        let end = offset + ch.len_utf8();
        if let Some((range, previous)) = tokens.last_mut()
            && *previous == token
        {
            range.end = end;
        } else {
            tokens.push((offset..end, token));
        }
    }
    tokens
}

pub fn location(source: &str, offset: usize) -> (usize, usize) {
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|&byte| byte == b'\n').count() + 1;
    let column = prefix
        .rsplit('\n')
        .next()
        .unwrap_or_default()
        .chars()
        .count()
        + 1;
    (line, column)
}

pub fn highlight(
    source: &str,
    visuals: &egui::Visuals,
    font: FontId,
    error: Option<&CompileError>,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    let error_range = error.and_then(|error| {
        let ch = source.get(error.offset..)?.chars().next()?;
        Some(error.offset..error.offset + ch.len_utf8())
    });
    for (range, token) in tokens(source) {
        let color = match (token, visuals.dark_mode) {
            (Token::Plain, _) => visuals.text_color(),
            (Token::Invalid, _) => visuals.error_fg_color,
            (Token::Comment, _) => visuals.weak_text_color(),
            (Token::Number, true) => Color32::from_rgb(130, 190, 255),
            (Token::Number, false) => Color32::from_rgb(20, 75, 170),
            (Token::DataMode, true) => Color32::from_rgb(240, 190, 100),
            (Token::DataMode, false) => Color32::from_rgb(140, 85, 0),
            (Token::Operator, _) => visuals.text_color(),
            (Token::Control, true) => Color32::from_rgb(215, 150, 245),
            (Token::Control, false) => Color32::from_rgb(135, 35, 155),
            (Token::Stack, true) => Color32::from_rgb(125, 215, 170),
            (Token::Stack, false) => Color32::from_rgb(0, 105, 65),
            (Token::Media, true) => Color32::from_rgb(245, 155, 125),
            (Token::Media, false) => Color32::from_rgb(160, 55, 25),
        };
        let format = TextFormat {
            font_id: font.clone(),
            color,
            ..Default::default()
        };
        if let Some(error) = &error_range
            && range.contains(&error.start)
        {
            job.append(&source[range.start..error.start], 0.0, format.clone());
            let mut marked = format.clone();
            marked.underline = Stroke::new(1.5_f32, visuals.error_fg_color);
            marked.background = visuals.error_fg_color.gamma_multiply(0.15);
            job.append(&source[error.clone()], 0.0, marked);
            job.append(&source[error.end..range.end], 0.0, format);
        } else {
            job.append(&source[range], 0.0, format);
        }
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlighting_matches_compiler_modes_and_preserves_all_source_bytes() {
        let source = "1.F d?M:xp; \\ $ invalid \u{e9}\n$ b101 q23 o7 hAF \\ comment\n";
        let spans = tokens(source);
        assert!(!spans.iter().any(|(_, token)| *token == Token::Invalid));
        assert_eq!(
            spans
                .iter()
                .map(|(range, _)| &source[range.clone()])
                .collect::<String>(),
            source
        );
        assert!(
            spans
                .iter()
                .any(|(range, token)| *token == Token::DataMode && &source[range.clone()] == "$")
        );
        assert_eq!(tokens("$ .")[2].1, Token::Invalid);
        assert_eq!(tokens("b")[0].1, Token::Invalid);
        for example in super::super::examples::EXAMPLES {
            assert!(
                !tokens(example.source)
                    .iter()
                    .any(|(_, token)| *token == Token::Invalid),
                "{}",
                example.filename
            );
        }
    }

    #[test]
    fn diagnostics_use_unicode_columns_and_mark_only_the_offending_character() {
        let source = "\\ \u{e9}\n1 \u{2603} 2";
        let error = ibniz::Program::compile(source).unwrap_err();
        assert_eq!(location(source, error.offset), (2, 3));
        for visuals in [egui::Visuals::dark(), egui::Visuals::light()] {
            let job = highlight(source, &visuals, FontId::monospace(14.0), Some(&error));
            assert_eq!(job.text, source);
            let marked: Vec<_> = job
                .sections
                .iter()
                .filter(|section| section.format.underline.width > 0.0)
                .collect();
            assert_eq!(marked.len(), 1);
            assert_eq!(&job.text[marked[0].byte_range.clone()], "\u{2603}");
        }
    }
}
