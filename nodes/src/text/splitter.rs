use super::types::{
    LineNumberInjectionConfig, TextSegment, TextSplitConfig, TextSplitMode, TextSplitResult,
    TextSplitRule,
};

fn compute_pad_width(cfg: &LineNumberInjectionConfig, total_lines: usize) -> usize {
    cfg.pad_width
        .unwrap_or_else(|| total_lines.to_string().len().max(1))
}

fn render_line_number(cfg: &LineNumberInjectionConfig, line_no: usize, pad_width: usize) -> String {
    let raw = line_no.to_string();
    let padded = if raw.len() >= pad_width {
        raw
    } else {
        let mut out = String::with_capacity(pad_width);
        for _ in 0..(pad_width - raw.len()) {
            out.push(cfg.pad_char);
        }
        out.push_str(&raw);
        out
    };
    cfg.template.replace("{line}", &padded)
}

fn is_heading_line(line: &str, rule: &TextSplitRule) -> bool {
    match rule {
        TextSplitRule::HeadingKeywords {
            keywords,
            require_prefix,
        } => {
            let trimmed = line.trim();
            if let Some(prefix) = require_prefix.as_deref() {
                if !trimmed.starts_with(prefix) {
                    return false;
                }
            }
            keywords.iter().any(|kw| trimmed.contains(kw))
        }
    }
}

fn finalize_segment(
    index: usize,
    lines: &[(usize, &str)],
    cfg: &TextSplitConfig,
    pad_width: usize,
) -> Option<TextSegment> {
    if lines.is_empty() {
        return None;
    }
    let start_line = lines.first().map(|(n, _)| *n).unwrap_or(0);
    let end_line = lines.last().map(|(n, _)| *n).unwrap_or(0);

    let mut out_lines = Vec::with_capacity(lines.len());
    for (line_no, line) in lines {
        if cfg.line_numbers.enabled {
            let prefix = render_line_number(&cfg.line_numbers, *line_no, pad_width);
            out_lines.push(format!("{prefix}{line}"));
        } else {
            out_lines.push((*line).to_string());
        }
    }

    Some(TextSegment {
        index,
        start_line,
        end_line,
        text: out_lines.join("\n"),
    })
}

