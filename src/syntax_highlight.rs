use eframe::egui;

use crate::file_content::CodeLanguage;

const CODE_BG: egui::Color32 = egui::Color32::from_rgb(8, 8, 9);
const DEFAULT: egui::Color32 = egui::Color32::from_rgb(238, 238, 239);
const COMMENT: egui::Color32 = egui::Color32::from_rgb(125, 135, 145);
const KEYWORD: egui::Color32 = egui::Color32::from_rgb(220, 150, 255);
const STRING: egui::Color32 = egui::Color32::from_rgb(150, 215, 170);
const NUMBER: egui::Color32 = egui::Color32::from_rgb(245, 185, 125);
const TYPE: egui::Color32 = egui::Color32::from_rgb(125, 195, 255);
const PREPROCESSOR: egui::Color32 = egui::Color32::from_rgb(245, 215, 140);

pub fn show_code(ui: &mut egui::Ui, code: &str, language: CodeLanguage) {
    egui::Frame::default()
        .fill(CODE_BG)
        .inner_margin(8)
        .show(ui, |ui| {
            let job = highlighted_code_job(ui, code, language);
            ui.add(egui::Label::new(job).wrap());
        });
}

pub fn highlighted_code_job(
    ui: &egui::Ui,
    code: &str,
    language: CodeLanguage,
) -> egui::text::LayoutJob {
    highlighted_code_job_with_width(ui.available_width(), code, language)
}

pub fn highlighted_code_job_with_width(
    wrap_width: f32,
    code: &str,
    language: CodeLanguage,
) -> egui::text::LayoutJob {
    if language == CodeLanguage::Makefile {
        return highlighted_makefile_job_with_width(wrap_width, code);
    }

    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = wrap_width;

    let chars: Vec<char> = code.chars().collect();
    let mut i = 0;
    let mut in_block_comment = false;

    while i < chars.len() {
        if in_block_comment {
            let start = i;
            while i < chars.len() && !starts_with(&chars, i, "*/") {
                i += 1;
            }
            if i < chars.len() {
                i += 2;
                in_block_comment = false;
            }
            append_chars(&mut job, &chars[start..i], COMMENT);
            continue;
        }

        if language != CodeLanguage::Python && starts_with(&chars, i, "/*") {
            let start = i;
            i += 2;
            while i < chars.len() && !starts_with(&chars, i, "*/") {
                i += 1;
            }
            if i < chars.len() {
                i += 2;
            } else {
                in_block_comment = true;
            }
            append_chars(&mut job, &chars[start..i], COMMENT);
            continue;
        }

        if language != CodeLanguage::Python && starts_with(&chars, i, "//") {
            let start = i;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            append_chars(&mut job, &chars[start..i], COMMENT);
            continue;
        }

        if language == CodeLanguage::Python && chars[i] == '#' {
            let start = i;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            append_chars(&mut job, &chars[start..i], COMMENT);
            continue;
        }

        if language != CodeLanguage::Python && chars[i] == '#' && is_line_start(&chars, i) {
            let start = i;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            append_chars(&mut job, &chars[start..i], PREPROCESSOR);
            continue;
        }

        if chars[i] == '"' || chars[i] == '\'' {
            let quote = chars[i];
            let start = i;

            if language == CodeLanguage::Python && starts_with_repeated(&chars, i, quote, 3) {
                i += 3;
                while i < chars.len() && !starts_with_repeated(&chars, i, quote, 3) {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                if i < chars.len() {
                    i += 3;
                }
            } else {
                i += 1;
                while i < chars.len() {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 2;
                    } else {
                        let done = chars[i] == quote;
                        i += 1;
                        if done {
                            break;
                        }
                    }
                }
            }

            append_chars(&mut job, &chars[start..i], STRING);
            continue;
        }

        if chars[i].is_ascii_digit() {
            let start = i;
            i += 1;
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric()
                    || matches!(chars[i], '_' | '.' | '+' | '-')
                    || (language == CodeLanguage::Rust && chars[i] == '\''))
            {
                i += 1;
            }
            append_chars(&mut job, &chars[start..i], NUMBER);
            continue;
        }

        if is_identifier_start(chars[i]) {
            let start = i;
            i += 1;
            while i < chars.len() && is_identifier_continue(chars[i]) {
                i += 1;
            }

            let word: String = chars[start..i].iter().collect();
            let color = if is_keyword(&word, language) {
                KEYWORD
            } else if is_type_or_builtin(&word, language) {
                TYPE
            } else {
                DEFAULT
            };
            append_text(&mut job, &word, color);
            continue;
        }

        append_text(&mut job, &chars[i].to_string(), DEFAULT);
        i += 1;
    }

    job
}

