use std::cmp::max;
use std::collections::HashSet;
use std::collections::VecDeque;

use crate::lang::ConditionalDirectiveKind::*;

use crate::lang::KeywordKind::*;
use crate::lang::OperatorKind::*;
use crate::lang::TokenType::*;
use crate::lang::*;
use crate::traits::LogicalLineParser;

#[derive(Debug, PartialEq, Eq, Clone)]
enum DirectiveBranchKind {
    Unreachable,
    Conditional,
}

#[derive(Hash, PartialEq, Eq)]
struct LocalLogicalLine {
    parent_token: Option<usize>,
    level: usize,
    token_indices: Vec<usize>,
    line_type: LogicalLineType,
}
#[derive(Clone, Copy)]
struct LocalLogicalLineRef {
    local_logical_line_index: usize,
}

struct InternalDelphiLogicalLineParser<'a> {
    tokens: &'a [Token<'a>],
    current_token_index: usize,
    result_hash: HashSet<LocalLogicalLine>,
    result_lines: Vec<LocalLogicalLine>,
    comments_before_next_token: Vec<usize>,
    pass_logical_lines: Vec<Vec<LocalLogicalLineRef>>,
    current_logical_line: Vec<LocalLogicalLineRef>,
    directive_context: Vec<(DirectiveBranchKind, LocalLogicalLineRef)>,
    directive_depth: usize,
    directive_branch_count: Vec<usize>,
    directive_branch_index: Vec<usize>,
    directive_chain_branch_index: VecDeque<usize>,
    parsing_pass: usize,
}
impl<'a> InternalDelphiLogicalLineParser<'a> {
    fn new(tokens: &'a [Token<'a>]) -> Self {
        InternalDelphiLogicalLineParser {
            tokens,
            current_token_index: 0,
            result_hash: std::default::Default::default(),
            result_lines: vec![],
            comments_before_next_token: vec![],
            pass_logical_lines: vec![vec![]],
            current_logical_line: vec![],
            directive_context: vec![],
            directive_depth: 0,
            directive_branch_count: vec![],
            directive_branch_index: vec![],
            directive_chain_branch_index: VecDeque::new(),
            parsing_pass: 0,
        }
    }
    fn reset_data(&mut self) {
        let line = self.create_new_local_line();
        self.current_logical_line = vec![line];
        self.directive_depth = 0;
        self.directive_context = vec![];
        self.current_token_index = 0;
        self.pass_logical_lines = vec![vec![]];
    }

    fn parse(&mut self) {
        self.reset_data();
        loop {
            self.read_next_token(&mut vec![]);
            self.parse_file();
            self.parsing_pass += 1;

            self.result_hash.extend(self.result_lines.drain(0..));
            self.reset_data();

            while matches!(
                (
                    self.directive_branch_index.last(),
                    self.directive_branch_count.last()
                ),
                (Some(&last_branch_index), Some(&last_branch_count))
                if last_branch_index + 1 >= last_branch_count
            ) {
                self.directive_branch_index.pop();
                self.directive_branch_count.pop();
            }
            if let Some(last_directive_branch_index) = self.directive_branch_index.last_mut() {
                *last_directive_branch_index += 1;
                assert_eq!(
                    self.directive_branch_index.len(),
                    self.directive_branch_count.len()
                );
                assert!(
                    self.directive_branch_index.last().unwrap()
                        <= self.directive_branch_count.last().unwrap()
                )
            } else {
                break;
            }
        }
        self.add_logical_line();
        self.set_logical_line_type(LogicalLineType::Eof);
        let eof_token = self.tokens.len() - 1;
        self.get_current_logical_line_mut()
            .token_indices
            .push(eof_token);
        self.result_hash.extend(self.result_lines.drain(0..));
    }

    fn parse_file(&mut self) {
        self.parse_level(None);
        self.add_logical_line();
    }

    fn parse_level(&mut self, opening_begin: Option<usize>) {
        loop {
            let token_type = match self.get_current_token_type_no_eof() {
                None => break,
                Some(token_type) => token_type,
            };
            match token_type {
                Comment(CommentKind::IndividualLine)
                | Comment(CommentKind::IndividualBlock)
                | Comment(CommentKind::MultilineBlock) => {
                    self.next_token();
                    self.add_logical_line();
                }
                Keyword(Class) => {
                    // If class is the first token in the line, allow the next
                    // token to dictate how it will be parsed.
                    self.next_token();
                }
                Keyword(Property) => {
                    self.parse_property_declaration();
                }
                IdentifierOrKeyword(Private | Protected | Public | Published | Automated) => {
                    self.add_logical_line();
                    self.next_token();
                    self.add_logical_line();
                }
                Op(Semicolon) => {
                    self.next_token();
                    self.add_logical_line();
                }
                Keyword(Begin) => {
                    self.parse_block();
                    self.add_logical_line();
                }
                Keyword(Asm) => {
                    self.parse_asm_block();
                }
                Keyword(End) => {
                    if opening_begin.is_some() {
                        return;
                    }

                    self.next_token();
                    self.add_logical_line();
                }
                _ => self.parse_structural_element(),
            }
        }
    }

    fn parse_block(&mut self) {
        let begin_index = self.current_token_index;
        self.next_token();
        self.add_logical_line();

        self.get_current_logical_line_mut().level += 1;
        self.parse_level(Some(begin_index));

        self.get_current_logical_line_mut().level -= 1;
        // for end
        self.next_token();
    }

    fn parse_asm_block(&mut self) {
        self.next_token();
        self.add_logical_line();

        self.get_current_logical_line_mut().level += 1;
        self.parse_asm_instructions();

        self.get_current_logical_line_mut().level -= 1;
        // for end
        self.next_token();
        self.add_logical_line();
    }

    fn parse_asm_instructions(&mut self) {
        let add_asm_instruction_line = |parser: &mut InternalDelphiLogicalLineParser| {
            parser.add_logical_line();
            parser.set_logical_line_type(LogicalLineType::AsmInstruction);
        };

        loop {
            let token = match self.get_current_token() {
                None => break,
                Some(token) if matches!(token.get_token_type(), Keyword(End) | Eof) => break,
                Some(token) => token,
            };

            if matches!(token.get_token_type(), Op(Semicolon)) {
                self.next_token();
                add_asm_instruction_line(self);
            } else if token.get_leading_whitespace().contains('\n') {
                add_asm_instruction_line(self);
                self.next_token();
            } else {
                self.next_token();
            }
        }
        self.add_logical_line();
    }

    fn parse_property_declaration(&mut self) {
        self.set_logical_line_type(LogicalLineType::PropertyDeclaration);
        self.next_token();
        self.parse_to_after_next_semicolon_or_before_end();
        if let Some(IdentifierOrKeyword(Default)) = self.get_current_token_type() {
            self.next_token();
            self.parse_to_after_next_semicolon_or_before_end();
        }
        self.add_logical_line();
    }

    fn parse_structural_element(&mut self) {
        loop {
            let token_type = match self.get_current_token_type_no_eof() {
                None => return,
                Some(token_type) => token_type,
            };
            match token_type {
                Op(LParen) => self.parse_parens(),
                Op(Semicolon) => {
                    self.next_token();
                    self.add_logical_line();
                    return;
                }
                Keyword(Class) => {
                    // If class is the first token in the line, allow the next
                    // token to dictate how it will be parsed.
                    self.next_token();
                }
                Keyword(Property) => {
                    self.parse_property_declaration();
                    return;
                }
                IdentifierOrKeyword(Private | Protected | Public | Published | Automated) => {
                    self.add_logical_line();
                    self.next_token();
                    self.add_logical_line();
                }
                Keyword(Uses) => {
                    self.add_logical_line();
                    self.set_logical_line_type(LogicalLineType::ImportClause);
                    self.parse_to_after_next_semicolon();
                    self.add_logical_line();
                }
                Keyword(Begin) => {
                    self.parse_child_block(self.current_token_index);
                }
                _ => self.next_token(),
            }
        }
    }

    fn parse_to_after_next_semicolon_or_before_end(&mut self) {
        while !matches!(
            self.get_current_token_type_no_eof(),
            Some(TokenType::Op(OperatorKind::Semicolon) | Keyword(End)) | None
        ) {
            self.next_token();
        }
        if let Some(TokenType::Op(OperatorKind::Semicolon)) = self.get_current_token_type() {
            self.next_token();
        }
    }

    fn parse_to_after_next_semicolon(&mut self) {
        while !matches!(
            self.get_current_token_type_no_eof(),
            Some(TokenType::Op(OperatorKind::Semicolon)) | None
        ) {
            self.next_token();
        }
        self.next_token();
    }

    fn parse_parens(&mut self) {
        assert!(self.get_current_token_type() == Some(Op(LParen)));
        self.next_token();
        loop {
            let token_type = match self.get_current_token_type_no_eof() {
                None => return,
                Some(token_type) => token_type,
            };
            match token_type {
                Op(LParen) => self.parse_parens(),
                Op(RParen) => {
                    self.next_token();
                    return;
                }
                Keyword(Begin) => self.parse_child_block(self.current_token_index),
                _ => self.next_token(),
            }
        }
    }

    fn parse_child_block(&mut self, parent_token: usize) {
        self.next_token();
        self.pass_logical_lines.push(vec![]);
        let new_line = self.create_new_local_line();
        self.current_logical_line.push(new_line);

        self.parse_level(Some(parent_token));

        self.current_logical_line.pop();
        self.pass_logical_lines
            .pop()
            .iter_mut()
            .flat_map(|lines| lines.iter_mut())
            .for_each(|line_ref| {
                if let Some(logical_line) = self.get_logical_line_from_ref_mut(line_ref) {
                    logical_line.parent_token = Some(parent_token);
                }
            });
        self.next_token();
    }

    // Token reading

    fn next_token(&mut self) {
        if self.is_eof() {
            return;
        }

        self.flush_comments_before_next_token();
        let mut comments_before_next_token: Vec<usize> = vec![];

        let owned_token = self.get_current_token().unwrap().get_index();
        self.get_current_logical_line_mut()
            .token_indices
            .push(owned_token);

        self.current_token_index += 1;
        self.read_next_token(&mut comments_before_next_token);
        self.distribute_comments(&mut comments_before_next_token);
    }

    fn read_next_token(&mut self, comments_before_next_token: &mut Vec<usize>) {
        loop {
            if self.is_eof() {
                break;
            }

            while !self.is_in_compiler_directive()
                && matches!(self.get_current_token_type(), Some(ConditionalDirective(_)))
            {
                self.distribute_comments(comments_before_next_token);
                let new_line =
                    self.create_new_local_line_with_type(LogicalLineType::ConditionalDirective);
                self.current_logical_line.push(new_line);
                self.set_logical_line_type(LogicalLineType::ConditionalDirective);
                self.parse_conditional_directive();
                self.current_logical_line.pop();
            }

            if matches!(
                self.directive_context.last(),
                Some((DirectiveBranchKind::Unreachable, _))
            ) && !self.is_in_compiler_directive()
            {
                self.current_token_index += 1;
                continue;
            }

            match self.get_current_token_type_no_eof() {
                Some(Comment(CommentKind::InlineBlock | CommentKind::InlineLine)) => {
                    comments_before_next_token.push(self.current_token_index);
                }
                Some(_) => {
                    self.distribute_comments(comments_before_next_token);
                    return;
                }
                None => (),
            };

            self.current_token_index += 1;
        }
    }

    fn add_logical_line_with_level_delta(&mut self, delta: isize) {
        if self.get_current_logical_line().token_indices.is_empty() {
            return;
        }

        let new_line_type = match self.is_in_compiler_directive() {
            true => LogicalLineType::ConditionalDirective,
            false => LogicalLineType::Unknown,
        };
        let new_logical_line = self.create_new_local_line_with_type(new_line_type);
        self.get_logical_line_from_ref_mut(&new_logical_line)
            .unwrap()
            .line_type = match self.is_in_compiler_directive() {
            true => LogicalLineType::ConditionalDirective,
            false => LogicalLineType::Unknown,
        };
        let prev_line_ref = &self.current_logical_line.pop().unwrap();
        let prev_line = self.get_logical_line_from_ref_mut(prev_line_ref).unwrap();
        let prev_level = prev_line.level;
        prev_line.level = match delta.is_negative() {
            true => prev_line.level.saturating_sub(delta.unsigned_abs()),
            false => prev_line.level + delta.unsigned_abs(),
        };
        self.current_logical_line.push(new_logical_line);
        self.get_current_logical_line_mut().level = prev_level;
    }
    fn add_logical_line(&mut self) {
        self.add_logical_line_with_level_delta(0);
    }

    fn set_logical_line_type(&mut self, line_type: LogicalLineType) {
        self.get_current_logical_line_mut().line_type = line_type;
    }

    fn flush_comments_before_next_token(&mut self) {
        if self.comments_before_next_token.is_empty() {
            return;
        }

        let comments_on_own_line = self.get_current_logical_line().token_indices.is_empty();
        if comments_on_own_line {
            self.add_logical_line();
        }

        for comment_index in 0..self.comments_before_next_token.len() {
            let comment_token = match self.comments_before_next_token.get(comment_index) {
                Some(&index) => index,
                None => continue,
            };
            self.get_current_logical_line_mut()
                .token_indices
                .push(comment_token);
            if comments_on_own_line {
                self.add_logical_line();
            }
        }

        self.comments_before_next_token.clear();
    }

    fn distribute_comments(&mut self, comments: &mut Vec<usize>) {
        if comments.is_empty() {
            return;
        }
        let mut should_push_to_current_line = true;
        for comment_index in 0..comments.len() {
            let comment_token = match comments.get(comment_index) {
                Some(&index) => index,
                None => continue,
            };
            if self.is_in_compiler_directive()
                || matches!(
                    self.tokens.get(comment_token).map(Token::get_token_type),
                    Some(Comment(CommentKind::IndividualLine))
                )
            {
                should_push_to_current_line = false;
            }
            if should_push_to_current_line {
                self.get_current_logical_line_mut()
                    .token_indices
                    .push(comment_token);
            } else {
                self.comments_before_next_token.push(comment_token);
            }
        }

        comments.clear();
    }

    // Compiler directives

    fn parse_conditional_directive(&mut self) {
        let prev_level = self.directive_context.len();
        match self.get_current_token_type() {
            Some(ConditionalDirective(Ifdef | Ifndef | Ifopt | ConditionalDirectiveKind::If)) => {
                self.parse_directive_if();
            }
            Some(ConditionalDirective(ConditionalDirectiveKind::Else | Elseif)) => {
                self.parse_directive_else();
            }
            Some(ConditionalDirective(Endif | Ifend)) => {
                self.parse_directive_endif();
            }
            _ => panic!(),
        }
        self.get_current_logical_line_mut().level =
            max(prev_level, self.directive_context.len()) - 1;
    }

    fn parse_directive_if(&mut self) {
        self.conditional_compilation_start();

        self.directive_depth -= 1;
        self.parse_directive_unknown();
        self.directive_depth += 1;
    }

    fn parse_directive_else(&mut self) {
        self.conditional_compilation_alternative();
        if self.directive_depth > 0 {
            self.directive_depth -= 1;
        }
        self.parse_directive_unknown();
        self.directive_depth += 1;
    }
    fn parse_directive_endif(&mut self) {
        self.conditional_compilation_end();
        self.parse_directive_unknown();
    }

    fn parse_directive_unknown(&mut self) {
        loop {
            self.next_token();
            if matches!(self.get_prev_token_type(), Some(ConditionalDirective(_))) {
                break;
            }
        }
    }

    fn conditional_compilation_start(&mut self) {
        if self.directive_depth == self.directive_branch_index.len() {
            self.directive_branch_index.push(0);
            self.directive_branch_count.push(0);
        }
        self.directive_chain_branch_index.push_front(0);
        self.conditional_compilation_condition(matches!(
            self.directive_branch_index.get(self.directive_depth),
            Some(&index) if index > 0
        ));
        self.directive_depth += 1;
    }

    fn conditional_compilation_end(&mut self) {
        assert!(self.directive_depth <= self.directive_branch_index.len());
        self.directive_depth = self.directive_depth.saturating_sub(1);
        if let (Some(last_directive_chain_branch_index), Some(last_branch_count)) = (
            self.directive_chain_branch_index.pop_back(),
            self.directive_branch_count.get_mut(self.directive_depth),
        ) {
            *last_branch_count = (last_directive_chain_branch_index + 1).max(*last_branch_count);
        }
        self.directive_context.pop();
    }

    fn conditional_compilation_condition(&mut self, unreachable: bool) {
        let last_is_unreachable = match self.directive_context.last() {
            None => false,
            Some((branch_kind, _)) => *branch_kind == DirectiveBranchKind::Unreachable,
        };
        let current_line = *self.get_current_logical_line_ref();
        let branch_kind = match unreachable || last_is_unreachable {
            true => DirectiveBranchKind::Unreachable,
            false => DirectiveBranchKind::Conditional,
        };
        self.directive_context.push((branch_kind, current_line));
    }

    fn conditional_compilation_alternative(&mut self) {
        self.directive_context.pop();
        assert!(self.directive_depth <= self.directive_branch_index.len());
        if let Some(top_index) = self.directive_chain_branch_index.pop_back() {
            self.directive_chain_branch_index.push_back(top_index + 1)
        }
        let prev_depth_branch_index = self
            .directive_depth
            .checked_sub(1)
            .and_then(|prev_index| self.directive_branch_index.get(prev_index));

        self.conditional_compilation_condition(matches!(
            (prev_depth_branch_index, self.directive_chain_branch_index.back()),
            (Some(index), Some(chain_index)) if index != chain_index
        ));
    }

    // Utils

    fn create_new_local_line_with_parent_and_type(
        &mut self,
        parent_token: Option<usize>,
        line_type: LogicalLineType,
    ) -> LocalLogicalLineRef {
        let line = LocalLogicalLine {
            parent_token,
            level: 0,
            token_indices: vec![],
            line_type,
        };
        self.result_lines.push(line);
        let line_ref = LocalLogicalLineRef {
            local_logical_line_index: self.result_lines.len() - 1,
        };
        if let Some(last_logical_line) = self.pass_logical_lines.last_mut() {
            last_logical_line.push(line_ref);
        }
        line_ref
    }
    fn create_new_local_line_with_parent(
        &mut self,
        parent_token: Option<usize>,
    ) -> LocalLogicalLineRef {
        self.create_new_local_line_with_parent_and_type(parent_token, LogicalLineType::Unknown)
    }
    fn create_new_local_line_with_type(
        &mut self,
        line_type: LogicalLineType,
    ) -> LocalLogicalLineRef {
        self.create_new_local_line_with_parent_and_type(None, line_type)
    }
    fn create_new_local_line(&mut self) -> LocalLogicalLineRef {
        self.create_new_local_line_with_parent(None)
    }

    fn is_in_compiler_directive(&self) -> bool {
        matches!(
            self.get_current_logical_line().line_type,
            LogicalLineType::ConditionalDirective
        )
    }

    fn get_current_logical_line_mut(&mut self) -> &mut LocalLogicalLine {
        let line_ref = *self.current_logical_line.last().unwrap();
        self.get_logical_line_from_ref_mut(&line_ref).unwrap()
    }

    fn get_logical_line_from_ref_mut(
        &mut self,
        line_ref: &LocalLogicalLineRef,
    ) -> Option<&mut LocalLogicalLine> {
        self.result_lines.get_mut(line_ref.local_logical_line_index)
    }

    fn get_logical_line_from_ref(
        &self,
        line_ref: &LocalLogicalLineRef,
    ) -> Option<&LocalLogicalLine> {
        self.result_lines.get(line_ref.local_logical_line_index)
    }

    fn get_current_logical_line_ref(&self) -> &LocalLogicalLineRef {
        self.current_logical_line.last().unwrap()
    }

    fn get_current_logical_line(&self) -> &LocalLogicalLine {
        let line_ref = *self.current_logical_line.last().unwrap();
        self.get_logical_line_from_ref(&line_ref).unwrap()
    }

    fn get_current_token(&self) -> Option<&Token> {
        self.tokens.get(self.current_token_index)
    }
    fn get_prev_token_type(&self) -> Option<TokenType> {
        let prev_index = self.current_token_index.checked_sub(1)?;
        self.tokens.get(prev_index).map(Token::get_token_type)
    }
    fn get_current_token_type(&self) -> Option<TokenType> {
        self.tokens
            .get(self.current_token_index)
            .map(Token::get_token_type)
    }
    fn get_current_token_type_no_eof(&self) -> Option<TokenType> {
        self.get_current_token_type()
            .filter(|&token_type| token_type != Eof)
    }
    fn is_eof(&self) -> bool {
        matches!(self.get_current_token_type(), Some(TokenType::Eof) | None)
    }
}
pub struct DelphiLogicalLineParser {}
impl LogicalLineParser for DelphiLogicalLineParser {
    fn parse<'a>(&self, input: &'a [Token<'a>]) -> Vec<LogicalLine> {
        let mut lines: Vec<LogicalLine> = {
            let mut parser = InternalDelphiLogicalLineParser::new(input);
            parser.parse();
            parser
                .result_hash
                .into_iter()
                .filter(|line| !line.token_indices.is_empty())
                .map(|line| {
                    LogicalLine::new(
                        line.parent_token,
                        line.level,
                        line.token_indices,
                        line.line_type,
                    )
                })
                .collect()
        };
        lines.sort_by(|a, b| a.get_tokens().first().cmp(&b.get_tokens().first()));
        lines
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use spectral::prelude::*;

    use super::*;
    use crate::{defaults::lexer::DelphiLexer, traits::Lexer};

    fn run_test(input: &str, expected_lines: Vec<LogicalLine>) {
        let lexer = DelphiLexer {};
        let parser = DelphiLogicalLineParser {};
        let tokens = lexer.lex(input);
        let mut lines = parser.parse(&tokens);

        lines.retain(|line| line.get_line_type() != LogicalLineType::Eof);
        lines.iter().for_each(|line| {
            print!(
                "Level: {} CD Line: {} Parent: {:?} Tokens: ",
                line.get_level(),
                match line.get_line_type() {
                    LogicalLineType::ConditionalDirective => "Y",
                    _ => "N",
                },
                line.get_parent_token()
            );

            line.get_tokens().iter().for_each(|token| {
                print!("{}:{} ", *token, tokens.get(*token).unwrap().get_content())
            });
            println!();
        });
        assert_that(&lines).has_length(expected_lines.len());
        expected_lines.iter().for_each(|expected_line| {
            assert_that(&lines).contains(expected_line);
        });
    }

    #[test]
    fn basic_conditional_directives() {
        run_test(
            "Foo {$ifdef A} Bar {$else} Baz {$endif}",
            vec![
                LogicalLine::new(None, 0, vec![0, 2], LogicalLineType::Unknown),
                LogicalLine::new(None, 0, vec![0, 4], LogicalLineType::Unknown),
                LogicalLine::new(None, 0, vec![1], LogicalLineType::ConditionalDirective),
                LogicalLine::new(None, 0, vec![3], LogicalLineType::ConditionalDirective),
                LogicalLine::new(None, 0, vec![5], LogicalLineType::ConditionalDirective),
            ],
        )
    }

    #[test]
    fn basic_conditional_directives_at_start() {
        run_test(
            "{$ifdef A} Foo {$else} Bar {$endif}",
            vec![
                LogicalLine::new(None, 0, vec![0], LogicalLineType::ConditionalDirective),
                LogicalLine::new(None, 0, vec![1], LogicalLineType::Unknown),
                LogicalLine::new(None, 0, vec![2], LogicalLineType::ConditionalDirective),
                LogicalLine::new(None, 0, vec![3], LogicalLineType::Unknown),
                LogicalLine::new(None, 0, vec![4], LogicalLineType::ConditionalDirective),
            ],
        )
    }

    #[test]
    fn nested_conditional_directives() {
        run_test(
            "{$ifdef A} {$ifdef B} {$endif} {$endif}",
            vec![
                LogicalLine::new(None, 0, vec![0], LogicalLineType::ConditionalDirective),
                LogicalLine::new(None, 1, vec![1], LogicalLineType::ConditionalDirective),
                LogicalLine::new(None, 1, vec![2], LogicalLineType::ConditionalDirective),
                LogicalLine::new(None, 0, vec![3], LogicalLineType::ConditionalDirective),
            ],
        )
    }

    #[test]
    fn line_comments() {
        run_test(
            "begin // inline line comment
              // individual line comment
              Foo(arg1) // inline line comment
              ;
              // individual line comment
            end",
            vec![
                LogicalLine::new(None, 0, vec![0, 1], LogicalLineType::Unknown),
                LogicalLine::new(None, 1, vec![2], LogicalLineType::Unknown),
                LogicalLine::new(None, 1, vec![3, 4, 5, 6, 7, 8], LogicalLineType::Unknown),
                LogicalLine::new(None, 1, vec![9], LogicalLineType::Unknown),
                LogicalLine::new(None, 0, vec![10], LogicalLineType::Unknown),
            ],
        )
    }

    #[test]
    fn block_comments() {
        run_test(
            "begin {inline block}
              {individual block}
              Foo(arg1) {inline block}
              ;
              {individual block}
              {
                multiline block
              }
            end",
            vec![
                LogicalLine::new(None, 0, vec![0, 1], LogicalLineType::Unknown),
                LogicalLine::new(None, 1, vec![2], LogicalLineType::Unknown),
                LogicalLine::new(None, 1, vec![3, 4, 5, 6, 7, 8], LogicalLineType::Unknown),
                LogicalLine::new(None, 1, vec![9], LogicalLineType::Unknown),
                LogicalLine::new(None, 1, vec![10], LogicalLineType::Unknown),
                LogicalLine::new(None, 0, vec![11], LogicalLineType::Unknown),
            ],
        );
        run_test(
            "begin {inline block}
              {
                multiline block
              }
              Foo(arg1) {inline block}
              ;
              {
                multiline block
              }
              {individual block}
            end",
            vec![
                LogicalLine::new(None, 0, vec![0, 1], LogicalLineType::Unknown),
                LogicalLine::new(None, 1, vec![2], LogicalLineType::Unknown),
                LogicalLine::new(None, 1, vec![3, 4, 5, 6, 7, 8], LogicalLineType::Unknown),
                LogicalLine::new(None, 1, vec![9], LogicalLineType::Unknown),
                LogicalLine::new(None, 1, vec![10], LogicalLineType::Unknown),
                LogicalLine::new(None, 0, vec![11], LogicalLineType::Unknown),
            ],
        )
    }

    #[test]
    fn nested_level() {
        run_test(
            "
            Foo(
              function(Arg1: String): String
              begin
                Bar(
                  procedure
                  begin
                    Baz1;
                  end,
                  procedure
                  begin
                    Baz2;
                  end
                );
                Flarp;
              end
            )",
            vec![
                LogicalLine::new(
                    None,
                    0,
                    vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 28, 29],
                    LogicalLineType::Unknown,
                ),
                LogicalLine::new(
                    Some(10),
                    0,
                    vec![11, 12, 13, 14, 17, 18, 19, 20, 23, 24, 25],
                    LogicalLineType::Unknown,
                ),
                LogicalLine::new(Some(10), 0, vec![26, 27], LogicalLineType::Unknown),
                LogicalLine::new(Some(14), 0, vec![15, 16], LogicalLineType::Unknown),
                LogicalLine::new(Some(20), 0, vec![21, 22], LogicalLineType::Unknown),
            ],
        );
    }

