use std::array::IntoIter;
use std::cmp::Ordering;
use std::iter::Flatten;
use std::ops::Add;
use std::rc::Rc;

use super::contexts::FormattingContextState;
use super::parent_pointer_tree::*;
use crate::lang::ReconstructionSettings;

/// When traversing the solution space, some decisions have requirements which
/// limit its possibilities.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DecisionRequirement {
    /// There is no restriction on the next decision, it can be either a
    /// [`Break`](RawDecision::Break) or a
    /// [`Continue`](RawDecision::Continue).
    #[default]
    Indifferent,

    /// There are no possible decisions that can come next.
    Invalid,

    /// The next decision can only be a [`Break`](RawDecision::Break).
    MustBreak,

    /// The next decision can only be a [`Continue`](RawDecision::Continue).
    MustNotBreak,
}
impl DecisionRequirement {
    pub(super) fn map_can_break(self, can_break: bool) -> Self {
        match (self, can_break) {
            (Self::MustBreak, false) => Self::Invalid,
            (Self::Indifferent, false) => Self::MustNotBreak,
            _ => self,
        }
    }
}

/// When a decision is made, it has no knowledge of the surrounding code. It
/// only knows if it is breaking or not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RawDecision {
    Break,
    Continue,
}

impl RawDecision {
    pub(super) fn with_continuation(self, continuations: u16) -> Decision {
        match self {
            Self::Break => Decision::Break { continuations },
            Self::Continue => Decision::Continue,
        }
    }
}

/// Once a decision is made and the impact on the rest of the formatting has
/// been updated, the continuation level can then be known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Decision {
    /// Put a line break before a token.
    ///
    /// After the first decision, subsequent breaks will have an associated
    /// continuation.
    Break { continuations: u16 },

    /// Remove all line breaks from before a token
    Continue,
}

impl Decision {
    pub(super) fn to_raw(self) -> RawDecision {
        match self {
            Self::Break { continuations: _ } => RawDecision::Break,
            Self::Continue => RawDecision::Continue,
        }
    }
}

/// The decision for the first token in a line is slightly different to that of
/// a [`Decision`].
/// - By definition, there can't be any
///   [`continuations`](LineWhitespace::continuation) for the first token in a
///   line.
/// - If the first decision is to be [`Decision::Continue`], the previous (or
///   parent) line's length needs to be known to uphold the line length limit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FirstDecision {
    /// The logical line starts with a newline, i.e., it is on its own line.
    Break,

    /// The logical line start by continuing the previous line.
    ///
    /// Used in the formatting of child lines, the current length of the line is
    /// used to inform the solution. Additionally whether the whole line can
    /// break.
    Continue { line_length: u32, can_break: bool },
}

/// Used to represent the global indentation and continuation for a line.
///
/// There can be both `indentation`s and `continuation`s when the line in
/// question is a child of another. In that case, the `continuation`s relate to
/// the parent lines' decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct LineWhitespace {
    pub(super) indentations: u16,
    pub(super) continuations: u16,
}

impl Add for LineWhitespace {
    type Output = LineWhitespace;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            indentations: self.indentations + rhs.indentations,
            continuations: self.continuations + rhs.continuations,
        }
    }
}

impl LineWhitespace {
    pub(super) fn len(&self, recon_settings: &ReconstructionSettings) -> u32 {
        self.indentations as u32 * recon_settings.get_indentation_str().len() as u32
            + self.continuations as u32 * recon_settings.get_continuation_str().len() as u32
    }

    pub(super) fn zero() -> Self {
        Self {
            indentations: 0,
            continuations: 0,
        }
    }
}