fn append_chars(job: &mut egui::text::LayoutJob, chars: &[char], color: egui::Color32) {
    let text: String = chars.iter().collect();
    append_text(job, &text, color);
}

fn highlighted_makefile_job_with_width(wrap_width: f32, code: &str) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = wrap_width;

    for line in code.split_inclusive('\n') {
        append_makefile_line(&mut job, line);
    }

    job
}

fn append_makefile_line(job: &mut egui::text::LayoutJob, line: &str) {
    let line_without_newline = line.strip_suffix('\n').unwrap_or(line);
    let newline = if line.ends_with('\n') { "\n" } else { "" };
    let trimmed_start = line_without_newline.trim_start();

    if trimmed_start.starts_with('#') {
        append_text(job, line_without_newline, COMMENT);
        append_text(job, newline, DEFAULT);
        return;
    }

    if line_without_newline.starts_with('\t') {
        append_makefile_recipe(job, line_without_newline);
        append_text(job, newline, DEFAULT);
        return;
    }

    if let Some(comment_index) = find_makefile_comment(line_without_newline) {
        append_makefile_declaration(job, &line_without_newline[..comment_index]);
        append_text(job, &line_without_newline[comment_index..], COMMENT);
    } else {
        append_makefile_declaration(job, line_without_newline);
    }
    append_text(job, newline, DEFAULT);
}

fn append_makefile_declaration(job: &mut egui::text::LayoutJob, line: &str) {
    if let Some((keyword, rest)) = split_makefile_directive(line) {
        let leading_spaces = line.len() - line.trim_start().len();
        append_text(job, &line[..leading_spaces], DEFAULT);
        append_text(job, keyword, KEYWORD);
        append_makefile_inline(job, rest, DEFAULT);
        return;
    }

    if let Some(operator) = find_makefile_assignment(line) {
        append_makefile_inline(job, &line[..operator], KEYWORD);
        let operator_end = operator + makefile_assignment_operator_len(&line[operator..]);
        append_text(job, &line[operator..operator_end], PREPROCESSOR);
        append_makefile_inline(job, &line[operator_end..], DEFAULT);
        return;
    }

    if let Some(colon) = find_makefile_target_colon(line) {
        append_makefile_inline(job, &line[..colon], KEYWORD);
        append_text(job, ":", PREPROCESSOR);
        append_makefile_inline(job, &line[colon + 1..], DEFAULT);
        return;
    }

    append_makefile_inline(job, line, DEFAULT);
}

fn split_makefile_directive(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_start();
    let keyword_end = trimmed
        .char_indices()
        .find(|(_, char)| char.is_ascii_whitespace())
        .map(|(index, _)| index)
        .unwrap_or(trimmed.len());
    let keyword = &trimmed[..keyword_end];

    if MAKEFILE_KEYWORDS.contains(&keyword) {
        Some((keyword, &trimmed[keyword_end..]))
    } else {
        None
    }
}

