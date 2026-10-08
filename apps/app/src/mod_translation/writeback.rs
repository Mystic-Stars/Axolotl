use std::collections::BTreeMap;

use crate::mod_translation::error::{
    Result, TranslateError, TranslateErrorCode,
};

/// 自由文本按行对齐用的内部键：/lines/000000。
pub fn localized_line_key(index: usize) -> String {
    format!("/lines/{index:06}")
}

pub fn localized_line_index(key: &str) -> Option<usize> {
    let rest = key.strip_prefix("/lines/")?;
    rest.parse::<usize>()
        .ok()
        .filter(|index| *index < 10_000_000)
}

/// 自由文本文件的字节级布局快照。
#[derive(Debug, Clone)]
pub struct FreeTextSnapshot {
    pub bom: bool,
    pub eol: String,
    pub trailing_newline: bool,
    pub lines: Vec<String>,
}

impl FreeTextSnapshot {
    pub fn parse(content: &str) -> Self {
        let body = content.strip_prefix('\u{feff}').unwrap_or(content);
        let bom = body.len() != content.len();
        let eol = if body.contains("\r\n") { "\r\n" } else { "\n" };
        let trailing_newline = body.ends_with("\r\n") || body.ends_with('\n');
        let mut lines: Vec<String> = body
            .split("\r\n")
            .flat_map(|part| part.split('\n'))
            .map(str::to_string)
            .collect();
        if trailing_newline {
            lines.pop();
        }
        Self {
            bom,
            eol: eol.to_string(),
            trailing_newline,
            lines,
        }
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        if self.bom {
            out.push('\u{feff}');
        }
        out.push_str(&self.lines.join(&self.eol));
        if self.trailing_newline {
            out.push_str(&self.eol);
        }
        out
    }
}

/// 保序 JSON 值，结构化资源写回时用（serde_json 默认 Map 是排序的，会打乱键序）。
#[derive(Debug, Clone)]
pub enum JsonValue {
    Object(Vec<(String, JsonValue)>),
    Array(Vec<JsonValue>),
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

impl JsonValue {
    pub fn parse(content: &str) -> Result<Self> {
        let mut parser = JsonParser {
            bytes: content.as_bytes(),
            position: 0,
        };
        let value = parser.parse_value()?;
        parser.skip_whitespace();
        if parser.position != content.len() {
            return Err(TranslateError::new(
                TranslateErrorCode::InvalidArchive,
                "JSON document has trailing content",
            ));
        }
        Ok(value)
    }

    /// 按 JSON pointer 定位并写入，路径不存在就报错。
    pub fn set_pointer(
        &mut self,
        pointer: &str,
        translation: String,
    ) -> Result<()> {
        if !pointer.starts_with('/') {
            return Err(TranslateError::new(
                TranslateErrorCode::Config,
                format!("structured pointer must start with '/': {pointer}"),
            ));
        }
        if pointer == "/" {
            if let JsonValue::String(value) = self {
                *value = translation;
                return Ok(());
            }
            return Err(TranslateError::new(
                TranslateErrorCode::Config,
                "structured pointer targets a non-string root",
            ));
        }
        let segments = pointer
            .split('/')
            .skip(1)
            .map(decode_json_pointer_segment)
            .collect::<Vec<_>>();
        let mut cursor = self;
        for (index, segment) in segments.iter().enumerate() {
            let last = index == segments.len() - 1;
            let child = match cursor {
                JsonValue::Object(entries) => {
                    if let Some(position) =
                        entries.iter().position(|(key, _)| key == segment)
                    {
                        if last {
                            entries[position].1 =
                                JsonValue::String(translation);
                            return Ok(());
                        }
                        &mut entries[position].1
                    } else {
                        return Err(TranslateError::new(
                            TranslateErrorCode::Config,
                            format!(
                                "structured pointer segment not found: {segment}"
                            ),
                        ));
                    }
                }
                JsonValue::Array(items) => {
                    let position = segment.parse::<usize>().map_err(|_| {
                        TranslateError::new(
                            TranslateErrorCode::Config,
                            format!("structured pointer segment is not an index: {segment}"),
                        )
                    })?;
                    if position >= items.len() {
                        return Err(TranslateError::new(
                            TranslateErrorCode::Config,
                            format!(
                                "structured pointer index out of range: {segment}"
                            ),
                        ));
                    }
                    if last {
                        items[position] = JsonValue::String(translation);
                        return Ok(());
                    }
                    &mut items[position]
                }
                _ => {
                    return Err(TranslateError::new(
                        TranslateErrorCode::Config,
                        format!(
                            "structured pointer traverses a scalar: {segment}"
                        ),
                    ));
                }
            };
            cursor = child;
        }
        Err(TranslateError::new(
            TranslateErrorCode::Config,
            format!(
                "structured pointer did not resolve to a string: {pointer}"
            ),
        ))
    }

