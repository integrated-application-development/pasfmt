/*
    The motivation behind the conditional directive parsing and reparsing is
    the idea of wanting to know the context and contents of the whole line
    prior to formatting.

    Other tools in this space tend to format as they are scanning, which removes
    the possibility for the formatting of lines to change based on later context.
    E.g., the line becoming too long, or containing tokens that must be on their
    own line.

    In _sane_ Delphi code the conditional directives are pretty much ignorable
    as they are entirely within a line or surrounding entire lines/blocks. This
    model breaks down, however, when blocks or contexts are started or stopped
    within the conditional directives.

    Knowing the entire contents of a line, and by extension its length, allows
    for lines to be confidently wrapped and unwrapped based on their length.
    Without knowing the entire contents of a line, it is impossible to remove
    user-added line breaks.
*/

use std::collections::HashMap;

use itertools::Itertools;

use crate::lang::ConditionalDirectiveKind as CDK;

use crate::lang::CommentKind as CK;
use crate::lang::KeywordKind as KK;
use crate::lang::OperatorKind as OK;
use crate::lang::RawTokenType as TT;
use crate::lang::*;
use crate::traits::LogicalLineParser;

type LogicalLineRef = usize;

pub struct DelphiLogicalLineParser {}
impl LogicalLineParser for DelphiLogicalLineParser {
    fn parse<'a>(&self, mut input: Vec<RawToken<'a>>) -> (Vec<LogicalLine>, Vec<Token<'a>>) {
        let logical_lines = parse_file(&mut input);
        let consolidated_tokens = input.into_iter().map(RawToken::into).collect();
        (logical_lines, consolidated_tokens)
    }
}

/*
    The grouping of tokens into a `LogicalLine` is loose and fuzzy. This is such
    to better support invalid code.

    There are two main parts to parsing a file, the `structures` and the `expressions`.
    Roughly `structures` represent things like:
    * If/While/Repeat/For etc.
    * Interface/Implementation etc.
    The control flow and overall structure of the code.

    `Expression` on the other hand represents more the actual lines of code themselves.
    The parts that perform the actions of the code. E.g., Variable assignment or routine invocation

    This function covers all the parts required to parse a file:
    * Finding all the conditional branches to repeatedly parse the file to cover all cases
    * Parsing the file with each branch of conditional tokens
    * Consolidating the tokens between passes
    * Creating the lines of the individual conditional directive tokens
*/
fn parse_file(tokens: &mut [RawToken]) -> Vec<LogicalLine> {
    let conditional_branches = get_conditional_branches_per_directive(tokens);
    let passes = get_all_conditional_branch_paths(&conditional_branches);
    let mut lines = HashMap::new();
    let mut attributed_directives = Vec::new();
    let mut pass_tokens = Vec::new();
    for pass in passes {
        get_pass_tokens(tokens, &pass, &conditional_branches, &mut pass_tokens);
        let pass_lines =
            InternalDelphiLogicalLineParser::new(tokens, &pass_tokens, &mut attributed_directives)
                .parse();
        /*
            This pass over the tokens ensures that their consolidated token type
            is cemented after the first run that encounters them.
        */
        for &pass_token in &pass_tokens {
            if let TT::IdentifierOrKeyword(_) = tokens[pass_token].get_token_type() {
                tokens[pass_token].set_token_type(TT::Identifier);
            }
        }
        consolidate_pass_lines(&mut lines, pass_lines);
    }

    let mut directive_lines = vec![];
    let mut directive_level = 0;
    for (token_index, token) in tokens
        .iter()
        .enumerate()
        .filter(|(token_index, _)| !attributed_directives.contains(token_index))
    {
        match token.get_token_type() {
            TT::CompilerDirective => directive_lines.push(LocalLogicalLine {
                parent: None,
                level: directive_level,
                tokens: vec![token_index],
                line_type: LLT::CompilerDirective,
            }),
            TT::ConditionalDirective(CDK::If | CDK::Ifdef | CDK::Ifndef | CDK::Ifopt) => {
                directive_lines.push(LocalLogicalLine {
                    parent: None,
                    level: directive_level,
                    tokens: vec![token_index],
                    line_type: LLT::ConditionalDirective,
                });
                directive_level += 1;
            }
            TT::ConditionalDirective(CDK::Endif | CDK::Ifend) => {
                directive_level = directive_level.saturating_sub(1);
                directive_lines.push(LocalLogicalLine {
                    parent: None,
                    level: directive_level,
                    tokens: vec![token_index],
                    line_type: LLT::ConditionalDirective,
                });
            }
            TT::ConditionalDirective(CDK::Else | CDK::Elseif) => {
                directive_lines.push(LocalLogicalLine {
                    parent: None,
                    level: directive_level.saturating_sub(1),
                    tokens: vec![token_index],
                    line_type: LLT::ConditionalDirective,
                })
            }
            _ => {}
        }
    }
    consolidate_pass_lines(&mut lines, directive_lines);

    lines
        .into_iter()
        .sorted_by_key(|&(_, index)| index)
        .map(|(line, _)| LogicalLine::new(line.parent, line.level, line.tokens, line.line_type))
        .collect()
}

fn consolidate_pass_lines(
    result_lines: &mut HashMap<LocalLogicalLine, usize>,
    pass_lines: Vec<LocalLogicalLine>,
) {
    let mut mapped_line_indices = Vec::new();
    for mut line in pass_lines {
        let mapped_line_index = match result_lines.get(&line) {
            None => result_lines.len(),
            Some(&index) => index,
        };
        mapped_line_indices.push(mapped_line_index);

        if line.tokens.is_empty() {
            continue;
        }

        line.parent = line.parent.map(|parent| LineParent {
            line_index: mapped_line_indices[parent.line_index],
            global_token_index: parent.global_token_index,
        });

        result_lines.insert(line, mapped_line_index);
    }
}

