//! This module's soul purpose is to reflow [`LogicalLine`]s within the bounds
//! (where possible) of the line length limit.
//!

use std::collections::BinaryHeap;
use std::rc::Rc;

use debug::DebugFormattingNode;
use debug::RawDebugLine;
use fxhash::FxHashMap;
use log::log_enabled;
use log::{error, trace};

use contexts::*;
use parent_pointer_tree::{NodeRef, ParentPointerTree};
use types::DecisionRequirement as DR;
use types::RawDecision as NL;
use types::*;

use crate::lang::ChevronKind as ChK;
use crate::lang::KeywordKind as KK;
use crate::lang::LogicalLineType as LLT;
use crate::lang::OperatorKind as OK;
use crate::lang::TokenType as TT;
use crate::prelude::*;

pub struct OptimisingLineFormatterSettings {
    pub max_line_length: u32,
    pub iteration_max: u32,
}

pub struct OptimisingLineFormatter {
    olf_settings: OptimisingLineFormatterSettings,
    recon_settings: ReconstructionSettings,
}

/// Realistically, the [`OptimisingLineFormatter`] is a
/// [`LogicalLineFormatter`]. However, it is beneficial to reuse much of the
/// data constructed in [`OptimisingLineFormatter::format`] across all lines in
/// a file.
impl LogicalLineFileFormatter for OptimisingLineFormatter {
    fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &[LogicalLine]) {
        let mut line_children: FxHashMap<LineParent, Vec<usize>> = FxHashMap::default();
        input
            .iter()
            .enumerate()
            .filter_map(|(line_index, line)| line.get_parent().map(|parent| (line_index, parent)))
            .for_each(|(line_index, parent)| {
                line_children.entry(parent).or_default().push(line_index);
            });

        let (token_types, token_lengths) = formatted_tokens
            .get_tokens()
            .iter()
            .map(|token| {
                (
                    token.0.get_token_type(),
                    TokenLength {
                        spaces_before: token.1.spaces_before as u32,
                        content: token.0.get_content().len() as u32,
                    },
                )
            })
            .collect();

        let mut olf = InternalOptimisingLineFormatter {
            settings: &self.olf_settings,
            recon_settings: &self.recon_settings,
            formatted_tokens,
            lines: input,
            line_children: &line_children,
            token_types,
            token_lengths,
        };

        for line in input
            .iter()
            .enumerate()
            .filter(|(_, line)| line.get_parent().is_none() && line.get_line_type() != LLT::Eof)
        {
            if let Some(solution) = olf.format_line(line) {
                olf.reconstruct_solution(&solution, line.1);
            }
        }

        /*
            After each line's solution has been finalised, the extra spaces
            provided by `TokenSpacing` can be removed at the starts of lines.
        */
        for token_index in 0..formatted_tokens.get_tokens().len() {
            if let Some(data) = formatted_tokens.get_formatting_data_mut(token_index) {
                if data.newlines_before > 0 {
                    data.spaces_before = 0;
                }
            }
        }
    }
}
impl OptimisingLineFormatter {
    pub fn new(
        olf_settings: OptimisingLineFormatterSettings,
        recon_settings: ReconstructionSettings,
    ) -> Self {
        OptimisingLineFormatter {
            olf_settings,
            recon_settings,
        }
    }
}

enum FormattingSolutionError {
    NoSolutionFound,
    IterationLimitReached,
}

/// Length of each token in a file
#[derive(Clone, Copy)]
struct TokenLength {
    spaces_before: u32,
    content: u32,
}
struct InternalOptimisingLineFormatter<'this, 'token> {
    settings: &'this OptimisingLineFormatterSettings,
    recon_settings: &'this ReconstructionSettings,
    formatted_tokens: &'this mut FormattedTokens<'token>,
    lines: &'this [LogicalLine],
    line_children: &'this FxHashMap<LineParent, Vec<usize>>,
    token_types: Vec<TokenType>,
    /// Pre-calculated length data for each token in the file. This minimises the
    /// time to recalculate this data on repeated passes of tokens. A particular
    /// example of this is in the case of child-lines and their reformatting.
    token_lengths: Vec<TokenLength>,
}