    /// 两空格缩进序列化。
    pub fn render_pretty(&self) -> String {
        let mut out = String::new();
        self.write_pretty(&mut out, 0);
        out.push('\n');
        out
    }

    fn write_pretty(&self, out: &mut String, indent: usize) {
        match self {
            JsonValue::Object(entries) => {
                if entries.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push_str("{\n");
                for (index, (key, value)) in entries.iter().enumerate() {
                    out.push_str(&"  ".repeat(indent + 1));
                    out.push_str(&json_escape(key));
                    out.push_str(": ");
                    value.write_pretty(out, indent + 1);
                    if index + 1 < entries.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&"  ".repeat(indent));
                out.push('}');
            }
            JsonValue::Array(items) => {
                if items.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push_str("[\n");
                for (index, value) in items.iter().enumerate() {
                    out.push_str(&"  ".repeat(indent + 1));
                    value.write_pretty(out, indent + 1);
                    if index + 1 < items.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&"  ".repeat(indent));
                out.push(']');
            }
            JsonValue::String(value) => out.push_str(&json_escape(value)),
            JsonValue::Number(value) => {
                if value.fract() == 0.0 && value.abs() < 9_007_199_254_740_992.0
                {
                    out.push_str(&format!("{}", *value as i64));
                } else {
                    out.push_str(&value.to_string());
                }
            }
            JsonValue::Bool(value) => {
                out.push_str(if *value { "true" } else { "false" })
            }
            JsonValue::Null => out.push_str("null"),
        }
    }
}

fn decode_json_pointer_segment(value: &str) -> String {
    value.replace("~1", "/").replace("~0", "~")
}

fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            character if (character as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => out.push(character),
        }
    }
    out.push('"');
    out
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> JsonParser<'a> {
    fn skip_whitespace(&mut self) {
        while self.position < self.bytes.len()
            && matches!(self.bytes[self.position], b' ' | b'\t' | b'\n' | b'\r')
        {
            self.position += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn parse_value(&mut self) -> Result<JsonValue> {
        self.skip_whitespace();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => Ok(JsonValue::String(self.parse_string()?)),
            Some(b't') => {
                self.expect_literal("true")?;
                Ok(JsonValue::Bool(true))
            }
            Some(b'f') => {
                self.expect_literal("false")?;
                Ok(JsonValue::Bool(false))
            }
            Some(b'n') => {
                self.expect_literal("null")?;
                Ok(JsonValue::Null)
            }
            Some(_) => self.parse_number(),
            None => Err(TranslateError::new(
                TranslateErrorCode::InvalidArchive,
                "JSON document ended unexpectedly",
            )),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue> {
        self.position += 1; // {
        let mut entries = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.position += 1;
            return Ok(JsonValue::Object(entries));
        }
        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'"') {
                return Err(TranslateError::new(
                    TranslateErrorCode::InvalidArchive,
                    "JSON object key must be a string",
                ));
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.peek() != Some(b':') {
                return Err(TranslateError::new(
                    TranslateErrorCode::InvalidArchive,
                    "JSON object key is missing a colon",
                ));
            }
            self.position += 1;
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.position += 1;
                }
                Some(b'}') => {
                    self.position += 1;
                    return Ok(JsonValue::Object(entries));
                }
                _ => {
                    return Err(TranslateError::new(
                        TranslateErrorCode::InvalidArchive,
                        "JSON object is missing a closing brace",
                    ));
                }
            }
        }
    }

    fn parse_array(&mut self) -> Result<JsonValue> {
        self.position += 1; // [
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.position += 1;
            return Ok(JsonValue::Array(items));
        }
        loop {
            items.push(self.parse_value()?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.position += 1;
                }
                Some(b']') => {
                    self.position += 1;
                    return Ok(JsonValue::Array(items));
                }
                _ => {
                    return Err(TranslateError::new(
                        TranslateErrorCode::InvalidArchive,
                        "JSON array is missing a closing bracket",
                    ));
                }
            }
        }
    }

    fn parse_string(&mut self) -> Result<String> {
        self.skip_whitespace();
        if self.peek() != Some(b'"') {
            return Err(TranslateError::new(
                TranslateErrorCode::InvalidArchive,
                "JSON string is missing an opening quote",
            ));
        }
        self.position += 1;
        let mut out = String::new();
        loop {
            let Some(byte) = self.bytes.get(self.position).copied() else {
                return Err(TranslateError::new(
                    TranslateErrorCode::InvalidArchive,
                    "JSON string is unterminated",
                ));
            };
            self.position += 1;
            match byte {
                b'"' => return Ok(out),
                b'\\' => {
                    let Some(escape) = self.bytes.get(self.position).copied()
                    else {
                        return Err(TranslateError::new(
                            TranslateErrorCode::InvalidArchive,
                            "JSON string escape is unterminated",
                        ));
                    };
                    self.position += 1;
                    match escape {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            if self.position + 4 > self.bytes.len() {
                                return Err(TranslateError::new(
                                    TranslateErrorCode::InvalidArchive,
                                    "JSON unicode escape is truncated",
                                ));
                            }
                            let hex = std::str::from_utf8(
                                &self.bytes[self.position..self.position + 4],
                            )
                            .map_err(|_| {
                                TranslateError::new(
                                    TranslateErrorCode::InvalidArchive,
                                    "JSON unicode escape is invalid",
                                )
                            })?;
                            let code =
                                u16::from_str_radix(hex, 16).map_err(|_| {
                                    TranslateError::new(
                                        TranslateErrorCode::InvalidArchive,
                                        "JSON unicode escape is invalid",
                                    )
                                })?;
                            self.position += 4;
                            out.push(
                                char::from_u32(code as u32)
                                    .unwrap_or('\u{fffd}'),
                            );
                        }
                        _ => {
                            return Err(TranslateError::new(
                                TranslateErrorCode::InvalidArchive,
                                format!(
                                    "JSON string contains an unknown escape: \\{}",
                                    escape as char
                                ),
                            ));
                        }
                    }
                }
                byte if byte < 0x20 => {
                    return Err(TranslateError::new(
                        TranslateErrorCode::InvalidArchive,
                        "JSON string contains a control character",
                    ));
                }
                _ => {
                    // Preserve UTF-8 sequences byte by byte.
                    let length = utf8_sequence_length(byte);
                    if self.position + length - 1 > self.bytes.len() {
                        return Err(TranslateError::new(
                            TranslateErrorCode::InvalidArchive,
                            "JSON string contains truncated UTF-8",
                        ));
                    }
                    let slice = &self.bytes
                        [self.position - 1..self.position - 1 + length];
                    let text = std::str::from_utf8(slice).map_err(|_| {
                        TranslateError::new(
                            TranslateErrorCode::InvalidArchive,
                            "JSON string contains invalid UTF-8",
                        )
                    })?;
                    out.push_str(text);
                    self.position += length - 1;
                }
            }
        }
    }

    fn parse_number(&mut self) -> Result<JsonValue> {
        let start = self.position;
        while self.position < self.bytes.len()
            && matches!(
                self.bytes[self.position],
                b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E'
            )
        {
            self.position += 1;
        }
        if self.position == start {
            return Err(TranslateError::new(
                TranslateErrorCode::InvalidArchive,
                "JSON contains an invalid token",
            ));
        }
        let text = std::str::from_utf8(&self.bytes[start..self.position])
            .map_err(|_| {
                TranslateError::new(
                    TranslateErrorCode::InvalidArchive,
                    "invalid JSON number",
                )
            })?;
        let value = text.parse::<f64>().map_err(|_| {
            TranslateError::new(
                TranslateErrorCode::InvalidArchive,
                "invalid JSON number",
            )
        })?;
        Ok(JsonValue::Number(value))
    }

    fn expect_literal(&mut self, literal: &str) -> Result<()> {
        if self.position + literal.len() > self.bytes.len()
            || &self.bytes[self.position..self.position + literal.len()]
                != literal.as_bytes()
        {
            return Err(TranslateError::new(
                TranslateErrorCode::InvalidArchive,
                format!("JSON expected `{literal}`"),
            ));
        }
        self.position += literal.len();
        Ok(())
    }
}