    #[test]
    fn eof_line_in_empty_file() {
        let lexer = DelphiLexer {};
        let parser = DelphiLogicalLineParser {};
        let tokens = lexer.lex("");
        let lines = parser.parse(&tokens);

        assert_that(&lines).is_equal_to(&vec![LogicalLine::new(
            None,
            0,
            vec![0],
            LogicalLineType::Eof,
        )]);
    }

    #[test]
    fn eof_line() {
        let lexer = DelphiLexer {};
        let parser = DelphiLogicalLineParser {};
        let tokens = lexer.lex("Foo;");
        let lines = parser.parse(&tokens);

        assert_that(&lines).is_equal_to(&vec![
            LogicalLine::new(None, 0, vec![0, 1], LogicalLineType::Unknown),
            LogicalLine::new(None, 0, vec![2], LogicalLineType::Eof),
        ]);
    }

    #[test]
    fn package_uses_clause() {
        run_test(
            indoc! {"
                package Foo;
                uses Unit1, Unit2;"
            },
            vec![
                LogicalLine::new(None, 0, vec![0, 1, 2], LogicalLineType::Unknown),
                LogicalLine::new(None, 0, vec![3, 4, 5, 6, 7], LogicalLineType::ImportClause),
            ],
        );
    }

    #[test]
    fn package_uses_clause_with_compiler_directive() {
        run_test(
            indoc! {"
                package Foo;
                {$R}
                uses Unit1 {Bar: TBar}, Unit2;"
            },
            vec![
                LogicalLine::new(None, 0, vec![0, 1, 2], LogicalLineType::Unknown),
                LogicalLine::new(None, 0, vec![3], LogicalLineType::Unknown),
                LogicalLine::new(
                    None,
                    0,
                    vec![4, 5, 6, 7, 8, 9],
                    LogicalLineType::ImportClause,
                ),
            ],
        );
    }