pub fn split_text(text: &str, cfg: &TextSplitConfig) -> TextSplitResult {
    let original_lines: Vec<&str> = text.lines().collect();
    let total_lines = original_lines.len();

    let mut indexed: Vec<(usize, &str)> = original_lines
        .iter()
        .enumerate()
        .map(|(i, line)| (i + 1, *line))
        .collect();

    if cfg.remove_empty_lines {
        indexed.retain(|(_, line)| !line.trim().is_empty());
    }

    let pad_width = compute_pad_width(&cfg.line_numbers, total_lines);

    let mut segments = Vec::new();
    match &cfg.mode {
        TextSplitMode::ByLineCount { max_lines_per_part } => {
            let max_lines_per_part = (*max_lines_per_part).max(1);
            let mut idx = 1usize;
            let mut start = 0usize;
            while start < indexed.len() {
                let end = (start + max_lines_per_part).min(indexed.len());
                if let Some(seg) = finalize_segment(idx, &indexed[start..end], cfg, pad_width) {
                    segments.push(seg);
                    idx += 1;
                }
                start = end;
            }
        }
        TextSplitMode::ByRule { rule } => {
            if cfg.rule_only_keep_matched_ranges {
                let mut current: Vec<(usize, &str)> = Vec::new();
                let mut idx = 1usize;
                let mut collecting = false;

                for (line_no, line) in indexed {
                    if is_heading_line(line, rule) {
                        if collecting {
                            if let Some(seg) = finalize_segment(idx, &current, cfg, pad_width) {
                                segments.push(seg);
                                idx += 1;
                            }
                            current.clear();
                        }
                        collecting = true;
                    }

                    if collecting {
                        current.push((line_no, line));
                    }
                }

                if collecting {
                    if let Some(seg) = finalize_segment(idx, &current, cfg, pad_width) {
                        segments.push(seg);
                    }
                }
            } else {
                let mut current: Vec<(usize, &str)> = Vec::new();
                let mut idx = 1usize;
                for (line_no, line) in indexed {
                    if is_heading_line(line, rule) && !current.is_empty() {
                        if let Some(seg) = finalize_segment(idx, &current, cfg, pad_width) {
                            segments.push(seg);
                            idx += 1;
                        }
                        current.clear();
                    }
                    current.push((line_no, line));
                }
                if let Some(seg) = finalize_segment(idx, &current, cfg, pad_width) {
                    segments.push(seg);
                }
            }
        }
    }

    TextSplitResult {
        total_lines,
        segments,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_by_line_count_keeps_original_line_numbers() {
        let text = "a\nb\nc\nd\ne\n";
        let cfg = TextSplitConfig {
            mode: TextSplitMode::ByLineCount {
                max_lines_per_part: 2,
            },
            remove_empty_lines: false,
            line_numbers: LineNumberInjectionConfig {
                enabled: true,
                template: "{line}|".to_string(),
                pad_width: Some(2),
                pad_char: '0',
            },
            rule_only_keep_matched_ranges: false,
        };
        let out = split_text(text, &cfg);
        assert_eq!(out.total_lines, 5);
        assert_eq!(out.segments.len(), 3);
        assert_eq!(out.segments[0].start_line, 1);
        assert_eq!(out.segments[0].end_line, 2);
        assert_eq!(out.segments[0].text, "01|a\n02|b");
        assert_eq!(out.segments[1].text, "03|c\n04|d");
        assert_eq!(out.segments[2].text, "05|e");
    }

    #[test]
    fn remove_empty_lines_does_not_reset_line_numbers() {
        let text = "a\n\nb\n\nc\n";
        let cfg = TextSplitConfig {
            mode: TextSplitMode::ByLineCount {
                max_lines_per_part: 10,
            },
            remove_empty_lines: true,
            line_numbers: LineNumberInjectionConfig {
                enabled: true,
                template: "{line}: ".to_string(),
                pad_width: None,
                pad_char: '0',
            },
            rule_only_keep_matched_ranges: false,
        };
        let out = split_text(text, &cfg);
        assert_eq!(out.total_lines, 5);
        assert_eq!(out.segments.len(), 1);
        assert_eq!(out.segments[0].text, "1: a\n3: b\n5: c");
    }
    #[test]
    fn rule_only_keep_matched_ranges_drops_preface() {
        let text = "";
        let cfg = TextSplitConfig {
            mode: TextSplitMode::ByRule {
                rule: TextSplitRule::HeadingKeywords {
                    require_prefix: Some("第".to_string()),
                    keywords: vec!["集".to_string()],
                },
            },
            remove_empty_lines: true,
            line_numbers: LineNumberInjectionConfig {
                enabled: true,
                template: "[L{line}] ".to_string(),
                pad_width: None,
                pad_char: '0',
            },
            rule_only_keep_matched_ranges: true,
        };
        let out = split_text(text, &cfg);
        for segment in out.segments {
            println!("{}", segment.text);
            println!("-----------------------------------")
        }
    }

    #[test]
    fn split_by_heading_keywords() {
        let text = "序\n第1章 开始\na\nb\n第2章 继续\nc\n";
        let cfg = TextSplitConfig {
            mode: TextSplitMode::ByRule {
                rule: TextSplitRule::HeadingKeywords {
                    keywords: vec!["章".to_string()],
                    require_prefix: Some("第".to_string()),
                },
            },
            remove_empty_lines: false,
            line_numbers: LineNumberInjectionConfig::default(),
            rule_only_keep_matched_ranges: false,
        };

        let out = split_text(text, &cfg);
        assert_eq!(out.segments.len(), 3);
        assert_eq!(out.segments[0].text, "序");
        assert_eq!(out.segments[1].text, "第1章 开始\na\nb");
        assert_eq!(out.segments[2].text, "第2章 继续\nc");
        assert_eq!(out.segments[2].start_line, 5);
        assert_eq!(out.segments[2].end_line, 6);
    }
}
