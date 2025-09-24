//! UAX #14 Line Breaking Types and Enums
//!
//! This module defines all the types, enums, and data structures used
//! for Unicode Line Breaking Algorithm implementation.

/// UAX #14 Line Breaking Classes (comprehensive)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LineBreakClass {
    // Basic classes
    BK = 1,   // Mandatory Break
    CR = 2,   // Carriage Return
    LF = 3,   // Line Feed
    CM = 4,   // Combining Mark
    NL = 5,   // Next Line
    SG = 6,   // Surrogate
    WJ = 7,   // Word Joiner
    ZW = 8,   // Zero Width Space
    GL = 9,   // Non-breaking Glue
    SP = 10,  // Space
    ZWJ = 11, // Zero Width Joiner

    // Letters and symbols
    AL = 12, // Alphabetic
    B2 = 13, // Break Opportunity Before and After
    BA = 14, // Break After
    BB = 15, // Break Before
    HY = 16, // Hyphen
    CB = 17, // Contingent Break Opportunity
    CL = 18, // Close Punctuation
    CP = 19, // Close Parenthesis
    EX = 20, // Exclamation/Interrogation
    IN = 21, // Inseparable
    NS = 22, // Nonstarter
    OP = 23, // Open Punctuation
    QU = 24, // Quotation
    IS = 25, // Infix Numeric Separator
    NU = 26, // Numeric
    PO = 27, // Postfix Numeric
    PR = 28, // Prefix Numeric
    SY = 29, // Symbols Allowing Break After

    // Complex scripts
    AI = 30, // Ambiguous
    CJ = 31, // Conditional Japanese Starter
    H2 = 32, // Hangul LV Syllable
    H3 = 33, // Hangul LVT Syllable
    HL = 34, // Hebrew Letter
    ID = 35, // Ideographic
    JL = 36, // Hangul L Jamo
    JV = 37, // Hangul V Jamo
    JT = 38, // Hangul T Jamo
    RI = 39, // Regional Indicator
    SA = 40, // South East Asian
    XX = 41, // Unknown

    // Extended classes for modern Unicode
    EB = 42, // Emoji Base
    EM = 43, // Emoji Modifier
    AK = 44, // Aksara
    AP = 45, // Aksara Prebase
    AS = 46, // Aksara Start
    VF = 47, // Virama Final
    VI = 48, // Virama
}

/// Break opportunity with contextual information
#[derive(Debug, Clone, Copy)]
pub struct BreakOpportunity {
    /// Character position in text
    pub position: usize,
    /// Break classification
    pub break_class: BreakClass,
    /// Priority for break selection
    pub priority: BreakPriority,
    /// Penalty for breaking at this position
    pub penalty: f32,
}

/// Break classification for opportunity selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BreakClass {
    /// Breaking is prohibited
    Prohibited = 0,
    /// Breaking is allowed
    Allowed = 1,
    /// Breaking is required (mandatory)
    Mandatory = 2,
    /// Break depends on context
    Indirect = 3,
}

/// Break priority for intelligent selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum BreakPriority {
    /// Low priority break (avoid if possible)
    Low = 1,
    /// Normal priority break
    Normal = 2,
    /// High priority break (prefer)
    High = 3,
    /// Mandatory break (must use)
    Mandatory = 4,
}

/// Context state for UAX #14 rule application
#[derive(Debug, Clone)]
pub struct BreakContextState {
    /// Previous character's line break class
    pub prev_class: LineBreakClass,
    /// Current character's line break class  
    pub curr_class: LineBreakClass,
    /// Look-ahead context (fixed-size circular buffer)
    pub lookahead: [LineBreakClass; 8],
    /// Current position in lookahead buffer
    pub lookahead_pos: usize,
    /// Regional indicator sequence state
    pub ri_sequence_odd: bool,
    /// Zero-width joiner context
    pub zwj_context: bool,
}

impl BreakContextState {
    pub fn new() -> Self {
        Self {
            prev_class: LineBreakClass::XX,
            curr_class: LineBreakClass::XX,
            lookahead: [LineBreakClass::XX; 8],
            lookahead_pos: 0,
            ri_sequence_odd: false,
            zwj_context: false,
        }
    }

    pub fn reset(&mut self) {
        self.prev_class = LineBreakClass::XX;
        self.curr_class = LineBreakClass::XX;
        self.lookahead.fill(LineBreakClass::XX);
        self.lookahead_pos = 0;
        self.ri_sequence_odd = false;
        self.zwj_context = false;
    }

    pub fn advance(&mut self, next_class: LineBreakClass) {
        self.prev_class = self.curr_class;
        self.curr_class = next_class;

        // Update regional indicator sequence state
        if next_class == LineBreakClass::RI {
            self.ri_sequence_odd = !self.ri_sequence_odd;
        } else if next_class != LineBreakClass::CM {
            self.ri_sequence_odd = false;
        }

        // Update zero-width joiner context
        self.zwj_context = next_class == LineBreakClass::ZWJ;

        // Update lookahead buffer
        self.lookahead[self.lookahead_pos] = next_class;
        self.lookahead_pos = (self.lookahead_pos + 1) % self.lookahead.len();
    }
}

/// Extension trait for character classification
pub trait CharacterExtensions {
    fn is_ideographic(&self) -> bool;
}

impl CharacterExtensions for char {
    fn is_ideographic(&self) -> bool {
        // CJK Unified Ideographs and related blocks
        matches!(*self,
            '\u{4E00}'..='\u{9FFF}' |   // CJK Unified Ideographs
            '\u{3400}'..='\u{4DBF}' |   // CJK Extension A
            '\u{20000}'..='\u{2A6DF}' | // CJK Extension B
            '\u{2A700}'..='\u{2B73F}' | // CJK Extension C
            '\u{2B740}'..='\u{2B81F}' | // CJK Extension D
            '\u{2B820}'..='\u{2CEAF}' | // CJK Extension E
            '\u{2CEB0}'..='\u{2EBEF}'   // CJK Extension F
        )
    }
}