fn utf8_sequence_length(first: u8) -> usize {
    if first < 0x80 {
        1
    } else if first >> 5 == 0b110 {
        2
    } else if first >> 4 == 0b1110 {
        3
    } else if first >> 3 == 0b11110 {
        4
    } else {
        1
    }
}

/// Reads the current target entries for a language source from raw content.
pub fn read_language_target(
    content: &str,
    source: &crate::mod_translation::analyze::LanguageSource,
) -> BTreeMap<String, String> {
    use crate::mod_translation::analyze::LanguageKind;
    match source.kind {
        LanguageKind::Json if source.is_structured_json() => {
            read_json_structured(content)
        }
        LanguageKind::Json => read_json_flat(content),
        LanguageKind::KeyValue => read_key_value(content),
        LanguageKind::FreeText => {
            if let Some(layout) = &source.localized_layout {
                let snapshot = FreeTextSnapshot::parse(content);
                read_localized_target_entries(&snapshot, layout)
            } else {
                let mut map = BTreeMap::new();
                if content.trim().is_empty() {
                    map
                } else {
                    map.insert("/".to_string(), content.to_string());
                    map
                }
            }
        }
    }
}

fn read_json_structured(content: &str) -> BTreeMap<String, String> {
    let mut entries = BTreeMap::new();
    if let Ok(root) = JsonValue::parse(content) {
        flatten_json_strings(&root, "", &mut entries);
    }
    entries
}