impl<'this> InternalOptimisingLineFormatter<'this, '_> {
    fn format_line(&self, line: (usize, &LogicalLine)) -> Option<FormattingSolution> {
        if line.1.get_line_type() == LLT::AsmInstruction {
            trace!(
                "Skipping formatting `{:?}` line:\n{:?}",
                line.1.get_line_type(),
                RawDebugLine::new(line, self.formatted_tokens)
            );
            return None;
        } else {
            trace!(
                "Formatting `{:?}` line:\n{:?}",
                line.1.get_line_type(),
                RawDebugLine::new(line, self.formatted_tokens)
            );
        }
        let optimal_solution = self.find_optimal_solution(
            LineWhitespace {
                indentations: line.1.get_level(),
                continuations: 0,
            },
            line,
            match line.1.get_tokens().first() {
                Some(0) => FirstDecision::Continue { line_length: 0 },
                _ => FirstDecision::Break,
            },
        );

        optimal_solution
            .inspect_err(|err| {
                error!(
                    "{} for\n{:?}",
                    match err {
                        FormattingSolutionError::IterationLimitReached => "Iteration limit reached",
                        FormattingSolutionError::NoSolutionFound => "No solution found",
                    },
                    RawDebugLine::new(line, self.formatted_tokens),
                )
            })
            .ok()
    }

    fn reconstruct_solution(&mut self, solution: &FormattingSolution, input_line: &LogicalLine) {
        for (decision_index, decision) in solution.decisions.iter().enumerate() {
            let global_token_index = *input_line
                .get_tokens()
                .get(decision_index)
                .expect("line should contain token for decision");

            let formatting_data = self
                .formatted_tokens
                .get_formatting_data_mut(global_token_index)
                .expect("formatting data should exist for token");

            match decision.decision {
                Decision::Break { continuations } => {
                    /*
                        We don't mutate `spaces_before` here, as there may be
                        another line to format which would also mutate the same
                        tokens.
                        This removal of the spaces is then deferred to the end
                        of the file's processing.
                    */
                    if decision_index == 0 {
                        /*
                            For the first token in a line, the user has the
                            ability to add a blank line to logically group
                            their statements together.
                        */
                        formatting_data.newlines_before =
                            formatting_data.newlines_before.clamp(1, 2);
                    } else {
                        formatting_data.newlines_before = 1;
                    }
                    formatting_data.indentations_before = solution.starting_ws.indentations;
                    formatting_data.continuations_before =
                        solution.starting_ws.continuations + continuations;
                }
                Decision::Continue => {
                    formatting_data.newlines_before = 0;
                    formatting_data.continuations_before = 0;
                    formatting_data.indentations_before = 0;
                }
            }

            decision
                .child_solutions
                .iter()
                .for_each(|(child_line_index, child_solution)| {
                    let child_line = &self.lines[*child_line_index];
                    self.reconstruct_solution(child_solution, child_line);
                });
        }
    }

    fn find_optimal_solution(
        &self,
        starting_ws: LineWhitespace,
        line: (usize, &LogicalLine),
        first_token_decision: FirstDecision,
    ) -> Result<FormattingSolution, FormattingSolutionError> {
        let context_tree = LineFormattingContexts::new_tree();
        let formatting_contexts =
            LineFormattingContexts::new(line.1, &self.token_types, &context_tree);

        let mut node_heap = BinaryHeap::new();
        let mut best_penalties = vec![u64::MAX; line.1.get_tokens().len()];

        let Some(&first_token_index) = line.1.get_tokens().first() else {
            // Trivial solution for a line with no tokens
            return Ok(FormattingSolution {
                starting_ws,
                decisions: vec![],
                penalty: 0,
                solution_length: 0,
            });
        };

        let TokenLength {
            spaces_before,
            content: content_len,
        } = self.token_lengths[first_token_index];

        let (new_line, requirement, last_line_length, base_can_break) = match first_token_decision {
            FirstDecision::Break => (
                Decision::Break { continuations: 0 },
                DR::MustBreak,
                starting_ws.len(self.recon_settings) + content_len,
                true,
            ),
            FirstDecision::Continue { line_length } => (
                Decision::Continue,
                DR::MustNotBreak,
                line_length + spaces_before + content_len,
                // If it is the first token, the line can still be broken
                first_token_index == 0,
            ),
        };

        let decision_tree = ParentPointerTree::new(TokenDecision {
            decision: new_line,
            requirement,
            last_line_length,
            child_solutions: Vec::new(),
        });

        let mut node = FormattingNode {
            starting_ws,
            decision: decision_tree.root(),
            next_line_index: 1,
            context_data: formatting_contexts.get_default_context_data(),
            penalty: 0,
        };
        if let Some(base_context) = Rc::make_mut(&mut node.context_data).first_mut() {
            base_context.can_break = base_can_break;
        }
        node_heap.push(node);

        let mut iteration_count = 0;
        let mut node_successors = Vec::new();

        'node_heap: while let Some(mut node) = node_heap.pop() {
            if iteration_count > self.settings.iteration_max {
                return Err(FormattingSolutionError::IterationLimitReached);
            }
            iteration_count += 1;

            trace!("Popping node from the heap");
            if node.next_line_index as usize >= line.1.get_tokens().len() {
                let solution = node.into();
                self.solution_debugging(line, &node_heap, iteration_count, &solution);
                return Ok(solution);
            }
            if node.penalty > best_penalties[(node.next_line_index - 1) as usize] {
                trace!(
                    "Pruning due to better node penalty ({}) for the current node ({})",
                    best_penalties[(node.next_line_index - 1) as usize],
                    node.penalty
                );
                continue;
            }

            /*
                This loop implements the "Successor Compression" optimisation.
                When a decision only has one successor, the path is continually
                explored until there is another branch in the decisions.
                This is most effective when the formatting rules are decisive
                (i.e., not `Indifferent`).
            */
            loop {
                node_successors.clear();

                let mut indifference_line: Option<(FormattingNode, SpecificContextStack)> = None;
                /*
                    This loop implements the "Indifference Compression" optimisation.
                    When exploring a solution, we can make note of the first
                    `Indifferent` requirement we come across. From here we can
                    assume all `Indifferent` requirements to be `Continue`
                    decisions until one of the following conditions is met:
                        - The current line is over line length limit
                          From here, we return to the initial `Indifferent`
                          requirement and treat it as a branch
                        - A `MustBreak` is encountered
                          The `Indifferent` `Continue` decisions are accepted
                          and the tree traversal continues
                        - There are multiple possible solutions
                          From here the possibilities are added to the heap
                */
                'indiff: loop {
                    let line_index = node.next_line_index;
                    let contexts = formatting_contexts.get_specific_context_stack(line_index);

                    trace!(
                        "{}\n{:?}",
                        if indifference_line.is_some() {
                            "Exploring node collapsing indifferences:"
                        } else {
                            "Exploring node:"
                        },
                        DebugFormattingNode::new(&node, line.1, &contexts, self)
                    );

                    let last_line_length = node.decision.get().last_line_length;
                    if last_line_length > self.settings.max_line_length {
                        trace!(
                            "Last line length {} > max line length {}, line is too long",
                            last_line_length,
                            self.settings.max_line_length
                        );
                        if let Some((indiff, _stack)) = indifference_line {
                            trace!(
                                "Returning to first `Indifferent` decision to push all successors"
                            );
                            let stack = formatting_contexts
                                .get_specific_context_stack(indiff.next_line_index);
                            node_successors.extend(self.get_potential_solution(
                                indiff.clone(),
                                line,
                                &stack,
                                NL::Break,
                                DR::Indifferent,
                            ));
                            node_successors.extend(self.get_potential_solution(
                                indiff,
                                line,
                                &stack,
                                NL::Continue,
                                DR::Indifferent,
                            ));
                            break 'indiff;
                        }
                    }

                    if line_index as usize >= line.1.get_tokens().len() {
                        // If the traversal is at the end, add it back to the
                        // heap. The first solution with a `FormattingDecision`
                        // for each token, is not necessarily the best solution.
                        trace!(
                            "Potential solution found, adding to heap\n{:?}",
                            DebugFormattingNode::new(&node, line.1, &contexts, self)
                        );
                        node_heap.push(node);
                        continue 'node_heap;
                    }

                    let requirement = self.get_formatting_requirement(
                        node.next_line_index,
                        line.1,
                        &contexts.with_data(&node),
                    );

                    let get_solutions = |raw_decision, node, stack| {
                        self.get_potential_solution(node, line, stack, raw_decision, requirement)
                    };

                    trace!("Decision requirement: {requirement:?}");
                    match requirement {
                        DR::Invalid => {
                            // If there is no point at which we have collapsed
                            // the indifferents then it really is invalid, skip
                            // this branch. Otherwise, we need to go back to the
                            // indifferent and add its possibilities.

                            if let Some((indiff, stack)) = indifference_line {
                                trace!("Returning to first `Indifferent` decision to push all successors");
                                node_successors.extend(get_solutions(
                                    NL::Break,
                                    indiff.clone(),
                                    &stack,
                                ));
                                node_successors.extend(get_solutions(NL::Continue, indiff, &stack));
                                break 'indiff;
                            } else {
                                trace!("Dead-end branch found");
                                continue 'node_heap;
                            }
                        }
                        DR::MustBreak => {
                            // If we have to break the line, the collapsing is
                            // over and we continue with the proposed solution.
                            // By breaking at this `MustBreak` we are segmenting
                            // the formatting of the lines. For example, this
                            // allows elements of a list to be formatted mostly
                            // independently from one another - once the best
                            // solution for an element has been found, it will
                            // continue to the next without exploring further
                            // permutations with future elements.
                            // This optimisation prunes branches that are
                            // sub-optimal at each `MustBreak`. A limitation of
                            // this approach is that, if later decisions are
                            // deemed incompatible with this partial solution,
                            // no solution will be found.

                            if node.penalty < best_penalties[node.next_line_index as usize] {
                                best_penalties[node.next_line_index as usize] = node.penalty;
                                node_successors.extend(get_solutions(NL::Break, node, &contexts));
                                trace!("Updating best penalty for node, exiting indifference loop");
                            } else {
                                trace!("There is has been a better penalty explored for the current node");
                            }
                            break 'indiff;
                        }
                        DR::MustNotBreak => {
                            node_successors.extend(get_solutions(NL::Continue, node, &contexts));
                        }
                        DR::Indifferent => {
                            if indifference_line.is_none() {
                                trace!("Saving back-tracking point");
                                indifference_line = Some((node.clone(), contexts.clone()));
                            }
                            node_successors.extend(get_solutions(NL::Continue, node, &contexts));
                        }
                    };
                    if node_successors.len() == 1 {
                        node = node_successors.remove(0);
                        trace!("Continuing to explore single successor branch");
                    } else {
                        if let Some((indiff, stack)) = indifference_line {
                            trace!("Multiple successors found, returning to first `Indifferent` decision to push all successors");
                            node_successors.extend(get_solutions(
                                NL::Break,
                                indiff.clone(),
                                &stack,
                            ));
                            node_successors.extend(get_solutions(NL::Continue, indiff, &stack));
                        }
                        break 'indiff;
                    }
                }

                if node_successors.len() == 1 {
                    node = node_successors.remove(0);
                    trace!("Continuing to explore single successor branch");
                } else {
                    node_heap.extend(node_successors.drain(..).inspect(|node| {
                        trace!(
                            "Adding successor node to heap\n{:?}",
                            DebugFormattingNode::new(
                                node,
                                line.1,
                                &formatting_contexts
                                    .get_specific_context_stack(node.next_line_index),
                                self
                            )
                        );
                    }));
                    continue 'node_heap;
                }
            }
        }
        Err(FormattingSolutionError::NoSolutionFound)
    }

    fn get_potential_solution(
        &self,
        mut next_node: FormattingNode<'this>,
        line: (usize, &LogicalLine),
        contexts: &SpecificContextStack<'this>,
        raw_decision: RawDecision,
        requirement: DecisionRequirement,
    ) -> PotentialNodes<'this> {
        let line_index = next_node.next_line_index;

        contexts.update_contexts(&mut next_node, raw_decision);
        let continuation_count = contexts
            .with_data(&next_node)
            .get_continuation_count(line_index);
        let decision = raw_decision.with_continuation(continuation_count);

        let token_line_length = self.get_token_line_length(
            next_node.starting_ws,
            &next_node.decision,
            decision,
            line.1
                .get_tokens()
                .get(next_node.next_line_index as usize)
                .copied(),
        );

        let child_line_solutions = self.find_optimal_child_lines_solution(
            line,
            &next_node,
            decision,
            token_line_length,
            continuation_count,
        );

        let get_next_node =
            |mut next_node: FormattingNode<'this>,
             child_solutions: Vec<(usize, FormattingSolution)>| {
                contexts.update_contexts_from_child_solutions(&mut next_node, &child_solutions);

                next_node.penalty += child_solutions
                    .iter()
                    .map(|(_, solution)| solution.penalty)
                    .sum::<u64>();

                let decision = TokenDecision {
                    requirement,
                    decision,
                    last_line_length: Self::get_last_child_line_len(&child_solutions)
                        .unwrap_or(token_line_length),
                    child_solutions,
                };
                next_node.penalty += self.get_decision_penalty(&decision);
                next_node.decision = next_node.decision.add_successor(decision);
                next_node.next_line_index += 1;

                next_node
            };

        child_line_solutions.map_with(next_node, get_next_node)
    }

    fn find_optimal_child_lines_solution(
        &self,
        line: (usize, &LogicalLine),
        node: &FormattingNode,
        decision: Decision,
        token_line_length: u32,
        parent_continuations: u16,
    ) -> PotentialSolutions {
        let global_token_index = line.1.get_tokens()[node.next_line_index as usize];

        let Some(child_lines) = self.line_children.get(&LineParent {
            line_index: line.0,
            global_token_index,
        }) else {
            // There are no child solutions with no child lines
            return PotentialSolutions::One(vec![]);
        };

        // Child lines have a base indentation level 1 greater than their parent
        let child_starting_ws = LineWhitespace {
            indentations: node.starting_ws.indentations + 1,
            continuations: parent_continuations,
        };

        let starting_options = match (
            self.get_token_type(global_token_index),
            child_lines.len(),
            decision,
        ) {
            (Some(TT::Keyword(KK::Begin)), 1, Decision::Continue) => {
                Potentials::One((LineWhitespace::zero(), RawDecision::Continue))
            }
            (Some(TT::Op(OK::LParen)), _, Decision::Continue) => {
                // Variant record fields
                if !child_lines
                    .iter()
                    .flat_map(|&index| self.lines.get(index))
                    .map(|line| line.get_line_type())
                    .any(|typ| typ == LLT::CaseHeader)
                {
                    Potentials::Two(
                        (child_starting_ws, RawDecision::Break),
                        (LineWhitespace::zero(), RawDecision::Continue),
                    )
                } else {
                    // If there is a nested `case` in the declaration, it must `Break`
                    Potentials::One((child_starting_ws, RawDecision::Break))
                }
            }
            (_, _, Decision::Continue) => return PotentialSolutions::None,
            _ => Potentials::One((child_starting_ws, RawDecision::Break)),
        };

        let must_break = matches!(
            child_lines
                .first()
                .and_then(|&child_line| self.lines.get(child_line))
                .and_then(|child_line| self.get_prev_token_type_for_line_index(child_line, 0)),
            Some(
                TT::ConditionalDirective(_)
                    | TT::Comment(
                        CommentKind::IndividualLine
                            | CommentKind::InlineLine
                            | CommentKind::MultilineBlock,
                    ),
            )
        );

        starting_options.and_then(|(child_starting_ws, first_token_decision)| {
            if first_token_decision == RawDecision::Continue && must_break {
                return None;
            }
            let mut child_solutions = Vec::with_capacity(child_lines.len());
            let mut last_line_length = token_line_length;
            for line in child_lines
                .iter()
                .map(|&child_line| (child_line, &self.lines[child_line]))
            {
                let child_solution = self
                    .find_optimal_solution(
                        child_starting_ws
                            + LineWhitespace {
                                indentations: line.1.get_level(),
                                continuations: 0,
                            },
                        line,
                        match first_token_decision {
                            NL::Break => FirstDecision::Break,
                            NL::Continue => FirstDecision::Continue {
                                line_length: last_line_length,
                            },
                        },
                    )
                    .map(|solution| (line.0, solution))
                    .ok()?;
                if let Some(solution_len) = child_solution
                    .1
                    .decisions
                    .last()
                    .map(|decision| decision.last_line_length)
                {
                    last_line_length = solution_len;
                }
                child_solutions.push(child_solution);
            }
            Some(child_solutions)
        })
    }

    fn get_last_child_line_len(child_solutions: &[(usize, FormattingSolution)]) -> Option<u32> {
        child_solutions
            .last()
            .and_then(|sln| sln.1.decisions.last())
            .map(|decision| decision.last_line_length)
    }

    fn get_token_line_length(
        &self,
        starting_ws: LineWhitespace,
        prev_decision: &NodeRef<'this, TokenDecision>,
        decision: Decision,
        token_index: Option<usize>,
    ) -> u32 {
        match (
            decision,
            prev_decision.get(),
            token_index.and_then(|index| self.token_lengths.get(index)),
        ) {
            (Decision::Continue, parent, Some(&token_length)) => {
                parent.last_line_length + token_length.spaces_before + token_length.content
            }
            (Decision::Break { continuations }, _, Some(&token_length)) => {
                (starting_ws
                    + LineWhitespace {
                        indentations: 0,
                        continuations,
                    })
                .len(self.recon_settings)
                    + token_length.content
            }
            _ => starting_ws.len(self.recon_settings),
        }
    }

    fn get_decision_penalty(&self, decision: &TokenDecision) -> u64 {
        match decision.decision {
            Decision::Break { continuations: _ } => 3,
            Decision::Continue if decision.last_line_length > self.settings.max_line_length => {
                999999
            }
            _ => 0,
        }
    }

    fn get_token_type(&self, token_index: usize) -> Option<TokenType> {
        self.formatted_tokens
            .get_token(token_index)
            .map(|(token, _)| token.get_token_type())
    }

    fn get_token_type_for_line_index(
        &self,
        line: &LogicalLine,
        line_index: u32,
    ) -> Option<TokenType> {
        let &token_index = line.get_tokens().get(line_index as usize)?;
        self.formatted_tokens.get_token_type_for_index(token_index)
    }

    fn get_prev_token_type_for_line_index(
        &self,
        line: &LogicalLine,
        line_index: u32,
    ) -> Option<TokenType> {
        let &token_index = line.get_tokens().get(line_index as usize)?;
        let prev_index = token_index.checked_sub(1)?;
        self.formatted_tokens.get_token_type_for_index(prev_index)
    }
}

