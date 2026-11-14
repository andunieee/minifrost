use minifrost_signer::api::backup_word_suggestions;
use minifrost_ui::prelude::*;

use crate::{
    ctl::Ctl,
    nav::{
        self,
        Nav,
    },
};

const WORDS: usize = 25;

/// Type in a physical backup: the key number and the 25 words, in order. The signer checks the
/// words' own checksums, stores the backup and shows its restore code for the coordinator.
#[derive(PartialEq)]
pub struct EnterBackupScreen;

impl Component for EnterBackupScreen {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let ctl = use_consume::<Ctl>();
        let nav = use_consume::<Nav>();
        let mut number = use_state(String::new);
        let words = use_hook(|| (0..WORDS).map(|_| State::create(String::new())).collect::<Vec<_>>());
        let mut error = use_state(|| None::<String>);
        let mut busy = use_state(|| false);

        // several words pasted into one field fill the fields from the first one on
        let fill = {
            let words = words.clone();
            move |pasted: &str| {
                for (field, word) in words.iter().zip(pasted.split_whitespace()) {
                    let mut field = *field;
                    field.set(word.to_uppercase());
                }
            }
        };

        let submit = {
            let words = words.clone();
            move |_| {
                let Some(index) = number.peek().trim().parse::<u32>().ok().filter(|i| *i >= 1) else {
                    error.set(Some("Enter the key number written on the backup (1 or higher).".into()));
                    return;
                };
                let entered: Vec<String> = words.iter().map(|w| w.peek().trim().to_string()).collect();
                if let Some(missing) = entered.iter().position(|w| w.is_empty()) {
                    error.set(Some(format!("Word {} is missing.", missing + 1)));
                    return;
                }
                error.set(None);
                busy.set(true);
                let ctl = ctl.clone();
                run(async move {
                    match ctl.submit_backup(index, entered).await {
                        Ok(r) => nav::next(nav, Ok(r)).await,
                        Err(e) => error.set(Some(e)),
                    }
                    busy.set(false);
                });
            }
        };

        rect().expanded().child(app_bar(&p, "Enter your backup", back(move |_| nav.pop()))).child(
            page(560.)
                .child(text("Type the key number and the 25 words exactly as written on your backup, in order."))
                .child(field(
                    &p,
                    "Key number",
                    Input::new(WritableSetter::new(number, move |v: String| {
                        number.set(v.chars().filter(char::is_ascii_digit).collect())
                    }))
                    .placeholder("#")
                    .width(Size::fill()),
                ))
                .children(words.iter().enumerate().map(|(i, w)| {
                    WordField {
                        number: i + 1,
                        word: *w,
                        fill: EventHandler::new({
                            let fill = fill.clone();
                            move |text: String| fill(&text)
                        }),
                    }
                    .into_element()
                }))
                .map(error.read().clone(), |el, e| el.child(text(e).color(p.error)))
                .child(Btn::new("Check backup").fill().busy(busy()).on_press(submit)),
        )
    }
}

/// One word of the backup, with the BIP39 words it could still be.
#[derive(PartialEq)]
struct WordField {
    number: usize,
    word: State<String>,
    fill: EventHandler<String>,
}

impl Component for WordField {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let mut word = self.word;
        let fill = self.fill.clone();
        let typed = word.read().trim().to_string();
        let suggestions: Vec<String> = if typed.is_empty() {
            vec![]
        } else {
            backup_word_suggestions(typed.clone()).into_iter().take(6).collect()
        };
        // nothing to suggest once the word is complete
        let suggestions = if suggestions.len() == 1 && suggestions[0] == typed { vec![] } else { suggestions };
        let only = (suggestions.len() == 1).then(|| suggestions[0].clone());

        rect()
            .width(Size::fill())
            .spacing(4.)
            .child(field(
                &p,
                format!("Word {}", self.number),
                Input::new(WritableSetter::new(word, move |v: String| {
                    if v.trim().contains(char::is_whitespace) {
                        fill.call(v);
                    } else {
                        word.set(v.to_uppercase());
                    }
                }))
                .width(Size::fill())
                // a single remaining suggestion is taken on Enter
                .on_submit(move |_| {
                    if let Some(only) = &only {
                        word.set(only.clone());
                    }
                }),
            ))
            .maybe(!suggestions.is_empty(), |el| {
                el.child(
                    rect()
                        .horizontal()
                        .width(Size::fill())
                        .content(Content::wrap_spacing(6.))
                        .spacing(6.)
                        .children(suggestions.into_iter().map(|s| {
                            rect()
                                .padding((6., 10.))
                                .corner_radius(8.)
                                .background(p.accent_soft)
                                .color(p.on_accent_soft)
                                .a11y_role(AccessibilityRole::Button)
                                .a11y_focusable(true)
                                .on_press({
                                    let s = s.clone();
                                    move |_| word.set(s.clone())
                                })
                                .child(mono(s.clone(), 14.))
                                .key(s)
                                .into_element()
                        })),
                )
            })
    }
}
