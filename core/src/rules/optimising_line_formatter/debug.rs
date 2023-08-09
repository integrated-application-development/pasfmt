use std::fmt::{Debug, Write};

use itertools::Itertools;

use super::*;

pub(super) struct RawDebugLine<'a> {
    line: (usize, &'a LogicalLine),
    formatted_tokens: &'a FormattedTokens<'a>,
}
impl<'a> RawDebugLine<'a> {
    pub fn new(line: (usize, &'a LogicalLine), formatted_tokens: &'a FormattedTokens<'a>) -> Self {
        Self {
            line,
            formatted_tokens,
        }
    }
}
impl Debug for RawDebugLine<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let minmax = self.line.1.get_tokens().iter().cloned().minmax();
        let (first, last) = minmax.into_option().unwrap_or((0, 0));

        let included_tokens = (first..last).flat_map(|token_index| {
            self.formatted_tokens
                .get_token(token_index)
                .map(|token| token.0)
        });
        for token in included_tokens {
            f.write_str(token.get_str())?;
        }
        Ok(())
    }
}

pub(super) struct TokenDecisions(Vec<TokenDecision>);
impl From<&'_ FormattingNode<'_>> for TokenDecisions {
    fn from(value: &'_ FormattingNode) -> Self {
        let mut decisions: Vec<_> = value
            .decision
            .walk_parents_data()
            .map(|d| d.clone())
            .collect();
        decisions.reverse();
        Self(decisions)
    }
}
impl From<&'_ FormattingSolution> for TokenDecisions {
    fn from(value: &'_ FormattingSolution) -> Self {
        Self(value.decisions.clone())
    }
}

pub(super) struct DebugPrintableLine<'a, 'b> {
    decisions: TokenDecisions,
    starting_ws: LineWhitespace,
    line: &'a LogicalLine,
    iolf: &'a InternalOptimisingLineFormatter<'a, 'b>,
}
impl<'a, 'b> DebugPrintableLine<'a, 'b> {
    pub fn new(
        decisions: impl Into<TokenDecisions>,
        starting_ws: LineWhitespace,
        line: &'a LogicalLine,
        iolf: &'a InternalOptimisingLineFormatter<'a, 'b>,
    ) -> Self {
        Self {
            decisions: decisions.into(),
            starting_ws,
            line,
            iolf,
        }
    }
}
impl Debug for DebugPrintableLine<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tokens_decisions = self
            .line
            .get_tokens()
            .iter()
            .map(|&token_idx| self.iolf.formatted_tokens.get_token(token_idx).unwrap())
            .zip(&self.decisions.0)
            .enumerate();

        for (line_index, ((token, formatting_data), decision)) in tokens_decisions {
            let starting_ind = self.starting_ws.indentations;
            let starting_cont = self.starting_ws.continuations;
            let (nl, ind, cont, sp) = match (decision.decision, line_index) {
                (Decision::Continue, 0) if self.line.get_parent().is_none() => {
                    (0, starting_ind, starting_cont, 0)
                }
                (Decision::Break { continuations }, 0) if self.line.get_parent().is_none() => {
                    (0, starting_ind, starting_cont + continuations, 0)
                }
                (Decision::Break { continuations }, _) => {
                    (1, starting_ind, starting_cont + continuations, 0)
                }
                (Decision::Continue, _) => (0, 0, 0, formatting_data.spaces_before),
            };
            for _ in 0..nl {
                f.write_str(self.iolf.recon_settings.get_newline_str())?;
            }
            for _ in 0..ind {
                f.write_str(self.iolf.recon_settings.get_indentation_str())?;
            }
            for _ in 0..cont {
                f.write_str(self.iolf.recon_settings.get_continuation_str())?;
            }
            for _ in 0..sp {
                f.write_char(' ')?;
            }

            f.write_str(token.get_content())?;

            for (line_index, child_solution) in &decision.child_solutions {
                DebugPrintableLine::new(
                    child_solution,
                    self.starting_ws,
                    &self.iolf.lines[*line_index],
                    self.iolf,
                )
                .fmt(f)?;
            }
        }

        if let Some((token, _)) = self
            .line
            .get_tokens()
            .get(self.decisions.0.len())
            .and_then(|token_idx| self.iolf.formatted_tokens.get_token(*token_idx))
        {
            write!(f, " [{}]", token.get_content())?;
        }
        Ok(())
    }
}