fn flatten_json_strings(
    value: &JsonValue,
    pointer: &str,
    out: &mut BTreeMap<String, String>,
) {
    match value {
        JsonValue::String(text) => {
            out.insert(
                if pointer.is_empty() { "/" } else { pointer }.to_string(),
                text.clone(),
            );
        }
        JsonValue::Object(entries) => {
            for (key, value) in entries {
                let escaped = key.replace('~', "~0").replace('/', "~1");
                flatten_json_strings(
                    value,
                    &format!("{pointer}/{escaped}"),
                    out,
                );
            }
        }
        JsonValue::Array(items) => {
            for (index, value) in items.iter().enumerate() {
                flatten_json_strings(value, &format!("{pointer}/{index}"), out);
            }
        }
        JsonValue::Number(_) | JsonValue::Bool(_) | JsonValue::Null => {}
    }
}

pub fn read_json_flat(content: &str) -> BTreeMap<String, String> {
    match JsonValue::parse(content) {
        Ok(JsonValue::Object(entries)) => entries
            .into_iter()
            .filter_map(|(key, value)| match value {
                JsonValue::String(text) => Some((key, text)),
                _ => None,
            })
            .collect(),
        _ => BTreeMap::new(),
    }
}

pub fn read_key_value(content: &str) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    for raw_line in content.split("\r\n").flat_map(|part| part.split('\n')) {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }
        let separator = find_unquoted_separator(line);
        let Some(separator) = separator else { continue };
        if separator < 1 {
            continue;
        }
        let key = line[..separator].trim().to_string();
        let value = line[separator + 1..].trim().to_string();
        if !key.is_empty() {
            result.insert(key, value);
        }
    }
    result
}

