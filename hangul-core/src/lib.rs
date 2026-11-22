//! 두벌식(2-beolsik) Hangul composition engine.
//!
//! Platform independent: feed it jamo (compatibility jamo, U+3131..U+3163),
//! it returns committed text and keeps the syllable currently being composed.

/// Initial consonants (초성) in Unicode order, as compatibility jamo.
const CHO: [char; 19] = [
    'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ', 'ㅋ', 'ㅌ',
    'ㅍ', 'ㅎ',
];

/// Final consonants (종성) in Unicode order; index 0 means "no final".
const JONG: [char; 28] = [
    '\0', 'ㄱ', 'ㄲ', 'ㄳ', 'ㄴ', 'ㄵ', 'ㄶ', 'ㄷ', 'ㄹ', 'ㄺ', 'ㄻ', 'ㄼ', 'ㄽ', 'ㄾ', 'ㄿ', 'ㅀ', 'ㅁ',
    'ㅂ', 'ㅄ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
];

const VOWEL_FIRST: u32 = 0x314F; // ㅏ
const VOWEL_LAST: u32 = 0x3163; // ㅣ

const VOWEL_PAIRS: [(char, char, char); 7] = [
    ('ㅗ', 'ㅏ', 'ㅘ'),
    ('ㅗ', 'ㅐ', 'ㅙ'),
    ('ㅗ', 'ㅣ', 'ㅚ'),
    ('ㅜ', 'ㅓ', 'ㅝ'),
    ('ㅜ', 'ㅔ', 'ㅞ'),
    ('ㅜ', 'ㅣ', 'ㅟ'),
    ('ㅡ', 'ㅣ', 'ㅢ'),
];

const JONG_PAIRS: [(char, char, char); 11] = [
    ('ㄱ', 'ㅅ', 'ㄳ'),
    ('ㄴ', 'ㅈ', 'ㄵ'),
    ('ㄴ', 'ㅎ', 'ㄶ'),
    ('ㄹ', 'ㄱ', 'ㄺ'),
    ('ㄹ', 'ㅁ', 'ㄻ'),
    ('ㄹ', 'ㅂ', 'ㄼ'),
    ('ㄹ', 'ㅅ', 'ㄽ'),
    ('ㄹ', 'ㅌ', 'ㄾ'),
    ('ㄹ', 'ㅍ', 'ㄿ'),
    ('ㄹ', 'ㅎ', 'ㅀ'),
    ('ㅂ', 'ㅅ', 'ㅄ'),
];

pub fn is_vowel(c: char) -> bool {
    (VOWEL_FIRST..=VOWEL_LAST).contains(&(c as u32))
}

pub fn is_consonant(c: char) -> bool {
    CHO.contains(&c)
}

fn combine_vowel(a: char, b: char) -> Option<char> {
    VOWEL_PAIRS.iter().find(|p| p.0 == a && p.1 == b).map(|p| p.2)
}

fn combine_jong(a: char, b: char) -> Option<char> {
    JONG_PAIRS.iter().find(|p| p.0 == a && p.1 == b).map(|p| p.2)
}

/// Splits a compound final into its two parts.
fn split_jong(c: char) -> Option<(char, char)> {
    JONG_PAIRS.iter().find(|p| p.2 == c).map(|p| (p.0, p.1))
}

fn can_be_jong(c: char) -> bool {
    c != '\0' && JONG.contains(&c)
}

/// One syllable being composed. Every field holds a compatibility jamo.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
struct Syllable {
    cho: Option<char>,
    jung: Option<char>,
    jong: Option<char>,
}

impl Syllable {
    fn is_empty(&self) -> bool {
        self.cho.is_none() && self.jung.is_none() && self.jong.is_none()
    }

    fn to_char(self) -> Option<char> {
        match (self.cho, self.jung, self.jong) {
            (None, None, _) => None,
            (Some(c), None, _) => Some(c),
            (None, Some(v), _) => Some(v),
            (Some(c), Some(v), jong) => {
                let ci = CHO.iter().position(|&x| x == c)? as u32;
                let vi = v as u32 - VOWEL_FIRST;
                let ji = match jong {
                    Some(j) => JONG.iter().position(|&x| x == j)? as u32,
                    None => 0,
                };
                char::from_u32(0xAC00 + (ci * 21 + vi) * 28 + ji)
            }
        }
    }
}

/// The composition engine.
///
/// `keys` records the jamo typed for the current syllable so that
/// backspace can undo exactly one keystroke by replaying the rest.
#[derive(Default, Debug)]
pub struct Composer {
    cur: Syllable,
    keys: Vec<char>,
}

