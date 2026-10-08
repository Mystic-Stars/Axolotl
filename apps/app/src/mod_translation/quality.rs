//! 占位符保护、机械/语义审计、术语表、待译判定、工作量权重。

use std::collections::BTreeMap;
use std::sync::LazyLock;

use regex::Regex;

/// 占位符提取正则。Rust regex 不支持 lookbehind，所以用 `(?:^|[^X])` + 捕获组
/// 等价实现，取捕获组 1 作为 token。
const PLACEHOLDER_PATTERNS: &[&str] = &[
    r"(?m)^[\t ]*(/[A-Za-z0-9_.:-]+(?:[\t ]+(?:[A-Za-z0-9_.:-]+|\{[^{}\s]+\}|<[^<>\s]+>|\[[^\[\]\s]+\]))*)",
    r"(%(?:\d+\$)?[-#+ 0,(]*\d*(?:\.\d+)?[a-zA-Z%])",
    r"\$\{[A-Za-z_][A-Za-z0-9_.-]*\}",
    r"(?:^|[^$])(\{(?:\d+|[A-Za-z_][A-Za-z0-9_.-]*)\})",
    r"§[0-9A-FK-ORa-fk-or]",
    r"\\[nrt]",
    r"[\u{0001}-\u{0008}\u{000b}\u{000c}\u{000e}-\u{001f}]",
];

static COMPILED: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    PLACEHOLDER_PATTERNS
        .iter()
        .map(|pattern| {
            Regex::new(pattern).expect("placeholder pattern should compile")
        })
        .collect()
});

pub fn extract_protected_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for regex in COMPILED.iter() {
        for captures in regex.captures_iter(text) {
            if let Some(capture) = captures.get(1) {
                tokens.push(capture.as_str().to_string());
            } else if let Some(full) = captures.get(0) {
                tokens.push(full.as_str().to_string());
            }
        }
    }
    tokens.sort();
    tokens
}

pub fn validate_protected_tokens(
    source: &str,
    translation: &str,
) -> Option<String> {
    let expected = extract_protected_tokens(source);
    let actual = extract_protected_tokens(translation);
    if expected == actual {
        None
    } else {
        Some(format!(
            "占位符不一致：期望 [{}]，实际 [{}]",
            expected.join(", "),
            actual.join(", ")
        ))
    }
}

pub fn normalize_model_translation(source: &str, translation: &str) -> String {
    let mut normalized = translation.to_string();
    for (escaped, control) in [("\\n", "\n"), ("\\r", "\r"), ("\\t", "\t")] {
        if source.contains(escaped) {
            if !normalized.contains(escaped) {
                normalized = normalized.replace(control, escaped);
            }
        } else {
            normalized = normalized.replace(escaped, control);
        }
    }
    normalized
}

