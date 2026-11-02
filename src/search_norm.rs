use skim::fuzzy_matcher::FuzzyMatcher;
use skim::fuzzy_matcher::fzy::FzyMatcher;

pub fn normalize_char(c: char) -> Option<&'static str> {
    if ('\u{FF01}'..='\u{FF5E}').contains(&c) {
        let ascii_code = (c as u32) - 0xFEE0;
        if let Some(ch) = char::from_u32(ascii_code) {
            return match ch {
                'A'..='Z' | 'a'..='z' => match ch.to_ascii_lowercase() {
                    'a' => Some("a"),
                    'b' => Some("b"),
                    'c' => Some("c"),
                    'd' => Some("d"),
                    'e' => Some("e"),
                    'f' => Some("f"),
                    'g' => Some("g"),
                    'h' => Some("h"),
                    'i' => Some("i"),
                    'j' => Some("j"),
                    'k' => Some("k"),
                    'l' => Some("l"),
                    'm' => Some("m"),
                    'n' => Some("n"),
                    'o' => Some("o"),
                    'p' => Some("p"),
                    'q' => Some("q"),
                    'r' => Some("r"),
                    's' => Some("s"),
                    't' => Some("t"),
                    'u' => Some("u"),
                    'v' => Some("v"),
                    'w' => Some("w"),
                    'x' => Some("x"),
                    'y' => Some("y"),
                    'z' => Some("z"),
                    _ => None,
                },
                '0' => Some("0"),
                '1' => Some("1"),
                '2' => Some("2"),
                '3' => Some("3"),
                '4' => Some("4"),
                '5' => Some("5"),
                '6' => Some("6"),
                '7' => Some("7"),
                '8' => Some("8"),
                '9' => Some("9"),
                '\'' | '`' => None,
                '-' | '_' | '~' => Some("-"),
                ' ' => Some(" "),
                _ => None,
            };
        }
    }

    if c == '\u{3000}' {
        return Some(" ");
    }

    match c {
        'a' | 'A' | 'á' | 'Á' | 'à' | 'À' | 'â' | 'Â' | 'ä' | 'Ä' | 'ã' | 'Ã' | 'å' | 'Å' | 'ā'
        | 'Ā' | 'ą' | 'Ą' => Some("a"),
        'æ' | 'Æ' => Some("ae"),
        'b' | 'B' => Some("b"),
        'c' | 'C' | 'ç' | 'Ç' | 'ć' | 'Ć' | 'č' | 'Č' => Some("c"),
        'd' | 'D' | 'ď' | 'Ď' | 'đ' | 'Đ' => Some("d"),
        'e' | 'E' | 'é' | 'É' | 'è' | 'È' | 'ê' | 'Ê' | 'ë' | 'Ë' | 'ē' | 'Ē' | 'ę' | 'Ę' | 'ě'
        | 'Ě' => Some("e"),
        'f' | 'F' => Some("f"),
        'g' | 'G' | 'ğ' | 'Ğ' => Some("g"),
        'h' | 'H' => Some("h"),
        'i' | 'I' | 'í' | 'Í' | 'ì' | 'Ì' | 'î' | 'Î' | 'ï' | 'Ï' | 'ī' | 'Ī' | 'ı' => {
            Some("i")
        }
        'j' | 'J' => Some("j"),
        'k' | 'K' => Some("k"),
        'l' | 'L' | 'ł' | 'Ł' | 'ĺ' | 'Ĺ' | 'ľ' | 'Ľ' => Some("l"),
        'm' | 'M' => Some("m"),
        'n' | 'N' | 'ñ' | 'Ñ' | 'ń' | 'Ń' | 'ň' | 'Ň' => Some("n"),
        'o' | 'O' | 'ó' | 'Ó' | 'ò' | 'Ò' | 'ô' | 'Ô' | 'ö' | 'Ö' | 'õ' | 'Õ' | 'ø' | 'Ø' | 'ō'
        | 'Ō' | 'ő' | 'Ő' => Some("o"),
        'œ' | 'Œ' => Some("oe"),
        'p' | 'P' => Some("p"),
        'q' | 'Q' => Some("q"),
        'r' | 'R' | 'ŕ' | 'Ŕ' | 'ř' | 'Ř' => Some("r"),
        's' | 'S' | 'ś' | 'Ś' | 'š' | 'Š' | 'ş' | 'Ş' => Some("s"),
        'ß' => Some("ss"),
        't' | 'T' | 'ť' | 'Ť' | 'ţ' | 'Ţ' => Some("t"),
        'u' | 'U' | 'ú' | 'Ú' | 'ù' | 'Ù' | 'û' | 'Û' | 'ü' | 'Ü' | 'ū' | 'Ū' | 'ů' | 'Ů' | 'ű'
        | 'Ű' => Some("u"),
        'v' | 'V' => Some("v"),
        'w' | 'W' => Some("w"),
        'x' | 'X' => Some("x"),
        'y' | 'Y' | 'ý' | 'Ý' | 'ÿ' | 'Ÿ' => Some("y"),
        'z' | 'Z' | 'ź' | 'Ź' | 'ż' | 'Ż' | 'ž' | 'Ž' => Some("z"),

        '0'..='9' => match c {
            '0' => Some("0"),
            '1' => Some("1"),
            '2' => Some("2"),
            '3' => Some("3"),
            '4' => Some("4"),
            '5' => Some("5"),
            '6' => Some("6"),
            '7' => Some("7"),
            '8' => Some("8"),
            '9' => Some("9"),
            _ => None,
        },

        '\'' | '’' | '‘' | '`' | '´' | '"' | '“' | '”' | '„' => Some(""),

        '-' | '–' | '—' | '−' | '‐' => Some("-"),

        ' ' | '\t' | '\n' | '\r' => Some(" "),
        '.' => Some("."),
        ',' => Some(","),
        '!' => Some("!"),
        '?' => Some("?"),
        '/' => Some("/"),
        '\\' => Some("\\"),
        '&' => Some("&"),
        '+' => Some("+"),
        '(' => Some("("),
        ')' => Some(")"),
        '[' => Some("["),
        ']' => Some("]"),
        '{' => Some("{"),
        '}' => Some("}"),

        _ => None,
    }
}