impl Composer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.cur.is_empty()
    }

    /// Text currently being composed (0 or 1 characters).
    pub fn preedit(&self) -> String {
        self.cur.to_char().map(String::from).unwrap_or_default()
    }

    /// Commits whatever is being composed and resets the engine.
    pub fn flush(&mut self) -> String {
        let s = self.preedit();
        self.reset();
        s
    }

    pub fn reset(&mut self) {
        self.cur = Syllable::default();
        self.keys.clear();
    }

    /// Feeds one jamo. Returns text that became final as a result.
    /// Characters that are not jamo flush the composition and are appended.
    pub fn input(&mut self, c: char) -> String {
        if is_vowel(c) {
            self.input_vowel(c)
        } else if is_consonant(c) {
            self.input_consonant(c)
        } else {
            let mut s = self.flush();
            s.push(c);
            s
        }
    }

    /// Removes the last keystroke. Returns false if there was nothing to delete.
    pub fn backspace(&mut self) -> bool {
        if self.keys.pop().is_none() {
            return false;
        }
        let keys = std::mem::take(&mut self.keys);
        self.cur = Syllable::default();
        for k in keys {
            // Replaying keys of a single syllable never commits anything.
            let committed = self.input(k);
            debug_assert!(committed.is_empty());
        }
        true
    }

    fn start_new(&mut self, syl: Syllable, keys: Vec<char>) -> String {
        let out = self.preedit();
        self.cur = syl;
        self.keys = keys;
        out
    }

    fn input_consonant(&mut self, c: char) -> String {
        let s = self.cur;
        match (s.cho, s.jung, s.jong) {
            (None, None, None) => {
                self.cur.cho = Some(c);
                self.keys.push(c);
                String::new()
            }
            // Have initial + vowel: try to take it as final consonant.
            (Some(_), Some(_), None) if can_be_jong(c) => {
                self.cur.jong = Some(c);
                self.keys.push(c);
                String::new()
            }
            (Some(_), Some(_), Some(j)) => {
                if let Some(comp) = combine_jong(j, c) {
                    self.cur.jong = Some(comp);
                    self.keys.push(c);
                    String::new()
                } else {
                    self.start_new(Syllable { cho: Some(c), ..Default::default() }, vec![c])
                }
            }
            _ => self.start_new(Syllable { cho: Some(c), ..Default::default() }, vec![c]),
        }
    }

    fn input_vowel(&mut self, v: char) -> String {
        let s = self.cur;
        match (s.cho, s.jung, s.jong) {
            (None, None, None) | (Some(_), None, None) => {
                self.cur.jung = Some(v);
                self.keys.push(v);
                String::new()
            }
            (_, Some(j), None) => {
                // Only combine if the current vowel was typed as a single key.
                let single = self.keys.last() == Some(&j);
                match combine_vowel(j, v) {
                    Some(comp) if single => {
                        self.cur.jung = Some(comp);
                        self.keys.push(v);
                        String::new()
                    }
                    _ => self.start_new(Syllable { jung: Some(v), ..Default::default() }, vec![v]),
                }
            }
            (Some(_), Some(_), Some(j)) => {
                // The final consonant (or its second half) moves to the next syllable.
                let (keep, moved) = match split_jong(j) {
                    Some((a, b)) => (Some(a), b),
                    None => (None, j),
                };
                self.cur.jong = keep;
                self.keys.pop();
                let out = self.preedit();
                self.cur = Syllable { cho: Some(moved), jung: Some(v), jong: None };
                self.keys = vec![moved, v];
                out
            }
            // cho=None with jong is impossible; be defensive.
            _ => self.start_new(Syllable { jung: Some(v), ..Default::default() }, vec![v]),
        }
    }
}

