use std::cell::Ref;
use std::cell::RefMut;
use std::rc::Rc;

use parent_pointer_tree::NodeRef;
use types::FormattingNode;

use super::*;
use crate::prelude::KeywordKind as KK;
use crate::prelude::LogicalLineType as LLT;
use crate::prelude::OperatorKind as OK;
use crate::prelude::TokenType as TT;
use parent_pointer_tree::ParentPointerTree;

use types::RawDecision as NL;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BracketKind {
    /// Round brackets, i.e., `(` and `)`
    Round,

    /// Square brackets, i.e., `[` and `]`
    Square,

    /// Angle brackets, i.e., `<` and `>`
    Angle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BracketStyle {
    /// For the kinds of brackets that are tightly bound to their internals,
    /// e.g., in an expression
    /// ```delphi
    /// (AAAA
    ///     + BBBBB)
    /// ```
    Invisible,

    /// For the kinds of brackets that are expanded for readability,
    /// e.g., as an argument
    /// ```delphi
    /// AAAA(
    ///      [
    ///         AAAA,
    ///         BBBB
    ///      ]
    /// );
    /// ```
    Expanded,

    /// For the kinds of brackets that are opened and closed on their own line,
    /// e.g., in a declaration
    /// ```delphi
    /// procedure AAAA(
    ///     BBBB: CCCCC
    /// )
    /// ```
    BreakClose,

    /// For the kinds of brackets that are opened on their own line but closed
    /// inline, e.g., in a routine invocation in an expression
    /// ```delphi
    /// AAAA(
    ///         BBBB, CCCC)
    ///     + DDDD
    /// ```
    ContClose,
}

/// A `ContextType` labels the part of a line that a [`FormattingContext`] is
/// representing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ContextType {
    // Base contexts
    Base,
    InlineDeclaration,
    Raise,
    PropDec,
    RoutineHeader,
    DirectivesLine,
    ForLoop,
    // To control the latter of the pair
    Brackets(BracketKind, BracketStyle),
    // Lists
    CommaList,
    CommaElem,
    SemicolonList,
    SemicolonElem,
    DirectiveList,
    Directive,
    // Statement
    ControlFlow,
    Assignment,
    TypedAssignment,
    // Nested
    Type,
    Precedence(u8),
    AssignLHS,
    AssignRHS,
    ControlFlowBegin,
    // `MemberAccess` allows non-fluent calls to be on the same line
    MemberAccess,
    Subject,
    AnonHeader,
}
use ContextType as CT;

/// A [`FormattingContext`] describes how a portion of a line should be continued
/// if it were to be broken.
///
/// For example:
/// ```delphi
/// Foo(Bar, Baz);
///    ^--------^ - "brackets" context
/// ```
/// The "brackets" context primarily represents if the brackets were to be
/// broken, to add a continuation the contents.
///
/// This information is computed up-front for the entire line. After this
/// initial computation, the data isn't mutated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FormattingContext {
    context_type: ContextType,
    continuation_delta: u16,
    starting_token: u32,
    /// Used to represent the last token that the formatting requirements will have.
    /// Means the indentation can last longer than the rules.
    ending_token: Option<u32>,
}
impl FormattingContext {
    fn new(context_type: ContextType) -> Self {
        FormattingContext {
            context_type,
            continuation_delta: 1,
            starting_token: 0,
            ending_token: None,
        }
    }

    pub fn context_type(&self) -> ContextType {
        self.context_type
    }

    /// Returns whether a context is to be affected by a decision
    fn is_active_at_token(&self, line_index: u32) -> bool {
        self.starting_token != line_index
            && self.ending_token.is_none_or(|index| line_index <= index)
    }
}

/// [`FormattingContextState`] is the mutable portion of the information required
/// to completely format a line. While a [`FormattingContext`] is shared across
/// a whole logical line, [`FormattingContextState`] is specific to a (possibly
/// partial) solution.
///
/// For example:
/// ```delphi
/// Foo(Bar, Baz);
///    ^--------^ - "brackets" context
///     ^------^  - "comma list" context
/// ```
/// There may be two possible solutions:
/// ### Possible Solution 1
/// ```delphi
/// Foo(
///     Bar,
///     Baz
/// );
/// ```
/// [`FormattingContextState`]
/// - "brackets" context is broken
///   - contents should add 1 continuation
/// - "comma list" context is broken
///   - contents should not be further indented
/// ### Possible Solution 2
/// ```delphi
/// Foo(Bar, Baz);
/// ```
/// [`FormattingContextState`]
/// - "brackets" context is not broken
/// - "comma list" context is not broken
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FormattingContextState {
    /// Whether the context directly has had a [`Decision::Break`] made in it.
    ///
    /// This has the effect of adding its
    /// [`FormattingContext::continuation_delta`] to tokens that are broken
    /// within the context.
    pub is_broken: bool,

    /// Whether a [`Decision::Break`] decision can be made within the context.
    pub can_break: bool,

    /// Whether a child [`FormattingContext`] has been
    /// [`broken`](FormattingContextState::is_broken).
    pub is_child_broken: bool,

    /// Whether the relevant elements in the context should be placed on their
    /// own line. E.g., routine directives in a [`ContextType::DirectiveList`]
    /// context.
    ///
    /// The value is only known once a decision pertaining to whether the
    /// elements are one per line has been made.
    /// E.g.,
    /// ```delphi
    /// procedure Foo;
    ///     overload; external;
    ///               ^
    /// ```
    /// When the decision is made for `external`, the value will be defined. Up
    /// to that point, the directives could have either been all on the same
    /// line, or one per line.
    pub one_element_per_line: Option<bool>,

    /// Whether the anonymous routine elements should be placed on their
    /// own line.
    ///
    /// The value is only known once a decision pertaining to whether the
    /// elements are one per line has been made.
    /// E.g.,
    /// ```delphi
    /// Foo(
    ///     procedure begin ...
    ///               ^
    /// ```
    /// When the decision is made for `begin`, the value will be defined. Up to
    /// that point, the anonymous routine could either be formatted on a single
    /// line or many. Factors such as child-line count affect whether an
    /// anonymous routine can actually be formatted to be on a single line.
    pub break_anonymous_routine: Option<bool>,
}
impl Default for FormattingContextState {
    fn default() -> Self {
        Self {
            is_broken: false,
            can_break: true,
            is_child_broken: false,
            one_element_per_line: None,
            break_anonymous_routine: None,
        }
    }
}

pub(super) trait ContextFilter: Copy {
    fn get_filter(self) -> impl Fn(ContextType) -> bool;
}
impl ContextFilter for ContextType {
    fn get_filter(self) -> impl Fn(ContextType) -> bool {
        move |ctx_type| ctx_type == self
    }
}
impl<F: Fn(ContextType) -> bool + Copy> ContextFilter for F {
    fn get_filter(self) -> impl Fn(ContextType) -> bool {
        self
    }
}

macro_rules! context_matches {
    ($contexts: pat) => {
        |ctx_type| matches!(ctx_type, $contexts)
    };
}
pub(crate) use context_matches;

/// Represents the [`FormattingContext`] stack at an individual token in
/// conjunction with a working solution's context stack state.
pub(super) struct SpecificContextDataStack<'a> {
    solution: &'a FormattingNode<'a>,
    specific_stack: &'a SpecificContextStack<'a>,
}
impl SpecificContextDataStack<'_> {
    pub(super) fn parents_support_break(&self) -> bool {
        self.specific_stack
            .ctx_data_iter(self.solution)
            .filter(|(ctx, _)| ctx.is_active_at_token(self.solution.next_line_index))
            .all(|(_, data)| data.can_break)
    }

    pub(super) fn get_last_context<F: ContextFilter + Copy>(
        &self,
        filter: F,
    ) -> Option<(Ref<'_, FormattingContext>, &FormattingContextState)> {
        self.specific_stack
            .ctx_data_iter(self.solution)
            // This intentionally doesn't use
            // `FormattingContext::is_active_at_token` to ensure
            // `FormattingContext`s which have passed their `ending_token`
            // value can still be affected when accessed directly.
            .filter(|(ctx, _)| ctx.starting_token != self.solution.next_line_index)
            .find(|(ctx, _)| filter.get_filter()(ctx.context_type))
    }

    pub(super) fn get_continuation_count(&self, line_index: u32) -> u16 {
        self.specific_stack
            .ctx_data_iter(self.solution)
            .filter_map(|(ctx, data)| {
                let is_closing_type = Some(line_index) == ctx.ending_token
                    && matches!(ctx.context_type, ContextType::Brackets(_, _));

                if data.is_broken && ctx.is_active_at_token(line_index) && !is_closing_type {
                    Some(ctx.continuation_delta as u64)
                } else {
                    None
                }
            })
            .sum::<u64>() as u16
    }

    pub(super) fn iter(
        &self,
    ) -> impl Iterator<Item = (Ref<'_, FormattingContext>, &FormattingContextState)> {
        self.specific_stack.ctx_data_iter(self.solution)
    }
}

