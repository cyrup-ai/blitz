//! UAX #14 rule application and break determination
//!
//! This module implements the core UAX #14 pair table rules for
//! determining break opportunities between character pairs.

use super::character_classification::CharacterExtensions;
use super::types::{BreakClass, BreakContextState, BreakPriority, LineBreakClass};

/// Apply UAX #14 pair table rules for break determination
pub fn apply_pair_table_rules(
    pos: usize,
    char_classes: &[LineBreakClass],
    context_state: &BreakContextState,
) -> BreakClass {
    let prev = context_state.prev_class;
    let curr = context_state.curr_class;

    // Core UAX #14 rules (simplified implementation)
    match (prev, curr) {
        // LB2: Never break at the start of text
        _ if pos == 0 => BreakClass::Prohibited,

        // LB3: Always break at the end of text
        _ if pos == char_classes.len() - 1 => BreakClass::Mandatory,

        // LB5: Treat CR LF as unbreakable (must come before general CR rule)
        (LineBreakClass::CR, LineBreakClass::LF) => BreakClass::Prohibited,

        // LB1: Assign BK to any character following BK, CR, LF, or NL
        (LineBreakClass::BK, _)
        | (LineBreakClass::CR, _)
        | (LineBreakClass::LF, _)
        | (LineBreakClass::NL, _) => BreakClass::Mandatory,

        // LB6: Don't break before hard line breaks
        (_, LineBreakClass::BK) | (_, LineBreakClass::LF) | (_, LineBreakClass::NL) => {
            BreakClass::Prohibited
        }

        // LB8: Break after zero-width space (must come before general ZW rule)
        (LineBreakClass::ZW, _) => BreakClass::Allowed,

        // LB7: Don't break before spaces or zero-width space
        (_, LineBreakClass::SP) | (_, LineBreakClass::ZW) => BreakClass::Prohibited,

        // LB9: Don't break a combining character sequence
        (_, LineBreakClass::CM) | (_, LineBreakClass::ZWJ) => BreakClass::Prohibited,

        // LB10: Don't break before or after Word Joiner
        (LineBreakClass::WJ, _) | (_, LineBreakClass::WJ) => BreakClass::Prohibited,

        // LB11: Don't break before or after GL
        (LineBreakClass::GL, _) | (_, LineBreakClass::GL) => BreakClass::Prohibited,

        // LB13: Don't break before ']' or '!' or ';' or '/'
        (_, LineBreakClass::CL)
        | (_, LineBreakClass::CP)
        | (_, LineBreakClass::EX)
        | (_, LineBreakClass::IS)
        | (_, LineBreakClass::SY) => BreakClass::Prohibited,

        // LB14: Don't break after '[' or '('
        (LineBreakClass::OP, _) => BreakClass::Prohibited,

        // LB15: Don't break within '"aaa"'
        (LineBreakClass::QU, LineBreakClass::OP) => BreakClass::Prohibited,
        (LineBreakClass::CL, LineBreakClass::QU) => BreakClass::Prohibited,

        // LB16: Don't break between closing punctuation and non-starters
        (LineBreakClass::CL, LineBreakClass::NS) | (LineBreakClass::CP, LineBreakClass::NS) => {
            BreakClass::Prohibited
        }

        // LB17: Don't break within 'a9', '3a', or 'H%'
        (LineBreakClass::B2, LineBreakClass::B2) => BreakClass::Prohibited,

        // LB18: Break after spaces
        (LineBreakClass::SP, _) => BreakClass::Allowed,

        // LB19: Don't break before or after quotation marks
        (_, LineBreakClass::QU) | (LineBreakClass::QU, _) => BreakClass::Prohibited,

        // LB20: Break before and after unresolved CB
        (_, LineBreakClass::CB) | (LineBreakClass::CB, _) => BreakClass::Allowed,

        // LB21: Don't break before hyphen-minus, other hyphens, or after BA, HY, NS
        (_, LineBreakClass::BA)
        | (_, LineBreakClass::HY)
        | (_, LineBreakClass::NS)
        | (LineBreakClass::BB, _) => BreakClass::Prohibited,

        // LB22: Don't break between two ellipses, or between letters, numbers or exclamations and ellipses
        (LineBreakClass::AL, LineBreakClass::IN)
        | (LineBreakClass::HL, LineBreakClass::IN)
        | (LineBreakClass::EX, LineBreakClass::IN)
        | (LineBreakClass::ID, LineBreakClass::IN)
        | (LineBreakClass::EB, LineBreakClass::IN)
        | (LineBreakClass::EM, LineBreakClass::IN)
        | (LineBreakClass::IN, LineBreakClass::IN)
        | (LineBreakClass::NU, LineBreakClass::IN) => BreakClass::Prohibited,

        // Complex script rules (simplified)
        (LineBreakClass::ID, LineBreakClass::ID) => BreakClass::Allowed,
        (LineBreakClass::AL, LineBreakClass::AL) => BreakClass::Allowed,
        (LineBreakClass::NU, LineBreakClass::NU) => BreakClass::Allowed,

        // Default case: allow breaking
        _ => BreakClass::Allowed,
    }
}

/// Calculate break priority based on character properties
pub fn calculate_break_priority(char_class: LineBreakClass, ch: &char) -> BreakPriority {
    match char_class {
        LineBreakClass::BK | LineBreakClass::CR | LineBreakClass::LF | LineBreakClass::NL => {
            BreakPriority::Mandatory
        }
        LineBreakClass::SP | LineBreakClass::ZW => BreakPriority::High,
        LineBreakClass::BA | LineBreakClass::HY => BreakPriority::Normal,
        LineBreakClass::ID if ch.is_ideographic() => BreakPriority::Normal,
        _ => BreakPriority::Low,
    }
}

/// Calculate break penalty for intelligent line breaking
pub fn calculate_break_penalty(pos: usize, char_classes: &[LineBreakClass]) -> f32 {
    let mut penalty = 0.0;

    // Penalize breaking in the middle of words
    if pos > 0 && pos < char_classes.len() - 1 {
        let prev = char_classes[pos - 1];
        let next = char_classes[pos + 1];

        // High penalty for breaking between letters
        if matches!(prev, LineBreakClass::AL | LineBreakClass::HL)
            && matches!(next, LineBreakClass::AL | LineBreakClass::HL)
        {
            penalty += 100.0;
        }

        // Moderate penalty for breaking between numbers
        if prev == LineBreakClass::NU && next == LineBreakClass::NU {
            penalty += 50.0;
        }
    }

    penalty
}
