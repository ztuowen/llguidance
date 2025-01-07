use anyhow::Result;
use std::fmt::Debug;
use toktrie::SimpleVob;

use crate::api::ParserLimits;

use super::{
    lexerspec::{LexemeIdx, LexerSpec},
    regexvec::{NextByte, RegexVec, StateDesc},
};

const DEBUG: bool = true;

macro_rules! debug {
    ($($arg:tt)*) => {
        if cfg!(feature = "logging") && DEBUG {
            eprintln!($($arg)*);
        }
    }
}

#[derive(Clone)]
pub struct Lexer {
    pub(crate) dfa: RegexVec,
    // set of bytes that are allowed in any of the lexemes
    // this is used to fail states quickly
    allowed_first_byte: SimpleVob,
    spec: LexerSpec,
}

pub type StateID = derivre::StateID;

/// PreLexeme contains index of the lexeme but not the bytes.
#[derive(Debug, Clone, Copy)]
pub struct PreLexeme {
    pub idx: LexemeIdx,
    pub byte: Option<u8>,
    /// Does the 'byte' above belong to the next lexeme?
    pub byte_next_row: bool,
    /// Length in bytes of the hidden part of the lexeme.
    pub hidden_len: usize,
}

#[derive(Debug)]
pub enum LexerResult {
    Lexeme(PreLexeme),
    SpecialToken(StateID),
    State(StateID, u8),
    Error,
}

impl Lexer {
    pub fn from(spec: &LexerSpec, limits: &mut ParserLimits, dbg: bool) -> Result<Self> {
        let mut dfa = spec.to_regex_vec(limits)?;

        if dbg {
            debug!("lexer: {:?}\n  ==> dfa: {:?}", spec, dfa);
        }

        let s0 = dfa.initial_state(&spec.all_lexemes());
        let mut allowed_first_byte = SimpleVob::alloc(256);
        for i in 0..=255 {
            if !dfa.transition(s0, i).is_dead() {
                allowed_first_byte.allow_token(i as u32);
            }
        }

        let lex = Lexer {
            dfa,
            allowed_first_byte,
            spec: spec.clone(), // TODO check perf of Rc<> ?
        };

        Ok(lex)
    }

    pub fn lexer_spec(&self) -> &LexerSpec {
        &self.spec
    }

    pub fn start_state(&mut self, allowed_lexemes: &SimpleVob) -> StateID {
        self.dfa.initial_state(allowed_lexemes)
    }

    pub fn transition_start_state(&mut self, s: StateID, first_byte: Option<u8>) -> StateID {
        first_byte.map(|b| self.dfa.transition(s, b)).unwrap_or(s)
    }

    pub fn a_dead_state(&self) -> StateID {
        StateID::DEAD
    }

    pub fn possible_hidden_len(&mut self, state: StateID) -> usize {
        self.dfa.possible_lookahead_len(state)
    }

    fn state_info(&self, state: StateID) -> &StateDesc {
        self.dfa.state_desc(state)
    }

    pub fn allows_eos(&mut self, state: StateID) -> bool {
        let mut l = self.spec.eos_ending_lexemes();
        l.and(&self.state_info(state).accepting);
        !l.is_zero()
    }

    pub fn limit_state_to(&mut self, state: StateID, allowed_lexemes: &SimpleVob) -> StateID {
        self.dfa.limit_state_to(state, allowed_lexemes)
    }

    pub fn possible_lexemes(&self, state: StateID) -> &SimpleVob {
        &self.state_info(state).possible
    }

    pub fn force_lexeme_end(&self, prev: StateID) -> LexerResult {
        let info = self.state_info(prev);
        match info.possible.first_bit_set() {
            Some(idx) => LexerResult::Lexeme(PreLexeme {
                idx: LexemeIdx::new(idx),
                byte: None,
                byte_next_row: false,
                hidden_len: 0,
            }),
            None => LexerResult::Error,
        }
    }

    pub fn try_lexeme_end(&mut self, prev: StateID) -> LexerResult {
        if let Some(idx) = self.state_info(prev).lowest_accepting {
            LexerResult::Lexeme(PreLexeme {
                idx: LexemeIdx::new(idx),
                byte: None,
                byte_next_row: false,
                hidden_len: 0,
            })
        } else {
            LexerResult::Error
        }
    }

    pub fn check_for_single_byte_lexeme(&mut self, state: StateID, b: u8) -> Option<PreLexeme> {
        if self.dfa.next_byte(state) == NextByte::ForcedEOI {
            let info = self.state_info(state);
            let idx = info.possible.first_bit_set().expect("no allowed lexemes");
            Some(PreLexeme {
                idx: LexemeIdx::new(idx),
                byte: Some(b),
                byte_next_row: false,
                hidden_len: 0,
            })
        } else {
            None
        }
    }

    pub fn subsume_possible(&mut self, state: StateID) -> bool {
        self.dfa.subsume_possible(state)
    }

    pub fn check_subsume(&mut self, state: StateID, extra_idx: usize, budget: u64) -> Result<bool> {
        self.dfa
            .check_subsume(state, self.spec.extra_lexeme(extra_idx).as_usize(), budget)
    }

    #[inline(always)]
    pub fn advance(&mut self, prev: StateID, byte: u8, enable_logging: bool) -> LexerResult {
        let state = self.dfa.transition(prev, byte);

        if enable_logging {
            let info = self.state_info(state);
            debug!(
                "lex: {:?} -{:?}-> {:?}, acpt={:?}",
                prev, byte as char, state, info.lowest_accepting
            );
        }

        if state.is_dead() {
            // if the left-over byte is not allowed as the first byte of any lexeme, we can fail early
            if !self.allowed_first_byte.is_allowed(byte as u32) {
                return LexerResult::Error;
            }
            let info = self.dfa.state_desc(prev);
            // we take the first token that matched
            // (eg., "while" will match both keyword and identifier, but keyword is first)
            if let Some(idx) = info.lowest_accepting {
                LexerResult::Lexeme(PreLexeme {
                    idx: LexemeIdx::new(idx),
                    byte: Some(byte),
                    byte_next_row: true,
                    hidden_len: 0,
                })
            } else {
                LexerResult::Error
            }
        } else if state.has_lowest_match() {
            if let Some((idx, hidden_len)) = self.dfa.lowest_match(state) {
                if self.dfa.state_desc(state).has_special_token {
                    return LexerResult::SpecialToken(state);
                }
                LexerResult::Lexeme(PreLexeme {
                    idx: LexemeIdx::new(idx),
                    byte: Some(byte),
                    byte_next_row: false,
                    hidden_len,
                })
            } else {
                unreachable!()
            }
        } else {
            LexerResult::State(state, byte)
        }
    }
}

impl LexerResult {
    #[inline(always)]
    pub fn is_error(&self) -> bool {
        matches!(self, LexerResult::Error)
    }
}