    #[test]
    fn unit_uses_clause() {
        run_test(
            indoc! {"
                unit Foo;
                implementation
                uses Unit1, Unit2;"
            },
            vec![
                LogicalLine::new(None, 0, vec![0, 1, 2], LogicalLineType::Unknown),
                LogicalLine::new(None, 0, vec![3], LogicalLineType::Unknown),
                LogicalLine::new(None, 0, vec![4, 5, 6, 7, 8], LogicalLineType::ImportClause),
            ],
        );
    }

    #[test]
    fn property_declaration() {
        run_test(
            "property Foo;",
            vec![LogicalLine::new(
                None,
                0,
                vec![0, 1, 2],
                LogicalLineType::PropertyDeclaration,
            )],
        );
        run_test(
            "property Foo read A;",
            vec![LogicalLine::new(
                None,
                0,
                vec![0, 1, 2, 3, 4],
                LogicalLineType::PropertyDeclaration,
            )],
        );
        run_test(
            "property Foo read A write A;",
            vec![LogicalLine::new(
                None,
                0,
                vec![0, 1, 2, 3, 4, 5, 6],
                LogicalLineType::PropertyDeclaration,
            )],
        );
        run_test(
            "property Foo read A write A; default;",
            vec![LogicalLine::new(
                None,
                0,
                vec![0, 1, 2, 3, 4, 5, 6, 7, 8],
                LogicalLineType::PropertyDeclaration,
            )],
        );
        // TODO begin to be replaced with class
        run_test(
            "begin property Foo read A write A end",
            vec![
                LogicalLine::new(None, 0, vec![0], LogicalLineType::Unknown),
                LogicalLine::new(
                    None,
                    1,
                    vec![1, 2, 3, 4, 5, 6],
                    LogicalLineType::PropertyDeclaration,
                ),
                LogicalLine::new(None, 0, vec![7], LogicalLineType::Unknown),
            ],
        );
        run_test(
            "begin property Foo read A write A; default end",
            vec![
                LogicalLine::new(None, 0, vec![0], LogicalLineType::Unknown),
                LogicalLine::new(
                    None,
                    1,
                    vec![1, 2, 3, 4, 5, 6, 7, 8],
                    LogicalLineType::PropertyDeclaration,
                ),
                LogicalLine::new(None, 0, vec![9], LogicalLineType::Unknown),
            ],
        );
    }