/// 두벌식 standard layout. `key` is an ASCII letter (either case), `shift`
/// selects the shifted jamo. Returns None for non-letters.
pub fn dubeolsik(key: char, shift: bool) -> Option<char> {
    let k = key.to_ascii_lowercase();
    if shift {
        let s = match k {
            'q' => Some('ㅃ'),
            'w' => Some('ㅉ'),
            'e' => Some('ㄸ'),
            'r' => Some('ㄲ'),
            't' => Some('ㅆ'),
            'o' => Some('ㅒ'),
            'p' => Some('ㅖ'),
            _ => None,
        };
        if s.is_some() {
            return s;
        }
    }
    Some(match k {
        'q' => 'ㅂ',
        'w' => 'ㅈ',
        'e' => 'ㄷ',
        'r' => 'ㄱ',
        't' => 'ㅅ',
        'y' => 'ㅛ',
        'u' => 'ㅕ',
        'i' => 'ㅑ',
        'o' => 'ㅐ',
        'p' => 'ㅔ',
        'a' => 'ㅁ',
        's' => 'ㄴ',
        'd' => 'ㅇ',
        'f' => 'ㄹ',
        'g' => 'ㅎ',
        'h' => 'ㅗ',
        'j' => 'ㅓ',
        'k' => 'ㅏ',
        'l' => 'ㅣ',
        'z' => 'ㅋ',
        'x' => 'ㅌ',
        'c' => 'ㅊ',
        'v' => 'ㅍ',
        'b' => 'ㅠ',
        'n' => 'ㅜ',
        'm' => 'ㅡ',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Types an ASCII string (uppercase = shift) and returns committed + preedit.
    fn typ(keys: &str) -> (String, String) {
        let mut c = Composer::new();
        let mut out = String::new();
        for k in keys.chars() {
            if k == '<' {
                if !c.backspace() {
                    out.pop();
                }
                continue;
            }
            match dubeolsik(k, k.is_ascii_uppercase()) {
                Some(j) => out.push_str(&c.input(j)),
                None => out.push_str(&c.input(k)),
            }
        }
        (out, c.preedit())
    }

    fn all(keys: &str) -> String {
        let (a, b) = typ(keys);
        a + &b
    }

    #[test]
    fn basic_words() {
        assert_eq!(all("dkssudgktpdy"), "안녕하세요");
        assert_eq!(all("gksrmf"), "한글");
        assert_eq!(all("rkskekfk"), "가나다라");
        assert_eq!(all("tkfkdgo"), "사랑해");
        assert_eq!(all("eocnfgkqslek"), "대출합니다");
    }

    #[test]
    fn preedit_split() {
        assert_eq!(typ("gks"), ("".into(), "한".into()));
        assert_eq!(typ("gksr"), ("한".into(), "ㄱ".into()));
        assert_eq!(typ("gksrm"), ("한".into(), "그".into()));
    }

    #[test]
    fn jong_migration() {
        assert_eq!(all("ekfr"), "닭");
        assert_eq!(all("ekfrdl"), "닭이");
        assert_eq!(all("ekfrl"), "달기");
        assert_eq!(all("dlTdj"), "있어");
    }

    #[test]
    fn compound_final() {
        assert_eq!(all("djqt"), "없");
        assert_eq!(all("djqtdj"), "없어");
        assert_eq!(all("djqtj"), "업서");
        assert_eq!(all("dlTek"), "있다");
        assert_eq!(all("dkswdk"), "앉아");
        assert_eq!(all("dksj"), "아너");
    }

    #[test]
    fn compound_vowels() {
        assert_eq!(all("ghk"), "화");
        assert_eq!(all("dnjs"), "원");
        assert_eq!(all("dml"), "의");
        assert_eq!(all("dho"), "왜");
        assert_eq!(all("dnp"), "웨");
        assert_eq!(all("ghkk"), "화ㅏ");
    }

    #[test]
    fn non_syllables() {
        assert_eq!(all("rr"), "ㄱㄱ");
        assert_eq!(all("kk"), "ㅏㅏ");
        assert_eq!(all("kr"), "ㅏㄱ");
        assert_eq!(all("R"), "ㄲ");
        assert_eq!(all("Rk"), "까");
        assert_eq!(all("rkE"), "가ㄸ");
        assert_eq!(all("rkEk"), "가따");
    }

    #[test]
    fn backspace() {
        assert_eq!(typ("gks<"), ("".into(), "하".into()));
        assert_eq!(typ("gks<<"), ("".into(), "ㅎ".into()));
        assert_eq!(typ("gks<<<"), ("".into(), "".into()));
        assert_eq!(typ("ghk<"), ("".into(), "호".into()));
        assert_eq!(typ("ekfr<"), ("".into(), "달".into()));
        assert_eq!(typ("djqtdj<"), ("없".into(), "ㅇ".into()));
        // migrated consonant: 달기 -> backspace -> 달ㄱ
        assert_eq!(typ("ekfrl<"), ("달".into(), "ㄱ".into()));
        assert_eq!(typ("gksrmf<<<"), ("한".into(), "".into()));
        assert_eq!(typ("gksrmf<<<<"), ("".into(), "".into()));
    }

    #[test]
    fn non_jamo_flushes() {
        assert_eq!(all("gks1rmf"), "한1글");
        assert_eq!(typ("gks "), ("한 ".into(), "".into()));
    }
}
