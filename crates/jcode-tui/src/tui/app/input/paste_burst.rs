//! Distinguish terminal-injected Enter storms from human key presses.
//!
//! Terminals without bracketed-paste support (legacy conhost, some SSH/tmux
//! setups) deliver a multiline clipboard paste as raw injected key events:
//! printable chars interleaved with `KeyCode::Enter` for every `\r` in the
//! payload. Without detection, the first injected newline submits the draft
//! mid-paste.
//!
//! Human typing cannot reproduce the event density of terminal injection:
//! injection floods events with sub-millisecond gaps, while even very fast
//! typists stay above ~30 ms between consecutive presses. Inter-event timing
//! is therefore a reliable classifier and keeps `App` untouched.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Maximum gap between two consecutive events for them to count as part of
/// the same injection burst.
const BURST_GAP: Duration = Duration::from_millis(12);

/// How far back the window reaches when counting recent burst events.
const BURST_WINDOW: Duration = Duration::from_millis(150);

/// Events that must land inside [`BURST_WINDOW`] before an Enter arriving in
/// the burst is classified as injected rather than typed.
const MIN_BURST_EVENTS: usize = 2;

/// Ring length: enough to cover the window at any realistic event rate.
const RING_LEN: usize = 16;

struct RawPasteRemainder {
    chars: VecDeque<char>,
    last_event: Instant,
}

thread_local! {
    static KEY_TIMES: std::cell::RefCell<VecDeque<Instant>> =
        const { std::cell::RefCell::new(VecDeque::new()) };
    static RAW_PASTE_REMAINDER: RefCell<Option<RawPasteRemainder>> = const { RefCell::new(None) };
}

/// Record one handled key event. Call exactly once per key event, before any
/// dispatch decision that consults [`enter_is_synthetic`].
pub(in crate::tui::app) fn note_key_event() {
    let now = Instant::now();
    KEY_TIMES.with(|cell| {
        let mut ring = cell.borrow_mut();
        ring.push_back(now);
        while ring.len() > RING_LEN {
            ring.pop_front();
        }
    });
}

/// True when the current Enter arrives inside a fast injection burst:
/// at most [`BURST_GAP`] since the previous event AND at least
/// [`MIN_BURST_EVENTS`] events within the trailing [`BURST_WINDOW`].
pub(in crate::tui::app) fn enter_is_synthetic() -> bool {
    KEY_TIMES.with(|cell| {
        let ring = cell.borrow();
        let Some(&last) = ring.iter().next_back() else {
            return false;
        };
        let Some(&prev) = ring.iter().rev().nth(1) else {
            return false;
        };
        if last.duration_since(prev) > BURST_GAP {
            return false;
        }
        let window_start = last.checked_sub(BURST_WINDOW);
        let in_window = ring
            .iter()
            .rev()
            .take_while(|&&t| window_start.is_none_or(|start| t >= start))
            .count();
        in_window >= MIN_BURST_EVENTS
    })
}

/// Record a key and consume a key belonging to an already recovered raw
/// paste. This is shared by local and remote dispatchers.
pub(super) fn observe_key(
    code: crossterm::event::KeyCode,
    modifiers: crossterm::event::KeyModifiers,
    text_input: Option<&str>,
) -> bool {
    if consume_raw_paste_event(code, modifiers, text_input) {
        return true;
    }
    note_key_event();
    false
}

/// Begin consuming the remainder of a paste that arrived as ordinary key
/// events. The first line has already reached the composer by the time this
/// recovery path is detected; the rest must never be dispatched as editor
/// input or Enter/submit events.
pub(super) fn begin_raw_paste_recovery(remainder: &str) {
    RAW_PASTE_REMAINDER.with(|cell| {
        if remainder.is_empty() {
            *cell.borrow_mut() = None;
            KEY_TIMES.with(|times| times.borrow_mut().clear());
            return;
        }
        *cell.borrow_mut() = Some(RawPasteRemainder {
            chars: remainder.chars().collect(),
            last_event: Instant::now(),
        });
    });
}