    #[test]
    fn class_property_declaration() {
        run_test(
            "published property Foo",
            vec![
                LogicalLine::new(None, 0, vec![0], LogicalLineType::Unknown),
                LogicalLine::new(None, 0, vec![1, 2], LogicalLineType::PropertyDeclaration),
            ],
        );
        run_test(
            "strict private property Foo",
            vec![
                LogicalLine::new(None, 0, vec![0], LogicalLineType::Unknown),
                LogicalLine::new(None, 0, vec![1], LogicalLineType::Unknown),
                LogicalLine::new(None, 0, vec![2, 3], LogicalLineType::PropertyDeclaration),
            ],
        );
        run_test(
            "class property Foo;",
            vec![LogicalLine::new(
                None,
                0,
                vec![0, 1, 2, 3],
                LogicalLineType::PropertyDeclaration,
            )],
        );
        run_test(
            "class property Foo read A;",
            vec![LogicalLine::new(
                None,
                0,
                vec![0, 1, 2, 3, 4, 5],
                LogicalLineType::PropertyDeclaration,
            )],
        );
        run_test(
            "class property Foo read A write A;",
            vec![LogicalLine::new(
                None,
                0,
                vec![0, 1, 2, 3, 4, 5, 6, 7],
                LogicalLineType::PropertyDeclaration,
            )],
        );
        run_test(
            "class property Foo read A write A; default;",
            vec![LogicalLine::new(
                None,
                0,
                vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
                LogicalLineType::PropertyDeclaration,
            )],
        );
        // TODO begin to be replaced with class
        run_test(
            "begin class property Foo read A write A end",
            vec![
                LogicalLine::new(None, 0, vec![0], LogicalLineType::Unknown),
                LogicalLine::new(
                    None,
                    1,
                    vec![1, 2, 3, 4, 5, 6, 7],
                    LogicalLineType::PropertyDeclaration,
                ),
                LogicalLine::new(None, 0, vec![8], LogicalLineType::Unknown),
            ],
        );
        run_test(
            "begin class property Foo read A write A; default end",
            vec![
                LogicalLine::new(None, 0, vec![0], LogicalLineType::Unknown),
                LogicalLine::new(
                    None,
                    1,
                    vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
                    LogicalLineType::PropertyDeclaration,
                ),
                LogicalLine::new(None, 0, vec![10], LogicalLineType::Unknown),
            ],
        );
    }

