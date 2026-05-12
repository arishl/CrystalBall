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
    };

    keywords.contains(&word)
}

fn is_type_or_builtin(word: &str, language: CodeLanguage) -> bool {
    let words = match language {
        CodeLanguage::Python => PYTHON_BUILTINS,
        CodeLanguage::Rust => RUST_TYPES,
        CodeLanguage::C => C_TYPES,
        CodeLanguage::Cpp => CPP_TYPES,
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