struct InternalDelphiLogicalLineParser<'a, 'b> {
    tokens: &'a mut [RawToken<'b>],
    pass_indices: &'a [usize],
    result_lines: NonEmptyVec<LocalLogicalLine>,
    current_line: NonEmptyVec<LogicalLineRef>,
    pass_index: usize,
    context: Vec<ParserContext>,
    unfinished_comment_lines: Vec<LogicalLineRef>,
    paren_level: u32,
    brack_level: u32,
    generic_level: u32,
    attributed_directives: &'a mut Vec<usize>,
}
use InternalDelphiLogicalLineParser as LLP;
impl<'a, 'b> InternalDelphiLogicalLineParser<'a, 'b> {
    fn new(
        tokens: &'a mut [RawToken<'b>],
        pass_indices: &'a [usize],
        attributed_directives: &'a mut Vec<usize>,
    ) -> Self {
        InternalDelphiLogicalLineParser {
            tokens,
            pass_indices,
            result_lines: NonEmptyVec::new(LocalLogicalLine {
                parent: None,
                level: 0,
                tokens: vec![],
                line_type: LLT::Unknown,
            }),
            current_line: NonEmptyVec::new(0),
            pass_index: 0,
            context: vec![],
            unfinished_comment_lines: vec![],
            paren_level: 0,
            brack_level: 0,
            generic_level: 0,
            attributed_directives,
        }
    }
    fn parse(mut self) -> Vec<LocalLogicalLine> {
        self.parse_structures();
        self.finish_logical_line();
        self.next_token(); // Eof
        self.set_logical_line_type(LLT::Eof);
        self.finish_logical_line();
        self.result_lines.into()
    }
    fn parse_structures(&mut self) {
        while let Some(token_type) = self.get_current_token_type() {
            if let Some(context) = self.get_last_context() {
                if (context.context_ending_predicate)(self) {
                    self.finish_logical_line();
                    return;
                }
            }
            match token_type {
                /*
                    This is to catch the cases where CompilerDirective tokens
                    are strictly between ConditionalDirective tokens. Tokens in
                    this scenario will be assigned their own line and level by the
                    surrounding ConditionalDirective context, rather than the
                    LogicalLine context.

                    E.g.,
                    ```
                    {$ifdef A}
                      {$define B}
                    {$endif}
                    ```
                    vs
                    ```
                    type
                      {$D+}
                      TFoo = class;
                    ```
                */
                TT::CompilerDirective
                    if self.is_directive_before_next_token()
                        && self.is_directive_after_prev_token() =>
                {
                    self.skip_token()
                }
                kind @ (TT::Comment(_) | TT::CompilerDirective) => {
                    if let Some(token_index) = self.get_current_token_index() {
                        self.get_current_logical_line_mut().tokens.push(token_index);
                        if let TT::CompilerDirective = kind {
                            self.attributed_directives.push(token_index);
                            self.set_logical_line_type(LLT::CompilerDirective);
                        }
                        self.pass_index += 1;
                    }
                    let line_ref = self.get_current_logical_line_ref();
                    self.finish_logical_line();
                    /*
                        The lines are considered "unfinished" if their level is determined by the
                        code that follows.
                        E.g.,
                        ```
                        type
                          // in `TypeBlock` context, at level of type decl
                          A = Integer;
                        // still in `TypeBlock` context, but at level of top-level `procedure`
                        procedure Top;
                          // in `SubRoutine` context, at level of subroutine
                          procedure Sub; begin end;
                        // in `SubRoutine` context, but at level of `begin`
                        begin
                        end;
                        ```
                    */
                    if matches!(
                        self.get_last_context_type(),
                        Some(ContextType::SubRoutine | ContextType::TypeBlock)
                    ) {
                        self.unfinished_comment_lines.push(line_ref);
                    }
                }
                TT::Keyword(
                    keyword_kind @ (KK::Library | KK::Unit | KK::Program | KK::Package),
                )
                | TT::IdentifierOrKeyword(keyword_kind @ KK::Package) => {
                    if self.get_prev_token_type().is_none() {
                        self.consolidate_current_keyword();
                        let mut push_program_head_context = |context_type| {
                            self.context.push(ParserContext {
                                context_type,
                                parent: None,
                                context_ending_predicate: never_ending,
                                level_delta: 0,
                            });
                        };
                        use ContextType as CT;
                        match keyword_kind {
                            KK::Library => push_program_head_context(CT::Library),
                            KK::Unit => push_program_head_context(CT::Unit),
                            KK::Program => push_program_head_context(CT::Program),
                            KK::Package => push_program_head_context(CT::Package),
                            _ => {}
                        };
                    }
                    self.next_token();
                    self.simple_op_until(
                        after_semicolon(),
                        keyword_consolidator(|keyword| PORTABILITY_DIRECTIVES.contains(&keyword)),
                    );
                    self.finish_logical_line();
                }
                TT::Op(OK::LBrack) if self.get_current_logical_line().tokens.is_empty() => {
                    // If there is a `[` at the start of a line, it must be an attribute
                    self.skip_pair();
                    self.set_logical_line_type(LogicalLineType::Attribute);
                    let line_ref = self.get_current_logical_line_ref();
                    self.finish_logical_line();
                    self.unfinished_comment_lines.push(line_ref);
                }
                TT::Keyword(
                    keyword_kind @ (KK::Interface
                    | KK::Implementation
                    | KK::Initialization
                    | KK::Finalization),
                ) => {
                    self.finish_logical_line();
                    self.next_token();
                    self.finish_logical_line();
                    let mut push_section_context = |context_type, level_delta| {
                        self.parse_block(ParserContext {
                            context_type,
                            parent: None,
                            context_ending_predicate: section_headings,
                            level_delta,
                        });
                    };
                    match keyword_kind {
                        KK::Interface => push_section_context(ContextType::Interface, 0),
                        KK::Implementation => push_section_context(ContextType::Implementation, 0),
                        KK::Initialization => push_section_context(ContextType::Initialization, 1),
                        KK::Finalization => push_section_context(ContextType::Finalization, 1),
                        _ => {}
                    };
                }
                TT::Keyword(KK::Begin) => {
                    self.next_token();
                    self.parse_block(ParserContext {
                        context_type: ContextType::CompoundStatement,
                        parent: None,
                        context_ending_predicate: end,
                        level_delta: 1,
                    });
                    self.next_token(); // End
                    if let Some(TT::Op(OK::Dot)) = self.get_current_token_type() {
                        self.next_token();
                    } else {
                        self.take_until(no_more_separators());
                    }
                    self.finish_logical_line();
                }
                TT::Keyword(KK::End) => {
                    self.next_token();
                    if let Some(TT::Op(OK::Dot)) = self.get_current_token_type() {
                        self.next_token();
                    }
                }
                TT::Keyword(KK::Repeat) => {
                    self.next_token(); // Repeat
                    self.parse_block(ParserContext {
                        context_type: ContextType::RepeatBlock,
                        parent: None,
                        context_ending_predicate: until,
                        level_delta: 1,
                    });
                    self.next_token(); // Until
                    let context_ending_predicate = match self.get_last_context() {
                        Some(context) => context.context_ending_predicate,
                        _ => never_ending,
                    };
                    self.context.push(ParserContext {
                        context_type: ContextType::BlockClause,
                        parent: None,
                        context_ending_predicate,
                        level_delta: 0,
                    });
                    self.parse_statement();
                    self.context.pop();
                    self.take_until(no_more_separators());
                    self.finish_logical_line();
                }
                TT::Keyword(KK::Try) => {
                    self.next_token(); // Try
                    self.parse_block(ParserContext {
                        context_type: ContextType::TryBlock,
                        parent: None,
                        context_ending_predicate: except_finally,
                        level_delta: 1,
                    });
                    let context_type = match self.get_current_token_type() {
                        Some(TT::Keyword(KK::Except)) => ContextType::ExceptBlock,
                        _ => ContextType::CompoundStatement,
                    };
                    self.next_token(); // Except/Finally
                    self.parse_block(ParserContext {
                        context_type,
                        parent: None,
                        context_ending_predicate: else_end,
                        level_delta: 1,
                    });
                    if let Some(TT::Keyword(KK::Else)) = self.get_current_token_type() {
                        self.next_token(); // Else
                        self.parse_block(ParserContext {
                            context_type: ContextType::CompoundStatement,
                            parent: None,
                            context_ending_predicate: end,
                            level_delta: 1,
                        });
                    }
                    self.next_token(); // End
                    self.take_until(no_more_separators());
                    self.finish_logical_line();
                }
                TT::Keyword(KK::On) | TT::IdentifierOrKeyword(KK::On)
                    if matches!(self.get_last_context_type(), Some(ContextType::ExceptBlock)) =>
                {
                    self.consolidate_current_keyword();
                    self.next_token(); // On

                    // Continue parsing until do pops this context
                    self.context.push(ParserContext {
                        context_type: ContextType::BlockClause,
                        parent: None,
                        context_ending_predicate: never_ending,
                        level_delta: 0,
                    });
                }
                TT::Keyword(keyword_kind @ (KK::For | KK::While | KK::With)) => {
                    self.next_token(); // For/While/With
                    self.set_logical_line_type(match keyword_kind {
                        KK::For => LLT::ForLoop,
                        _ => LLT::Unknown,
                    });
                    // Continue parsing until do pops this context
                    self.context.push(ParserContext {
                        context_type: ContextType::BlockClause,
                        parent: None,
                        context_ending_predicate: never_ending,
                        level_delta: 0,
                    });
                    self.parse_statement();
                }
                TT::Keyword(KK::If) => {
                    self.next_token(); // If

                    // Continue parsing until then pops this context
                    self.context.push(ParserContext {
                        context_type: ContextType::BlockClause,
                        parent: None,
                        context_ending_predicate: never_ending,
                        level_delta: 0,
                    })
                }
                TT::Keyword(KK::Then) => {
                    if let Some(ContextType::BlockClause) = self.get_last_context_type() {
                        self.context.pop();
                    }
                    self.next_token();
                }
                TT::Keyword(KK::Else) => {
                    self.next_token();
                }
                TT::Keyword(KK::Case) => {
                    let tagged_record =
                        self.context
                            .iter()
                            .any(|ParserContext { context_type, .. }| {
                                matches!(context_type, ContextType::TypeDeclaration)
                            });
                    if tagged_record {
                        self.context.push(ParserContext {
                            context_type: ContextType::DeclarationBlock,
                            parent: None,
                            context_ending_predicate: never_ending,
                            level_delta: -1,
                        })
                    }
                    self.next_token(); // Case

                    // Continue parsing until of pops this context
                    self.set_logical_line_type(LLT::CaseHeader);
                    self.context.push(ParserContext {
                        context_type: ContextType::BlockClause,
                        parent: None,
                        context_ending_predicate: never_ending,
                        level_delta: 0,
                    });
                }
                TT::Keyword(KK::Of) => {
                    self.next_token(); // Of
                    self.finish_logical_line();
                    self.parse_block(ParserContext {
                        context_type: ContextType::CaseStatement,
                        parent: None,
                        context_ending_predicate: else_end,
                        level_delta: 1,
                    });
                    if let Some(TT::Keyword(KK::Else)) = self.get_current_token_type() {
                        self.next_token(); // Else
                        self.finish_logical_line();
                        self.parse_block(ParserContext {
                            context_type: ContextType::CompoundStatement,
                            parent: None,
                            context_ending_predicate: end,
                            level_delta: 1,
                        });
                    }
                    let tagged_record =
                        self.context
                            .iter()
                            .any(|ParserContext { context_type, .. }| {
                                matches!(context_type, ContextType::TypeDeclaration)
                            });
                    if !tagged_record {
                        // There is no end if it is a tagged record
                        self.next_token(); // End
                        self.take_until(no_more_separators());
                        self.finish_logical_line();
                    } else {
                        self.context.pop();
                    }
                }
                TT::Keyword(KK::Uses) => {
                    self.finish_logical_line();
                    self.next_token();
                    self.set_logical_line_type(LLT::ImportClause);
                    self.take_until(after_semicolon());
                    self.finish_logical_line();
                }
                TT::Keyword(KK::Contains | KK::Requires)
                | TT::IdentifierOrKeyword(KK::Contains | KK::Requires)
                    if matches!(self.get_last_context_type(), Some(ContextType::Package)) =>
                {
                    self.finish_logical_line();
                    self.consolidate_current_keyword();
                    self.set_logical_line_type(LLT::ImportClause);
                    self.next_token();
                    self.take_until(after_semicolon());
                    self.finish_logical_line();
                }
                TT::Keyword(KK::Exports) => {
                    self.finish_logical_line();
                    self.next_token(); // Exports
                    self.parse_expression(); // Identifier
                    self.simple_op_until(after_semicolon(), parse_exports);
                    self.finish_logical_line();
                }
                TT::Keyword(KK::Class) => {
                    // If class is the first token in the line, allow the next
                    // token to dictate how it will be parsed.
                    self.next_token();
                    if let Some(KK::Operator) = self.get_current_keyword_kind() {
                        self.consolidate_current_keyword();
                    }
                }
                TT::Keyword(KK::Strict) | TT::IdentifierOrKeyword(KK::Strict) => {
                    self.next_token();
                }
                TT::Keyword(
                    KK::Private | KK::Protected | KK::Public | KK::Published | KK::Automated,
                )
                | TT::IdentifierOrKeyword(
                    KK::Private | KK::Protected | KK::Public | KK::Published | KK::Automated,
                ) => {
                    if let Some(TT::IdentifierOrKeyword(KK::Strict)) = self.get_prev_token_type() {
                        self.consolidate_prev_keyword();
                    }
                    self.consolidate_current_keyword();
                    self.next_token();
                    self.finish_logical_line();

                    self.parse_block(ParserContext {
                        context_type: ContextType::VisibilityBlock,
                        parent: None,
                        context_ending_predicate: visibility_block_ending,
                        level_delta: 1,
                    });
                }
                TT::Keyword(
                    keyword_kind @ (KK::Var | KK::ThreadVar | KK::Const | KK::Label | KK::Type),
                ) => {
                    if let Some(ContextType::CompoundStatement) = self.get_last_context_type() {
                        // Inline declaration
                        self.set_logical_line_type(LLT::InlineDeclaration);
                        self.next_token();
                        continue;
                    }

                    self.next_token();
                    let reduce_level =
                        matches!(self.get_last_context_type(), Some(ContextType::SubRoutine));
                    if reduce_level {
                        /*
                            To ensure the declaration blocks within a function declaration are at
                            the same level as the function they are within
                        */
                        self.context.push(ParserContext {
                            context_type: ContextType::SubRoutine,
                            parent: None,
                            context_ending_predicate: never_ending,
                            level_delta: -1,
                        })
                    }
                    self.finish_logical_line();
                    let context_type = match keyword_kind {
                        KK::Type => ContextType::TypeBlock,
                        _ => ContextType::DeclarationBlock,
                    };
                    self.parse_block(ParserContext {
                        context_type,
                        parent: None,
                        context_ending_predicate: declaration_section,
                        level_delta: 1,
                    });
                    if reduce_level {
                        self.context.pop();
                    }
                }
                TT::Keyword(KK::Property) => self.parse_property_declaration(),
                TT::Keyword(
                    KK::Function | KK::Procedure | KK::Constructor | KK::Destructor | KK::Operator,
                ) => {
                    self.set_logical_line_type(LLT::RoutineHeader);
                    self.parse_routine_header();
                    let is_forward_declaration = self
                        .get_current_logical_line_token_types()
                        .rev()
                        .any(|token_type| {
                            matches!(token_type, TT::Keyword(KK::Forward | KK::External))
                        })
                        || self.context.iter().any(|context| {
                            matches!(
                                context.context_type,
                                ContextType::Interface | ContextType::TypeDeclaration
                            )
                        });
                    self.finish_logical_line();
                    if !is_forward_declaration {
                        self.parse_block(ParserContext {
                            context_type: ContextType::SubRoutine,
                            parent: None,
                            context_ending_predicate: begin_asm,
                            level_delta: 1,
                        });
                        match self.get_current_token_type() {
                            Some(TT::Keyword(KK::Asm)) => self.parse_asm_block(),
                            Some(TT::Keyword(KK::Begin)) => {
                                self.parse_begin_end(false, 1);
                                self.finish_logical_line();
                            }
                            _ => {}
                        }
                    }
                }
                TT::Keyword(KK::Asm) => self.parse_asm_block(),
                TT::Keyword(KK::Raise) => {
                    self.next_token();
                    self.parse_expression();
                    if let Some(KK::At) = self.get_current_keyword_kind() {
                        self.consolidate_current_keyword();
                        self.parse_expression();
                    }
                }
                _ => self.parse_statement(),
            }
        }
    }

    fn parse_statement(&mut self) {
        while let Some(token_type) = self.get_current_token_type() {
            if let Some(context) = self.get_last_context() {
                if (context.context_ending_predicate)(self) {
                    self.finish_logical_line();
                    return;
                }
            }
            match token_type {
                TT::Keyword(
                    KK::Class | KK::Interface | KK::DispInterface | KK::Record | KK::Object,
                ) => {
                    self.next_token(); // Class/Interface/DispInterface/Record/Object
                    if let Some(KK::Abstract | KK::Sealed) = self.get_current_keyword_kind() {
                        if self.get_next_token_type() != Some(TT::Op(OK::Colon)) {
                            self.consolidate_current_keyword();
                        }
                        self.next_token(); // Abstract/Sealed
                    }
                    if let Some(KK::Helper) = self.get_current_keyword_kind() {
                        if self.get_next_token_type() == Some(TT::Keyword(KK::For)) {
                            self.consolidate_current_keyword();
                            self.next_token(); // Helper
                            self.next_token(); // For
                            self.parse_expression(); // Type name
                        }
                    }
                    if self.get_current_token_type() == Some(TT::Op(OK::LParen)) {
                        self.parse_parens(); // Parent types
                    }
                    match self.get_current_token_type() {
                        Some(TT::Keyword(KK::Of)) => {
                            // class of ... - continue parsing statement
                            self.next_token();
                            break;
                        }
                        Some(TT::Op(OK::Semicolon)) => return, // class; - forward class declaration, statement over
                        _ => {}
                    };

                    self.finish_logical_line();
                    self.context.push(ParserContext {
                        context_type: ContextType::TypeDeclaration,
                        parent: None,
                        context_ending_predicate: end,
                        level_delta: 0,
                    });
                    // For the implicit published visibility section
                    self.context.push(ParserContext {
                        context_type: ContextType::VisibilityBlock,
                        parent: None,
                        context_ending_predicate: visibility_block_ending,
                        level_delta: 1,
                    });
                    if let Some((TT::Op(OK::LBrack), TT::TextLiteral(_))) = self
                        .get_current_token_type()
                        .zip(self.get_next_token_type())
                    {
                        // Guid block
                        self.next_token();
                        self.take_until(|parser| {
                            matches!(parser.get_current_token_type(), Some(TT::Op(OK::RBrack)))
                        });
                        self.next_token();
                        self.set_logical_line_type(LLT::Guid);
                        self.finish_logical_line();
                    }
                    self.parse_structures();
                    self.context.pop();
                    self.parse_structures();
                    self.context.pop();
                    self.finish_logical_line();

                    self.next_token(); // End
                    self.simple_op_until(
                        after_semicolon(),
                        keyword_consolidator(|keyword| {
                            PORTABILITY_DIRECTIVES.contains(&keyword) || keyword == KK::Align
                        }),
                    );
                    self.take_until(no_more_separators());
                    self.finish_logical_line();
                    return;
                }
                TT::Keyword(KK::Do | KK::Then) => {
                    if let Some(ContextType::BlockClause) = self.get_last_context_type() {
                        self.context.pop();
                    }
                    self.next_token(); // Do/Then

                    if let Some(TT::Keyword(KK::Begin)) = self.get_current_token_type() {
                        return;
                    }

                    self.finish_logical_line();

                    self.context.push(ParserContext {
                        context_type: ContextType::InlineStatement,
                        parent: None,
                        context_ending_predicate: semicolon_else_or_parent,
                        level_delta: 1,
                    });
                    self.parse_structures();
                    self.finish_logical_line();
                    self.context.pop();

                    match (self.get_current_token_type(), self.get_last_context_type()) {
                        (Some(TT::Keyword(KK::Else)), Some(ContextType::ExceptBlock)) => return,
                        (Some(TT::Keyword(KK::Else)), _) => self.next_token(),
                        _ => return,
                    }

                    if let Some(TT::Keyword(KK::Begin | KK::If)) = self.get_current_token_type() {
                        return;
                    }

                    self.finish_logical_line();

                    self.context.push(ParserContext {
                        context_type: ContextType::InlineStatement,
                        parent: None,
                        context_ending_predicate: semicolon_else_or_parent,
                        level_delta: 1,
                    });
                    self.parse_structures();
                    self.finish_logical_line();
                    self.context.pop();
                }
                TT::Keyword(KK::Of) => {
                    if let Some(ContextType::BlockClause) = self.get_last_context_type() {
                        // case ... of
                        self.context.pop();
                        return;
                    } else {
                        self.next_token();
                        if let Some(KK::Const) = self.get_current_keyword_kind() {
                            self.next_token();
                        }
                    }
                }
                TT::Op(OK::LParen) => self.parse_parens(),
                TT::Op(OK::Semicolon) => {
                    self.take_until(no_more_separators());
                    self.finish_logical_line();
                    return;
                }
                TT::Op(OK::LessThan)
                    if matches!(self.get_last_context_type(), Some(ContextType::TypeBlock)) =>
                {
                    self.skip_pair();
                }
                TT::Op(OK::Colon) => {
                    self.next_token();
                    if let Some(
                        ContextType::VisibilityBlock
                        | ContextType::DeclarationBlock
                        | ContextType::TypeDeclaration,
                    ) = self.get_last_context_type()
                    {
                        match self.get_current_token_type() {
                            Some(TT::Keyword(KK::Class)) => self.next_token(),
                            Some(TT::Keyword(KK::Function | KK::Procedure)) => {
                                self.parse_routine_header();
                                self.finish_logical_line();
                                return;
                            }
                            _ => {}
                        }
                    }
                }
                TT::Op(OK::Equal) => {
                    self.next_token();
                    if let Some(ContextType::TypeBlock) = self.get_last_context_type() {
                        match self.get_current_token_type() {
                            Some(TT::Keyword(KK::Type)) => {
                                self.next_token();
                                if let Some(TT::Keyword(KK::Of)) = self.get_current_token_type() {
                                    // type TFoo = type of
                                    self.next_token()
                                }
                            }
                            Some(TT::Keyword(KK::Function | KK::Procedure)) => {
                                self.parse_routine_header();
                                self.finish_logical_line();
                                return;
                            }
                            _ => {}
                        }
                    }
                }
                TT::IdentifierOrKeyword(KK::Reference) => {
                    if let Some(TT::Keyword(KK::To)) = self.get_next_token_type() {
                        self.consolidate_current_keyword();
                    }
                    self.next_token();
                }
                TT::Keyword(KK::To) => {
                    self.next_token();
                    if let Some(TT::Keyword(KK::Function | KK::Procedure)) =
                        self.get_current_token_type()
                    {
                        self.parse_routine_header();
                        self.finish_logical_line();
                        return;
                    }
                }
                TT::IdentifierOrKeyword(KK::Absolute)
                    if matches!(
                        self.get_prev_token_type(),
                        Some(TT::Identifier | TT::IdentifierOrKeyword(_))
                    ) =>
                {
                    self.consolidate_current_keyword();
                    self.next_token();
                }
                TT::Op(OK::Assign) => {
                    self.next_token();
                    if self.get_current_logical_line().line_type == LLT::Unknown {
                        self.set_logical_line_type(LLT::Assignment);
                    }
                }
                TT::Keyword(KK::Function | KK::Procedure) => {
                    self.parse_anonymous_routine();
                }
                TT::Keyword(KK::Begin) => {
                    self.next_token();
                    self.parse_block(ParserContext {
                        context_type: ContextType::CompoundStatement,
                        parent: None,
                        context_ending_predicate: end,
                        level_delta: 1,
                    });
                    self.next_token();
                    self.take_until(no_more_separators());
                    self.finish_logical_line();
                }
                _ => self.next_token(),
            }
        }
    }

    fn parse_block(&mut self, context: ParserContext) {
        let context_has_parent = context.parent.is_some();
        if context_has_parent {
            let new_logical_line = self.result_lines.len();
            self.result_lines.push(LocalLogicalLine {
                parent: context.parent,
                level: 0,
                tokens: vec![],
                line_type: LLT::Unknown,
            });
            self.current_line.push(new_logical_line);
        } else {
            self.finish_logical_line();
        }

        self.context.push(context);
        self.parse_structures();
        self.context.pop();

        self.finish_logical_line();
        if context_has_parent {
            self.current_line.pop();
        }
    }
    fn parse_parens(&mut self) {
        self.next_token(); // (
        loop {
            let token_type = match self.get_current_token_type() {
                None => return,
                Some(token_type) => token_type,
            };
            match token_type {
                TT::Op(OK::LParen) => self.parse_parens(),
                TT::Op(OK::RParen) => {
                    self.next_token(); // )
                    return;
                }
                TT::Keyword(KK::Function | KK::Procedure) => {
                    self.parse_anonymous_routine();
                }
                _ => self.next_token(),
            }
        }
    }
    fn skip_pair(&mut self) {
        let paren_level = self.paren_level;
        let brack_level = self.brack_level;
        let generic_level = self.generic_level;

        self.next_token();
        while (self.paren_level != paren_level
            || self.brack_level != brack_level
            || self.generic_level != generic_level)
            && self.get_current_token_type().is_some()
        {
            self.next_token();
        }
    }
    fn parse_anonymous_routine(&mut self) {
        self.next_token(); // procedure/function
        loop {
            let token_type = match self.get_current_token_type() {
                None => return,
                Some(token_type) => token_type,
            };
            match token_type {
                TT::Op(OK::LParen) => self.parse_parameter_list(),
                TT::Keyword(keyword_kind @ (KK::Label | KK::Const | KK::Type | KK::Var)) => {
                    let context_type = match keyword_kind {
                        KK::Type => ContextType::TypeBlock,
                        _ => ContextType::DeclarationBlock,
                    };
                    self.next_token(); // Label/Const/Type/Var
                    self.parse_block(ParserContext {
                        context_type,
                        parent: Some(self.get_line_parent_of_prev_token()),
                        context_ending_predicate: local_declaration_section,
                        level_delta: 0i16.saturating_sub_unsigned(self.get_context_level()),
                    });
                }
                TT::Keyword(KK::Begin) => {
                    self.parse_begin_end(
                        true,
                        0i16.saturating_sub_unsigned(self.get_context_level()),
                    );
                    break;
                }
                TT::Op(OK::Semicolon | OK::RParen | OK::RBrack) => break,
                _ => self.next_token(),
            }
        }
    }

    fn parse_routine_header(&mut self) {
        self.next_token(); // Function/Procedure/Constructor/Destructor/Operator
        const METHOD_DIRECTIVES_WITH_ARGS: [KeywordKind; 6] = [
            KK::Message,
            KK::Deprecated,
            KK::DispId,
            KK::External,
            KK::Index,
            KK::Name,
        ];
        let is_directive = |keyword_kind: &KeywordKind| {
            keyword_kind.is_method_directive()
                || matches!(keyword_kind, KK::Name | KK::Index | KK::Delayed)
        };
        // Allow the op and the context to dictate the ending of the loop
        self.op_until(never_ending, |parser| {
            match parser.get_current_token_type() {
                Some(TT::IdentifierOrKeyword(_))
                    if matches!(
                        parser.get_prev_token_type(),
                        Some(
                            TT::Op(OK::Colon | OK::Dot)
                                | TT::Keyword(
                                    KK::Function | KK::Procedure | KK::Constructor | KK::Destructor
                                )
                        )
                    ) =>
                {
                    parser.consolidate_current_ident();
                    parser.next_token();
                }
                Some(TT::Op(OK::LParen)) => parser.parse_parameter_list(),
                Some(TT::Keyword(keyword_kind) | TT::IdentifierOrKeyword(keyword_kind))
                    if is_directive(&keyword_kind) && parser.paren_level == 0 =>
                {
                    parser.consolidate_current_keyword();
                    parser.next_token();
                    if let (KK::External, Some(KK::Name)) =
                        (keyword_kind, parser.get_current_keyword_kind())
                    {
                        /*
                            The argument for `External` is optional and if the next token is `Name`
                            it is considered to have gone to the next directive.
                        */
                    } else if METHOD_DIRECTIVES_WITH_ARGS.contains(&keyword_kind) {
                        parser.parse_expression();
                    }
                }
                Some(TT::Op(OK::Semicolon))
                    if parser.paren_level == 0 && parser.brack_level == 0 =>
                {
                    parser.next_token();
                    parser.take_until(no_more_separators());
                    if parser
                        .get_current_keyword_kind()
                        .filter(is_directive)
                        .is_none()
                    {
                        return OpResult::Break;
                    }
                }
                _ => parser.next_token(),
            }
            OpResult::Continue
        });
        self.take_until(no_more_separators());
    }
    fn parse_parameter_list(&mut self) {
        self.simple_op_until(
            predicate_and(after_rparen(), outside_parens(self.paren_level)),
            |parser| {
                match parser.get_current_token_type_window() {
                    (Some(TT::Op(OK::Colon | OK::Dot)), Some(TT::IdentifierOrKeyword(_)), _) => {
                        parser.consolidate_current_ident()
                    }
                    (
                        Some(TT::Op(OK::Semicolon | OK::LParen)),
                        Some(TT::IdentifierOrKeyword(KK::Out)),
                        _,
                    ) => parser.consolidate_current_keyword(),

                    (_, Some(TT::Keyword(KK::Of)), Some(TT::Keyword(KK::Const))) => {
                        parser.next_token();
                    }
                    (
                        _,
                        Some(TT::Op(OK::Semicolon | OK::LParen)),
                        Some(TT::Keyword(KK::Const | KK::Var)),
                    ) => {
                        // By skipping the Const, and Var keywords, it ensures that parent
                        // contexts that are ended with these keywords, are not ended prematurely.
                        parser.next_token();
                    }
                    _ => {}
                };
                parser.next_token();
            },
        );
    }
    fn parse_property_declaration(&mut self) {
        self.set_logical_line_type(LLT::PropertyDeclaration);
        self.next_token(); // Property
        self.parse_expression(); // Identifier

        const PROPERTY_DIRECTIVES_WITHOUT_ARGS: [KeywordKind; 3] =
            [KK::ReadOnly, KK::WriteOnly, KK::NoDefault];

        self.simple_op_until(
            predicate_and(
                after_semicolon(),
                predicate_and(
                    outside_parens(self.paren_level),
                    outside_bracks(self.brack_level),
                ),
            ),
            |parser| match parser.get_current_token_type_window() {
                (_, Some(TT::IdentifierOrKeyword(_)), Some(TT::Op(OK::Colon)))
                | (
                    Some(TT::Op(OK::Colon) | TT::Keyword(KK::Property)),
                    Some(TT::IdentifierOrKeyword(_)),
                    _,
                ) => {
                    parser.consolidate_current_ident();
                    parser.next_token();
                }
                (_, Some(TT::IdentifierOrKeyword(keyword_kind) | TT::Keyword(keyword_kind)), _)
                    if keyword_kind.is_property_directive() && parser.brack_level == 0 =>
                {
                    parser.consolidate_current_keyword();
                    parser.next_token();
                    if !PROPERTY_DIRECTIVES_WITHOUT_ARGS.contains(&keyword_kind) {
                        parser.parse_expression();
                    }
                }
                _ => parser.next_token(),
            },
        );
        if let Some(KK::Default) = self.get_current_keyword_kind() {
            // Handles `property ...; default;`
            self.consolidate_current_keyword();
            self.next_token(); // Default
            self.take_until(no_more_separators());
        }
        self.finish_logical_line();
    }
    fn parse_expression(&mut self) {
        match self.get_current_token_type() {
            Some(TT::Op(OK::LParen | OK::LBrack)) => self.skip_pair(),
            Some(TT::Op(OK::Semicolon | OK::Colon)) => return,
            Some(token_type @ TT::Keyword(_)) if !is_operator(token_type) => return,
            _ => self.next_token(),
        };
        loop {
            match self.get_current_token_type() {
                Some(TT::Op(OK::LParen | OK::LBrack)) => self.skip_pair(),
                Some(TT::Op(OK::Pointer)) => {
                    self.next_token();
                    return;
                }
                Some(TT::Op(OK::Semicolon | OK::Colon)) => return,
                Some(token_type) if is_operator(token_type) => {
                    self.next_token();
                    match self.get_current_token_type() {
                        Some(TT::IdentifierOrKeyword(_)) => {
                            self.consolidate_current_ident();
                            self.next_token()
                        }
                        Some(TT::Identifier | TT::TextLiteral(_) | TT::NumberLiteral(_)) => {
                            self.next_token()
                        }
                        _ => {}
                    }
                }
                None | Some(TT::Keyword(_)) => return,
                Some(
                    TT::Identifier
                    | TT::IdentifierOrKeyword(_)
                    | TT::TextLiteral(_)
                    | TT::NumberLiteral(_),
                ) => return,
                Some(_) => self.next_token(),
            }
        }
    }
    fn parse_asm_block(&mut self) {
        self.next_token(); // Asm
        self.finish_logical_line();

        self.context.push(ParserContext {
            context_type: ContextType::CompoundStatement,
            parent: None,
            context_ending_predicate: never_ending,
            level_delta: 1,
        });
        self.parse_asm_instructions();
        self.context.pop();

        self.next_token(); // End
        self.take_until(no_more_separators());
        self.finish_logical_line();
    }

    fn parse_asm_instructions(&mut self) {
        let add_asm_instruction_line = |parser: &mut InternalDelphiLogicalLineParser| {
            parser.finish_logical_line();
            parser.set_logical_line_type(LLT::AsmInstruction);
        };

        while let Some(token) = self
            .get_current_token()
            .filter(|token| !matches!(token.get_token_type(), TT::Keyword(KK::End)))
        {
            if let TT::Op(OK::Semicolon) = token.get_token_type() {
                self.next_token();
                add_asm_instruction_line(self);
            } else if token.get_leading_whitespace().contains(['\r', '\n']) {
                add_asm_instruction_line(self);
                self.next_token();
            } else {
                self.next_token();
            }
        }
        self.finish_logical_line();
    }
    fn parse_begin_end(&mut self, parent: bool, level_delta: i16) {
        self.next_token(); // Begin
        let parent = if parent {
            Some(self.get_line_parent_of_prev_token())
        } else {
            None
        };
        self.parse_block(ParserContext {
            context_type: ContextType::CompoundStatement,
            parent,
            context_ending_predicate: end,
            level_delta,
        });
        self.next_token(); // End
        self.take_until(no_more_separators());
    }
    fn consolidate_portability_directives(&mut self) {
        fn get_token_type_of_line_index(
            parser: &mut InternalDelphiLogicalLineParser,
            line_index: usize,
        ) -> Option<RawTokenType> {
            parser
                .get_current_logical_line()
                .tokens
                .get(line_index)
                .and_then(|&token_index| parser.tokens.get(token_index))
                .map(RawToken::get_token_type)
        }
        /*
            Portability directives can only occur in a few specific contexts:
            1. const/var/type/field declarations
            2. file headers (on the `unit`/`library` line)
            3. routine declarations

            Cases 2 and 3 are handled inline while parsing. Case 1,
            however, due to its widespread possibility, is handled separately.
            With that, if the line doesn't contain `:` or `=` tokens, it cannot
            be one of the cases, and so an early exit.
        */
        if !self
            .get_current_logical_line_token_types()
            .any(|token_type| matches!(token_type, TT::Op(OK::Equal | OK::Colon)))
        {
            return;
        }

        let mut line_index = self.get_current_logical_line().tokens.len() - 1;
        if let Some(TT::Op(OK::Semicolon)) = self.get_current_logical_line_token_types().next_back()
        {
            line_index -= 1;
        }
        /*
            Traverse backward from the end of line. Set portability directives
            to Keywords until a break case.
        */
        while let Some(&token_index) = self.get_current_logical_line().tokens.get(line_index) {
            let prev_token_type = line_index
                .checked_sub(1)
                .and_then(|index| get_token_type_of_line_index(self, index));
            /*
                When traversing the line there are some cases where there is
                guaranteed to be no more portability directives before a token.
                These are:
                - Binary operators, identifiers, or non-text literals, indicating an expression,
                  Text literals can be associated with the deprecated portability directive.
                  e.g., `const Bar = (1) deprecated;` - `)`
                  e.g., `const Bar = [1] deprecated;` - `]`
                  e.g., `const Bar = 1 + 2 deprecated;` - `+` and `2`
                  e.g., `const Bar = 1 div 2 deprecated;` - `div` and `2`
                  e.g., `const Bar = Foo deprecated;` - `Foo`
                  Portability directives can only occur at the end of a line, after the value expression
                - Keywords indicating the contents of a line,
                  e.g., `TFoo = type Bar; deprecated;` - `type` or `Bar`
                  e.g., `TFoo = procedure of object; deprecated;` - `of`
            */
            if matches!(
                get_token_type_of_line_index(self, line_index),
                Some(
                    TT::Identifier
                        | TT::NumberLiteral(_)
                        | TT::Keyword(KK::Type)
                        | TT::Op(OK::RBrack | OK::RParen | OK::RGeneric | OK::Semicolon)
                )
            ) {
                break;
            }
            match prev_token_type {
                Some(TT::Op(OK::RBrack | OK::RGeneric | OK::RParen)) => {}
                Some(TT::Keyword(KK::Type | KK::Of)) => break,
                Some(token_type) if is_operator(token_type) => break,
                _ => {}
            }

            if let Some(TT::IdentifierOrKeyword(
                directive @ (KK::Deprecated | KK::Experimental | KK::Platform | KK::Library),
            )) = self.tokens.get(token_index).map(RawToken::get_token_type)
            {
                if let Some(token) = self.tokens.get_mut(token_index) {
                    token.set_token_type(TT::Keyword(directive))
                }
            }

            line_index -= 1;
        }
    }

    fn take_until(&mut self, predicate: impl Fn(&LLP) -> bool) {
        self.simple_op_until(predicate, |parser| parser.next_token())
    }
    fn simple_op_until(
        &mut self,
        predicate: impl Fn(&LLP) -> bool,
        simple_next_token_op: impl Fn(&mut LLP),
    ) {
        self.op_until(predicate, |parser| {
            simple_next_token_op(parser);
            OpResult::Continue
        })
    }
    fn op_until(
        &mut self,
        predicate: impl Fn(&LLP) -> bool,
        next_token_op: impl Fn(&mut LLP) -> OpResult,
    ) {
        while self.get_current_token_type().is_some()
            && !predicate(self)
            && !context_over()(self)
            && matches!(next_token_op(self), OpResult::Continue)
        {}
    }

    fn finish_logical_line(&mut self) {
        if self.get_current_logical_line().tokens.is_empty() {
            self.get_current_logical_line_mut().line_type = LLT::Unknown;
            return;
        }

        self.consolidate_portability_directives();
        while let Some((TT::Comment(CK::InlineBlock | CK::InlineLine), token_index)) = self
            .get_current_token_type()
            .zip(self.get_current_token_index())
        {
            self.get_current_logical_line_mut().tokens.push(token_index);
            self.pass_index += 1;
        }
        let mut context_level = self.get_context_level();

        for unfinished_line in self.unfinished_comment_lines.drain(..).collect_vec() {
            if let Some(line) = self.get_logical_line_from_ref_mut(unfinished_line) {
                line.level = context_level;
            }
        }

        let line_ref = *self.current_line.last();
        let parent = self
            .context
            .iter()
            .filter_map(|context| context.parent)
            .next_back();

        if let Some(parent) = parent {
            context_level = context_level.saturating_sub(
                self.get_logical_line_from_ref(parent.line_index)
                    .unwrap()
                    .level,
            );
        }

        {
            let line = self.get_logical_line_from_ref_mut(line_ref).unwrap();
            line.parent = parent;
            line.level = context_level;
        }

        let new_logical_line = self.result_lines.len();
        self.result_lines.push(LocalLogicalLine {
            parent: None,
            level: context_level,
            tokens: vec![],
            line_type: LLT::Unknown,
        });
        *self.current_line.last_mut() = new_logical_line;
    }
    fn get_current_logical_line_ref(&self) -> LogicalLineRef {
        *self.current_line.last()
    }
    fn get_current_logical_line(&self) -> &LocalLogicalLine {
        let line_ref = self.get_current_logical_line_ref();
        self.get_logical_line_from_ref(line_ref).unwrap()
    }
    fn get_current_logical_line_token_types(
        &self,
    ) -> impl DoubleEndedIterator<Item = RawTokenType> + '_ {
        self.get_current_logical_line()
            .tokens
            .iter()
            .filter_map(|&index| self.tokens.get(index))
            .map(RawToken::get_token_type)
    }
    fn get_current_logical_line_mut(&mut self) -> &mut LocalLogicalLine {
        let line_ref = *self.current_line.last();
        self.get_logical_line_from_ref_mut(line_ref).unwrap()
    }
    fn get_logical_line_from_ref(&self, line_ref: LogicalLineRef) -> Option<&LocalLogicalLine> {
        self.result_lines.get(line_ref)
    }
    fn get_logical_line_from_ref_mut(
        &mut self,
        line_ref: LogicalLineRef,
    ) -> Option<&mut LocalLogicalLine> {
        self.result_lines.get_mut(line_ref)
    }
    fn set_logical_line_type(&mut self, line_type: LogicalLineType) {
        self.get_current_logical_line_mut().line_type = line_type;
    }

    fn next_token(&mut self) {
        loop {
            if let Some(token_index) = self.get_current_token_index() {
                self.get_current_logical_line_mut().tokens.push(token_index);
            }

            match self.get_current_token_type() {
                Some(TT::Op(OK::LParen)) => self.paren_level += 1,
                Some(TT::Op(OK::RParen)) => self.paren_level = self.paren_level.saturating_sub(1),
                Some(TT::Op(OK::LBrack)) => self.brack_level += 1,
                Some(TT::Op(OK::RBrack)) => self.brack_level = self.brack_level.saturating_sub(1),
                Some(TT::Op(OK::LessThan)) => self.generic_level += 1,
                Some(TT::Op(OK::GreaterThan)) => {
                    self.generic_level = self.generic_level.saturating_sub(1)
                }
                _ => {}
            }
            self.pass_index += 1;
            let Some(TT::Comment(CK::InlineBlock | CK::InlineLine)) = self.get_current_token_type()
            else {
                break;
            };
        }
    }

    fn skip_token(&mut self) {
        self.pass_index += 1;
    }

    fn get_prev_token(&self) -> Option<&RawToken> {
        let rev_skip = self.pass_indices.len().checked_sub(self.pass_index)?;
        self.pass_indices
            .iter()
            .rev()
            .skip(rev_skip)
            .find_map(|&index| self.tokens.get(index).filter(token_type_filter))
    }
    fn is_directive_after_prev_token(&self) -> bool {
        let Some(mut last_index) = self.get_current_token_index() else {
            return false;
        };
        let Some(rev_skip) = self.pass_indices.len().checked_sub(self.pass_index) else {
            return false;
        };
        for &index in self.pass_indices.iter().rev().skip(rev_skip) {
            if last_index - index > 1 {
                return true;
            }
            if self.tokens.get(index).filter(token_type_filter).is_some() {
                return false;
            }
            last_index = index;
        }
        // If the first token in is a conditional directive that needs to be caught too
        self.pass_indices[0] != 0
    }
    fn get_prev_token_type(&self) -> Option<RawTokenType> {
        self.get_prev_token().map(RawToken::get_token_type)
    }
    fn consolidate_prev_keyword(&mut self) {
        if let Some(token) = self
            .pass_index
            .checked_sub(1)
            .and_then(|prev_pass_index| self.pass_indices.get(prev_pass_index))
            .and_then(|&token_index| self.tokens.get_mut(token_index))
        {
            if let TT::IdentifierOrKeyword(keyword_kind) = token.get_token_type() {
                token.set_token_type(TT::Keyword(keyword_kind));
            }
        }
    }
    fn get_current_token_index(&self) -> Option<usize> {
        self.pass_indices.get(self.pass_index).cloned()
    }
    fn get_current_token(&self) -> Option<&RawToken> {
        let token_index = self.get_current_token_index()?;
        self.tokens
            .get(token_index)
            .filter(|token| token.get_token_type() != TT::Eof)
    }
    fn get_current_token_type(&self) -> Option<RawTokenType> {
        self.get_current_token().map(RawToken::get_token_type)
    }
    fn get_current_token_type_window(
        &self,
    ) -> (
        Option<RawTokenType>,
        Option<RawTokenType>,
        Option<RawTokenType>,
    ) {
        (
            self.get_prev_token_type(),
            self.get_current_token_type(),
            self.get_next_token_type(),
        )
    }
    fn get_current_keyword_kind(&self) -> Option<KeywordKind> {
        self.get_current_token_type()
            .and_then(|token_type| match token_type {
                TT::IdentifierOrKeyword(kind) | TT::Keyword(kind) => Some(kind),
                _ => None,
            })
    }
    fn consolidate_current_ident(&mut self) {
        if let Some(token) = self
            .get_current_token_index()
            .and_then(|token_index| self.tokens.get_mut(token_index))
        {
            if let TT::IdentifierOrKeyword(_) = token.get_token_type() {
                token.set_token_type(TT::Identifier);
            }
        }
    }
    fn consolidate_current_keyword(&mut self) {
        if let Some(token) = self
            .get_current_token_index()
            .and_then(|token_index| self.tokens.get_mut(token_index))
        {
            if let TT::IdentifierOrKeyword(keyword_kind) = token.get_token_type() {
                token.set_token_type(TT::Keyword(keyword_kind));
            }
        }
    }
    fn get_next_token(&self) -> Option<&RawToken> {
        self.pass_indices
            .iter()
            .skip(self.pass_index + 1)
            .find_map(|&index| self.tokens.get(index).filter(token_type_filter))
    }
    fn is_directive_before_next_token(&self) -> bool {
        let mut last_index = self.pass_index;
        for &index in self.pass_indices.iter().skip(self.pass_index + 1) {
            if index - last_index > 1 {
                return true;
            }
            if self.tokens.get(index).filter(token_type_filter).is_some() {
                return false;
            }
            last_index = index;
        }
        false
    }
    fn get_next_token_type(&self) -> Option<RawTokenType> {
        self.get_next_token().map(RawToken::get_token_type)
    }

    fn get_last_context(&self) -> Option<&ParserContext> {
        self.context.last()
    }
    fn get_last_context_type(&self) -> Option<ContextType> {
        self.context
            .last()
            .map(|&ParserContext { context_type, .. }| context_type)
    }
    fn get_context_level(&self) -> u16 {
        /*
            The approach of widening and then narrowing is used to ensure there is no
            intermediary overflow while summing the level deltas.
        */
        self.context
            .iter()
            .map(|&ParserContext { level_delta, .. }| level_delta as i64)
            .sum::<i64>()
            .clamp(u16::MIN as i64, u16::MAX as i64) as u16
    }
    fn get_line_parent_of_prev_token(&self) -> LineParent {
        let global_token_index =
            if let Some(rev_skip) = self.pass_indices.len().checked_sub(self.pass_index) {
                *self
                    .pass_indices
                    .iter()
                    .rev()
                    .skip(rev_skip)
                    .find(|&&index| self.tokens.get(index).filter(token_type_filter).is_some())
                    .unwrap_or(&0)
            } else {
                self.pass_indices[self.pass_index.saturating_sub(1)]
            };
        LineParent {
            line_index: *self.current_line.last(),
            global_token_index,
        }
    }
}
enum OpResult {
    Break,
    Continue,
}
fn token_type_filter(token: &&RawToken) -> bool {
    !matches!(
        token.get_token_type(),
        TT::Comment(_) | TT::CompilerDirective | TT::Eof
    )
}