    #[test]
    fn parse_inline_assembly_nested_semicolon_separated() {
        run_test(
            indoc! {"
                begin
                    asm
                        MOV EAX, 1; XOR EAX, EAX
                    end
                end"
            },
            vec![
                LogicalLine::new(None, 0, vec![0], LogicalLineType::Unknown),
                LogicalLine::new(None, 1, vec![1], LogicalLineType::Unknown),
                LogicalLine::new(
                    None,
                    2,
                    vec![2, 3, 4, 5, 6],
                    LogicalLineType::AsmInstruction,
                ),
                LogicalLine::new(None, 2, vec![7, 8, 9, 10], LogicalLineType::AsmInstruction),
                LogicalLine::new(None, 1, vec![11], LogicalLineType::Unknown),
                LogicalLine::new(None, 0, vec![12], LogicalLineType::Unknown),
            ],
        );
    }

    #[test]
    fn parse_inline_assembly_nested_newline_separated() {
        run_test(
            indoc! {"
                begin
                    asm
                        MOV EAX, 1
                        XOR EAX, EAX
                    end
                end"
            },
            vec![
                LogicalLine::new(None, 0, vec![0], LogicalLineType::Unknown),
                LogicalLine::new(None, 1, vec![1], LogicalLineType::Unknown),
                LogicalLine::new(None, 2, vec![2, 3, 4, 5], LogicalLineType::AsmInstruction),
                LogicalLine::new(None, 2, vec![6, 7, 8, 9], LogicalLineType::AsmInstruction),
                LogicalLine::new(None, 1, vec![10], LogicalLineType::Unknown),
                LogicalLine::new(None, 0, vec![11], LogicalLineType::Unknown),
            ],
        );
    }