fn append_makefile_recipe(job: &mut egui::text::LayoutJob, line: &str) {
    let command_start = line.chars().take_while(|char| *char == '\t').count();
    append_text(job, &line[..command_start], DEFAULT);

    let rest = &line[command_start..];
    if let Some(first_word_end) = rest
        .char_indices()
        .find(|(_, char)| char.is_ascii_whitespace())
        .map(|(index, _)| index)
    {
        append_text(job, &rest[..first_word_end], TYPE);
        append_makefile_inline(job, &rest[first_word_end..], DEFAULT);
    } else {
        append_text(job, rest, TYPE);
    }
}

fn append_makefile_inline(job: &mut egui::text::LayoutJob, text: &str, base_color: egui::Color32) {
    let mut index = 0;
    while index < text.len() {
        let rest = &text[index..];
        if let Some(variable_len) = makefile_variable_len(rest) {
            append_text(job, &rest[..variable_len], TYPE);
            index += variable_len;
            continue;
        }

        let Some((offset, char)) = rest.char_indices().next() else {
            break;
        };
        let char_len = char.len_utf8();
        append_text(job, &rest[offset..offset + char_len], base_color);
        index += char_len;
    }
}

fn find_makefile_comment(line: &str) -> Option<usize> {
    let mut escaped = false;
    for (index, char) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if char == '\\' {
            escaped = true;
            continue;
        }
        if char == '#' {
            return Some(index);
        }
    }
    None
}

fn find_makefile_assignment(line: &str) -> Option<usize> {
    [":=", "?=", "+=", "!="]
        .iter()
        .filter_map(|operator| line.find(operator))
        .chain(line.find('='))
        .min()
}

fn makefile_assignment_operator_len(text: &str) -> usize {
    [":=", "?=", "+=", "!="]
        .iter()
        .find(|operator| text.starts_with(**operator))
        .map_or(1, |operator| operator.len())
}

fn find_makefile_target_colon(line: &str) -> Option<usize> {
    let colon = line.find(':')?;
    if colon == 0
        || line[..colon]
            .chars()
            .any(|char| char == '=' || char == '\t')
    {
        return None;
    }
    Some(colon)
}

fn makefile_variable_len(text: &str) -> Option<usize> {
    if !text.starts_with('$') {
        return None;
    }

    let mut chars = text.char_indices();
    chars.next();
    match chars.next() {
        Some((_, '(')) => text.find(')').map(|index| index + 1),
        Some((_, '{')) => text.find('}').map(|index| index + 1),
        Some((index, char)) if matches!(char, '@' | '<' | '^' | '?' | '*' | '+' | '%' | '|') => {
            Some(index + char.len_utf8())
        }
        Some((index, char)) if char.is_ascii_alphanumeric() || char == '_' => {
            Some(index + char.len_utf8())
        }
        _ => None,
    }
}

fn append_text(job: &mut egui::text::LayoutJob, text: &str, color: egui::Color32) {
    job.append(
        text,
        0.0,
        egui::text::TextFormat {
            font_id: egui::FontId::monospace(13.0),
            color,
            ..Default::default()
        },
    );
}

fn starts_with(chars: &[char], index: usize, needle: &str) -> bool {
    needle
        .chars()
        .enumerate()
        .all(|(offset, char)| chars.get(index + offset) == Some(&char))
}

fn starts_with_repeated(chars: &[char], index: usize, quote: char, count: usize) -> bool {
    (0..count).all(|offset| chars.get(index + offset) == Some(&quote))
}

fn is_line_start(chars: &[char], index: usize) -> bool {
    chars[..index]
        .iter()
        .rev()
        .take_while(|char| **char != '\n')
        .all(|char| char.is_ascii_whitespace())
}

fn is_identifier_start(char: char) -> bool {
    char == '_' || char.is_ascii_alphabetic()
}

fn is_identifier_continue(char: char) -> bool {
    char == '_' || char.is_ascii_alphanumeric()
}