const HIGHEST_PRECEDENCE: u8 = 0;
const LOWEST_PRECEDENCE: u8 = 5;
fn get_operator_precedence(token_type: TokenType) -> Option<u8> {
    match token_type {
        TT::Op(OK::Dot) => Some(0),

        TT::Op(OK::AddressOf) | TT::Keyword(KK::Not) => Some(1),

        TT::Op(OK::Star | OK::Slash)
        | TT::Keyword(KK::Div | KK::Mod | KK::And | KK::Shl | KK::Shr | KK::As) => Some(2),

        TT::Op(OK::Plus | OK::Minus) | TT::Keyword(KK::Or | KK::Xor) => Some(3),

        TT::Op(
            OK::Equal(EqKind::Comp)
            | OK::NotEqual
            | OK::LessThan(ChK::Comp)
            | OK::GreaterThan(ChK::Comp)
            | OK::LessEqual
            | OK::GreaterEqual,
        )
        | TT::Keyword(KK::In(InKind::Op) | KK::Is) => Some(4),
        // The import clause `in`s is most simply represented as a precedence
        // relationship
        TT::Keyword(KK::In(InKind::Import)) => Some(4),
        TT::Op(OK::DotDot) => Some(5),

        TT::Op(_)
        | TT::Identifier
        | TT::Keyword(_)
        | TT::TextLiteral(_)
        | TT::NumberLiteral(_)
        | TT::ConditionalDirective(_)
        | TT::CompilerDirective
        | TT::Comment(_)
        | TT::Eof
        | TT::Unknown => None,
    }
}