// Parser predicates

fn after_semicolon() -> impl Fn(&LLP) -> bool {
    |parser| {
        matches!(parser.get_prev_token_type(), Some(TT::Op(OK::Semicolon)))
            && !matches!(parser.get_current_token_type(), Some(TT::Op(OK::Semicolon)))
    }
}
fn after_rparen() -> impl Fn(&LLP) -> bool {
    |parser| matches!(parser.get_prev_token_type(), Some(TT::Op(OK::RParen)))
}

fn outside_parens(initial_level: u32) -> impl Fn(&LLP) -> bool {
    move |parser| parser.paren_level <= initial_level
}
fn outside_bracks(initial_level: u32) -> impl Fn(&LLP) -> bool {
    move |parser| parser.brack_level <= initial_level
}

fn no_more_separators() -> impl Fn(&LLP) -> bool {
    |parser| !matches!(parser.get_current_token_type(), Some(TT::Op(OK::Semicolon)))
}
fn context_over() -> impl Fn(&LLP) -> bool {
    |parser| {
        parser
            .get_last_context()
            .map(|context| (context.context_ending_predicate)(parser))
            .unwrap_or(false)
    }
}

fn predicate_and(a: impl Fn(&LLP) -> bool, b: impl Fn(&LLP) -> bool) -> impl Fn(&LLP) -> bool {
    move |parser| a(parser) && b(parser)
}
fn predicate_or(a: impl Fn(&LLP) -> bool, b: impl Fn(&LLP) -> bool) -> impl Fn(&LLP) -> bool {
    move |parser| a(parser) || b(parser)
}