pub fn normalize_search_text(input: &str) -> String {
    let mut normalized = String::with_capacity(input.len());
    let mut last_was_space = false;

    for c in input.chars() {
        match normalize_char(c) {
            Some("") => {}
            Some(" ") => {
                if !last_was_space && !normalized.is_empty() {
                    normalized.push(' ');
                    last_was_space = true;
                }
            }
            Some(rep) => {
                normalized.push_str(rep);
                last_was_space = false;
            }
            None => {
                for lower_c in c.to_lowercase() {
                    if lower_c.is_whitespace() {
                        if !last_was_space && !normalized.is_empty() {
                            normalized.push(' ');
                            last_was_space = true;
                        }
                    } else {
                        normalized.push(lower_c);
                        last_was_space = false;
                    }
                }
            }
        }
    }

    if last_was_space && normalized.ends_with(' ') {
        normalized.pop();
    }

    normalized
}

pub fn clean_noise(input: &str) -> String {
    let mut cleaned = String::with_capacity(input.len());
    let mut in_parenthesis = 0;
    let mut in_bracket = 0;

    for c in input.chars() {
        match c {
            '(' | '\u{FF08}' => in_parenthesis += 1,
            ')' | '\u{FF09}' => {
                if in_parenthesis > 0 {
                    in_parenthesis -= 1;
                }
            }
            '[' | '\u{FF3B}' | '【' | '〔' | '〈' | '《' | '「' | '『' | '〖' | '〘' | '〚' => {
                in_bracket += 1
            }
            ']' | '\u{FF3D}' | '】' | '〕' | '〉' | '》' | '」' | '』' | '〗' | '〙' | '〛' => {
                if in_bracket > 0 {
                    in_bracket -= 1;
                }
            }
            _ => {
                if in_parenthesis == 0 && in_bracket == 0 {
                    cleaned.push(c);
                }
            }
        }
    }

    let mut result = cleaned.trim();
    if let Some(pos) = result.to_lowercase().find(" feat. ") {
        result = result[..pos].trim();
    } else if let Some(pos) = result.to_lowercase().find(" ft. ") {
        result = result[..pos].trim();
    } else if let Some(pos) = result.to_lowercase().find(" feat ") {
        result = result[..pos].trim();
    } else if let Some(pos) = result.to_lowercase().find(" ft ") {
        result = result[..pos].trim();
    }

    if result.is_empty() {
        input.trim().to_string()
    } else {
        result.to_string()
    }
}