/// Consume one raw key event when it matches the recovered clipboard payload.
/// A mismatch means the paste has ended and the key belongs to the user.
///
/// `text_input` is preferred because enhanced keyboard protocols may attach
/// multiple Unicode characters to one event. The key-code fallback accepts
/// SHIFT on printable characters: Windows console paste commonly reports an
/// uppercase pasted character as `Char('A') + SHIFT`.
pub(super) fn consume_raw_paste_event(
    code: crossterm::event::KeyCode,
    modifiers: crossterm::event::KeyModifiers,
    text_input: Option<&str>,
) -> bool {
    let event_text = if let Some(text) = text_input.filter(|text| !text.is_empty()) {
        text.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        match code {
            crossterm::event::KeyCode::Char(c)
                if !modifiers.intersects(
                    crossterm::event::KeyModifiers::CONTROL
                        | crossterm::event::KeyModifiers::ALT
                        | crossterm::event::KeyModifiers::SUPER
                        | crossterm::event::KeyModifiers::HYPER
                        | crossterm::event::KeyModifiers::META,
                ) =>
            {
                c.to_string()
            }
            crossterm::event::KeyCode::Enter if modifiers.is_empty() => "\n".to_string(),
            crossterm::event::KeyCode::Tab if modifiers.is_empty() => "\t".to_string(),
            _ => return false,
        }
    };

    RAW_PASTE_REMAINDER.with(|cell| {
        let mut remainder = cell.borrow_mut();
        let Some(state) = remainder.as_mut() else {
            return false;
        };
        let event_chars: Vec<char> = event_text.chars().collect();
        if state.last_event.elapsed() > Duration::from_millis(300)
            || !state
                .chars
                .iter()
                .take(event_chars.len())
                .copied()
                .eq(event_chars.iter().copied())
        {
            *remainder = None;
            return false;
        }
        for _ in 0..event_chars.len() {
            state.chars.pop_front();
        }
        state.last_event = Instant::now();
        if state.chars.is_empty() {
            *remainder = None;
            KEY_TIMES.with(|times| times.borrow_mut().clear());
        }
        true
    })
}

/// Test hook: clear recorded timings so a subsequent classification starts
/// from an empty slate regardless of earlier test activity on this thread.
#[cfg(test)]
pub(in crate::tui::app) fn reset_for_test() {
    KEY_TIMES.with(|cell| cell.borrow_mut().clear());
    RAW_PASTE_REMAINDER.with(|cell| *cell.borrow_mut() = None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn recovered_remainder_consumes_newlines_blank_lines_and_tabs_atomically() {
        reset_for_test();
        begin_raw_paste_recovery("line 2\n\n\tline 4");

        for code in [
            KeyCode::Char('l'),
            KeyCode::Char('i'),
            KeyCode::Char('n'),
            KeyCode::Char('e'),
            KeyCode::Char(' '),
            KeyCode::Char('2'),
            KeyCode::Enter,
            KeyCode::Enter,
            KeyCode::Tab,
            KeyCode::Char('l'),
            KeyCode::Char('i'),
            KeyCode::Char('n'),
            KeyCode::Char('e'),
            KeyCode::Char(' '),
            KeyCode::Char('4'),
        ] {
            assert!(consume_raw_paste_event(code, KeyModifiers::NONE, None));
        }

        assert!(!consume_raw_paste_event(
            KeyCode::Char('x'),
            KeyModifiers::NONE,
            None
        ));
    }

    #[test]
    fn mismatched_key_ends_recovery_without_consuming_user_input() {
        reset_for_test();
        begin_raw_paste_recovery("remaining");

        assert!(!consume_raw_paste_event(
            KeyCode::Char('x'),
            KeyModifiers::NONE,
            None
        ));
        assert!(!consume_raw_paste_event(
            KeyCode::Char('r'),
            KeyModifiers::NONE,
            None
        ));
    }

    #[test]
    fn shifted_and_multi_character_events_match_recovered_text() {
        reset_for_test();
        begin_raw_paste_recovery("ABC unicode");

        assert!(consume_raw_paste_event(
            KeyCode::Char('A'),
            KeyModifiers::SHIFT,
            None
        ));
        assert!(consume_raw_paste_event(
            KeyCode::Char('x'),
            KeyModifiers::NONE,
            Some("BC unicode")
        ));
    }
}