pub(super) struct DebugFormattingNode<'a, 'b> {
    node: &'a FormattingNode<'a>,
    line: &'a LogicalLine,
    contexts: &'a SpecificContextStack<'a>,
    iolf: &'a InternalOptimisingLineFormatter<'a, 'b>,
}
impl<'a, 'b> DebugFormattingNode<'a, 'b> {
    pub fn new(
        node: &'a FormattingNode<'a>,
        line: &'a LogicalLine,
        contexts: &'a SpecificContextStack<'a>,
        iolf: &'a InternalOptimisingLineFormatter<'a, 'b>,
    ) -> Self {
        Self {
            node,
            line,
            contexts,
            iolf,
        }
    }
}
impl Debug for DebugFormattingNode<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Penalty: {}", self.node.penalty)?;
        writeln!(f, "Decision count: {}", self.node.next_line_index)?;
        writeln!(
            f,
            "Last line length: {}",
            self.node.decision.get().last_line_length
        )?;
        writeln!(
            f,
            "{:?}",
            DebugPrintableLine::new(self.node, self.node.starting_ws, self.line, self.iolf)
        )?;
        writeln!(f, "Contexts:")?;
        write!(
            f,
            "{:?}",
            DebugContextDataStack::new(&self.contexts.with_data(self.node))
        )?;
        Ok(())
    }
}

impl InternalOptimisingLineFormatter<'_, '_> {
    pub(super) fn solution_debugging(
        &self,
        line: (usize, &LogicalLine),
        node_heap: &BinaryHeap<FormattingNode<'_>>,
        iteration_count: u32,
        solution: &FormattingSolution,
    ) {
        if !log_enabled!(log::Level::Trace) {
            return;
        }
        /*
            These are constructed for the reconstruction of the decision path
            in `DebugFormattingSolution`. This is done outside of the logging
            to avoid the `Rc` panic caused by nested logging calls.
        */
        let context_tree = LineFormattingContexts::new_tree();
        let line_contexts = LineFormattingContexts::new(line.1, &self.token_types, &context_tree);
        trace!(
            "Solution found!\n\
            Total nodes pushed to heap: {}\n\
            Heap capacity: {}\n\
            Iteration count: {}\n\
            {:?}",
            node_heap.len() + iteration_count as usize,
            node_heap.capacity(),
            iteration_count,
            DebugFormattingSolution::new(solution, line.1, self, &line_contexts)
        );
    }
}

struct DebugFormattingSolution<'a, 'b, 'c> {
    solution: &'a FormattingSolution,
    line: &'a LogicalLine,
    iolf: &'a InternalOptimisingLineFormatter<'a, 'b>,
    line_contexts: &'a LineFormattingContexts<'c>,
}
impl<'a, 'b, 'c> DebugFormattingSolution<'a, 'b, 'c> {
    fn new(
        solution: &'a FormattingSolution,
        line: &'a LogicalLine,
        iolf: &'a InternalOptimisingLineFormatter<'a, 'b>,
        line_contexts: &'a LineFormattingContexts<'c>,
    ) -> Self {
        Self {
            solution,
            line,
            iolf,
            line_contexts,
        }
    }
}
impl Debug for DebugFormattingSolution<'_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Solution found:")?;
        writeln!(
            f,
            "{:?}",
            DebugPrintableLine::new(
                self.solution,
                self.solution.starting_ws,
                self.line,
                self.iolf
            )
        )?;
        writeln!(f)?;
        writeln!(f, "Decision reconstruction:")?;

        let decision_tree = ParentPointerTree::new(self.solution.decisions[0].clone());

        let mut recon_node = FormattingNode {
            context_data: self.line_contexts.get_default_context_data(),
            starting_ws: self.solution.starting_ws,
            decision: decision_tree.root(),
            next_line_index: 0,
            penalty: 0,
        };

        for (line_index, token_decision) in self.solution.decisions.iter().enumerate() {
            let context_stack = &self
                .line_contexts
                .get_specific_context_stack(line_index as u32);
            writeln!(
                f,
                "Token: {:?}, Requirement: {:?}, Decision: {:?}, LineLength {}, parents_support_break? {}",
                self.line
                    .get_tokens()
                    .get(line_index)
                    .and_then(|&token_idx| self.iolf.get_token_type(token_idx)),
                token_decision.requirement,
                token_decision.decision,
                token_decision.last_line_length,
                &self
                    .line_contexts
                    .get_specific_context_stack(line_index as u32)
                    .with_data(&recon_node)
                    .parents_support_break()
            )?;
            writeln!(f, "Updated contexts:")?;

            context_stack.update_contexts(&mut recon_node, token_decision.decision.to_raw());
            context_stack.update_contexts_from_child_solutions(
                &mut recon_node,
                &token_decision.child_solutions,
            );
            recon_node.next_line_index += 1;

            writeln!(
                f,
                "{:?}",
                DebugContextDataStack::new(
                    &self
                        .line_contexts
                        .get_specific_context_stack(line_index as u32)
                        .with_data(&recon_node)
                )
            )?;
            writeln!(f, "---")?;
        }
        Ok(())
    }
}

pub(super) struct DebugContextDataStack<'a> {
    stack: &'a SpecificContextDataStack<'a>,
}
impl<'a> DebugContextDataStack<'a> {
    pub fn new(stack: &'a SpecificContextDataStack<'a>) -> Self {
        Self { stack }
    }
}
impl Debug for DebugContextDataStack<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for (ctx, data) in self.stack.iter() {
            if !first {
                writeln!(f)?;
            }
            write!(f, "{:?}\n> {:?}", ctx, data)?;
            first = false;
        }
        Ok(())
    }
}