pub fn has_chinese(text: &str) -> bool {
    text.chars()
        .any(|character| matches!(character, '\u{3400}'..='\u{9fff}' | '\u{3000}'..='\u{303f}' | '\u{ff00}'..='\u{ffef}'))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct AuditIssue {
    pub severity: AuditSeverity,
    pub key: String,
    pub message: String,
}

/// 机械不变量审计：缺译、占位符、多余键、已有中文被改写。
pub fn audit_invariants(
    source: &BTreeMap<String, String>,
    target: &BTreeMap<String, String>,
) -> Vec<AuditIssue> {
    let mut issues = Vec::new();
    for (key, english) in source {
        match target.get(key) {
            Some(chinese) if !chinese.trim().is_empty() => {
                if let Some(error) = validate_protected_tokens(english, chinese)
                {
                    issues.push(AuditIssue {
                        severity: AuditSeverity::Error,
                        key: key.clone(),
                        message: error,
                    });
                }
                if requires_work(key, english, Some(chinese))
                    && !has_chinese(chinese)
                {
                    issues.push(AuditIssue {
                        severity: AuditSeverity::Error,
                        key: key.clone(),
                        message: "译文不含简体中文；若必须保留原文，需要显式标记 keep-source".to_string(),
                    });
                }
            }
            _ => issues.push(AuditIssue {
                severity: AuditSeverity::Error,
                key: key.clone(),
                message: "缺少中文译文".to_string(),
            }),
        }
    }
    for key in target.keys() {
        if !source.contains_key(key) {
            issues.push(AuditIssue {
                severity: AuditSeverity::Warning,
                key: key.clone(),
                message: "中文文件包含源语言中不存在的额外条目".to_string(),
            });
        }
    }
    issues
}

/// 官方术语规则：英文命中时必须含指定中文，否则 error。
pub struct TermRule {
    pub source: &'static str,
    pub chinese: &'static str,
    pub label: &'static str,
}

pub const OFFICIAL_TERMS: &[TermRule] = &[
    TermRule {
        source: r"\bMacaw(?:'s)?\b",
        chinese: "Macaw",
        label: "模组品牌 Macaw 应保留原文，不应擅自音译",
    },
    TermRule {
        source: r"\bAdditions\b",
        chinese: "扩展",
        label: "物品组标题中的 Additions 应译为自然的“扩展”",
    },
    TermRule {
        source: r"\bIron Bars\b",
        chinese: "铁栏杆",
        label: "Iron Bars 应沿用 Minecraft 官方简中术语“铁栏杆”",
    },
    TermRule {
        source: r"\bResizeable\b",
        chinese: "可变形",
        label: "Resizeable 在可改变外形的窗户语境中应译为自然的“可变形”",
    },
    TermRule {
        source: r"\bEnd(?:er)? Brick\b",
        chinese: "末地石砖",
        label: "End Brick 应沿用 Minecraft 官方材料名“末地石砖”",
    },
    TermRule {
        source: r"\bCrimson\b",
        chinese: "绯红",
        label: "Crimson 应沿用“绯红”",
    },
    TermRule {
        source: r"\bWarped\b",
        chinese: "诡异",
        label: "Warped 应沿用“诡异”",
    },
    TermRule {
        source: r"\bPale Oak\b",
        chinese: "苍白橡木",
        label: "Pale Oak 应沿用“苍白橡木”",
    },
    TermRule {
        source: r"\bDark Oak\b",
        chinese: "深色橡木",
        label: "Dark Oak 应沿用“深色橡木”",
    },
    TermRule {
        source: r"\bOak Planks\b",
        chinese: "橡木木板",
        label: "Oak Planks 应沿用完整材料名“橡木木板”",
    },
    TermRule {
        source: r"\bDark Oak Planks\b",
        chinese: "深色橡木木板",
        label: "Dark Oak Planks 应沿用完整材料名“深色橡木木板”",
    },
    TermRule {
        source: r"\bSpruce Planks\b",
        chinese: "云杉木板",
        label: "Spruce Planks 应沿用“云杉木板”",
    },
    TermRule {
        source: r"\bBirch Planks\b",
        chinese: "白桦木板",
        label: "Birch Planks 应沿用“白桦木板”",
    },
    TermRule {
        source: r"\bJungle Planks\b",
        chinese: "丛林木板",
        label: "Jungle Planks 应沿用“丛林木板”",
    },
    TermRule {
        source: r"\bAcacia Planks\b",
        chinese: "金合欢木板",
        label: "Acacia Planks 应沿用“金合欢木板”",
    },
    TermRule {
        source: r"\bCherry Planks\b",
        chinese: "樱花木板",
        label: "Cherry Planks 应沿用“樱花木板”",
    },
    TermRule {
        source: r"\bMangrove Planks\b",
        chinese: "红树木板",
        label: "Mangrove Planks 应沿用“红树木板”",
    },
    TermRule {
        source: r"\bCrimson Planks\b",
        chinese: "绯红木板",
        label: "Crimson Planks 应沿用“绯红木板”",
    },
    TermRule {
        source: r"\bWarped Planks\b",
        chinese: "诡异木板",
        label: "Warped Planks 应沿用“诡异木板”",
    },
];

const MATERIAL_ORDER: &[(&str, &str)] = &[
    (r"\bDark Oak\b", "深色橡木"),
    (r"\bPale Oak\b", "苍白橡木"),
    (r"\bOak\b", "橡木"),
    (r"\bSpruce\b", "云杉"),
    (r"\bBirch\b", "白桦"),
    (r"\bJungle\b", "丛林"),
    (r"\bAcacia\b", "金合欢"),
    (r"\bCherry\b", "樱花"),
    (r"\bMangrove\b", "红树"),
];

/// 语义审计：官方术语 + 材料族 + 动作区分 + 同原文同译法。
pub fn audit_semantic(
    source: &BTreeMap<String, String>,
    target: &BTreeMap<String, String>,
) -> Vec<AuditIssue> {
    let mut issues = Vec::new();
    let mut by_english: BTreeMap<&str, Vec<(&String, &String)>> =
        BTreeMap::new();

    for (key, english) in source {
        let Some(chinese) = target.get(key) else {
            continue;
        };
        if chinese.trim().is_empty() {
            continue;
        }
        for term in OFFICIAL_TERMS {
            let regex = Regex::new(term.source).unwrap();
            if regex.is_match(english) && !chinese.contains(term.chinese) {
                issues.push(AuditIssue {
                    severity: AuditSeverity::Error,
                    key: key.clone(),
                    message: term.label.to_string(),
                });
            }
        }
        if Regex::new(r"\bPlanks\b").unwrap().is_match(english)
            && !chinese.contains("木板")
        {
            issues.push(AuditIssue {
                severity: AuditSeverity::Error,
                key: key.clone(),
                message: "Planks 必须保留“木板”材料含义".to_string(),
            });
        }
        if Regex::new(r"\bStem\b").unwrap().is_match(english)
            && !chinese.contains("菌柄")
        {
            issues.push(AuditIssue {
                severity: AuditSeverity::Error,
                key: key.clone(),
                message: "Stem 必须保留“菌柄”材料含义".to_string(),
            });
        }

        let log_key = Regex::new(r"(?:^|_)log(?:_|$)").unwrap().is_match(key);
        let plank_sibling = log_key
            && source.keys().any(|candidate| {
                candidate
                    .replace("_log", "_plank")
                    .replace(".log", ".plank")
                    == *key
                    || candidate == &key.replace("_log", "_plank")
            });
        let source_means_timber = Regex::new(r"\b(?:Log|Stem|Timber|Wood)\b")
            .unwrap()
            .is_match(english)
            && !Regex::new(r"\b(?:Journal|Logbook|Research Log|Data Log)\b")
                .unwrap()
                .is_match(english);
        if log_key
            && (plank_sibling || source_means_timber)
            && !chinese.contains("原木")
        {
            issues.push(AuditIssue {
                severity: AuditSeverity::Error,
                key: key.clone(),
                message: "键名和本地结构表明是原木版本，译名必须保留“原木”"
                    .to_string(),
            });
        }

        let pane_sibling = key.rsplit_once('.').and_then(|(prefix, tail)| {
            tail.strip_suffix("_pane_window")
                .map(|stem| (prefix.to_string(), stem.to_string()))
        });
        if let Some((prefix, stem)) = pane_sibling
            && !stem.contains("plank")
            && source.keys().any(|candidate| {
                candidate == &format!("{prefix}.{stem}_plank_pane_window")
            })
            && !Regex::new(r"(?:原木|菌柄)").unwrap().is_match(chinese)
        {
            issues.push(AuditIssue {
                severity: AuditSeverity::Error,
                key: key.clone(),
                message: "存在对应木板玻璃板窗兄弟键，普通版本必须保留原木或菌柄材质差异".to_string(),
            });
        }

        let window_sibling = key.rsplit_once('.').and_then(|(prefix, tail)| {
            tail.strip_suffix("_window")
                .map(|stem| (prefix.to_string(), stem.to_string()))
        });
        if let Some((prefix, stem)) = window_sibling
            && !stem.contains("plank")
            && source.keys().any(|candidate| {
                candidate == &format!("{prefix}.{stem}_plank_window")
            })
            && !Regex::new(r"(?:原木|菌柄)").unwrap().is_match(chinese)
        {
            issues.push(AuditIssue {
                severity: AuditSeverity::Error,
                key: key.clone(),
                message:
                    "存在对应木板窗兄弟键，普通版本必须保留原木或菌柄材质差异"
                        .to_string(),
            });
        }

        if Regex::new(r"(?:^|[._])open$").unwrap().is_match(key)
            && !Regex::new(r"(?:打开|开启)").unwrap().is_match(chinese)
        {
            issues.push(AuditIssue {
                severity: AuditSeverity::Error,
                key: key.clone(),
                message: "键名表示打开动作，译文必须与关闭动作明确区分"
                    .to_string(),
            });
        }
        if Regex::new(r"(?:^|[._])close$").unwrap().is_match(key)
            && !Regex::new(r"(?:关闭|合上)").unwrap().is_match(chinese)
        {
            issues.push(AuditIssue {
                severity: AuditSeverity::Error,
                key: key.clone(),
                message: "键名表示关闭动作，译文必须与打开动作明确区分"
                    .to_string(),
            });
        }

        if Regex::new(r"\b(?:Four )?Pane Window\b")
            .unwrap()
            .is_match(english)
            && !chinese.contains("玻璃板")
            && !Regex::new(r"Four Pane").unwrap().is_match(english)
        {
            issues.push(AuditIssue {
                severity: AuditSeverity::Warning,
                key: key.clone(),
                message: "Pane Window 通常应体现“玻璃板”；请结合模型确认"
                    .to_string(),
            });
        }
        if Regex::new(r"\bPane Window\b").unwrap().is_match(english)
            && chinese.contains("玻璃板")
        {
            for (pattern, material) in MATERIAL_ORDER {
                if Regex::new(pattern).unwrap().is_match(english)
                    && chinese.find(material).unwrap_or(usize::MAX)
                        > chinese.find("玻璃板").unwrap_or(usize::MAX)
                {
                    issues.push(AuditIssue {
                        severity: AuditSeverity::Error,
                        key: key.clone(),
                        message: "物品名应采用“材质 + 玻璃板 + 窗”的自然语序"
                            .to_string(),
                    });
                    break;
                }
            }
        }
        if Regex::new(r"\bWindow (?:Four |Half )?Pane Base\b|\bWindow Base\b")
            .unwrap()
            .is_match(english)
            && !chinese.contains("底座")
        {
            issues.push(AuditIssue {
                severity: AuditSeverity::Warning,
                key: key.clone(),
                message: "作为合成组件的 Base 通常译为“底座”比“基础”自然"
                    .to_string(),
            });
        }

        by_english
            .entry(english.as_str())
            .or_default()
            .push((key, chinese));
    }

    for (_, group) in by_english {
        if group.len() <= 1 {
            continue;
        }
        let variants: std::collections::HashSet<&String> =
            group.iter().map(|(_, chinese)| *chinese).collect();
        let distinguishes_action = group.iter().any(|(key, _)| {
            Regex::new(r"(?:^|[._])open$").unwrap().is_match(key)
        }) && group.iter().any(|(key, _)| {
            Regex::new(r"(?:^|[._])close$").unwrap().is_match(key)
        });
        if variants.len() > 1 && !distinguishes_action {
            for (key, _) in group {
                issues.push(AuditIssue {
                    severity: AuditSeverity::Warning,
                    key: key.clone(),
                    message: format!(
                        "相同英文原文出现 {} 种译法，请确认是否需要统一",
                        variants.len()
                    ),
                });
            }
        }
    }
    issues
}

/// 待翻译条目判定。
pub fn requires_work(
    key: &str,
    source_text: &str,
    existing_target: Option<&str>,
) -> bool {
    let original = source_text.trim();
    if original.is_empty() {
        return false;
    }
    let target = existing_target.map(str::trim).unwrap_or("");
    let protected_only = is_passthrough_entry(key, original);
    let credited_work = (key.contains("music_disc")
        || key.contains("soundtrack")
        || key.contains("credit")
        || key.contains("author")
        || key.contains("artist"))
        && Regex::new(r"^.{2,80}\s[-–—]\s.{2,120}$")
            .unwrap()
            .is_match(original);
    let identical_needs_review = !target.is_empty()
        && target == original
        && !protected_only
        && !credited_work;

    if !target.is_empty() && has_chinese(target) {
        // 已有中文且非"原文==译文"的复核场景 → 不动
        if !identical_needs_review {
            return false;
        }
    }
    if protected_only || credited_work {
        return false;
    }
    if target.is_empty() {
        return true;
    }
    identical_needs_review || !has_chinese(target)
}

pub fn is_passthrough_entry(key: &str, source_text: &str) -> bool {
    let original = source_text.trim();
    let protected_only = Regex::new(
        r"^(?:https?://\S+|\/[a-z0-9_.:-]+(?:\s+[a-z0-9_.:<>{}\[\]-]+)*|[a-z0-9_.-]+:[a-z0-9_./-]+)$",
    )
    .unwrap()
    .is_match(original);
    let credited_work = (key.contains("music_disc")
        || key.contains("soundtrack")
        || key.contains("credit")
        || key.contains("author")
        || key.contains("artist"))
        && Regex::new(r"^.{2,80}\s[-–—]\s.{2,120}$")
            .unwrap()
            .is_match(original);
    protected_only || credited_work
}

/// 语言条目权重（用于排序/进度，非定价）。
pub fn language_work_weight(text: &str) -> f64 {
    let characters = text.chars().count();
    let protected =
        (Regex::new(r"%\d*\$?[a-z]|\{\w+\}|\$\{[^}]+\}|§[0-9a-fk-or]")
            .unwrap()
            .find_iter(text))
        .count();
    (1.0 + (characters as f64 / 80.0).min(3.0) + protected as f64 * 0.35)
        .round()
}

/// class 文本权重。
pub fn visible_text_work_weight(text: &str) -> f64 {
    (2.0 + (text.chars().count() as f64 / 60.0).min(4.0)).round()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn printf_placeholders_must_survive() {
        assert!(
            validate_protected_tokens("Spawn %d zombies", "生成 %d 只僵尸")
                .is_none()
        );
        assert!(
            validate_protected_tokens("Spawn %d zombies", "生成 %s 只僵尸")
                .is_some()
        );
        assert!(
            validate_protected_tokens(
                "%1$s took %2$d damage",
                "%1$s 受到 %2$d 点伤害"
            )
            .is_none()
        );
    }

    #[test]
    fn named_and_dollar_placeholders_must_survive() {
        assert!(
            validate_protected_tokens("{count} items", "{count} 个物品")
                .is_none()
        );
        assert!(
            validate_protected_tokens("${path} not found", "${path} 未找到")
                .is_none()
        );
        assert!(
            validate_protected_tokens("${path} not found", "路径未找到")
                .is_some()
        );
    }

    #[test]
    fn format_codes_and_escapes_must_survive() {
        assert!(
            validate_protected_tokens("§aGreen text", "§a绿色文字").is_none()
        );
        assert!(
            validate_protected_tokens("line\\nbreak", "换行\\n测试").is_none()
        );
        assert!(
            validate_protected_tokens("line\\nbreak", "换行测试").is_some()
        );
    }

    #[test]
    fn model_control_escapes_are_normalized_to_runtime_text() {
        assert_eq!(
            normalize_model_translation(
                "Rendering disabled: %s",
                "渲染已禁用：%s\\n请重新启用"
            ),
            "渲染已禁用：%s\n请重新启用"
        );
        assert_eq!(
            normalize_model_translation("line\\nbreak", "第一行\n第二行"),
            "第一行\\n第二行"
        );
    }

    #[test]
    fn commands_are_protected_as_whole() {
        assert!(
            validate_protected_tokens(
                "/give @p minecraft:diamond",
                "/give @p minecraft:diamond 给予"
            )
            .is_none()
        );
    }

    #[test]
    fn prose_after_a_slash_is_not_treated_as_a_command() {
        let source = "Total Blocks: §f%s§r / Volume: §f%s§r";
        let translation = "总方块数：§f%s§r / 体积：§f%s§r";
        assert!(validate_protected_tokens(source, translation).is_none());
        assert_eq!(
            extract_protected_tokens(source),
            vec!["%s", "%s", "§f", "§f", "§r", "§r"]
        );
    }

    #[test]
    fn requires_work_skips_urls_and_ids() {
        assert!(!requires_work("k", "https://example.com", None));
        assert!(!requires_work("k", "minecraft:iron_ingot", None));
        assert!(!requires_work("k", "", None));
        assert!(requires_work("k", "Iron Ingot", None));
        assert!(!requires_work(
            "item.music_disc.cat.desc",
            "C418 - Cat",
            Some("C418 - Cat")
        ));
    }

    #[test]
    fn latin_only_targets_require_an_explicit_keep_source_decision() {
        assert!(requires_work("button.reload", "Reload", Some("Reload")));
        let source = BTreeMap::from([(
            "button.reload".to_string(),
            "Reload".to_string(),
        )]);
        let target = BTreeMap::from([(
            "button.reload".to_string(),
            "Reload".to_string(),
        )]);
        assert!(
            audit_invariants(&source, &target)
                .iter()
                .any(|issue| issue.severity == AuditSeverity::Error)
        );
    }

    #[test]
    fn deterministic_passthrough_is_valid_after_copying_the_source() {
        let source = BTreeMap::from([
            ("homepage".to_string(), "https://example.com".to_string()),
            ("item.id".to_string(), "minecraft:iron_ingot".to_string()),
        ]);
        assert!(
            audit_invariants(&source, &source)
                .iter()
                .all(|issue| issue.severity != AuditSeverity::Error)
        );
    }

    #[test]
    fn audit_catches_missing_and_placeholder_breaks() {
        let source =
            BTreeMap::from([("a".to_string(), "Spawn %d".to_string())]);
        let target = BTreeMap::from([("a".to_string(), "生成".to_string())]);
        let issues = audit_invariants(&source, &target);
        assert!(
            issues
                .iter()
                .any(|issue| issue.severity == AuditSeverity::Error
                    && issue.key == "a")
        );
    }

    #[test]
    fn official_terms_are_enforced() {
        let source =
            BTreeMap::from([("k".to_string(), "Iron Bars".to_string())]);
        let target = BTreeMap::from([("k".to_string(), "铁条".to_string())]);
        let issues = audit_semantic(&source, &target);
        assert!(
            issues
                .iter()
                .any(|issue| issue.severity == AuditSeverity::Error
                    && issue.message.contains("铁栏杆"))
        );
    }

    #[test]
    fn plank_family_rule_requires_木板() {
        let source =
            BTreeMap::from([("k".to_string(), "Oak Planks".to_string())]);
        let target = BTreeMap::from([("k".to_string(), "橡树板".to_string())]);
        let issues = audit_semantic(&source, &target);
        assert!(
            issues
                .iter()
                .any(|issue| issue.severity == AuditSeverity::Error
                    && issue.message.contains("木板"))
        );
    }

    #[test]
    fn same_source_must_share_translation() {
        let mut source = BTreeMap::new();
        source.insert("a".to_string(), "Wrench".to_string());
        source.insert("b".to_string(), "Wrench".to_string());
        let mut target = BTreeMap::new();
        target.insert("a".to_string(), "扳手".to_string());
        target.insert("b".to_string(), "螺丝刀".to_string());
        let issues = audit_semantic(&source, &target);
        assert!(issues.iter().any(|issue| issue.message.contains("种译法")));
    }
}