    #[test]
    fn parse_inline_assembly_nested() {
        run_test(
            indoc! {"
                begin
                    asm
                        MOV EAX, 1
                    end
                end"
            },
            vec![
                LogicalLine::new(None, 0, vec![0], LogicalLineType::Unknown),
                LogicalLine::new(None, 1, vec![1], LogicalLineType::Unknown),
                LogicalLine::new(None, 2, vec![2, 3, 4, 5], LogicalLineType::AsmInstruction),
                LogicalLine::new(None, 1, vec![6], LogicalLineType::Unknown),
                LogicalLine::new(None, 0, vec![7], LogicalLineType::Unknown),
            ],
        );
    }

    #[test]
    fn parse_inline_assembly_top_level_semicolon_separated() {
        run_test(
            indoc! {"
                asm
                    MOV EAX, 1; XOR EAX, EAX
                end"
            },
            vec![
                LogicalLine::new(None, 0, vec![0], LogicalLineType::Unknown),
                LogicalLine::new(
                    None,
                    1,
                    vec![1, 2, 3, 4, 5],
                    LogicalLineType::AsmInstruction,
                ),
                LogicalLine::new(None, 1, vec![6, 7, 8, 9], LogicalLineType::AsmInstruction),
                LogicalLine::new(None, 0, vec![10], LogicalLineType::Unknown),
            ],
        );
    }

    #[test]
    fn parse_inline_assembly_top_level_newline_separated() {
        run_test(
            indoc! {"
                asm
                    MOV EAX, 1
                    XOR EAX, EAX
                end"
            },
            vec![
                LogicalLine::new(None, 0, vec![0], LogicalLineType::Unknown),
                LogicalLine::new(None, 1, vec![1, 2, 3, 4], LogicalLineType::AsmInstruction),
                LogicalLine::new(None, 1, vec![5, 6, 7, 8], LogicalLineType::AsmInstruction),
                LogicalLine::new(None, 0, vec![9], LogicalLineType::Unknown),
            ],
        );
    }

    #[test]
    fn parse_inline_assembly_top_level() {
        run_test(
            indoc! {"
                asm
                    MOV EAX, 1
                end"
            },
            vec![
                LogicalLine::new(None, 0, vec![0], LogicalLineType::Unknown),
                LogicalLine::new(None, 1, vec![1, 2, 3, 4], LogicalLineType::AsmInstruction),
                LogicalLine::new(None, 0, vec![5], LogicalLineType::Unknown),
            ],
        );
    }
}