type ContextEndingPredicate = fn(&LLP) -> bool;
fn never_ending(_parser: &LLP) -> bool {
    false
}

fn section_headings(parser: &LLP) -> bool {
    match parser.get_current_token_type() {
        Some(TT::Keyword(KK::Implementation | KK::Initialization | KK::Finalization)) => true,
        Some(TT::Keyword(KK::Interface)) => {
            !matches!(parser.get_prev_token_type(), Some(TT::Op(OK::Equal)))
        }
        _ => false,
    }
}

fn visibility_specifier(parser: &LLP) -> bool {
    matches!(
        parser.get_current_keyword_kind(),
        Some(KK::Private | KK::Protected | KK::Public | KK::Published | KK::Automated | KK::Strict)
    )
}

fn visibility_block_ending(parser: &LLP) -> bool {
    predicate_or(visibility_specifier, end)(parser)
}

fn begin_asm(parser: &LLP) -> bool {
    matches!(
        parser.get_current_token_type(),
        Some(TT::Keyword(KK::Begin | KK::Asm))
    )
}
fn else_end(parser: &LLP) -> bool {
    matches!(
        parser.get_current_token_type(),
        Some(TT::Keyword(KK::End | KK::Else))
    )
}
fn semicolon_else_or_parent(parser: &LLP) -> bool {
    matches!(parser.get_current_token_type(), Some(TT::Keyword(KK::Else)))
        || matches!(parser.get_prev_token_type(), Some(TT::Op(OK::Semicolon)))
        || parser
            .context
            .iter()
            .rev()
            .skip_while(|context| context.context_type != ContextType::InlineStatement)
            .find(|context| context.context_type != ContextType::InlineStatement)
            .map(|context| (context.context_ending_predicate)(parser))
            .unwrap_or(false)
}
fn end(parser: &LLP) -> bool {
    matches!(parser.get_current_token_type(), Some(TT::Keyword(KK::End)))
}
fn until(parser: &LLP) -> bool {
    matches!(
        parser.get_current_token_type(),
        Some(TT::Keyword(KK::Until))
    )
}

