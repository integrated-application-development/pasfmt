use std::fmt::{Debug, Write};

use itertools::Itertools;

use super::*;

pub(super) struct RawDebugLine<'a, 'b, 'c> {
    line: (usize, &'a LogicalLine),
    iolf: &'a InternalOptimisingLineFormatter<'b, 'c>,
}
impl<'a, 'b, 'c> RawDebugLine<'a, 'b, 'c> {
    pub fn new(
        line: (usize, &'a LogicalLine),
        iolf: &'a InternalOptimisingLineFormatter<'b, 'c>,
    ) -> Self {
        Self { line, iolf }
    }
}
impl Debug for RawDebugLine<'_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let minmax = self.line.1.get_tokens().iter().cloned().minmax();
        let (first, mut last) = minmax.into_option().unwrap_or((0, 0));

        let mut current_parent = LineParent {
            line_index: self.line.0,
            global_token_index: last,
        };
        while let Some(children) = self.iolf.line_children.get(&current_parent) {
            let Some(&line_index) = children.line_indices.last() else {
                break;
            };
            let Some(&last_token_index) = self.iolf.lines[line_index].get_tokens().last() else {
                break;
            };
            last = last_token_index;
            current_parent = LineParent {
                line_index,
                global_token_index: last_token_index,
            };
        }

        let included_tokens = (first..=last).flat_map(|token_index| {
            self.iolf
                .formatted_tokens
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
        InternalDebugPrintableLine {
            dpl: self,
            top_level: true,
        }
        .fmt(f)
    }
}
struct InternalDebugPrintableLine<'a, 'b, 'c> {
    dpl: &'a DebugPrintableLine<'b, 'c>,
    top_level: bool,
}
impl Debug for InternalDebugPrintableLine<'_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tokens_decisions = self
            .dpl
            .line
            .get_tokens()
            .iter()
            .map(|&token_idx| self.dpl.iolf.formatted_tokens.get_token(token_idx).unwrap())
            .zip(&self.dpl.decisions.0)
            .enumerate();

        for (line_index, ((token, formatting_data), decision)) in tokens_decisions {
            let starting_ind = self.dpl.starting_ws.indentations;
            let starting_cont = self.dpl.starting_ws.continuations;
            if self.top_level && line_index == 0 && matches!(decision.decision, Decision::Continue)
            {
                for _ in 0..(decision.last_line_length
                    - token.get_content().len() as u32
                    - formatting_data.spaces_before as u32)
                {
                    f.write_char('█')?;
                }
            }
            let (nl, ind, cont, sp) = match (decision.decision, line_index) {
                (Decision::Continue, 0) if self.dpl.line.get_parent().is_none() => {
                    (0, starting_ind, starting_cont, 0)
                }
                (Decision::Break { continuations }, 0) if self.dpl.line.get_parent().is_none() => {
                    (0, starting_ind, starting_cont + continuations, 0)
                }
                (Decision::Break { continuations }, _) => {
                    (1, starting_ind, starting_cont + continuations, 0)
                }
                (Decision::Continue, _) => (0, 0, 0, formatting_data.spaces_before),
            };
            for _ in 0..nl {
                f.write_str(self.dpl.iolf.recon_settings.get_newline_str())?;
            }
            for _ in 0..ind {
                f.write_str(self.dpl.iolf.recon_settings.get_indentation_str())?;
            }
            for _ in 0..cont {
                f.write_str(self.dpl.iolf.recon_settings.get_continuation_str())?;
            }
            for _ in 0..sp {
                f.write_char(' ')?;
            }

            f.write_str(token.get_content())?;

            for (line_index, child_solution) in &decision.child_solutions {
                InternalDebugPrintableLine {
                    dpl: &DebugPrintableLine::new(
                        child_solution,
                        child_solution.starting_ws,
                        &self.dpl.iolf.lines[*line_index],
                        self.dpl.iolf,
                    ),
                    top_level: false,
                }
                .fmt(f)?;
            }
        }

        if let Some((token, _)) = self
            .dpl
            .line
            .get_tokens()
            .get(self.dpl.decisions.0.len())
            .and_then(|token_idx| self.dpl.iolf.formatted_tokens.get_token(*token_idx))
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
        writeln!(f, "Penalty: {}", self.solution.penalty)?;
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