/// When dealing with precedences, binary and unary operators should often be
/// treated differently.
///
/// Returns whether the given [`TokenType`] is a binary operator by looking at
/// its previous (real) token type
fn is_binary(token_type: TokenType, prev_token_type: Option<TokenType>) -> bool {
    match token_type {
        TT::Op(OK::Plus | OK::Minus | OK::AddressOf) | TT::Keyword(KK::Not) => {}
        _ => return true,
    }
    match (prev_token_type, token_type) {
        (_, TT::Op(OK::AddressOf) | TT::Keyword(KK::Not)) => false,
        // binary after closing bracket or closing generics or special
        // keywords
        (
            Some(
                TT::Op(OK::RBrack | OK::RParen | OK::GreaterThan(ChevronKind::Generic))
                | TT::Keyword(KeywordKind::Inherited | KeywordKind::Nil),
            ),
            TT::Op(OK::Plus | OK::Minus),
        ) => true,
        /*
            unary after:
            - other keywords
            - start of line
            - any other operator
            - comments/directives
        */
        (
            None
            | Some(
                TT::Op(_)
                | TT::Keyword(_)
                | TT::Comment(_)
                | TT::CompilerDirective
                | TT::ConditionalDirective(_),
            ),
            TT::Op(OK::Plus | OK::Minus),
        ) => false,
        // default to binary
        _ => true,
    }
}

mod contexts;
mod debug;
mod parent_pointer_tree;
mod requirements;
mod types;
