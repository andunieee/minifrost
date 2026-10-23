//! The "check backup" quiz. Ported from frostsnap's device UI
//! (`frostsnap_widgets::backup::check_backup`): the user proves their paper backup is readable
//! and correct by answering 26 multiple-choice questions, one for the key number and one for each
//! of the 25 words. Each question has three options; the wrong words are the two BIP39 words
//! closest to the right one, so a smudged or mis-copied word is likely to be caught.
//!
//! The signer decrypts its own share to build the expected answers, so nothing the coordinator
//! sends decides what counts as correct.

use frost_backup::{bip39_words::BIP39_WORDS, ShareBackup};
use rand::{seq::SliceRandom, RngCore};

/// 1 question for the key number + 25 words.
pub const TOTAL_SCREENS: usize = 26;
const NUM_OPTIONS: usize = 3;

#[derive(Debug, Clone)]
pub struct Question {
    pub title: String,
    pub options: Vec<String>,
    correct: usize,
}

pub enum Answer {
    Wrong,
    Next,
    Done,
}

pub struct Quiz {
    word_indices: [u16; 25],
    share_index: u32,
    screen: usize,
    current: Question,
}

impl Quiz {
    pub fn new(backup: &ShareBackup, rng: &mut impl RngCore) -> Self {
        let share_index: u32 =
            backup.index().try_into().expect("share indices of real wallets fit in a u32");
        let word_indices = backup.to_word_indices();
        let current = question(0, share_index, &word_indices, rng);
        Self { word_indices, share_index, screen: 0, current }
    }

    pub fn question(&self) -> &Question {
        &self.current
    }

    /// 0-based position of the current question, for a progress bar.
    pub fn screen(&self) -> usize {
        self.screen
    }

    pub fn answer(&mut self, choice: usize, rng: &mut impl RngCore) -> Answer {
        if choice != self.current.correct {
            return Answer::Wrong;
        }
        self.screen += 1;
        if self.screen >= TOTAL_SCREENS {
            return Answer::Done;
        }
        self.current = question(self.screen, self.share_index, &self.word_indices, rng);
        Answer::Next
    }
}

fn question(screen: usize, share_index: u32, words: &[u16; 25], rng: &mut impl RngCore) -> Question {
    let (title, mut options, correct) = if screen == 0 {
        // Neighbouring key numbers, never 0 (share indices start at 1).
        let mut options = vec![share_index];
        let mut offset = 1u32;
        while options.len() < NUM_OPTIONS {
            if share_index > offset {
                options.push(share_index - offset);
            }
            if options.len() < NUM_OPTIONS {
                options.push(share_index + offset);
            }
            offset += 1;
        }
        let labels = options.iter().map(|i| format!("#{i}")).collect::<Vec<_>>();
        ("Select key number".to_string(), labels, format!("#{share_index}"))
    } else {
        let right = words[screen - 1];
        let [a, b] = closest_distractors(right);
        let options = [right, a, b].iter().map(|&i| BIP39_WORDS[i as usize].to_string()).collect();
        (format!("Select word {screen}"), options, BIP39_WORDS[right as usize].to_string())
    };
    options.shuffle(rng);
    let correct = options.iter().position(|o| *o == correct).expect("correct option is present");
    Question { title, options, correct }
}

fn levenshtein(a: &[u8], b: &[u8]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        curr[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

fn shared_suffix_len(a: &[u8], b: &[u8]) -> usize {
    a.iter().rev().zip(b.iter().rev()).take_while(|(x, y)| x == y).count()
}

/// Same scoring as the device: edit distance counts most, a shared ending helps.
fn distractor_score(target: &[u8], candidate: &[u8]) -> i32 {
    levenshtein(target, candidate) as i32 * 3 - shared_suffix_len(target, candidate) as i32 * 2
}

fn closest_distractors(correct: u16) -> [u16; 2] {
    let target = BIP39_WORDS[correct as usize].as_bytes();
    let mut best = [(i32::MAX, 0u16); 2];
    for (i, word) in BIP39_WORDS.iter().enumerate() {
        if i as u16 == correct {
            continue;
        }
        let score = distractor_score(target, word.as_bytes());
        if score < best[0].0 {
            best[1] = best[0];
            best[0] = (score, i as u16);
        } else if score < best[1].0 {
            best[1] = (score, i as u16);
        }
    }
    [best[0].1, best[1].1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distractors_are_close_and_distinct() {
        for correct in [0u16, 5, 1000, 2047] {
            let [a, b] = closest_distractors(correct);
            assert!(a != correct && b != correct && a != b);
        }
    }
}