/// Finds the first `=` or `:` that is not escaped by a backslash.
fn find_unquoted_separator(line: &str) -> Option<usize> {
    let mut previous_escape = false;
    for (index, character) in line.char_indices() {
        if character == '\\' {
            previous_escape = !previous_escape;
            continue;
        }
        if (character == '=' || character == ':') && !previous_escape {
            return Some(index);
        }
        previous_escape = false;
    }
    None
}

/// Serialises language target content for a source .
pub fn serialize_language_target(
    source: &crate::mod_translation::analyze::LanguageSource,
    entries: &BTreeMap<String, String>,
) -> Result<String> {
    use crate::mod_translation::analyze::LanguageKind;
    match source.kind {
        LanguageKind::FreeText => {
            Ok(serialize_localized_target(entries, source))
        }
        LanguageKind::Json if source.is_structured_json() => {
            let template =
                source.structured_template.as_deref().ok_or_else(|| {
                    TranslateError::new(
                        TranslateErrorCode::Config,
                        format!(
                            "structured template is unavailable for {}",
                            source.source_path
                        ),
                    )
                })?;
            let mut root = JsonValue::parse(template)?;
            for (pointer, translation) in entries {
                root.set_pointer(pointer, translation.clone())?;
            }
            Ok(root.render_pretty())
        }
        LanguageKind::Json => Ok(serialize_json_flat(entries)),
        LanguageKind::KeyValue => {
            let mut out = String::new();
            for (key, value) in entries {
                out.push_str(key);
                out.push('=');
                out.push_str(value);
                out.push('\n');
            }
            Ok(out)
        }
    }
}

pub fn serialize_json_flat(entries: &BTreeMap<String, String>) -> String {
    let mut out = String::from("{\n");
    let entries = entries.iter().collect::<Vec<_>>();
    for (index, (key, value)) in entries.iter().enumerate() {
        out.push_str("  ");
        out.push_str(&json_escape(key));
        out.push_str(": ");
        out.push_str(&json_escape(value));
        if index + 1 < entries.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("}\n");
    out
}

/// Merge ordering: existing target keys first, then source keys, then the
/// current batch, preserving already-written values.
pub fn ordered_language_target(
    source: &crate::mod_translation::analyze::LanguageSource,
    current: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    if source.kind == crate::mod_translation::analyze::LanguageKind::FreeText {
        return current.clone();
    }
    let mut keys: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for key in source
        .existing_target
        .keys()
        .chain(source.entries.keys())
        .chain(current.keys())
    {
        if seen.insert(key.clone()) {
            keys.push(key.clone());
        }
    }
    keys.retain(|key| current.contains_key(key));
    keys.into_iter()
        .filter_map(|key| current.get(&key).cloned().map(|value| (key, value)))
        .collect()
}

fn read_localized_target_entries(
    snapshot: &FreeTextSnapshot,
    layout: &crate::mod_translation::analyze::LocalizedLayout,
) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    for (index, line) in layout.source_lines.iter().enumerate() {
        if !line.trim().is_empty()
            && let Some(translated) = snapshot.lines.get(index)
            && !translated.trim().is_empty()
        {
            result.insert(localized_line_key(index), translated.clone());
        }
    }
    result
}