fn match_token(token: &str, norm_target: &str, norm_clean: &str) -> Option<i64> {
    if norm_clean == token || norm_target == token {
        return Some(100_000);
    }
    if norm_clean.starts_with(token) || norm_target.starts_with(token) {
        return Some(50_000);
    }

    let words = norm_target
        .split(|c: char| !c.is_alphanumeric())
        .chain(norm_clean.split(|c: char| !c.is_alphanumeric()))
        .filter(|w| !w.is_empty());

    let mut found_word_exact = false;
    let mut found_word_prefix = false;
    for word in words {
        if word == token {
            found_word_exact = true;
            break;
        } else if word.starts_with(token) {
            found_word_prefix = true;
        }
    }

    if found_word_exact {
        return Some(30_000);
    }
    if found_word_prefix {
        return Some(20_000);
    }

    if norm_clean.contains(token) || norm_target.contains(token) {
        return Some(10_000);
    }

    let token_len = token.chars().count();
    if token_len >= 3 {
        let matcher = FzyMatcher::default().ignore_case().max_typos(Some(1));
        let words = norm_target
            .split(|c: char| !c.is_alphanumeric())
            .chain(norm_clean.split(|c: char| !c.is_alphanumeric()));

        let mut best_score = None;
        for word in words {
            if word.is_empty() {
                continue;
            }
            if word.chars().count().abs_diff(token_len) <= 1
                && let Some(score) = matcher.fuzzy_match(word, token)
            {
                best_score = Some(best_score.map_or(score, |b: i64| b.max(score)));
            }
        }
        return best_score;
    }

    None
}

pub fn score_match(query: &str, target: &str) -> Option<i64> {
    let norm_target = normalize_search_text(target);
    let norm_clean = normalize_search_text(&clean_noise(target));
    let norm_query = normalize_search_text(query);

    let tokens: Vec<&str> = norm_query.split_whitespace().collect();
    if tokens.is_empty() {
        return Some(0);
    }

    let mut total_score = 0;
    for token in tokens {
        total_score += match_token(token, &norm_target, &norm_clean)?;
    }

    let len_diff = norm_target.len().saturating_sub(norm_query.len()) as i64;
    total_score -= len_diff.min(100);

    Some(total_score)
}

pub fn match_all_tokens(query: &str, target: &str) -> bool {
    score_match(query, target).is_some()
}