fn except_finally(parser: &LLP) -> bool {
    matches!(
        parser.get_current_token_type(),
        Some(TT::Keyword(KK::Except | KK::Finally))
    )
}

fn declaration_section(parser: &LLP) -> bool {
    match (
        parser.get_prev_token_type(),
        parser.get_current_token_type(),
    ) {
        /*
            Other similar constructs such as `var TFoo = type Foo` is handled by
            consuming the `type` after consuming the `=`. `class` is not given
            the same treatment due to the rest of the struct type parsing.
        */
        (Some(TT::Op(OK::Equal) | TT::Keyword(KK::Packed)), Some(TT::Keyword(KK::Class))) => false,
        (
            _,
            Some(
                TT::Keyword(
                    KK::Label
                    | KK::Const
                    | KK::Type
                    | KK::Var
                    | KK::Exports
                    | KK::ThreadVar
                    | KK::Begin
                    | KK::Asm
                    | KK::Strict
                    | KK::Private
                    | KK::Protected
                    | KK::Public
                    | KK::Published
                    | KK::Automated
                    | KK::Class
                    | KK::Property
                    | KK::Function
                    | KK::Procedure
                    | KK::Constructor
                    | KK::Destructor
                    | KK::End
                    | KK::Implementation
                    | KK::Initialization
                    | KK::Finalization,
                )
                | TT::IdentifierOrKeyword(
                    KK::Strict
                    | KK::Private
                    | KK::Protected
                    | KK::Public
                    | KK::Published
                    | KK::Automated,
                ),
            ),
        ) => true,
        _ => false,
    }
}