fn is_keyword(word: &str, language: CodeLanguage) -> bool {
    let keywords = match language {
        CodeLanguage::Python => PYTHON_KEYWORDS,
        CodeLanguage::Rust => RUST_KEYWORDS,
        CodeLanguage::C => C_KEYWORDS,
        CodeLanguage::Cpp => CPP_KEYWORDS,
        CodeLanguage::Makefile => MAKEFILE_KEYWORDS,
    };

    keywords.contains(&word)
}

fn is_type_or_builtin(word: &str, language: CodeLanguage) -> bool {
    let words = match language {
        CodeLanguage::Python => PYTHON_BUILTINS,
        CodeLanguage::Rust => RUST_TYPES,
        CodeLanguage::C => C_TYPES,
        CodeLanguage::Cpp => CPP_TYPES,
        CodeLanguage::Makefile => MAKEFILE_BUILTINS,
    };

    words.contains(&word)
}

const PYTHON_KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
    "with", "yield",
];

const PYTHON_BUILTINS: &[&str] = &[
    "bool",
    "bytes",
    "dict",
    "enumerate",
    "float",
    "int",
    "len",
    "list",
    "object",
    "open",
    "print",
    "range",
    "set",
    "str",
    "super",
    "tuple",
    "type",
    "zip",
];

const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while",
];

const RUST_TYPES: &[&str] = &[
    "Box", "Option", "Result", "String", "Vec", "bool", "char", "f32", "f64", "i8", "i16", "i32",
    "i64", "i128", "isize", "str", "u8", "u16", "u32", "u64", "u128", "usize",
];

const C_KEYWORDS: &[&str] = &[
    "auto", "break", "case", "const", "continue", "default", "do", "else", "enum", "extern", "for",
    "goto", "if", "inline", "register", "restrict", "return", "sizeof", "static", "struct",
    "switch", "typedef", "union", "volatile", "while",
];

const C_TYPES: &[&str] = &[
    "FILE", "bool", "char", "double", "float", "int", "int16_t", "int32_t", "int64_t", "int8_t",
    "long", "short", "size_t", "ssize_t", "uint16_t", "uint32_t", "uint64_t", "uint8_t",
    "unsigned", "void",
];

const CPP_KEYWORDS: &[&str] = &[
    "alignas",
    "alignof",
    "and",
    "asm",
    "auto",
    "break",
    "case",
    "catch",
    "class",
    "concept",
    "const",
    "consteval",
    "constexpr",
    "continue",
    "decltype",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "explicit",
    "export",
    "extern",
    "false",
    "for",
    "friend",
    "if",
    "inline",
    "namespace",
    "new",
    "noexcept",
    "nullptr",
    "operator",
    "private",
    "protected",
    "public",
    "requires",
    "return",
    "sizeof",
    "static",
    "struct",
    "switch",
    "template",
    "this",
    "throw",
    "true",
    "try",
    "typedef",
    "typename",
    "union",
    "using",
    "virtual",
    "volatile",
    "while",
];

const CPP_TYPES: &[&str] = &[
    "bool", "char", "double", "float", "int", "int16_t", "int32_t", "int64_t", "int8_t", "long",
    "short", "size_t", "std", "string", "uint16_t", "uint32_t", "uint64_t", "uint8_t", "unsigned",
    "vector", "void",
];

const MAKEFILE_KEYWORDS: &[&str] = &[
    "define", "else", "endef", "endif", "export", "if", "ifdef", "ifeq", "ifndef", "ifneq",
    "include", "override", "private", "sinclude", "undefine", "unexport", "vpath",
];

const MAKEFILE_BUILTINS: &[&str] = &[
    "abspath",
    "addprefix",
    "addsuffix",
    "basename",
    "call",
    "dir",
    "error",
    "eval",
    "filter",
    "filter-out",
    "findstring",
    "firstword",
    "foreach",
    "if",
    "join",
    "lastword",
    "notdir",
    "or",
    "patsubst",
    "realpath",
    "shell",
    "sort",
    "strip",
    "subst",
    "suffix",
    "value",
    "warning",
    "wildcard",
    "word",
    "wordlist",
    "words",
];