/// `FormattingNode`s are the nodes within the solution space. It represents a
/// partial solution, with decisions made for only a certain number of tokens.
///
/// Once a node is found with a decision for each token in the line, it is
/// converted into a [`FormattingSolution`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FormattingNode<'a> {
    /// The global position of the line. Continued lines are based of this.
    pub(super) starting_ws: LineWhitespace,

    /// The most recent [`TokenDecision`] built atop all previous decisions for
    /// this node.
    pub(super) decision: NodeRef<'a, TokenDecision>,

    /// The line index for the next token for a decision to be made for.
    pub(super) next_line_index: u32,

    /// While exploring these [`FormattingNode`]s, the decisions made affect
    /// contexts' [`FormattingContextState`]. This is to ensure consistency
    /// across all decisions made in a line.
    ///
    /// This contains a [`FormattingContextState`] for each
    /// [`FormattingContext`](super::contexts::FormattingContext) in the line.
    /// That way, the contexts and their data can be paired to facilitate
    /// modification.
    ///
    /// Whenever a successor [`FormattingNode`] is constructed, this whole
    /// struct is duplicated. By wrapping this data in an [`Rc`], it defers the
    /// cloning of the data until there is a change. This also means that nodes
    /// that aren't explored further, will not unnecessarily clone the data.
    pub(super) context_data: Rc<Vec<FormattingContextState>>,

    /// The total penalty for formatting the line with the given decisions.
    pub(super) penalty: u64,
}

impl Ord for FormattingNode<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        /*
            Ordered by increasing preference.
            Lower penalty and higher decision count is preferable because a
            solution with a lower penalty is the better solution and a higher
            decision count makes it closer to being a complete solution.
        */
        (other.penalty)
            .cmp(&self.penalty)
            .then_with(|| self.next_line_index.cmp(&other.next_line_index))
    }
}

impl PartialOrd for FormattingNode<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The final output when formatting a logical line. A decision for every token
/// has been made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FormattingSolution {
    pub(super) starting_ws: LineWhitespace,
    pub(super) decisions: Vec<TokenDecision>,
    pub(super) penalty: u64,
    pub(super) solution_length: u32,
}

impl From<FormattingNode<'_>> for FormattingSolution {
    fn from(value: FormattingNode<'_>) -> Self {
        let mut decisions: Vec<_> = value
            .decision
            .walk_parents_data()
            .map(|d| d.clone())
            .collect();
        decisions.reverse();

        FormattingSolution {
            starting_ws: value.starting_ws,
            decisions,
            penalty: value.penalty,
            solution_length: value.decision.get().last_line_length,
        }
    }
}

/// In a [`FormattingSolution`], there is a [`TokenDecision`] for each token in
/// the line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TokenDecision {
    pub(super) decision: Decision,
    pub(super) requirement: DecisionRequirement,
    pub(super) child_solutions: Vec<(usize, FormattingSolution)>,
    pub(super) last_line_length: u32,
}

/// Provides an easy way to represent 0-2 possible values and some utilities for
/// them. Conceptually equivalent to `[Option<T>; 2]`.
pub(super) enum Potentials<T> {
    None,
    One(T),
    Two(T, T),
}
impl<T> Potentials<T> {
    /// Maps the internal values to another type, while minimising the amount of
    /// cloning of `source` required.
    pub fn map_with<R, V: Clone, F: Fn(V, T) -> R>(self, source: V, map: F) -> Potentials<R> {
        match self {
            Self::None => Potentials::None,
            Self::One(val) => Potentials::One(map(source, val)),
            Self::Two(val1, val2) => Potentials::Two(map(source.clone(), val1), map(source, val2)),
        }
    }
    pub fn and_then<R, F: Fn(T) -> Option<R>>(self, map: F) -> Potentials<R> {
        match self {
            Self::None => Potentials::None,
            Self::One(val) => match map(val) {
                Some(mapped) => Potentials::One(mapped),
                None => Potentials::None,
            },
            Self::Two(val1, val2) => match (map(val1), map(val2)) {
                (Some(mapped1), Some(mapped2)) => Potentials::Two(mapped1, mapped2),
                (Some(mapped), None) | (None, Some(mapped)) => Potentials::One(mapped),
                (None, None) => Potentials::None,
            },
        }
    }
}
impl<T> IntoIterator for Potentials<T> {
    type Item = T;
    type IntoIter = Flatten<IntoIter<Option<T>, 2>>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::None => [None, None],
            Self::One(val) => [Some(val), None],
            Self::Two(val1, val2) => [Some(val1), Some(val2)],
        }
        .into_iter()
        .flatten()
    }
}
pub(super) type PotentialSolutions = Potentials<Vec<(usize, FormattingSolution)>>;
pub(super) type PotentialNodes<'a> = Potentials<FormattingNode<'a>>;