/// A `SpecificContextStack` refers to the stack of contexts at an individual
/// token.
#[derive(Clone)]
pub(super) struct SpecificContextStack<'a> {
    stack: Option<&'a NodeRef<'a, FormattingContext>>,
    formatting_contexts: &'a LineFormattingContexts<'a>,
}
impl<'a> SpecificContextStack<'a> {
    /// Pairs up this context stack with a solution's
    /// [`FormattingContextState`].
    pub(super) fn with_data(&'a self, node: &'a FormattingNode) -> SpecificContextDataStack<'a> {
        SpecificContextDataStack {
            solution: node,
            specific_stack: self,
        }
    }
    /// Iterate over a specific token's [`FormattingContext`] and their context
    /// indices. Typically for modification.
    pub(super) fn ctx_iter_indices(
        &self,
    ) -> impl Iterator<Item = (usize, Ref<'a, FormattingContext>)> {
        self.stack
            .into_iter()
            .flat_map(|stack| stack.walk_parents())
            .map(|ctx| (ctx.index(), ctx.get()))
    }
    /// Iterate over a specific token's [`FormattingContext`] and a specific
    /// solution's [`FormattingContextState`].
    pub(super) fn ctx_data_iter(
        &'a self,
        node: &'a FormattingNode,
    ) -> impl Iterator<Item = (Ref<'a, FormattingContext>, &'a FormattingContextState)> {
        self.stack
            .into_iter()
            .flat_map(|stack| stack.walk_parents())
            .map(|ctx| (ctx.get(), &node.context_data[ctx.index()]))
    }

    fn get_last_matching_context_mut(
        &self,
        node: &'a mut FormattingNode,
        filter: impl ContextFilter,
    ) -> Option<(Ref<'_, FormattingContext>, &'a mut FormattingContextState)> {
        self.ctx_iter_indices()
            .filter(|(_, ctx)| ctx.is_active_at_token(node.next_line_index))
            .find(|(_, context)| filter.get_filter()(context.context_type))
            .and_then(|(index, context)| {
                Some(context).zip(Rc::make_mut(&mut node.context_data).get_mut(index))
            })
    }

    fn update_last_matching_context(
        &self,
        node: &mut FormattingNode,
        filter: impl ContextFilter,
        operation: impl Fn(Ref<'_, FormattingContext>, &mut FormattingContextState),
    ) -> bool {
        if let Some((context, data)) = self.get_last_matching_context_mut(node, filter) {
            operation(context, data);
            true
        } else {
            false
        }
    }
    fn update_operator_precedences(&self, node: &mut FormattingNode, is_break: bool) {
        self.update_last_matching_context(node, context_matches!(CT::Precedence(_)), |_, data| {
            data.one_element_per_line.get_or_insert(is_break);
            data.can_break &= is_break;
        });

        if is_break {
            self.ctx_iter_indices()
                .take_while(|(_, context)| matches!(context.context_type, CT::Precedence(_)))
                .for_each(|(index, _)| {
                    if let Some(context) = Rc::make_mut(&mut node.context_data).get_mut(index) {
                        context.is_broken = true;
                        context.one_element_per_line = Some(true);
                    }
                });
        }
    }
    fn get_token_type_from_line_index(&self, line_index: u32) -> Option<TokenType> {
        self.formatting_contexts
            .token_types
            .get(
                *self
                    .formatting_contexts
                    .line
                    .get_tokens()
                    .get(line_index as usize)?,
            )
            .cloned()
    }

    /// Updates all contexts to reflect the decision provided.
    pub(super) fn update_contexts(&self, node: &mut FormattingNode, decision: RawDecision) {
        let line_index = node.next_line_index;

        let is_break = decision == NL::Break;
        let last_real_token_type = (0..line_index)
            .rev()
            .filter_map(|index| self.get_token_type_from_line_index(index))
            .find(|token_type| !token_type.is_comment_or_directive());

        self.ctx_iter_indices()
            .skip(1)
            .filter(|(_, ctx)| ctx.is_active_at_token(line_index))
            .for_each(|(index, _)| {
                if let Some(context) = Rc::make_mut(&mut node.context_data).get_mut(index) {
                    context.is_child_broken |= is_break;
                }
            });

        let apply_pivotal_break =
            |_: Ref<'_, FormattingContext>, data: &mut FormattingContextState| {
                data.is_broken |= is_break;
                data.can_break &= is_break;
            };

        let curr_token_type = self.get_token_type_from_line_index(line_index);
        match (last_real_token_type, curr_token_type) {
            (_, Some(TT::Comment(CommentKind::InlineBlock | CommentKind::InlineLine))) => {
                /*
                    When formatting comments, there are some considerations to be made:
                    - `Inline` comments are unique in that they will always stay
                      at the end of the line the are on. This necessitates not
                      updating any contexts as they are effectively ignored.
                    - `Individual` and `Multiline` comments on the other hand,
                      interact with the code around them. And so they must affect
                      the surrounding contexts.
                */
            }
            (Some(TT::Op(OK::LParen | OK::LBrack | OK::LessThan(ChK::Generic))), _) => {
                self.update_last_matching_context(
                    node,
                    context_matches!(CT::Brackets(_, _)),
                    |ctx, data| {
                        if !matches!(ctx.context_type, CT::Brackets(_, BracketStyle::Invisible)) {
                            data.can_break &= is_break;
                        }
                        data.is_broken |= is_break;
                    },
                );
            }
            (_, Some(TT::Keyword(kk))) if kk.is_directive() => {
                self.update_last_matching_context(
                    node,
                    context_matches!(CT::DirectivesLine | CT::DirectiveList | CT::CommaElem),
                    apply_pivotal_break,
                );
            }
            (Some(TT::Keyword(kk)), _) if kk.is_directive() => {
                self.update_last_matching_context(node, CT::Directive, |_, data| {
                    data.can_break &= is_break;
                    data.is_broken |= is_break;
                });
            }
            (Some(TT::Op(OK::Comma)), _) => {
                self.update_last_matching_context(node, CT::CommaList, apply_pivotal_break);
            }
            (Some(TT::Op(OK::Semicolon)), _) => {
                self.update_last_matching_context(node, CT::SemicolonList, |_, data| {
                    data.one_element_per_line.get_or_insert(is_break);
                    data.can_break &= is_break;
                    data.is_broken |= is_break;
                });
            }
            (Some(TT::Op(OK::Colon)), Some(TT::Op(OK::LParen))) => {
                // This is to ensure cases like this are possible, by not
                // forcing non-breaking if not broken
                // ```delphi
                // case Foo of
                // Bar: (
                // );
                // ```
                self.update_last_matching_context(
                    node,
                    context_matches!(CT::SemicolonElem),
                    |_, data| {
                        data.is_broken |= is_break;
                    },
                );
            }
            (Some(TT::Op(OK::Colon)), _) => {
                self.update_last_matching_context(node, context_matches!(_), apply_pivotal_break);
            }
            (
                Some(TT::Keyword(KK::If | KK::While | KK::Until | KK::With | KK::On | KK::Case)),
                _,
            ) => {
                self.update_last_matching_context(node, CT::ControlFlow, apply_pivotal_break);
                self.update_last_matching_context(node, CT::ControlFlowBegin, |_, data| {
                    data.is_broken |= is_break;
                });
            }
            (_, Some(TT::Keyword(KK::Begin | KK::End))) => {
                self.update_last_matching_context(node, CT::ControlFlowBegin, |_, data| {
                    data.is_broken |= is_break;
                });
                self.update_last_matching_context(node, CT::CommaElem, |_, data| {
                    data.can_break &= is_break;
                });
            }
            (Some(TT::Keyword(KK::For)), _)
                if self.formatting_contexts.line.get_line_type() != LLT::ForLoop =>
            {
                self.update_last_matching_context(node, CT::Subject, apply_pivotal_break);
            }
            (Some(TT::Keyword(KK::To)), Some(TT::Keyword(KK::Procedure | KK::Function))) => {
                self.update_last_matching_context(node, CT::Subject, apply_pivotal_break);
            }
            (Some(TT::Op(OK::Assign | OK::Equal(EqKind::Decl))), _) => {
                let is_type_parens = self
                    .ctx_data_iter(node)
                    .next()
                    .filter(|(ctx, _)| {
                        ctx.context_type
                            == CT::Brackets(BracketKind::Round, BracketStyle::BreakClose)
                    })
                    .is_some();

                self.update_last_matching_context(
                    node,
                    context_matches!(
                        CT::Base
                            | CT::Type
                            | CT::TypedAssignment
                            | CT::Assignment
                            | CT::Subject
                            | CT::SemicolonElem
                            | CT::CommaElem
                    ),
                    |_, data| {
                        data.is_broken |= is_break;
                        if self.formatting_contexts.line.get_line_type() != LLT::Declaration
                            || !is_type_parens
                        {
                            // This is to handle the case of record arrays and
                            // enums and the `(` after the `=`
                            // e.g., ```
                            // type
                            //   AAA = (
                            //       BB,
                            //       CC
                            //   );
                            // ```
                            data.can_break &= is_break;
                        }
                    },
                );
            }
            (_, Some(TT::Keyword(KK::In(InKind::ForLoop) | KK::To | KK::Downto))) => {
                self.update_last_matching_context(node, CT::ForLoop, apply_pivotal_break);
            }
            (Some(TT::Keyword(KK::In(InKind::ForLoop) | KK::To | KK::Downto)), _) => {
                self.update_last_matching_context(node, context_matches!(_), apply_pivotal_break);
            }
            (Some(TT::Keyword(KK::Raise)), _) => {
                self.update_last_matching_context(node, CT::Raise, apply_pivotal_break);
            }
            (_, Some(TT::Keyword(KK::At))) => {
                self.update_last_matching_context(node, CT::Raise, apply_pivotal_break);
            }
            (Some(TT::Keyword(KK::At)), _) => {
                self.update_last_matching_context(node, CT::Subject, apply_pivotal_break);
            }
            (Some(TT::Keyword(KK::Type)), Some(tt)) if tt != TT::Keyword(KK::Of) => {
                self.update_last_matching_context(node, context_matches!(_), apply_pivotal_break);
            }
            (Some(TT::Keyword(KK::Of)), _) => {
                self.update_last_matching_context(node, context_matches!(_), apply_pivotal_break);
            }
            (
                _,
                Some(TT::Keyword(KK::Label | KK::Const(_) | KK::Type | KK::Var(_) | KK::ThreadVar)),
            ) => {
                let _ = self.update_last_matching_context(
                    node,
                    context_matches!(CT::CommaElem | CT::AssignRHS),
                    |_, data| {
                        data.break_anonymous_routine.get_or_insert(is_break);
                    },
                ) || (self.formatting_contexts.line.get_line_type() != LLT::ForLoop
                    && self.update_last_matching_context(node, CT::Subject, apply_pivotal_break))
                    || self.update_last_matching_context(node, context_matches!(_), |_, data| {
                        data.is_broken |= is_break;
                    });
            }
            (Some(TT::Keyword(KK::Uses | KK::Contains | KK::Requires | KK::Exports)), _) => {
                self.update_last_matching_context(node, CT::Base, apply_pivotal_break);
            }
            (_, Some(TT::Op(OK::Dot))) => {
                match self
                    .get_last_matching_context_mut(
                        node,
                        context_matches!(CT::Precedence(_) | CT::MemberAccess),
                    )
                    .map(|(ctx, data)| (ctx.context_type, data))
                {
                    Some((CT::Precedence(_), _)) => {
                        self.update_operator_precedences(node, is_break);
                    }
                    Some((CT::MemberAccess, data)) => {
                        data.one_element_per_line.get_or_insert(is_break);
                        data.is_broken |= is_break;
                    }
                    _ => {}
                }
            }
            (prev, Some(op @ (TT::Op(_) | TT::Keyword(_))))
                if super::get_operator_precedence(op).is_some() && is_binary(op, prev) =>
            {
                self.update_operator_precedences(node, is_break);
            }
            _ => {
                self.update_last_matching_context(node, context_matches!(_), |_, data| {
                    data.is_broken |= is_break;
                });
            }
        }

        // Some contexts need updating if their children get updated
        for (ctx_index, ctx) in self
            .ctx_iter_indices()
            .filter(|(_, context)| context.is_active_at_token(line_index))
        {
            if let Some(data) = Rc::make_mut(&mut node.context_data).get_mut(ctx_index) {
                match ctx.context_type {
                    CT::TypedAssignment | CT::ForLoop => data.is_broken |= data.is_child_broken,
                    CT::AssignLHS
                        if self.formatting_contexts.line.get_line_type() == LLT::Assignment =>
                    {
                        data.is_broken |= data.is_child_broken
                    }
                    CT::SemicolonList | CT::CommaList | CT::Precedence(_) | CT::DirectiveList => {
                        data.is_broken |= data.is_child_broken;
                        if is_break {
                            data.one_element_per_line = Some(true);
                        }
                    }
                    CT::CommaElem | CT::AssignRHS => {
                        if is_break {
                            data.break_anonymous_routine = Some(true);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub(super) fn update_contexts_from_child_solutions(
        &self,
        node: &mut FormattingNode,
        child_solutions: &[(usize, types::FormattingSolution)],
    ) {
        if child_solutions
            .iter()
            .flat_map(|(_, solution)| &solution.decisions)
            .any(|decision| matches!(decision.decision, Decision::Break { .. }))
        {
            if let Some(data) = self
                .ctx_iter_indices()
                .find(|(_, context)| {
                    matches!(
                        context.context_type,
                        CT::Brackets(BracketKind::Round, BracketStyle::BreakClose)
                    )
                })
                .map(|(index, _)| {
                    Rc::make_mut(&mut node.context_data).get_mut(index).expect(
                        "FormattingContext at Brackets(Round, BreakClose) index should exist",
                    )
                })
            {
                data.is_child_broken = true;
            }
        }
    }
}

/// Represents a context stack for each token in a line.
///
/// For a simple example, there are multiple context stacks for a whole line. E.g.,
/// ```delphi
/// Foo(Bar, Baz);
///    ^--------^ - "brackets" context
///     ^------^  - "comma list" context
/// ```
/// This diagram represents for which tokens a context is present.
///
/// I.e., at the `(` token, the context stack is:
/// - "brackets"
///
/// At the `Bar` token, the context stack is:
/// - "brackets"
/// - "comma list"
///
/// Between these two tokens, the "brackets" context refers to the same
/// instance. Each of these [`FormattingContext`]s contain information on what
/// will happen to the contents should it be broken.
pub(super) struct LineFormattingContexts<'a> {
    context_count: usize,
    update_indices: Vec<(u32, NodeRef<'a, FormattingContext>)>,
    line: &'a LogicalLine,
    token_types: &'a [TokenType],
}
impl<'a> LineFormattingContexts<'a> {
    pub fn new_tree() -> ParentPointerTree<FormattingContext> {
        ParentPointerTree::new(FormattingContext::new(CT::Base))
    }

    pub fn new(
        line: &'a LogicalLine,
        token_types: &'a [TokenType],
        context_tree: &'a ParentPointerTree<FormattingContext>,
    ) -> Self {
        let get_token_type_from_line_index = |line_index| {
            token_types
                .get(*line.get_tokens().get(line_index as usize)?)
                .cloned()
        };

        let builder_context_tree = Self::new_tree();
        let mut contexts = LineFormattingContextsBuilder::new(&builder_context_tree);

        match line.get_line_type() {
            LLT::CaseArm => {
                contexts.push_utility((CT::ControlFlowBegin, 0));
                contexts.push_utility((CT::CommaList, 0));
                contexts.push_utility(CT::CommaElem);
                contexts.push_expression();
            }
            LLT::Declaration => {
                contexts.push_utility(CT::Assignment);
                contexts.push_utility(CT::AssignLHS);
                contexts.push_utility((CT::CommaList, 0));
            }
            LLT::ImportClause | LLT::ExportClause => {
                contexts.push((CT::CommaList, 0));
                contexts.push(CT::CommaElem);
                contexts.push_expression();
            }
            LLT::RoutineHeader => {
                contexts.push_utility(CT::DirectivesLine);
                contexts.push(CT::RoutineHeader);
            }
            LLT::PropertyDeclaration => {
                contexts.push_utility(CT::DirectivesLine);
                contexts.push(CT::PropDec);
            }
            LLT::Assignment => {
                contexts.push(CT::Assignment);
                contexts.push(CT::AssignLHS);
                contexts.push_expression();
            }
            LLT::ForLoop => {}
            _ => {
                contexts.push_utility(CT::Assignment);
                contexts.push_utility(CT::AssignLHS);
                contexts.push_expression();
            }
        }

        let mut prev_prev_token_type = None;
        let mut prev_token_type = None;
        let mut current = get_token_type_from_line_index(0);
        let mut next_token_type = get_token_type_from_line_index(1);
        while let Some(current_token_type) = current {
            if !current_token_type.is_comment_or_directive() {
                let last_context_type = contexts.current_context.get().context_type;
                // New contexts relating to the previous token are pushed here
                // to avoid including any leading comments
                if let Some(prev_token_type) = prev_token_type {
                    match (prev_token_type, last_context_type) {
                        (TT::Op(OK::LParen | OK::LBrack | OK::LessThan(ChK::Generic)), _)
                        | (TT::Op(OK::Semicolon), CT::SemicolonList) => {
                            if last_context_type != CT::SemicolonList {
                                contexts.push_utility((CT::SemicolonList, 0));
                            }
                            contexts.push_utility(CT::SemicolonElem);
                            contexts.push_utility(CT::Assignment);
                            contexts.push_utility(CT::AssignLHS);
                            contexts.push_utility((CT::CommaList, 0));
                            contexts.push_utility(CT::CommaElem);
                            contexts.push_utility(CT::Assignment);
                            contexts.push_utility(CT::AssignLHS);
                            contexts.push_expression();
                        }
                        (TT::Op(OK::Comma), CT::CommaList) => {
                            contexts.push(CT::CommaElem);
                            contexts.push_utility(CT::Assignment);
                            contexts.push_utility(CT::AssignLHS);
                            contexts.push_expression();
                        }
                        _ => {}
                    }
                    match prev_token_type {
                        TT::Keyword(KK::Of) => {
                            contexts.push(CT::Subject);
                            contexts.push_expression();
                        }
                        TT::Keyword(KK::Type)
                            if !matches!(next_token_type, Some(TT::Keyword(KK::Of))) =>
                        {
                            contexts.push(CT::Subject);
                            contexts.push_expression();
                        }
                        TT::Keyword(KK::To)
                            if matches!(
                                next_token_type,
                                Some(TT::Keyword(KK::Function | KK::Procedure))
                            ) =>
                        {
                            contexts.push(CT::Subject);
                        }
                        TT::Keyword(
                            KK::Function | KK::Procedure | KK::Destructor | KK::Constructor,
                        ) => {
                            contexts.push_expression();
                        }
                        TT::Keyword(KK::For) if line.get_line_type() == LLT::ForLoop => {
                            contexts.push(CT::Subject);
                        }
                        TT::Keyword(KK::For) => {
                            contexts.push_expression();
                        }
                        TT::Keyword(KK::In(InKind::ForLoop) | KK::To | KK::Downto) => {
                            contexts.push_expression();
                        }
                        TT::Keyword(KK::If | KK::While | KK::On | KK::Until | KK::Case) => {
                            contexts.push_expression();
                        }
                        TT::Keyword(KK::With) => {
                            contexts.push_utility((CT::CommaList, 0));
                            contexts.push_utility(CT::CommaElem);
                            contexts.push_expression();
                        }
                        TT::Keyword(KK::Var(DeclKind::Inline) | KK::Const(DeclKind::Inline)) => {
                            contexts.push_utility(CT::Assignment);
                            contexts.push(CT::InlineDeclaration);
                        }
                        TT::Keyword(KK::Raise) => {
                            contexts.push_expression();
                        }
                        TT::Keyword(KK::At) => {
                            contexts.push_expression();
                        }
                        TT::Op(OK::Equal(EqKind::Decl) | OK::Assign) => {
                            contexts.push(CT::AssignRHS);
                            contexts.push_expression();
                        }
                        TT::Keyword(KK::Abstract)
                            if matches!(prev_prev_token_type, Some(TT::Keyword(KK::Class))) => {}
                        TT::Keyword(kk) if kk.is_directive() => {
                            contexts.push_expression();
                        }
                        TT::Op(OK::Colon) if line.get_line_type() != LLT::CaseArm => {
                            if contexts
                                .type_stack()
                                .find(|ctx_typ| matches!(ctx_typ, CT::Brackets(_, _)))
                                .is_some_and(|ctx_type| {
                                    matches!(ctx_type, CT::Brackets(BracketKind::Angle, _))
                                })
                            {
                                contexts.push_utility((CT::CommaList, 0));
                                contexts.push_utility(CT::CommaElem);
                            } else {
                                contexts.push(CT::Type);
                            }
                            contexts.push_expression();
                        }
                        op if super::get_operator_precedence(op).is_some()
                            && is_binary(op, prev_prev_token_type) =>
                        {
                            contexts.push_operators();
                        }
                        _ => {}
                    }
                }
            }
            let last_context_type = contexts.current_context.get().context_type;

            // For contexts that apply to the current token
            match current_token_type {
                TT::Op(OK::LParen | OK::LBrack | OK::LessThan(ChK::Generic)) => {
                    let current_kind = match current_token_type {
                        TT::Op(OK::LBrack) => BracketKind::Square,
                        TT::Op(OK::LessThan(ChevronKind::Generic)) => BracketKind::Angle,
                        _ => BracketKind::Round,
                    };
                    let (typ, cont_delta) = match prev_token_type {
                        // routine invocations
                        Some(TT::Identifier | TT::Op(OK::GreaterThan(ChevronKind::Generic))) => {
                            (BracketStyle::BreakClose, 1)
                        }

                        // variant record definitions
                        Some(TT::Op(OK::Colon)) => (BracketStyle::BreakClose, 1),

                        // type definitions
                        Some(
                            TT::Op(OK::Equal(EqKind::Decl))
                            | TT::Op(OK::Semicolon)
                            | TT::Keyword(
                                KK::Function
                                | KK::Procedure
                                | KK::Sealed
                                | KK::Abstract
                                | KK::Class
                                | KK::Interface
                                | KK::Helper
                                | KK::Of,
                            ),
                        ) => (BracketStyle::BreakClose, 1),

                        // arguments
                        Some(TT::Op(OK::LParen | OK::Comma))
                            if current_kind == BracketKind::Square =>
                        {
                            (BracketStyle::BreakClose, 1)
                        }
                        Some(TT::Op(OK::LParen | OK::Comma)) => (BracketStyle::Expanded, 1),

                        _ if current_kind == BracketKind::Square => (BracketStyle::BreakClose, 1),
                        _ => (BracketStyle::Invisible, 0),
                    };
                    contexts.push((CT::Brackets(current_kind, typ), cont_delta))
                }
                TT::Op(OK::GreaterThan(ChK::Generic) | OK::RParen | OK::RBrack) => {
                    contexts.pop_until(context_matches!(CT::Brackets(_, _)));
                }
                TT::Op(OK::Semicolon) => {
                    if let (true, Some(CT::DirectiveList)) = (
                        (contexts.line_index + 1) as usize == line.get_tokens().len(),
                        contexts.pop_until(CT::DirectiveList),
                    ) {
                        contexts.pop_until(context_matches!(
                            CT::Base | CT::RoutineHeader | CT::DirectivesLine
                        ));
                    } else {
                        contexts.pop_until(context_matches!(
                            CT::DirectiveList | CT::SemicolonList | CT::Base | CT::RoutineHeader
                        ));
                        contexts.retain_current();
                    }
                }
                TT::Op(OK::Comma) => {
                    contexts.retain_first(CT::CommaElem);
                    contexts.pop_until(CT::CommaList);
                    contexts.retain_current();
                }
                TT::Op(OK::Colon) => {
                    match contexts.pop_until(context_matches!(
                        CT::CommaList | CT::SemicolonElem | CT::AnonHeader
                    )) {
                        Some(CT::CommaList) => {
                            contexts.pop();
                            contexts.retain_first(CT::SemicolonElem);
                        }
                        Some(CT::SemicolonElem) => {
                            contexts.retain_current();
                        }
                        _ => {}
                    };
                    if let Some(mut context) = contexts.last_context_matching_mut(context_matches!(
                        CT::Assignment | CT::TypedAssignment
                    )) {
                        context.context_type = CT::TypedAssignment;
                    }
                }
                TT::Op(OK::Equal(EqKind::Decl) | OK::Assign) => {
                    contexts.retain_first(context_matches!(CT::CommaElem | CT::SemicolonElem));
                    match contexts.pop_until(context_matches!(
                        CT::CommaElem
                            | CT::SemicolonElem
                            | CT::Assignment
                            | CT::TypedAssignment
                            | CT::AssignLHS
                    )) {
                        Some(CT::TypedAssignment | CT::Assignment) => {
                            contexts.retain_current();
                            let line_index = contexts.line_index;
                            contexts.last_context_mut().ending_token =
                                Some(line_index.saturating_sub(1));
                        }
                        Some(CT::AssignLHS) => {
                            contexts.retain_current();
                            contexts.pop();
                            if matches!(
                                contexts.type_stack().next(),
                                Some(CT::TypedAssignment | CT::Assignment)
                            ) {
                                let line_index = contexts.line_index;
                                contexts.last_context_mut().ending_token =
                                    Some(line_index.saturating_sub(1));
                            }
                            contexts.retain_current();
                        }
                        _ => {}
                    }
                }
                TT::Keyword(KK::If | KK::While | KK::With | KK::On) => {
                    contexts.push_utility((CT::ControlFlowBegin, 0));
                    contexts.push(CT::ControlFlow);
                }
                TT::Keyword(KK::Case | KK::Until) => {
                    contexts.push(CT::ControlFlow);
                }
                TT::Keyword(KK::For) if line.get_line_type() == LLT::ForLoop => {
                    contexts.push_utility((CT::ControlFlowBegin, 0));
                    contexts.push(CT::ForLoop);
                }
                TT::Keyword(KK::For) => {
                    contexts.push(CT::Subject);
                }
                TT::Keyword(KK::In(InKind::ForLoop) | KK::To | KK::Downto) => {
                    contexts.pop_until(CT::ForLoop);
                    contexts.push(CT::Subject);
                }
                TT::Keyword(KK::Raise) => {
                    contexts.push(CT::Raise);
                }
                TT::Keyword(KK::At) => {
                    contexts.pop_until(CT::Raise);
                    contexts.push(CT::Subject);
                }
                TT::Op(OK::Dot) => {
                    if CT::Precedence(0) == last_context_type {
                        contexts.retain_current();
                        if let Some(TT::Op(
                            OK::RParen | OK::RBrack | OK::GreaterThan(ChK::Generic),
                        )) = prev_token_type
                        {
                            contexts.fluent(contexts.current_context.clone());
                        }
                    }
                }
                op if super::get_operator_precedence(op).is_some()
                    && is_binary(op, prev_token_type) =>
                {
                    let op_prec = super::get_operator_precedence(op).unwrap();
                    contexts.pop_until_and_retain(CT::Precedence(op_prec));
                }
                TT::Keyword(KK::Then | KK::Do | KK::Of) => {
                    contexts.pop_until(context_matches!(CT::ControlFlow | CT::ForLoop));
                }
                TT::Keyword(KK::Function | KK::Procedure)
                    if last_context_type != CT::RoutineHeader =>
                {
                    contexts.push(CT::AnonHeader);
                }
                TT::Keyword(
                    KK::Label
                    | KK::Const(DeclKind::AnonSection | DeclKind::Other | DeclKind::Section)
                    | KK::Type
                    | KK::Var(DeclKind::AnonSection | DeclKind::Other | DeclKind::Section)
                    | KK::ThreadVar,
                ) => {
                    contexts.pop_until_after(CT::AnonHeader);
                }
                TT::Keyword(KK::Begin) => {
                    if contexts.pop_until(CT::CommaElem) == Some(last_context_type) {
                        // Not in a CommaList, therefore top-level statement
                        contexts.pop_until(CT::ControlFlowBegin);
                    } else {
                        contexts.retain_current();
                    }
                    contexts.pop_until_after(CT::AnonHeader);
                }
                TT::Keyword(KK::Abstract)
                    if matches!(prev_token_type, Some(TT::Keyword(KK::Class))) => {}
                TT::Keyword(kk) if kk.is_directive() => {
                    if contexts.pop_until(CT::DirectiveList) != Some(CT::DirectiveList) {
                        if contexts
                            .pop_until_after(context_matches!(CT::PropDec | CT::RoutineHeader))
                        {
                            contexts.retain_current();
                        }
                        contexts.push((CT::DirectiveList, 0));
                    }
                    contexts.push(CT::Directive);
                }
                _ => {}
            }

            trace!("Moving to next token with type: {:?}", current_token_type);
            contexts.next_token();

            // After the current token, some contexts needs to be popped
            match current_token_type {
                TT::Op(OK::GreaterThan(ChK::Generic)) => {
                    contexts.pop_until_after(context_matches!(CT::Brackets(BracketKind::Angle, _)));
                }
                TT::Op(OK::RParen) => {
                    contexts.pop_until_after(context_matches!(CT::Brackets(BracketKind::Round, _)));
                }
                TT::Op(OK::RBrack) => {
                    contexts
                        .pop_until_after(context_matches!(CT::Brackets(BracketKind::Square, _)));
                }
                _ => {}
            }

            if !current_token_type.is_comment_or_directive() {
                prev_prev_token_type = prev_token_type;
                prev_token_type = current;
            }
            current = next_token_type;
            next_token_type = get_token_type_from_line_index(contexts.line_index + 1);
        }

        contexts.finalise();

        LineFormattingContexts {
            update_indices: Self::write_context_tree(context_tree, contexts),
            context_count: context_tree.len(),
            line,
            token_types,
        }
    }

    fn write_context_tree(
        tree: &'a ParentPointerTree<FormattingContext>,
        builder: LineFormattingContextsBuilder<'_>,
    ) -> Vec<(u32, NodeRef<'a, FormattingContext>)> {
        tree.reserve(
            builder
                .contexts
                .len()
                .saturating_sub(builder.contexts_to_remove.len()),
        );

        let mut node_mappings: Vec<NodeRef<'a, FormattingContext>> =
            Vec::with_capacity(builder.contexts.len());

        for builder_node in builder.contexts {
            if let Some(parent) = builder_node.parent() {
                let parent = node_mappings[parent.index()].clone();
                if builder.contexts_to_remove.contains(&builder_node) {
                    node_mappings.push(parent);
                } else {
                    let context = builder_node.get().clone();
                    let new_ref = parent.add_successor(context);
                    node_mappings.push(new_ref);
                }
            } else {
                node_mappings.push(tree.root());
            }
        }

        builder
            .update_indices
            .into_iter()
            .map(|(index, node)| (index, node_mappings[node.index()].clone()))
            .collect()
    }

    pub fn get_default_context_data(&self) -> Rc<Vec<FormattingContextState>> {
        Rc::new(vec![Default::default(); self.context_count])
    }

    /// Returns the [`SpecificContextStack`] at a given line index.
    pub fn get_specific_context_stack(&self, line_index: u32) -> SpecificContextStack {
        SpecificContextStack {
            formatting_contexts: self,
            stack: self
                .update_indices
                .iter()
                .rev()
                .find(|&&(context_token_index, _)| context_token_index <= line_index)
                .map(|(_, indices)| indices),
        }
    }
}

/// Facilitates the construction of [`LineFormattingContexts`] incrementally while
/// iterating a line's tokens.
struct LineFormattingContextsBuilder<'builder> {
    contexts: &'builder ParentPointerTree<FormattingContext>,
    update_indices: Vec<(u32, NodeRef<'builder, FormattingContext>)>,
    current_context: NodeRef<'builder, FormattingContext>,
    contexts_to_remove: NodeRefSet,
    line_index: u32,
    member_access_contexts: NodeRefSet,
}
impl<'builder> LineFormattingContextsBuilder<'builder> {
    fn new(contexts: &'builder ParentPointerTree<FormattingContext>) -> Self {
        Self {
            contexts,
            update_indices: Vec::new(),
            current_context: contexts.root(),
            contexts_to_remove: NodeRefSet::with_capacity(contexts.len()),
            line_index: 0,
            member_access_contexts: NodeRefSet::new(),
        }
    }
    fn type_stack(&self) -> impl Iterator<Item = ContextType> + 'builder {
        self.current_context
            .walk_parents_data()
            .map(|index| index.context_type)
    }

    /// To ensure the context specified will not be eliminated as unused
    fn retain(&mut self, context: NodeRef<'builder, FormattingContext>) {
        trace!(
            "Retaining context with type: {:?}",
            context.get().context_type
        );
        self.contexts_to_remove.remove(&context);
    }

    /// To ensure the current top of the context stack will not be eliminated as unused
    fn retain_current(&mut self) {
        self.retain(self.current_context.clone());
    }

    /// To ensure the first matching context on the stack will not be eliminated as unused
    fn retain_first<F: ContextFilter>(&mut self, filter: F) {
        let Some(ctx) = self
            .current_context
            .walk_parents()
            .find(|ctx| filter.get_filter()(ctx.get().context_type))
        else {
            return;
        };
        self.retain(ctx);
    }

    /// To indicate the Precedence(0) context is actually a fluent context
    fn fluent(&mut self, context: NodeRef<'builder, FormattingContext>) {
        self.member_access_contexts.remove(&context);
    }

    fn add_context<C: Into<FormattingContext>, L: Fn(ContextType)>(
        &mut self,
        context: C,
        log_callback: L,
    ) -> NodeRef<'builder, FormattingContext> {
        let formatting_context = FormattingContext {
            starting_token: self.line_index,
            ..context.into()
        };
        let context_type = formatting_context.context_type;
        log_callback(context_type);
        let node = self.current_context.add_successor(formatting_context);

        // By default `ContextType::Precedence(0)` will be changed to
        // `MemberAccess` at the end of the process. That is unless it is
        // removed from the list with `fluent`.
        if context_type == CT::Precedence(0) {
            self.member_access_contexts.insert(node.clone());
        }
        self.current_context = node.clone();
        node
    }

    fn push<C: Into<FormattingContext>>(&mut self, context: C) {
        self.add_context(context, |context_type| {
            trace!("Pushing context with type: {context_type:?}")
        });
    }
    fn push_utility<C: Into<FormattingContext>>(&mut self, context: C) {
        let context_index = self.add_context(context, |context_type| {
            trace!("Pushing utility context with type: {context_type:?}")
        });
        self.contexts_to_remove.insert(context_index);
    }

    const ADD_ALL_PRECEDENCES: u8 = super::LOWEST_PRECEDENCE + 1;
    fn push_operator_precedences(&mut self, starting_precedence: u8) {
        for precedence in (super::HIGHEST_PRECEDENCE..starting_precedence).rev() {
            self.push_utility(CT::Precedence(precedence));
        }
    }
    fn push_operators(&mut self) {
        let starting_precedence = match self.type_stack().next() {
            Some(CT::Precedence(p)) => p,
            _ => Self::ADD_ALL_PRECEDENCES,
        };
        self.push_operator_precedences(starting_precedence);
    }
    fn push_expression(&mut self) {
        self.push_operator_precedences(Self::ADD_ALL_PRECEDENCES);
    }

    fn pop(&mut self) {
        if self.current_context.get().ending_token.is_none() {
            self.current_context.get_mut().ending_token = Some(self.line_index.saturating_sub(1));
        }
        if let Some(node) = self.current_context.parent() {
            trace!(
                "Popping context with type: {:?}",
                self.current_context.get().context_type
            );
            self.current_context = node;
        }
    }

    fn find_stack_depth<F: ContextFilter>(&mut self, context_filter: F) -> Option<usize> {
        self.type_stack()
            .enumerate()
            .find(|&(_, context_type)| context_filter.get_filter()(context_type))
            .map(|(depth, _)| depth)
    }

    /// Pops contexts until the context type is found, leaving that as the top
    /// of the stack. Returns the new top of the context stack, only pops if
    /// the context type is found in the stack.
    fn pop_until<F: ContextFilter>(&mut self, context_filter: F) -> Option<ContextType> {
        if let Some(depth) = self.find_stack_depth(context_filter) {
            for _ in 0..depth {
                self.pop();
            }
        }

        self.type_stack().next()
    }

    /// Pops contexts until the context type is found, leaving that as the top
    /// of the stack and retaining it.
    fn pop_until_and_retain<F: ContextFilter>(&mut self, context_filter: F) {
        if let Some(depth) = self.find_stack_depth(context_filter) {
            for _ in 0..depth {
                self.pop();
            }
            self.retain_current();
        }
    }

    /// Pops contexts until the context type is found, popping that too.
    /// Returns whether the stack was changed, only pops if the context type is
    /// found in the stack.
    fn pop_until_after<F: ContextFilter>(&mut self, context_filter: F) -> bool {
        let Some(depth) = self.find_stack_depth(context_filter) else {
            return false;
        };
        for _ in 0..=depth {
            self.pop();
        }
        true
    }

    fn next_token(&mut self) {
        if !matches!(
            self.current_context.get().context_type,
            CT::Precedence(_)
                | CT::TypedAssignment
                | CT::Assignment
                | CT::AssignLHS
                | CT::AssignRHS
                | CT::CommaList
                | CT::SemicolonList
        ) {
            self.retain_current();
        }
        if self
            .update_indices
            .last()
            .is_none_or(|(_, last_contexts)| last_contexts != &self.current_context)
        {
            self.update_indices
                .push((self.line_index, self.current_context.clone()));
        }

        self.line_index += 1;
    }

    fn last_context_matching_mut<F: ContextFilter>(
        &mut self,
        filter: F,
    ) -> Option<RefMut<'builder, FormattingContext>> {
        self.current_context
            .walk_parents()
            .find(|ctx| filter.get_filter()(ctx.get().context_type))
            .map(|ctx| ctx.get_mut())
    }

    fn last_context_mut(&mut self) -> RefMut<'builder, FormattingContext> {
        self.current_context.get_mut()
    }

    /// Finalises the context types and returns which contexts can be removed
    fn finalise(&mut self) {
        // Precedence(0) contexts that have not been deemed as "fluent" are
        // converted to `MemberAccess`
        self.contexts
            .into_iter()
            .filter(|ctx| self.member_access_contexts.contains(ctx))
            .for_each(|ctx| {
                let mut ctx = ctx.get_mut();
                if ctx.context_type == CT::Precedence(0) {
                    ctx.context_type = CT::MemberAccess;
                }
            });

        // A context that is only a single token is useless
        self.contexts_to_remove
            .extend(self.contexts.into_iter().filter(|node_ref| {
                let ctx = node_ref.get();
                ctx.ending_token == Some(ctx.starting_token)
                    || ctx.ending_token.is_none()
                        && matches!(ctx.context_type, CT::CommaList | CT::Assignment)
            }));

        // If an `Brackets` context is in an expression, then the ending token
        // must not be broken before.
        for context in self.contexts {
            use BracketStyle as BS;
            let new_context_type = match context.get().context_type {
                CT::Brackets(kind, BS::Expanded) => CT::Brackets(kind, BS::Invisible),
                CT::Brackets(kind, BS::BreakClose) => CT::Brackets(kind, BS::ContClose),
                _ => continue,
            };
            if let Some(CT::Precedence(_) | CT::Brackets(_, BracketStyle::Invisible)) = context
                .walk_parents()
                .skip(1)
                .find(|ctx| {
                    !self.contexts_to_remove.contains(ctx)
                        && ctx.get().context_type != CT::MemberAccess
                })
                .map(|ctx| ctx.get().context_type)
            {
                context.get_mut().context_type = new_context_type;
            }
        }
    }
}
impl From<ContextType> for FormattingContext {
    fn from(val: ContextType) -> Self {
        FormattingContext::new(val)
    }
}
impl From<(ContextType, u16)> for FormattingContext {
    fn from(val: (ContextType, u16)) -> Self {
        FormattingContext {
            continuation_delta: val.1,
            ..FormattingContext::new(val.0)
        }
    }
}

impl KeywordKind {
    pub(super) fn is_directive(self) -> bool {
        self.is_method_directive()
            || self.is_property_directive()
            || matches!(self, KK::Index | KK::Name)
    }
}

struct NodeRefSet {
    set: Vec<bool>,
}
impl<'a> NodeRefSet {
    fn new() -> Self {
        Self { set: Vec::new() }
    }
    fn with_capacity(capacity: usize) -> Self {
        Self {
            set: vec![false; capacity],
        }
    }
    fn insert(&mut self, node: NodeRef<'a, FormattingContext>) {
        if node.index() >= self.set.len() {
            self.set
                .extend((self.set.len()..=node.index()).map(|_| false));
        }
        self.set[node.index()] = true;
    }
    fn remove(&mut self, node: &NodeRef<'a, FormattingContext>) {
        if node.index() < self.set.len() {
            self.set[node.index()] = false;
        }
    }
    fn contains(&self, node: &NodeRef<'a, FormattingContext>) -> bool {
        *self.set.get(node.index()).unwrap_or(&false)
    }
    fn len(&self) -> usize {
        self.set.iter().filter(|&&val| val).count()
    }
    fn extend(&mut self, iter: impl IntoIterator<Item = NodeRef<'a, FormattingContext>>) {
        for item in iter {
            self.insert(item);
        }
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use crate::prelude::{
        DelphiLexer, DelphiLogicalLineParser, Lexer, LogicalLineParser, TokenData,
    };

    use super::*;

    mod get {
        use std::{error::Error, fmt, iter::Peekable, str::FromStr};

        use itertools::FoldWhile;

        use super::*;

        struct DslParseError(String);
        impl fmt::Debug for DslParseError {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
        impl fmt::Display for DslParseError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
        impl Error for DslParseError {}

        #[expect(clippy::from_over_into)]
        impl Into<DslParseError> for String {
            fn into(self) -> DslParseError {
                DslParseError(self)
            }
        }
        #[expect(clippy::from_over_into)]
        impl Into<DslParseError> for &str {
            fn into(self) -> DslParseError {
                DslParseError(self.to_owned())
            }
        }

        /// Represents the data required for a test case.
        ///
        /// These tests are written with a DSL looking like:
        ///
        /// ```text
        ///                begin var AA: BB := 11; end;
        /// 1 Base               ^----------------
        /// 1 TypedAssignment        ^----$------
        /// 1 InlineDeclaration      ^----$
        /// ```
        ///
        /// This example is comprised of a few key parts:
        /// - the input string; the first line
        /// - one or more [`AssertionContext`] lines
        struct AssertionData<'a> {
            asserting_line: LogicalLine,
            all_tokens: Vec<Token<'a>>,
            contexts: Vec<AssertionContext>,
        }
        impl<'a> AssertionData<'a> {
            fn parse(s: &'a str) -> Result<Self, DslParseError> {
                let mut context_lines = s.lines().filter(|line| !line.trim().is_empty()).peekable();

                let code_line = context_lines
                    .next()
                    .ok_or("there is no initial code line".into())?;
                let first_line = context_lines
                    .peek()
                    .ok_or("there are no contexts specified".into())?;

                let tokens = DelphiLexer {}.lex(code_line);
                let (logical_lines, mut all_tokens) = DelphiLogicalLineParser {}.parse(tokens);
                DistinguishGenericTypeParamsConsolidator {}.consolidate(&mut all_tokens);

                let initial_column = first_line
                    .find('^')
                    .ok_or("cannot find initial line's ^".into())?;

                let initial_token = find_token_index_for_col(&all_tokens, initial_column)
                    .ok_or("second line's `^` should point at a valid token".into())?;
                let asserting_line = logical_lines
                    .into_iter()
                    .find(|line| line.get_tokens().contains(&initial_token))
                    .ok_or("cannot find first indicated token's line".into())?;

                let mut contexts = Vec::new();
                for context_line in context_lines {
                    let context_line =
                        AssertionContext::parse(context_line, &all_tokens, &asserting_line)
                            .map_err(|e| {
                                format!("{e}\nfailed to parse context line '{context_line}'").into()
                            })?;
                    contexts.push(context_line);
                }
                Ok(AssertionData {
                    asserting_line,
                    all_tokens,
                    contexts,
                })
            }
        }

        fn take_ws(chars: &mut Peekable<impl Iterator<Item = (usize, u8)>>) {
            chars
                .peeking_take_while(|(_, c)| c.is_ascii_whitespace())
                .for_each(drop);
        }
        fn take_no_ws(chars: &mut Peekable<impl Iterator<Item = (usize, u8)>>) {
            chars
                .peeking_take_while(|(_, c)| !c.is_ascii_whitespace())
                .for_each(drop);
        }
        fn take_hyphens(chars: &mut Peekable<impl Iterator<Item = (usize, u8)>>) {
            chars.peeking_take_while(|(_, c)| *c == b'-').for_each(drop);
        }
        fn parse_part<'a>(
            input: &'a str,
            chars: &'a mut Peekable<impl Iterator<Item = (usize, u8)>>,
        ) -> Option<&'a str> {
            take_ws(chars);
            let (first, _) = chars.next()?;
            take_no_ws(chars);
            let (last, _) = chars.peek()?;
            Some(&input[first..*last])
        }

        fn find_token_index_for_col(tokens: &[Token], column: usize) -> Option<usize> {
            tokens
                .iter()
                .enumerate()
                .fold_while((0usize, None), |(mut str_len, _), (index, token)| {
                    str_len += token.get_leading_whitespace().len();
                    let content_len = token.get_content().len();
                    if (str_len..(str_len + content_len)).contains(&column) {
                        FoldWhile::Done((str_len, Some(index)))
                    } else {
                        FoldWhile::Continue((str_len + content_len, None))
                    }
                })
                .into_inner()
                .1
        }
        fn find_line_index_for_col(
            tokens: &[Token],
            line: &LogicalLine,
            column: usize,
        ) -> Option<u32> {
            let global_token_index = find_token_index_for_col(tokens, column)?;
            line.get_tokens()
                .iter()
                .enumerate()
                .find(|(_, token_index)| **token_index == global_token_index)
                .map(|(line_index, _)| line_index as u32)
        }

        impl FromStr for ContextType {
            type Err = String;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                use BracketKind as BK;
                use BracketStyle as BS;
                const PRECEDENCE_INIT: &str = "Precedence(";
                match s {
                    "Base" => Ok(CT::Base),
                    "CParens" => Ok(CT::Brackets(BK::Round, BS::ContClose)),
                    "IParens" => Ok(CT::Brackets(BK::Round, BS::Invisible)),
                    "BParens" => Ok(CT::Brackets(BK::Round, BS::BreakClose)),
                    "EParens" => Ok(CT::Brackets(BK::Round, BS::Expanded)),
                    "CGenerics" => Ok(CT::Brackets(BK::Angle, BS::ContClose)),
                    "IGenerics" => Ok(CT::Brackets(BK::Angle, BS::Invisible)),
                    "BGenerics" => Ok(CT::Brackets(BK::Angle, BS::BreakClose)),
                    "EGenerics" => Ok(CT::Brackets(BK::Angle, BS::Expanded)),
                    "CBrackets" => Ok(CT::Brackets(BK::Square, BS::ContClose)),
                    "IBrackets" => Ok(CT::Brackets(BK::Square, BS::Invisible)),
                    "BBrackets" => Ok(CT::Brackets(BK::Square, BS::BreakClose)),
                    "EBrackets" => Ok(CT::Brackets(BK::Square, BS::Expanded)),

                    "CommaList" => Ok(CT::CommaList),
                    "CommaElem" => Ok(CT::CommaElem),
                    "SemicolonList" => Ok(CT::SemicolonList),
                    "SemicolonElem" => Ok(CT::SemicolonElem),

                    "Type" => Ok(CT::Type),
                    "Assignment" => Ok(CT::Assignment),
                    "TypedAssignment" => Ok(CT::TypedAssignment),
                    "AssignLHS" => Ok(CT::AssignLHS),
                    "AssignRHS" => Ok(CT::AssignRHS),

                    "ControlFlow" => Ok(CT::ControlFlow),
                    "ControlFlowBegin" => Ok(CT::ControlFlowBegin),
                    "MemberAccess" => Ok(CT::MemberAccess),
                    "ForLoop" => Ok(CT::ForLoop),
                    "Subject" => Ok(CT::Subject),
                    "AnonHeader" => Ok(CT::AnonHeader),
                    "RoutineHeader" => Ok(CT::RoutineHeader),
                    "PropDec" => Ok(CT::PropDec),
                    "Raise" => Ok(CT::Raise),
                    "DirectivesLine" => Ok(CT::DirectivesLine),
                    "DirectiveList" => Ok(CT::DirectiveList),
                    "Directive" => Ok(CT::Directive),
                    "InlineDeclaration" => Ok(CT::InlineDeclaration),
                    context if context.starts_with(PRECEDENCE_INIT) && context.ends_with(")") => {
                        Ok(CT::Precedence(
                            context[PRECEDENCE_INIT.len()..context.len() - 1]
                                .parse()
                                .map_err(|e| format!("{e}"))?,
                        ))
                    }
                    _ => Err(format!("could not parse context type {:?}", s)),
                }
            }
        }

        /// Represents the information needed to make as assertion about the
        /// lifetime and data of a [`FormattingContext`]
        #[derive(Debug)]
        struct AssertionContext {
            context: FormattingContext,
            kill_token_id: Option<u32>,
        }
        impl AssertionContext {
            /// Parse a line describing all necessary features to make an
            /// assertion about [`FormattingContext`]
            /// <hr>
            ///
            /// <continuation_delta> (an unsigned integer)
            ///
            /// <[`ContextType`]> (the context type of the [`FormattingContext`])
            ///
            /// `^` (pointing to the [`FormattingContext`]'s starting token)
            ///
            /// `-`* (hyphens to indicate for which tokens the context is in
            ///     the [`SpecificContextStack`])
            ///
            /// `$`? (optionally pointing to the [`FormattingContext`]'s ending
            ///     token, not every [`FormattingContext`] has an ending token)
            ///
            /// `-`* (possibly more hyphens to indicate for which tokens the
            ///     context is in the [`SpecificContextStack`]; used to assert when
            ///     the [`FormattingContext`]'s lifetime in a [`SpecificContextStack`]
            ///     and the [`FormattingContext::ending_token`] are different)
            ///
            /// An example of this is:
            /// ```text
            ///                  begin var AA: BB := 11; end;
            ///   1 Base               ^----------------
            ///   1 TypedAssignment        ^----$------
            ///   1 InlineDeclaration      ^----$
            /// ```
            fn parse(
                context_line: &str,
                tokens: &[Token],
                line: &LogicalLine,
            ) -> Result<AssertionContext, DslParseError> {
                let mut chars = context_line.bytes().enumerate().peekable();
                take_ws(&mut chars);

                let continuation_delta = parse_part(context_line, &mut chars)
                    .and_then(|part| part.parse().ok())
                    .ok_or("could not parse continuation delta".into())?;

                let part = parse_part(context_line, &mut chars)
                    .ok_or("could not find context type".into())?;
                let context_type = part.parse().map_err(|e: String| e.into())?;

                take_ws(&mut chars);

                let start_col = chars
                    .next()
                    .and_then(|(start_col, c)| (c == b'^').then_some(start_col))
                    .ok_or(
                        "start indicator is not at the start of the context's lifetime".into(),
                    )?;

                take_hyphens(&mut chars);

                let end_col = chars.next().filter(|(_, c)| *c == b'$').map(|(i, _)| i);
                let kill_col = context_line.len() - 1;

                let ending_token = if let Some(column) = end_col {
                    Some(
                        find_line_index_for_col(tokens, line, column)
                            .ok_or("end marker should point to a token in the line".into())?,
                    )
                } else {
                    None
                };

                Ok(AssertionContext {
                    context: FormattingContext {
                        context_type,
                        continuation_delta,
                        starting_token: find_line_index_for_col(tokens, line, start_col)
                            .ok_or("the starting token should be on the given line".into())?,
                        ending_token,
                    },
                    kill_token_id: find_line_index_for_col(tokens, line, kill_col),
                })
            }
        }

        fn assert_contexts(input: &str) -> Result<(), DslParseError> {
            let AssertionData {
                asserting_line,
                all_tokens: tokens,
                contexts: expected_contexts,
            } = AssertionData::parse(input)
                .map_err(|e| format!("failed to parse test dsl\n{e}").into())?;

            eprintln!("expected contexts:");
            for context in &expected_contexts {
                eprintln!("_: {context:?}");
            }

            fn get_context_stack_at(
                expected_contexts: &[AssertionContext],
                line_index: u32,
            ) -> Vec<&FormattingContext> {
                let mut stack = Vec::new();
                for assertion in expected_contexts.iter() {
                    if line_index >= assertion.context.starting_token
                        && assertion
                            .kill_token_id
                            .is_none_or(|kill_tok| line_index <= kill_tok)
                    {
                        stack.push(&assertion.context);
                    }
                }
                stack
            }

            let token_types = tokens.iter().map(Token::get_token_type).collect_vec();
            let context_tree = LineFormattingContexts::new_tree();
            let actual_contexts =
                LineFormattingContexts::new(&asserting_line, &token_types, &context_tree);

            for line_index in 0..(asserting_line.get_tokens().len() as u32) {
                eprintln!(
                    "asserting at line index {line_index} -> {:?}",
                    asserting_line
                        .get_tokens()
                        .get(line_index as usize)
                        .and_then(|&idx| tokens.get(idx))
                        .unwrap()
                        .get_content()
                );
                let mut actual_contexts = actual_contexts
                    .get_specific_context_stack(line_index)
                    .ctx_iter_indices()
                    .map(|(_, c)| c.clone())
                    .collect_vec();
                actual_contexts.reverse();
                let expected_contexts = get_context_stack_at(&expected_contexts, line_index)
                    .into_iter()
                    .cloned()
                    .collect_vec();

                pretty_assertions::assert_eq!(&actual_contexts, &expected_contexts);
            }
            Ok(())
        }

        #[yare::parameterized(
            empty = {"
                          AAA();
                1 Base    ^-----
                1 BParens    ^$
            "},
            one = {"
                          AAA(BB);
                1 Base    ^-------
                1 BParens    ^--$
            "},
            many = {"
                          AAA(BB, CC, DD);
                1 Base    ^---------------
                1 BParens    ^----------$
                0 CommaList   ^--------$
            "},
        )]
        fn arguments(input: &str) -> Result<(), DslParseError> {
            assert_contexts(input)
        }

        #[yare::parameterized(
            empty = {"
                                 procedure AAA();
                1 Base           ^---------------
                1 RoutineHeader  ^---------------
                1 BParens                     ^$
            "},
            one = {"
                                  procedure AAA(BB: TT);
                1  Base           ^---------------------
                1  RoutineHeader  ^---------------------
                1  BParens                     ^------$
                1  SemicolonElem                ^----$
            "},
            default = {"
                                  procedure AAA(BB: TT = 11);
                1  Base           ^--------------------------
                1  RoutineHeader  ^--------------------------
                1  BParens                     ^-----------$
                1  SemicolonElem                ^---------$
                1  TypedAssignment              ^----$-----
                1  AssignLHS                    ^----$
            "},
            many = {"
                                  procedure AAA(BB: TT; CC: TT; DD: TT);
                1  Base           ^-------------------------------------
                1  RoutineHeader  ^-------------------------------------
                1  BParens                     ^----------------------$
                0  SemicolonList                ^--------------------$
                1  SemicolonElem                ^----$
                1  SemicolonElem                        ^----$
                1  SemicolonElem                                ^----$
            "},
            grouped = {"
                                  procedure AAA(BB, CC: TT; DD, EE: TT);
                1  Base           ^-------------------------------------
                1  RoutineHeader  ^-------------------------------------
                1  BParens                     ^----------------------$
                0  SemicolonList                ^--------------------$
                1  SemicolonElem                ^--------$
                0  CommaList                    ^----$
                1  SemicolonElem                            ^--------$
                0  CommaList                                ^----$
            "},
        )]
        fn parameters(input: &str) -> Result<(), DslParseError> {
            assert_contexts(input)
        }

        #[yare::parameterized(
            continued_after = {"
                                 (AAA + BBB) + CCC
                1 Base           ^----------------
                1 Precedence(3)  ^----------------
                0 IParens        ^---------$
                1 Precedence(3)   ^-------$
            "},
            continued_before = {"
                                 AAA + (BBB + CCC)
                1 Base           ^----------------
                1 Precedence(3)  ^----------------
                0 IParens              ^---------$
                1 Precedence(3)         ^-------$
            "},
        )]
        fn paren_expressions(input: &str) -> Result<(), DslParseError> {
            assert_contexts(input)
        }

        #[yare::parameterized(
            continued_after = {"
                                 FFF(AAA + BBB) + CCC
                1 Base           ^-------------------
                1 Precedence(3)  ^-------------------
                1 CParens           ^---------$
                1 Precedence(3)      ^-------$
            "},
            continued_before = {"
                                 AAA + FFFF(BBB + CCC)
                1 Base           ^--------------------
                1 Precedence(3)  ^--------------------
                1 CParens                  ^---------$
                1 Precedence(3)             ^-------$
            "},
            not_continued_outside = {"
                                 FFFF(BBB + CCC)
                1 Base           ^--------------
                1 BParens            ^---------$
                1 Precedence(3)       ^-------$
            "},
            member_access = {"
                                 FFFF.GGGG(BBB + CCC)
                1 Base           ^-------------------
                1 MemberAccess   ^-------------------
                1 BParens                 ^---------$
                1 Precedence(3)            ^-------$
            "},
        )]
        fn cont_break_parens(input: &str) -> Result<(), DslParseError> {
            assert_contexts(input)
        }

        #[yare::parameterized(
            one = {"
                          type AA<AA> = class end;
                1 Base         ^-------------
                1 Assignment   ^----$--------
                1 AssignLHS    ^----$
                1 BGenerics      ^--$
                1 AssignRHS             ^----
            "},
            commas = {"
                          type AA<AA, BB, CC> = class end;
                1 Base         ^---------------------
                1 Assignment   ^------------$--------
                1 AssignLHS    ^------------$
                1 BGenerics      ^----------$
                0 CommaList       ^--------$
                1 AssignRHS                     ^----
            "},
            semicolons = {"
                          type AA<AA; BB; CC> = class end;
                1 Base         ^---------------------
                1 Assignment   ^------------$--------
                1 AssignLHS    ^------------$
                1 BGenerics      ^----------$
                0 SemicolonList   ^--------$
                1 AssignRHS                     ^----
            "},
            constraints = {"
                           type AA<AA: class, constructor; BB: record> = class end;
                1 Base          ^---------------------------------------------
                1 Assignment    ^------------------------------------$--------
                1 AssignLHS     ^------------------------------------$
                1 BGenerics       ^----------------------------------$
                0 SemicolonList    ^--------------------------------$
                1 SemicolonElem    ^--------------------$
                0 CommaList            ^----------------$
                1 SemicolonElem                            ^--------$
                1 AssignRHS                                              ^----
            "},
        )]
        fn generics(input: &str) -> Result<(), DslParseError> {
            assert_contexts(input)
        }

        #[yare::parameterized(
            abstract_class_parents = {"
                          type AA = class abstract() end;
                1 Base         ^--------------------
                1 AssignRHS         ^---------------
                1 BParens                         ^$
            "},
            sealed_class_parents = {"
                          type AA = class sealed() end;
                1 Base         ^------------------
                1 AssignRHS         ^-------------
                1 BParens                       ^$
            "},
        )]
        fn type_headings(input: &str) -> Result<(), DslParseError> {
            assert_contexts(input)
        }

        #[yare::parameterized(
            one_level = {"
                                AA + BB
                1 Base          ^------
                1 Precedence(3) ^------
            "},
            bigger_first = {"
                                AA + BB * CC
                1 Base          ^-----------
                1 Precedence(3) ^-----------
                1 Precedence(2)      ^------
            "},
            smaller_first = {"
                                AA * BB + CC
                1 Base          ^-----------
                1 Precedence(3) ^-----------
                1 Precedence(2) ^-----$
            "},
            routine_arguments = {"
                                AA(BB, CC) + DD
                1 Base          ^--------------
                1 Precedence(3) ^--------------
                1 CParens         ^------$
                0 CommaList        ^----$
            "},
            member_access = {"
                                AA.BB(CC, DD) + EE
                1 Base          ^-----------------
                1 Precedence(3) ^-----------------
                1 MemberAccess  ^-----------$
                1 CParens            ^------$
                0 CommaList           ^----$
            "},
            fluent_routine_arguments = {"
                                AA(BB, CC).DD + EE
                1 Base          ^-----------------
                1 Precedence(3) ^-----------------
                1 Precedence(0) ^-----------$
                1 CParens         ^------$
                0 CommaList        ^----$
            "},
        )]
        fn precedence(input: &str) -> Result<(), DslParseError> {
            assert_contexts(input)
        }

        #[yare::parameterized(
            inline_if = {"
                                if AA then;
                1 Base          ^---------
                1 ControlFlow   ^---------
            "},
            compound_if = {"
                                    if AA then begin end;
                1 Base              ^---------------
                0 ControlFlowBegin  ^---------------
                1 ControlFlow       ^--------$
            "},
            inline_while = {"
                                while AA do;
                1 Base          ^----------
                1 ControlFlow   ^----------
            "},
            compound_while = {"
                                    while AA do begin end;
                1 Base              ^----------------
                0 ControlFlowBegin  ^----------------
                1 ControlFlow       ^---------$
            "},
            inline_on = {"
                                try except on AA do end;
                1 Base                     ^-------
                1 ControlFlow              ^-------
            "},
            compound_on = {"
                                    try except on AA do begin end end;
                1 Base                         ^-------------
                0 ControlFlowBegin             ^-------------
                1 ControlFlow                  ^------$
            "},
        )]
        fn control_flow(input: &str) -> Result<(), DslParseError> {
            assert_contexts(input)
        }

        #[yare::parameterized(
            inline = {"
                                with AA do;
                1 Base          ^---------
                1 ControlFlow   ^---------
            "},
            compound = {"
                                    with AA do begin end;
                1 Base              ^---------------
                0 ControlFlowBegin  ^---------------
                1 ControlFlow       ^--------$
            "},
            multi = {"
                                    with AA, BB, CC do;
                1 Base              ^-----------------
                1 ControlFlow       ^-----------------
                0 CommaList              ^--------$
            "},
        )]
        fn with(input: &str) -> Result<(), DslParseError> {
            assert_contexts(input)
        }

        #[yare::parameterized(
            basic = {"
                                case AA of end;
                1 Base          ^---------
                1 ControlFlow   ^---------
            "},
            complex = {"
                                    case AA.BB(CC) of end;
                1 Base              ^----------------
                1 ControlFlow       ^----------------
                1 MemberAccess           ^-------$
                1 BParens                     ^--$
            "},
        )]
        fn case(input: &str) -> Result<(), DslParseError> {
            assert_contexts(input)
        }

        #[yare::parameterized(
            basic = {"
                                repeat until AA;
                1 Base                 ^--------
                1 ControlFlow          ^------$
            "},
            complex = {"
                                    repeat until AA.BB(CC);
                1 Base                     ^---------------
                1 ControlFlow              ^-------------$
                1 MemberAccess                   ^-------$
                1 BParens                             ^--$
            "},
        )]
        fn until(input: &str) -> Result<(), DslParseError> {
            assert_contexts(input)
        }

        #[yare::parameterized(
            for_in_inline = {"
                                for AA in BB do;
                1 Base          ^--------------
                1 ForLoop       ^--------------
                1 Subject              ^---$
            "},
            for_in_compound = {"
                                    for AA in BB do begin end;
                1 Base              ^--------------------
                0 ControlFlowBegin  ^--------------------
                1 ForLoop           ^-------------$
                1 Subject                  ^---$
            "},
            for_to_inline = {"
                                for AA := BB to CC do;
                1 Base          ^--------------------
                1 ForLoop       ^--------------------
                1 Subject           ^------$
                1 Subject                    ^---$
            "},
            for_to_compound = {"
                                    for AA := BB to CC do begin end;
                1 Base              ^--------------------------
                0 ControlFlowBegin  ^--------------------------
                1 ForLoop           ^-------------------$
                1 Subject               ^------$
                1 Subject                        ^---$
            "},
            for_downto_inline = {"
                                for AA := BB downto CC do;
                1 Base          ^------------------------
                1 ForLoop       ^------------------------
                1 Subject           ^------$
                1 Subject                    ^-------$
            "},
            for_downto_compound = {"
                                    for AA := BB downto CC do begin end;
                1 Base              ^------------------------------
                0 ControlFlowBegin  ^------------------------------
                1 ForLoop           ^-----------------------$
                1 Subject               ^------$
                1 Subject                        ^-------$
            "},
        )]
        fn for_loop(input: &str) -> Result<(), DslParseError> {
            assert_contexts(input)
        }

        #[yare::parameterized(
            raise = {"
                          raise AA.BB;
                1 Base    ^-----------
                1 Raise   ^---------$
                1 MemberAccess  ^---$
            "},
            raise_at = {"
                          raise AA.BB at CC + DD;
                1 Base    ^----------------------
                1 Raise   ^--------------------$
                1 MemberAccess  ^---$
                1 Subject             ^--------$
                1 Precedence(3)          ^-----$
            "},
        )]
        fn raise(input: &str) -> Result<(), DslParseError> {
            assert_contexts(input)
        }

        #[yare::parameterized(
            procedure = {"
                                 procedure AA; static; deprecated 'message';
                1 Base           ^------------------------------------------
                1 DirectivesLine ^------------------------------------------
                1 RoutineHeader  ^-----------$
                0 DirectiveList                ^--------------------------$
                1 Directive                            ^------------------$
            "},
            function = {"
                                 function AA: BB; static; deprecated 'message';
                1 Base           ^---------------------------------------------
                1 DirectivesLine ^---------------------------------------------
                1 RoutineHeader  ^--------------$
                0 DirectiveList                   ^--------------------------$
                1 Directive                               ^------------------$
            "},
            bare_property = {"
                                 property AA;
                1  Base          ^-----------
                1  PropDec       ^---------$
            "},
            typed_bare_property = {"
                                property AA: TT;
                1 Base          ^---------------
                1 PropDec       ^-------------$
            "},
            directives_property = {"
                                 property AA: TT read BB write CC index 1 + 2;
                1 Base           ^--------------------------------------------
                1 DirectivesLine ^--------------------------------------------
                1 PropDec        ^-------------$
                0 DirectiveList                  ^--------------------------$
                1 Directive                      ^-----$
                1 Directive                              ^------$
                1 Directive                                       ^---------$
                1 Precedence(3)                                         ^---$
            "},
        )]
        fn directives(input: &str) -> Result<(), DslParseError> {
            assert_contexts(input)
        }

        #[yare::parameterized(
            variable = {"
                                AA := BB;
                1 Base          ^--------
            "},
            expr_to_field = {"
                                AA.BB := CC + DD;
                1 Base          ^----------------
                1 Assignment    ^---$-----------
                1 AssignLHS     ^---$
                1 MemberAccess  ^---$
                1 AssignRHS              ^-----$
                1 Precedence(3)          ^-----$
            "},
            inline_const = {"
                            begin const AA = 11; end;
                1 Base            ^-------------
            "},
            inline_var = {"
                            begin var AA := 11; end;
                1 Base            ^------------
            "},
            typed_inline_var = {"
                               begin var AA: BB := 11; end;
                1 Base               ^----------------
                1 TypedAssignment        ^----$------
                1 InlineDeclaration      ^----$
            "},
        )]
        fn assignment(input: &str) -> Result<(), DslParseError> {
            assert_contexts(input)
        }

        #[yare::parameterized(
            assignment = {"
                                AA := procedure(BB: CC) var DD: EE; begin FF; end;
                1 Base          ^-------------------------------------------------
                1 AssignRHS           ^-----------------------------------------$
                1 AnonHeader          ^---------------$
                1 BParens                      ^------$
                1 SemicolonElem                 ^----$
            "},
            argument = {"
                                AA(function(BB: CC): DD var EE: FF; begin FF; end);
                1 Base          ^--------------------------------------------------
                1 BParens         ^----------------------------------------------$
                1 CommaElem        ^--------------------------------------------$
                1 AnonHeader       ^------------------$
                1 BParens                  ^------$
                1 SemicolonElem             ^----$
            "},
        )]
        fn anonymous(input: &str) -> Result<(), DslParseError> {
            assert_contexts(input)
        }

        #[yare::parameterized(
            uses = {"
                                uses AA, BB, CC in 'CC.pas';
                1 Base               ^----------------------
                0 CommaList          ^--------------------$
                1 CommaElem                  ^------------$
                1 Precedence(4)              ^------------$
            "},
            requires = {"
                               package A; requires AA, BB, CC;
                1 Base                             ^----------
                0 CommaList                        ^--------$
            "},
            contains = {"
                               package A; contains AA, BB, CC in 'CC.pas';
                1 Base                             ^----------------------
                0 CommaList                        ^--------------------$
                1 CommaElem                                ^------------$
                1 Precedence(4)                            ^------------$
            "},
        )]
        fn import(input: &str) -> Result<(), DslParseError> {
            assert_contexts(input)
        }

        #[yare::parameterized(
            names = {"
                               exports AA, BB, CC;
                1 Base                 ^----------
                0 CommaList            ^--------$
            "},
            directives = {"
                               exports AA index AA name AA, BB index BB name BB;
                1 Base                 ^----------------------------------------
                0 CommaList            ^--------------------------------------$
                1 CommaElem            ^-----------------$
                0 DirectiveList           ^--------------$
                1 Directive               ^-----$
                1 Directive                        ^-----$
                1 CommaElem                                 ^-----------------$
                0 DirectiveList                                ^--------------$
                1 Directive                                    ^-----$
                1 Directive                                             ^-----$
            "},
        )]
        fn exports(input: &str) -> Result<(), DslParseError> {
            assert_contexts(input)
        }

        #[yare::parameterized(
            label = {"
                           label a, b, c;
                1 Base           ^-------
                0 CommaList      ^-----$
            "},
            var = {"
                           var a, b, c;
                1 Base         ^-------
                0 CommaList    ^-----$
            "},
            enum_with_val = {"
                           type AA = (B = 0, C = 1);
                1 Base          ^-------------------
                1 AssignRHS          ^------------$
                1 BParens            ^------------$
                0 CommaList           ^----------$
                1 CommaElem           ^---$
                1 CommaElem                  ^---$
            "},
        )]
        fn declarations(input: &str) -> Result<(), DslParseError> {
            assert_contexts(input)
        }

        #[yare::parameterized(
            var_decl = {"
                           var A: B;
                1 Base         ^----
            "},
            var_list_decl = {"
                           var A, B: C;
                1 Base         ^-------
                0 CommaList    ^--$
            "},
            var_decl_assign = {"
                              var A: B = 1;
                1 Base            ^--------
                1 TypedAssignment ^--$----
                1 AssignLHS       ^--$
            "},
            record_initialiser = {"
                              const A: array of T = ((A: B; C: D), (E: F; G: H));
                1 Base              ^--------------------------------------------
                1 TypedAssignment   ^-----------$-------------------------------
                1 AssignLHS         ^-----------$
                1 Type                 ^--------$
                1 AssignRHS                         ^--------------------------$
                1 BParens                           ^--------------------------$
                0 CommaList                          ^------------------------$
                1 CommaElem                          ^----------$
                1 EParens                            ^----------$
                0 SemicolonList                       ^--------$
                1 SemicolonElem                       ^--$
                1 SemicolonElem                             ^--$
                1 CommaElem                                        ^----------$
                1 EParens                                          ^----------$
                0 SemicolonList                                     ^--------$
                1 SemicolonElem                                     ^--$
                1 SemicolonElem                                           ^--$
            "},
            anonymous_header = {"
                                A(function A: B begin end, function C: D begin end);
                1 Base          ^---------------------------------------------------
                1 BParens        ^------------------------------------------------$
                0 CommaList       ^----------------------------------------------$
                1 CommaElem       ^---------------------$
                1 AnonHeader      ^-----------$
                1 CommaElem                                ^---------------------$
                1 AnonHeader                               ^-----------$
            "},
            param_decl = {"
                                procedure A(B: C; D: E);
                1 Base          ^-----------------------
                1 RoutineHeader ^-----------------------
                1 BParens                  ^----------$
                0 SemicolonList             ^--------$
                1 SemicolonElem             ^--$
                1 SemicolonElem                   ^--$
            "},
            param_list_decl = {"

                                procedure A(B, B: C; D, D: E);
                1 Base          ^-----------------------------
                1 RoutineHeader ^-----------------------------
                1 BParens                  ^----------------$
                0 SemicolonList             ^--------------$
                1 SemicolonElem             ^-----$
                0 CommaList                 ^--$
                1 SemicolonElem                      ^-----$
                0 CommaList                          ^--$
           "},
            param_decl_default = {"
                                procedure A(B: C = C; D: E = E);
                1 Base          ^-------------------------------
                1 RoutineHeader ^-------------------------------
                1 BParens                  ^------------------$
                0 SemicolonList             ^----------------$
                1 SemicolonElem             ^------$
                1 TypedAssignment           ^--$----
                1 AssignLHS                 ^--$
                1 SemicolonElem                       ^------$
                1 TypedAssignment                     ^--$----
                1 AssignLHS                           ^--$
            "},
            function_return = {"
                                function A: B.C;
                1 Base          ^---------------
                1 RoutineHeader ^---------------
                1 Type                      ^-$
                1 MemberAccess              ^-$
            "},
            property_type = {"
                           property A: B.C;
                1 Base     ^---------------
                1 PropDec  ^-------------$
                1 Type                 ^-$
                1 MemberAccess         ^-$
            "},
        )]
        fn colon(input: &str) -> Result<(), DslParseError> {
            assert_contexts(input)
        }

        #[yare::parameterized(
            after_semicolon = {"
                                  procedure A(B: C; {} D: E);
                1 Base            ^--------------------------
                1 RoutineHeader   ^--------------------------
                1 BParens                    ^-------------$
                0 SemicolonList               ^-----------$
                1 SemicolonElem               ^--$
                1 SemicolonElem                        ^--$
            "},
            before_semicolon = {"
                                  procedure A(B: C {}; D: E);
                1 Base            ^--------------------------
                1 RoutineHeader   ^--------------------------
                1 BParens                    ^-------------$
                0 SemicolonList               ^-----------$
                1 SemicolonElem               ^-----$
                1 Type                           ^--$
                1 SemicolonElem                        ^--$
            "},
            after_comma = {"
                                  procedure A(B, C, {} D, E);
                1 Base            ^--------------------------
                1 RoutineHeader   ^--------------------------
                1 BParens                    ^-------------$
                0 CommaList                   ^-----------$
            "},
            before_comma = {"
                                  procedure A(B, C {}, D, E);
                1 Base            ^--------------------------
                1 RoutineHeader   ^--------------------------
                1 BParens                    ^-------------$
                0 CommaList                   ^-----------$
                1 CommaElem                      ^--$
            "},
        )]
        fn comments_in_lists(input: &str) -> Result<(), DslParseError> {
            assert_contexts(input)
        }
    }
}
