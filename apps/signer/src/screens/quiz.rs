use std::time::Duration;

use minifrost_signer::api::QuizQuestion;
use minifrost_ui::prelude::*;

use crate::{
    ctl::Ctl,
    nav::{
        self,
        Nav,
    },
};

/// The backup check: 26 multiple-choice questions answered from the paper backup (the key number,
/// then each of the 25 words). A wrong answer changes nothing; the same question stays up.
#[derive(PartialEq)]
pub struct QuizScreen {
    pub key_name: String,
}

impl Component for QuizScreen {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let ctl = use_consume::<Ctl>();
        let nav = use_consume::<Nav>();
        let mut question = use_state({
            let ctl = ctl.clone();
            move || ctl.quiz_question()
        });
        // the option just pressed, and whether it was right (None while checking)
        let mut flashed = use_state(|| None::<(usize, Option<bool>)>);
        let mut error = use_state(|| None::<String>);

        let answer = move |choice: usize| {
            if flashed.peek().is_some() {
                return;
            }
            flashed.set(Some((choice, None)));
            let ctl = ctl.clone();
            run(async move {
                match ctl.quiz_answer(choice as u32).await {
                    Ok(a) => {
                        flashed.set(Some((choice, Some(a.correct))));
                        timer(Duration::from_millis(if a.correct { 350 } else { 1200 })).await;
                        if let Some(done) = a.done {
                            nav::next(nav, Ok(done)).await;
                            return;
                        }
                        if let Some(q) = a.next {
                            question.set(Ok(q));
                        }
                    }
                    Err(e) => error.set(Some(e)),
                }
                flashed.set(None);
            });
        };

        let body: Element = match &*question.read() {
            Err(e) => rect().expanded().center().child(text(e.clone())).into(),
            Ok(q) => questions(&p, q, *flashed.read(), answer).into(),
        };

        let back_ctl = use_consume::<Ctl>();
        rect()
            .expanded()
            .child(app_bar(
                &p,
                format!("Check backup · {}", self.key_name),
                back(move |_| {
                    back_ctl.reject();
                    nav.home();
                }),
            ))
            .child(
                page(560.)
                    .padding(24.)
                    .child(body)
                    .map(error.read().clone(), |el, e| el.child(text(e).color(p.error))),
            )
    }
}

fn questions(p: &Palette, q: &QuizQuestion, flashed: Option<(usize, Option<bool>)>, answer: impl FnMut(usize) + Clone + 'static) -> Rect {
    rect()
        .width(Size::fill())
        .spacing(12.)
        .child(progress_line(p, q.screen as f32 / q.total as f32))
        .child(small(format!("{} of {}", q.screen + 1, q.total), p.text_muted).text_align(TextAlign::Center))
        .child(heading(q.title.clone()).text_align(TextAlign::Center))
        .child(text("Look at your paper backup and pick what it says.").text_align(TextAlign::Center))
        .child(gap(12.))
        .children(q.options.iter().enumerate().map(|(i, o)| {
            let mut answer = answer.clone();
            let state = flashed.filter(|(f, _)| *f == i).map(|(_, right)| right);
            let (background, color) = match state {
                Some(Some(true)) => (p.success, Color::WHITE),
                Some(Some(false)) => (p.error, p.on_error),
                _ => (p.accent_soft, p.on_accent_soft),
            };
            rect()
                .width(Size::fill())
                .height(Size::px(64.))
                .center()
                .corner_radius(32.)
                .background(background)
                .color(color)
                .a11y_role(AccessibilityRole::Button)
                .a11y_focusable(true)
                .maybe(flashed.is_some() && state.is_none(), |el| el.opacity(0.6))
                .on_press(move |_| answer(i))
                .child(mono(o.clone(), 22.).font_weight(FontWeight::BOLD))
                .key(i)
                .into_element()
        }))
}