pub fn serialize_localized_target(
    entries: &BTreeMap<String, String>,
    source: &crate::mod_translation::analyze::LanguageSource,
) -> String {
    let Some(layout) = &source.localized_layout else {
        return entries.get("/").cloned().unwrap_or_default();
    };
    let base = if let Some(existing) = &layout.existing_target_lines {
        existing.clone()
    } else {
        vec![String::new(); layout.source_lines.len()]
    };
    let mut lines = base;
    let length = layout.source_lines.len().max(lines.len());
    lines.resize(length, String::new());
    for (key, value) in entries {
        if let Some(index) = localized_line_index(key)
            && index < layout.source_lines.len()
        {
            lines[index] = value.clone();
        }
    }
    let snapshot = FreeTextSnapshot {
        bom: layout.bom,
        eol: layout.eol.clone(),
        trailing_newline: layout.trailing_newline,
        lines,
    };
    snapshot.render()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_json_round_trips_keep_key_order() {
        let content =
            "{\n  \"z\": \"1\",\n  \"a\": \"2\",\n  \"m\": \"3\"\n}\n";
        let root = JsonValue::parse(content).unwrap();
        let rendered = root.render_pretty();
        let z = rendered.find("\"z\"").unwrap();
        let a = rendered.find("\"a\"").unwrap();
        let m = rendered.find("\"m\"").unwrap();
        assert!(z < a && a < m, "key order must be preserved: {rendered}");
    }

    #[test]
    fn pointers_set_nested_values() {
        let content = r#"{"a":{"b":[{"c":"old"}]}}"#;
        let mut root = JsonValue::parse(content).unwrap();
        root.set_pointer("/a/b/0/c", "new".to_string()).unwrap();
        assert_eq!(root.render_pretty().trim(), "{\n  \"a\": {\n    \"b\": [\n      {\n        \"c\": \"new\"\n      }\n    ]\n  }\n}".trim());
    }

    #[test]
    fn flat_language_key_is_not_accepted_as_a_json_pointer() {
        let mut root = JsonValue::parse(r#"{"demo.hello":"Hello"}"#).unwrap();
        let error = root
            .set_pointer("demo.hello", "你好".to_string())
            .unwrap_err();
        assert_eq!(error.code, TranslateErrorCode::Config);
        assert_eq!(
            root.render_pretty().trim(),
            "{\n  \"demo.hello\": \"Hello\"\n}"
        );
    }

    #[test]
    fn structured_language_serialization_round_trips_pointer_values() {
        let source = crate::mod_translation::analyze::LanguageSource {
            kind: crate::mod_translation::analyze::LanguageKind::Json,
            namespace: "demo".to_string(),
            source_path: "assets/demo/en_us/menu.json".to_string(),
            target_path: "assets/demo/zh_cn/menu.json".to_string(),
            entries: BTreeMap::from([(
                "/menu/title".to_string(),
                "Hello".to_string(),
            )]),
            existing_target: BTreeMap::new(),
            structured_template: Some(
                r#"{"menu":{"title":"Hello","width":10}}"#.to_string(),
            ),
            localized_layout: None,
        };
        let translated =
            BTreeMap::from([("/menu/title".to_string(), "你好".to_string())]);
        let serialized =
            serialize_language_target(&source, &translated).unwrap();
        let reread = read_language_target(&serialized, &source);
        assert_eq!(reread.get("/menu/title").map(String::as_str), Some("你好"));
        assert!(serialized.contains("\"width\": 10"));
    }

    #[test]
    fn key_value_parser_skips_comments_and_escaped_separators() {
        let content = "a=1\n# comment\nb:two\nc\\=x=3\n! bang\n";
        let map = read_key_value(content);
        assert_eq!(map.get("a").map(String::as_str), Some("1"));
        assert_eq!(map.get("b").map(String::as_str), Some("two"));
        assert_eq!(map.get("c\\=x").map(String::as_str), Some("3"));
        assert!(!map.contains_key("comment"));
    }

    #[test]
    fn free_text_snapshot_preserves_bom_eol_and_trailing_newline() {
        let content = "\u{feff}line1\r\nline2\r\n";
        let snapshot = FreeTextSnapshot::parse(content);
        assert!(snapshot.bom);
        assert_eq!(snapshot.eol, "\r\n");
        assert!(snapshot.trailing_newline);
        assert_eq!(snapshot.lines, vec!["line1", "line2"]);
        assert_eq!(snapshot.render(), content);
    }

    #[test]
    fn local_line_keys_round_trip() {
        assert_eq!(localized_line_key(0), "/lines/000000");
        assert_eq!(localized_line_key(12), "/lines/000012");
        assert_eq!(localized_line_index("/lines/000042"), Some(42));
        assert_eq!(localized_line_index("/other"), None);
    }
}