fn parse_exports(parser: &mut LLP) {
    match parser.get_current_token_type() {
        Some(TT::Op(OK::Comma)) => {
            parser.next_token(); // ,
            parser.parse_expression(); // Identifier
        }
        Some(TT::Op(OK::LParen)) => parser.skip_pair(),
        Some(TT::IdentifierOrKeyword(KK::Name | KK::Index) | TT::Keyword(KK::Name | KK::Index)) => {
            parser.consolidate_current_keyword();
            parser.next_token(); // Name/Index
            parser.parse_expression(); // Value
        }
        Some(TT::IdentifierOrKeyword(KK::Resident) | TT::Keyword(KK::Resident)) => {
            parser.consolidate_current_keyword();
            parser.next_token(); // Resident
        }
        _ => parser.next_token(),
    }
}

fn local_declaration_section(parser: &LLP) -> bool {
    matches!(
        parser.get_current_token_type(),
        Some(TT::Keyword(
            KK::Label
                | KK::Const
                | KK::Type
                | KK::Var
                | KK::Exports
                | KK::Begin
                | KK::Asm
                | KK::End
                | KK::Function
                | KK::Procedure
        ))
    )
}

// Parser consolidators

const PORTABILITY_DIRECTIVES: [KeywordKind; 4] =
    [KK::Deprecated, KK::Experimental, KK::Platform, KK::Library];