pub fn sort_by_relevance<T, F>(items: &mut [T], query: &str, field_extractor: F)
where
    F: Fn(&T) -> &str,
{
    items.sort_by(|a, b| {
        let score_a = score_match(query, field_extractor(a)).unwrap_or(i64::MIN);
        let score_b = score_match(query, field_extractor(b)).unwrap_or(i64::MIN);
        score_b.cmp(&score_a)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relevance_scoring_tiers() {
        let exact = score_match("july", "July").unwrap();
        let prefix = score_match("july", "July Morning").unwrap();
        let word_boundary = score_match("july", "And July").unwrap();
        let substring = score_match("july", "Unjuly").unwrap();
        let typo = score_match("jlu", "July").unwrap();

        assert!(exact > prefix);
        assert!(prefix > word_boundary);
        assert!(word_boundary > substring);
        assert!(substring > typo);
        assert!(score_match("xyz", "July").is_none());
        assert!(
            score_match(
                "july and",
                "One Piece : Drums of Liberation x Overtaken - TU Symphony Orchestra"
            )
            .is_none()
        );
        assert!(score_match("july and", "And July").is_some());
        assert!(score_match("jlu and", "And July").is_some());

        let mut titles = vec!["Unjuly", "And July", "July", "July Morning"];
        sort_by_relevance(&mut titles, "july", |t| t);
        assert_eq!(titles, vec!["July", "July Morning", "And July", "Unjuly"]);
    }
    #[test]
    fn test_case_and_accents() {
        assert_eq!(normalize_search_text("Café"), "cafe");
        assert_eq!(normalize_search_text("MÖTLEY CRÜE"), "motley crue");
        assert_eq!(normalize_search_text("Tōkyō"), "tokyo");
        assert_eq!(normalize_search_text("Beyoncé"), "beyonce");
    }

    #[test]
    fn test_fullwidth_japanese_characters() {
        assert_eq!(normalize_search_text("Ｊｕｌｙ"), "july");
        assert_eq!(
            normalize_search_text("Ｈｅｌｌｏ　Ｗｏｒｌｄ"),
            "hello world"
        );
    }

    #[test]
    fn test_apostrophes_quotes_dashes() {
        assert_eq!(normalize_search_text("Don’t Speak"), "dont speak");
        assert_eq!(normalize_search_text("Dont Speak"), "dont speak");
        assert_eq!(normalize_search_text("I'm Yours"), "im yours");
        assert_eq!(normalize_search_text("Im Yours"), "im yours");
        assert_eq!(normalize_search_text("Rock – Roll"), "rock - roll");
        assert_eq!(normalize_search_text("“Hello”"), "hello");
    }

    #[test]
    fn test_whitespace_collapsing() {
        assert_eq!(normalize_search_text("   July   Song   "), "july song");
        assert_eq!(normalize_search_text("July\t\nSong"), "july song");
    }

    #[test]
    fn test_cjk_preservation() {
        assert_eq!(normalize_search_text("夜に駆ける"), "夜に駆ける");
        assert_eq!(normalize_search_text("아이유"), "아이유");
        assert_eq!(
            normalize_search_text("YOASOBI - 夜に駆ける"),
            "yoasobi - 夜に駆ける"
        );
    }

    #[test]
    fn test_clean_noise() {
        assert_eq!(clean_noise("July (feat. DEAN)"), "July");
        assert_eq!(clean_noise("Creep [Remastered 2021]"), "Creep");
        assert_eq!(clean_noise("Song (Official Audio) [HD]"), "Song");
        assert_eq!(clean_noise("July feat. DEAN"), "July");
        assert_eq!(clean_noise("(Acoustic)"), "(Acoustic)");
        assert_eq!(
            clean_noise("夜に駆ける（Official Music Video）"),
            "夜に駆ける"
        );
        assert_eq!(clean_noise("紅蓮華【アニメver.】"), "紅蓮華");
        assert_eq!(clean_noise("Song「TV Size」"), "Song");
    }

    #[test]
    fn test_match_all_tokens() {
        assert!(match_all_tokens("july dean", "July (feat. DEAN)"));
        assert!(match_all_tokens("dean july", "July (feat. DEAN)"));
        assert!(match_all_tokens("creep radiohead", "Radiohead - Creep"));
        assert!(!match_all_tokens("july heize", "July (feat. DEAN)"));

        assert!(match_all_tokens("jlu", "July"));
        assert!(match_all_tokens("jlu", "And July"));
        assert!(match_all_tokens("crep", "Creep"));
        assert!(match_all_tokens("raduohead", "Radiohead"));
    }
}
