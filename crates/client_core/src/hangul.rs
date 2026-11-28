//! 한글 자모를 음절로 조합합니다. 시작 화면의 이름 칸이 씁니다.
//!
//! 키보드는 한글을 자모 하나씩(`ㅎ`, `ㅏ`, `ㄴ`) 보냅니다. 이 모듈은 칸의 마지막 글자와 새 자모를 보고 두벌식의 규칙으로 합칩니다.
//! 조합하는 중의 상태를 따로 두지 않습니다. 마지막 글자가 곧 상태입니다.

/// 첫소리의 순서입니다. 음절의 번호가 이 순서로 정해집니다.
const INITIALS: [char; 19] = [
    'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ', 'ㅋ',
    'ㅌ', 'ㅍ', 'ㅎ',
];
/// 받침의 순서입니다. 받침이 없는 자리(0)는 여기에 없습니다.
const FINALS: [char; 27] = [
    'ㄱ', 'ㄲ', 'ㄳ', 'ㄴ', 'ㄵ', 'ㄶ', 'ㄷ', 'ㄹ', 'ㄺ', 'ㄻ', 'ㄼ', 'ㄽ', 'ㄾ', 'ㄿ', 'ㅀ', 'ㅁ',
    'ㅂ', 'ㅄ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
];
/// 두 모음이 합쳐진 모음입니다.
const VOWEL_PAIRS: [(char, char, char); 7] = [
    ('ㅗ', 'ㅏ', 'ㅘ'),
    ('ㅗ', 'ㅐ', 'ㅙ'),
    ('ㅗ', 'ㅣ', 'ㅚ'),
    ('ㅜ', 'ㅓ', 'ㅝ'),
    ('ㅜ', 'ㅔ', 'ㅞ'),
    ('ㅜ', 'ㅣ', 'ㅟ'),
    ('ㅡ', 'ㅣ', 'ㅢ'),
];
/// 두 자음이 합쳐진 받침입니다.
const FINAL_PAIRS: [(char, char, char); 11] = [
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
/// 두벌식 자판의 키와 자모입니다. 한글 입력 상태에서 누른 키를 영문자로 되돌릴 때 씁니다.
const KEYS: [(char, char); 33] = [
    ('q', 'ㅂ'),
    ('w', 'ㅈ'),
    ('e', 'ㄷ'),
    ('r', 'ㄱ'),
    ('t', 'ㅅ'),
    ('y', 'ㅛ'),
    ('u', 'ㅕ'),
    ('i', 'ㅑ'),
    ('o', 'ㅐ'),
    ('p', 'ㅔ'),
    ('a', 'ㅁ'),
    ('s', 'ㄴ'),
    ('d', 'ㅇ'),
    ('f', 'ㄹ'),
    ('g', 'ㅎ'),
    ('h', 'ㅗ'),
    ('j', 'ㅓ'),
    ('k', 'ㅏ'),
    ('l', 'ㅣ'),
    ('z', 'ㅋ'),
    ('x', 'ㅌ'),
    ('c', 'ㅊ'),
    ('v', 'ㅍ'),
    ('b', 'ㅠ'),
    ('n', 'ㅜ'),
    ('m', 'ㅡ'),
    ('q', 'ㅃ'),
    ('w', 'ㅉ'),
    ('e', 'ㄸ'),
    ('r', 'ㄲ'),
    ('t', 'ㅆ'),
    ('o', 'ㅒ'),
    ('p', 'ㅖ'),
];

const FIRST_SYLLABLE: u32 = '가' as u32;
const VOWELS: u32 = 21;
const FINAL_SLOTS: u32 = 28;

/// 낱자로 적은 한글 자모(`ㄱ`부터 `ㅣ`까지)입니다.
pub fn is_jamo(c: char) -> bool {
    ('ㄱ'..='ㅣ').contains(&c)
}

fn is_vowel(c: char) -> bool {
    ('ㅏ'..='ㅣ').contains(&c)
}

/// 두벌식 자판에서 이 자모가 있는 키의 영문자입니다.
pub fn key_of(jamo: char) -> Option<char> {
    let found = KEYS.iter().find(|(_, of_key)| *of_key == jamo);
    found.map(|(key, _)| *key)
}

/// 음절을 첫소리, 모음, 받침으로 나눕니다. 음절이 아니면 `None`입니다.
fn split(syllable: char) -> Option<(char, char, Option<char>)> {
    let index = (syllable as u32).checked_sub(FIRST_SYLLABLE)?;
    let initial = *INITIALS.get((index / (VOWELS * FINAL_SLOTS)) as usize)?;
    let vowel = char::from_u32('ㅏ' as u32 + index / FINAL_SLOTS % VOWELS)?;
    let last = (index % FINAL_SLOTS).checked_sub(1);
    Some((initial, vowel, last.map(|at| FINALS[at as usize])))
}

/// 첫소리, 모음, 받침을 음절 하나로 합칩니다. `initial`과 `last`는 각각 `INITIALS`와 `FINALS`에 있는 자모입니다.
fn join(initial: char, vowel: char, last: Option<char>) -> char {
    let place =
        |list: &[char], c: char| list.iter().position(|&each| each == c).unwrap_or(0) as u32;
    let last = last.map_or(0, |last| place(&FINALS, last) + 1);
    let vowel = vowel as u32 - 'ㅏ' as u32;
    let index = (place(&INITIALS, initial) * VOWELS + vowel) * FINAL_SLOTS + last;
    char::from_u32(FIRST_SYLLABLE + index).unwrap_or(initial)
}

fn paired(pairs: &[(char, char, char)], first: char, second: char) -> Option<char> {
    let found = pairs.iter().find(|(a, b, _)| (*a, *b) == (first, second));
    found.map(|(_, _, both)| *both)
}

/// 마지막 글자 `last`의 뒤에 자모 `jamo`를 쳤을 때의 글자입니다. 합쳐지면 한 글자이고, 합쳐지지 않으면 두 글자입니다.
/// 받침의 뒤에 모음을 치면, 받침이 다음 음절의 첫소리로 넘어갑니다.
pub fn typed_after(last: char, jamo: char) -> (char, Option<char>) {
    let apart = (last, Some(jamo));
    match (split(last), is_vowel(jamo)) {
        (Some((initial, vowel, None)), true) => match paired(&VOWEL_PAIRS, vowel, jamo) {
            Some(both) => (join(initial, both, None), None),
            None => apart,
        },
        (Some((initial, vowel, Some(end))), true) => {
            let halves = FINAL_PAIRS.iter().find(|(_, _, both)| *both == end);
            let (stays, moves) = halves.map_or((None, end), |&(a, b, _)| (Some(a), b));
            (join(initial, vowel, stays), Some(join(moves, jamo, None)))
        }
        (Some((initial, vowel, None)), false) if FINALS.contains(&jamo) => {
            (join(initial, vowel, Some(jamo)), None)
        }
        (Some((initial, vowel, Some(end))), false) => match paired(&FINAL_PAIRS, end, jamo) {
            Some(both) => (join(initial, vowel, Some(both)), None),
            None => apart,
        },
        (None, true) if INITIALS.contains(&last) => (join(last, jamo, None), None),
        (None, true) => match paired(&VOWEL_PAIRS, last, jamo) {
            Some(both) => (both, None),
            None => apart,
        },
        _ => apart,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 자모를 차례로 쳤을 때의 글입니다.
    fn typed(jamos: &str) -> String {
        let mut text = String::new();
        for jamo in jamos.chars() {
            match text.pop() {
                Some(last) => {
                    let (first, second) = typed_after(last, jamo);
                    text.push(first);
                    text.extend(second);
                }
                None => text.push(jamo),
            }
        }
        text
    }

    #[test]
    fn a_consonant_a_vowel_and_a_consonant_make_one_syllable() {
        assert_eq!(typed("ㅎㅏㄴ"), "한");
        assert_eq!(typed("ㄱㅏㅇㄱㅣㅌㅐ"), "강기태");
    }

    #[test]
    fn a_vowel_after_a_final_consonant_takes_that_consonant() {
        assert_eq!(typed("ㅎㅏㄴㅏ"), "하나");
        assert_eq!(
            typed("ㄷㅏㄹㄱㅏ"),
            "달가",
            "a double final gives its second half"
        );
        assert_eq!(typed("ㄷㅏㄹㄱ"), "닭");
    }

    #[test]
    fn two_vowels_and_two_final_consonants_join_only_when_korean_has_the_pair() {
        assert_eq!(typed("ㄱㅗㅏ"), "과");
        assert_eq!(typed("ㅇㅡㅣ"), "의");
        assert_eq!(typed("ㄱㅏㅏ"), "가ㅏ");
        assert_eq!(typed("ㅇㅓㅂㅅㄷㅏ"), "없다");
        assert_eq!(typed("ㅇㅏㄸ"), "아ㄸ", "ㄸ is never a final");
    }

    #[test]
    fn jamo_without_a_partner_stay_as_they_are() {
        assert_eq!(typed("ㅋㅋㅋ"), "ㅋㅋㅋ");
        assert_eq!(typed("ㅗㅏ"), "ㅘ");
        assert_eq!(typed_after('K', 'ㅏ'), ('K', Some('ㅏ')));
    }

    #[test]
    fn a_jamo_gives_back_its_key() {
        assert_eq!(key_of('ㅎ'), Some('g'));
        assert_eq!(key_of('ㅃ'), Some('q'));
        assert_eq!(key_of('a'), None);
    }
}