fn keyword_consolidator(keyword_predicate: impl Fn(KeywordKind) -> bool) -> impl Fn(&mut LLP) {
    move |parser| {
        if let Some(TT::IdentifierOrKeyword(keyword_kind)) = parser.get_current_token_type() {
            if keyword_predicate(keyword_kind) {
                parser.consolidate_current_keyword()
            }
        }
        parser.next_token();
    }
}

// Parsing auxiliaries

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum ContextType {
    Unit,
    Library,
    Package,
    Program,
    Interface,
    Implementation,
    Initialization,
    Finalization,
    TypeBlock,
    VisibilityBlock,
    TypeDeclaration,
    DeclarationBlock,
    SubRoutine,
    CaseStatement,
    CompoundStatement,
    InlineStatement,
    TryBlock,
    RepeatBlock,
    ExceptBlock,
    BlockClause,
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct LocalLogicalLine {
    parent: Option<LineParent>,
    level: u16,
    tokens: Vec<usize>,
    line_type: LogicalLineType,
}
use LogicalLineType as LLT;

#[derive(Debug)]
struct ParserContext {
    context_type: ContextType,
    parent: Option<LineParent>,
    context_ending_predicate: ContextEndingPredicate,
    level_delta: i16,
}

fn is_operator(token_type: RawTokenType) -> bool {
    matches!(
        token_type,
        TT::Op(_)
            | TT::Keyword(
                KK::And
                    | KK::As
                    | KK::Div
                    | KK::In
                    | KK::Is
                    | KK::Mod
                    | KK::Not
                    | KK::Or
                    | KK::Shl
                    | KK::Shr
                    | KK::Xor,
            )
    )
}

// Pass construction

#[derive(Debug, Clone)]
struct ConditionalBranchLevel {
    branch: usize,
    last: bool,
}
#[derive(Debug, Clone, Default)]
struct ConditionalBranch {
    levels: Vec<ConditionalBranchLevel>,
}
impl ConditionalBranch {
    pub fn includes(&self, branch: &ConditionalBranch) -> bool {
        self.levels
            .iter()
            .zip(&branch.levels)
            .all(|(a, b)| a.branch == b.branch || a.branch > b.branch && b.last)
    }
}

/*
    This function takes a collection of `ConditionalBranch` and returns a Vec of
    the index of the last branch at each level of conditional directive nesting.
*/
fn get_directive_level_last_indices(conditional_branches: &[ConditionalBranch]) -> Vec<usize> {
    conditional_branches
        .iter()
        .fold(vec![0], |mut result, branch| {
            for (index, &ConditionalBranchLevel { branch, .. }) in branch.levels.iter().enumerate()
            {
                if let Some(branch_count) = result.get_mut(index) {
                    *branch_count = (*branch_count).max(branch);
                } else {
                    result.push(branch);
                }
            }
            result
        })
}

fn get_all_conditional_branch_paths(
    branches: &[ConditionalBranch],
) -> impl IntoIterator<Item = ConditionalBranch> {
    get_pass_directive_indices(&get_directive_level_last_indices(branches))
        .into_iter()
        .map(|ids| ConditionalBranch {
            levels: ids
                .into_iter()
                .map(|id| ConditionalBranchLevel {
                    branch: id,
                    last: false,
                })
                .collect_vec(),
        })
}

fn get_conditional_branches_per_directive(tokens: &[RawToken]) -> Vec<ConditionalBranch> {
    let mut counters = ConditionalBranch { levels: vec![] };
    let mut levels: Vec<ConditionalBranch> = vec![];

    for cdk in tokens.iter().map(RawToken::get_token_type).filter_map(|t| {
        if let TT::ConditionalDirective(cdk) = t {
            Some(cdk)
        } else {
            None
        }
    }) {
        match cdk {
            CDK::If | CDK::Ifdef | CDK::Ifndef | CDK::Ifopt => {
                counters.levels.push(ConditionalBranchLevel {
                    branch: 0,
                    last: false,
                });
            }
            CDK::Else | CDK::Elseif => {
                if let Some(c) = counters.levels.last_mut() {
                    c.branch += 1
                };
            }
            CDK::Endif | CDK::Ifend => {
                if let Some(c) = counters.levels.last() {
                    for dir in levels.iter_mut().rev() {
                        if let Some(c2) = dir.levels.get_mut(counters.levels.len() - 1) {
                            if c2.branch == c.branch {
                                c2.last = true;
                            } else {
                                break;
                            }
                        }
                    }
                }
                counters.levels.pop();
            }
        }
        levels.push(counters.clone());
    }

    levels
}

fn get_pass_tokens(
    tokens: &[RawToken],
    conditional_branch: &ConditionalBranch,
    conditional_branches: &[ConditionalBranch],
    pass_tokens: &mut Vec<usize>,
) {
    pass_tokens.clear();
    let mut current_branch;
    let mut branch_included = true;
    let mut conditional_index = 0;
    for (index, token_type) in tokens.iter().map(RawToken::get_token_type).enumerate() {
        match token_type {
            TT::ConditionalDirective(_) => {
                current_branch = &conditional_branches[conditional_index];
                branch_included = conditional_branch.includes(current_branch);
                conditional_index += 1;
            }
            _ if branch_included => pass_tokens.push(index),
            _ => {}
        }
    }
}

fn get_pass_directive_indices(last_branches: &[usize]) -> impl IntoIterator<Item = Vec<usize>> {
    last_branches
        .iter()
        .map(|&index| 0..=index)
        .multi_cartesian_product()
}
// Utility types

struct NonEmptyVec<T> {
    head: T,
    tail: Vec<T>,
}
impl<T> NonEmptyVec<T> {
    fn new(head: T) -> Self {
        Self { head, tail: vec![] }
    }
    fn pop(&mut self) -> Option<T> {
        let mut t = self.tail.pop()?;
        std::mem::swap(&mut t, &mut self.head);
        Some(t)
    }
    fn push(&mut self, t: T) {
        let mut t = t;
        std::mem::swap(&mut t, &mut self.head);
        self.tail.push(t);
    }
    fn last(&self) -> &T {
        &self.head
    }
    fn last_mut(&mut self) -> &mut T {
        &mut self.head
    }
    fn len(&self) -> usize {
        self.tail.len() + 1
    }
    fn get(&self, index: usize) -> Option<&T> {
        if index == self.tail.len() {
            Some(&self.head)
        } else {
            self.tail.get(index)
        }
    }
    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index == self.tail.len() {
            Some(&mut self.head)
        } else {
            self.tail.get_mut(index)
        }
    }
}
impl<T> From<NonEmptyVec<T>> for Vec<T> {
    fn from(value: NonEmptyVec<T>) -> Self {
        let mut result = value.tail;
        result.push(value.head);
        result
    }
}

// Tests

#[cfg(test)]
mod utility_tests;
