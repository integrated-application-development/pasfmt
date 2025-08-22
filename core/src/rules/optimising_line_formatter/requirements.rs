use super::contexts::*;
use super::get_operator_precedence;
use super::is_binary;
use super::types::DecisionRequirement;
use super::InternalOptimisingLineFormatter;
use super::SpecificContextDataStack;
use crate::lang::*;

use super::contexts::ContextType as CT;
use super::types::DecisionRequirement as DR;
use crate::lang::ChevronKind as ChK;
use crate::lang::KeywordKind as KK;
use crate::lang::LogicalLineType as LLT;
use crate::lang::OperatorKind as OK;
use crate::lang::TokenType as TT;

impl InternalOptimisingLineFormatter<'_, '_> {
    pub(super) fn get_formatting_requirement(
        &self,
        line_index: u32,
        line: &LogicalLine,
        contexts_data: &SpecificContextDataStack,
    ) -> DecisionRequirement {
        let Some((last_context, ctx_data)) = contexts_data.iter().next() else {
            return DR::Invalid;
        };

        let parents_support_break = contexts_data.parents_support_break();

        if let Some(value) = self.get_formatting_invariant(line_index, line) {
            return value.map_can_break(parents_support_break);
        }

        let token_type_window = self.get_token_type_window(line_index, line);
        let requirement = match token_type_window {
            (Some(TT::Op(OK::LParen)), Some(TT::Op(OK::RParen))) => contexts_data
                .get_last_context(context_matches!(CT::Brackets(BracketKind::Round, _)))
                .map(|(_, data)| data.is_child_broken | data.is_broken)
                .if_else_or(DR::MustBreak, DR::MustNotBreak, DR::MustNotBreak),
            (Some(TT::Op(OK::LBrack)), Some(TT::Op(OK::RBrack))) => DR::MustNotBreak,
            (Some(TT::Op(OK::GreaterThan(ChK::Generic))), Some(TT::Op(OK::LParen))) => {
                DR::MustNotBreak
            }
            // `<>` is not a valid type parameter list, and will be interpreted as `NotEqual`
            (
                Some(TT::Keyword(KK::Class | KK::Record)),
                Some(TT::Keyword(KK::Helper | KK::Abstract | KK::Sealed)),
            ) => DR::MustNotBreak,
            (Some(TT::Keyword(KK::Helper) | TT::Op(OK::RParen)), Some(TT::Keyword(KK::For))) => {
                DR::MustNotBreak
            }
            (Some(TT::Op(OK::Caret(CaretKind::Type))), _) => DR::MustNotBreak,
            (_, Some(TT::Op(OK::Caret(CaretKind::Deref)))) => DR::MustNotBreak,
            (_, Some(TT::Keyword(kk)))
                if matches!(
                    line.get_line_type(),
                    LLT::RoutineHeader | LLT::PropertyDeclaration
                ) && kk.is_directive() =>
            {
                contexts_data
                    .get_last_context(context_matches!(CT::DirectiveList))
                    .and_then(|(_, data)| data.one_element_per_line)
                    .if_else_or_default(DR::MustBreak, DR::MustNotBreak)
            }
            (Some(TT::Op(OK::Comma)), _) => {
                let import_requirement =
                    matches!(line.get_line_type(), LLT::ImportClause | LLT::ExportClause)
                        .then_some(DR::MustBreak);

                let comma_list_requirement = contexts_data
                    .get_last_context(CT::CommaList)
                    .and_then(|(_, data)| data.one_element_per_line)
                    .if_else_map(DR::MustBreak, DR::MustNotBreak);

                let parens_requirement = contexts_data
                    .get_last_context(context_matches!(CT::Brackets(_, _) | CT::SemicolonList))
                    .and_then(|(ctx, data)| {
                        Some(data.is_broken).filter(|_| ctx.context_type() != CT::SemicolonList)
                    })
                    .if_else_map(DR::MustBreak, DR::Indifferent);

                import_requirement
                    .or(comma_list_requirement)
                    .or(parens_requirement)
                    .unwrap_or_default()
            }
            (Some(TT::Op(OK::Semicolon)), _) => {
                let semicolon_list_requirement = contexts_data
                    .get_last_context(CT::SemicolonList)
                    .and_then(|(_, data)| data.one_element_per_line)
                    .if_else_map(DR::MustBreak, DR::MustNotBreak);
                let parens_requirement = contexts_data
                    .get_last_context(context_matches!(CT::Brackets(_, _)))
                    .map(|(_, data)| data.is_broken)
                    .if_else_map(DR::MustBreak, DR::Indifferent);

                semicolon_list_requirement
                    .or(parens_requirement)
                    .unwrap_or_default()
            }
            (
                Some(TT::Keyword(KK::Class | KK::To)),
                Some(TT::Keyword(KK::Function | KK::Procedure | KK::Constructor | KK::Destructor)),
            ) => DR::MustNotBreak,
            (
                Some(TT::Keyword(KK::Function | KK::Procedure | KK::Constructor | KK::Destructor)),
                Some(TT::Identifier | TT::ConditionalDirective(_)),
            ) => DR::MustNotBreak,
            (Some(TT::ConditionalDirective(kind)), _)
            | (_, Some(TT::ConditionalDirective(kind)))
                if kind.is_if() =>
            {
                /*
                    The `if` type conditional directives effectively have no
                    bearing on the formatting.
                    This is to support these styles:
                    ```delphi
                    A({$if} A {$else} B {$endif});
                    A(
                        {$if} A {$else} B {$endif}
                    );
                    A(
                        {$if}
                        A
                        {$else}
                        B
                        {$endif}
                    );
                    ```
                */
                DR::Indifferent
            }
            (Some(TT::ConditionalDirective(kind)), _) if kind.is_else() => contexts_data
                .get_last_context(context_matches!(CT::ConditionalDirective))
                .and_then(|(_, data)| data.one_element_per_line)
                .if_else_or_default(DR::MustBreak, DR::MustNotBreak),
            (_, Some(TT::ConditionalDirective(_))) => contexts_data
                .get_last_context(context_matches!(CT::ConditionalDirective))
                .and_then(|(_, data)| data.one_element_per_line)
                .if_else_or_default(DR::MustBreak, DR::MustNotBreak),
            (
                Some(
                    TT::Identifier
                    | TT::Keyword(
                        KK::Interface
                        | KK::Class
                        | KK::Helper
                        | KK::Abstract
                        | KK::Sealed
                        | KK::Function
                        | KK::Procedure
                        | KK::Array
                        | KK::String,
                    ),
                ),
                Some(TT::Op(OK::LParen | OK::LBrack | OK::LessThan(ChK::Generic))),
            ) => DR::MustNotBreak,
            (Some(TT::Op(OK::Colon)), Some(TT::Op(OK::LParen)))
                if line.get_line_type() == LLT::VariantRecordCaseArm =>
            {
                DR::MustNotBreak
            }
            (Some(TT::Op(OK::LParen | OK::LBrack | OK::LessThan(ChK::Generic))), _) => {
                contexts_data
                    .get_last_context(context_matches!(CT::Brackets(_, _)))
                    .map(|(ctx, _)| {
                        matches!(ctx.context_type(), CT::Brackets(_, BracketStyle::Invisible))
                    })
                    .if_else_or_default(DR::MustNotBreak, DR::Indifferent)
            }
            (Some(TT::Keyword(KK::Reference)), Some(TT::Keyword(KK::To))) => DR::MustNotBreak,
            (_, Some(TT::Op(OK::RParen | OK::GreaterThan(ChK::Generic) | OK::RBrack))) => {
                contexts_data
                    .get_last_context(context_matches!(CT::Brackets(_, _)))
                    .map(|(ctx, data)| {
                        matches!(
                            ctx.context_type(),
                            CT::Brackets(_, BracketStyle::BreakClose | BracketStyle::Expanded)
                        ) && data.is_broken
                    })
                    .if_else_or_default(DR::MustBreak, DR::MustNotBreak)
            }
            (_, Some(TT::Op(OK::Comma | OK::Semicolon | OK::Colon | OK::Assign))) => {
                DR::MustNotBreak
            }
            (_, Some(TT::Op(OK::Dot)))
                if matches!(last_context.context_type(), CT::MemberAccess) =>
            {
                if let Some(CT::RoutineHeader) = contexts_data
                    .get_last_context(context_matches!(
                        CT::RoutineHeader | CT::Brackets(_, _) | CT::Type
                    ))
                    .map(|ctx| ctx.0.context_type())
                {
                    DR::MustNotBreak
                } else {
                    ctx_data
                        .one_element_per_line
                        .if_else_or_default(DR::MustBreak, DR::MustNotBreak)
                }
            }
            (_, Some(TT::Op(OK::Equal(EqKind::Decl)))) => DR::MustNotBreak,
            (_, Some(TT::Keyword(KK::In(InKind::ForLoop) | KK::To | KK::Downto))) => contexts_data
                .get_last_context(CT::ForLoop)
                .map(|(_, data)| data.is_child_broken)
                .if_else_or_default(DR::MustBreak, DR::Indifferent),
            (Some(TT::Keyword(KK::In(InKind::ForLoop) | KK::To | KK::Downto)), _)
                if line.get_line_type() == LLT::ForLoop =>
            {
                DR::MustNotBreak
            }
            (_, Some(TT::Keyword(KK::At))) => contexts_data
                .get_last_context(CT::RaiseAt)
                .map(|(_, data)| data.is_broken | data.is_child_broken)
                .if_else_or_default(DR::MustBreak, DR::Indifferent),
            (prev, Some(op @ (TT::Op(_) | TT::Keyword(_))))
                if get_operator_precedence(op).is_some() && is_binary(op, prev) =>
            {
                contexts_data
                    .iter()
                    .next()
                    .filter(|ctx| matches!(ctx.0.context_type(), CT::Precedence(_)))
                    .and_then(|(_, data)| data.one_element_per_line)
                    .if_else_or_default(DR::MustBreak, DR::MustNotBreak)
            }
            (
                Some(TT::Op(OK::Equal(EqKind::Decl))),
                Some(TT::Keyword(
                    KK::Class
                    | KK::Interface
                    | KK::Record
                    | KK::DispInterface
                    | KK::Packed
                    | KK::Object,
                )),
            ) => DR::Indifferent,
            (Some(TT::Op(OK::Equal(EqKind::Decl) | OK::Assign)), _) => contexts_data
                .get_last_context(context_matches!(CT::TypedAssignment | CT::Assignment))
                .map(|(_, data)| data.is_broken | data.is_child_broken)
                .if_else_or_default(DR::MustBreak, DR::Indifferent),
            (Some(op @ (TT::Op(_) | TT::Keyword(_))), _)
                if get_operator_precedence(op).is_some() =>
            {
                DR::MustNotBreak
            }
            (_, Some(TT::Keyword(KK::End))) => contexts_data
                .get_last_context(context_matches!(CT::CommaElem | CT::AssignRHS))
                .and_then(|(_, data)| data.break_anonymous_routine)
                .if_else_or_default(DR::MustBreak, DR::MustNotBreak),
            (Some(TT::Keyword(KK::If | KK::Case | KK::While | KK::Until | KK::On)), _) => {
                DR::MustNotBreak
            }
            /*
                `with` has special handling based on whether there is a comma
                list of elements. If so, treat it more like a `uses` clause,
                otherwise treat it more like an `if` statement.
            */
            (Some(TT::Keyword(KK::With)), _) => contexts_data
                .iter()
                .find(|(ctx, _)| matches!(ctx.context_type(), CT::CommaList | CT::GuardClause))
                .map(|(ctx, _)| ctx.context_type() == CT::CommaList)
                .if_else_or_default(DR::Indifferent, DR::MustNotBreak),
            (Some(TT::Keyword(KK::Raise | KK::At)), _) => DR::MustNotBreak,
            // For `set of` anonymous enums
            (Some(TT::Keyword(KK::Of)), Some(TT::Op(OK::LParen))) => DR::MustNotBreak,
            // For `procedure of object` to allow wrapping `object`
            (Some(TT::Keyword(KK::Of)), Some(TT::Keyword(KK::Object))) => contexts_data
                .iter()
                .find(|(ctx, _)| matches!(ctx.context_type(), CT::AssignRHS))
                .map(|(_, data)| data.is_child_broken)
                .if_else_or_default(DR::Indifferent, DR::MustNotBreak),
            (Some(TT::Keyword(KK::Of)), _) => contexts_data
                .get_last_context(context_matches!(ContextType::Base))
                .map(|(_, data)| data.is_child_broken)
                .if_else_or(DR::Indifferent, DR::MustNotBreak, DR::Indifferent),
            (_, Some(TT::Keyword(KK::Then | KK::Do | KK::Of))) => DR::MustNotBreak,
            (Some(TT::Keyword(KK::Then | KK::Do)), Some(TT::Keyword(KK::Begin))) => contexts_data
                .get_last_context(CT::ControlFlowBegin)
                .map(|(_, data)| data.is_child_broken)
                .if_else_or_default(DR::MustBreak, DR::Indifferent),
            (_, Some(TT::Keyword(KK::Else))) => DR::MustBreak,
            (
                Some(_),
                Some(TT::Keyword(
                    KK::Label
                    | KK::Const(DeclKind::AnonSection)
                    | KK::Type
                    | KK::Var(DeclKind::AnonSection)
                    | KK::ThreadVar
                    | KK::Begin,
                )),
            ) => contexts_data
                .get_last_context(context_matches!(CT::CommaElem | CT::AssignRHS))
                .and_then(|(_, data)| data.break_anonymous_routine)
                .if_else_or_default(DR::MustBreak, DR::MustNotBreak),
            (
                Some(TT::Keyword(
                    KK::Const(DeclKind::Inline | DeclKind::Param)
                    | KK::Var(DeclKind::Inline | DeclKind::Param),
                )),
                _,
            ) => DR::MustNotBreak,
            (Some(TT::Keyword(KK::Property)), Some(TT::Identifier)) => DR::MustNotBreak,
            (Some(TT::Keyword(KK::For)), _) if line.get_line_type() == LLT::ForLoop => {
                DR::MustNotBreak
            }
            _ => DR::Indifferent,
        };

        let requirement = match (token_type_window, requirement) {
            ((Some(TT::ConditionalDirective(kind)), Some(TT::Identifier)), DR::Indifferent)
                if kind.is_end() =>
            {
                contexts_data
                    .get_last_context(context_matches!(_))
                    .map(|(_, data)| data.is_child_broken)
                    .if_else_or_default(DR::MustBreak, DR::Indifferent)
            }
            ((Some(TT::ConditionalDirective(kind)), Some(_)), DR::Indifferent) if kind.is_end() => {
                DR::MustBreak
            }
            _ => requirement,
        };
        requirement.map_can_break(parents_support_break)
    }

    pub(super) fn get_formatting_invariant(
        &self,
        line_index: u32,
        line: &LogicalLine,
    ) -> Option<DecisionRequirement> {
        match (
            self.get_prev_token_type_for_line_index(line, line_index),
            self.get_token_type_for_line_index(line, line_index),
        ) {
            (None, _) => Some(DR::MustNotBreak),
            (_, Some(TT::Comment(CommentKind::InlineLine | CommentKind::InlineBlock))) => {
                Some(DR::MustNotBreak)
            }
            (
                _,
                Some(TT::Comment(
                    CommentKind::IndividualLine
                    | CommentKind::IndividualBlock
                    | CommentKind::MultilineBlock,
                )),
            ) => Some(DR::MustBreak),
            (
                Some(
                    TT::Comment(
                        CommentKind::IndividualLine
                        | CommentKind::InlineLine
                        | CommentKind::MultilineBlock,
                    )
                    | TT::TextLiteral(TextLiteralKind::Unterminated),
                ),
                _,
            ) => Some(DR::MustBreak),
            (Some(TT::ConditionalDirective(_)), _)
                if line
                    .get_tokens()
                    .get(line_index.wrapping_sub(1) as usize)
                    .copied()
                    != line
                        .get_tokens()
                        .get(line_index as usize)
                        .map(|&idx| idx.wrapping_sub(1)) =>
            {
                // If the conditional directive is not in the current line
                Some(DR::MustBreak)
            }
            _ => None,
        }
    }

    fn get_token_type_window(
        &self,
        line_index: u32,
        line: &LogicalLine,
    ) -> (Option<TokenType>, Option<TokenType>) {
        (
            (0..line_index)
                .rev()
                .flat_map(|index| self.get_token_type_for_line_index(line, index))
                .find(|token_type| !token_type.is_comment_or_compiler_directive()),
            self.get_token_type_for_line_index(line, line_index),
        )
    }
}

trait IfElse<T: Default> {
    fn if_else_or_default(self, yes: T, no: T) -> T;
    fn if_else_or(self, yes: T, no: T, el: T) -> T;
    fn if_else_map(self, yes: T, no: T) -> Option<T>;
}
impl<T: Default> IfElse<T> for Option<bool> {
    fn if_else_or_default(self, yes: T, no: T) -> T {
        self.if_else_or(yes, no, T::default())
    }
    fn if_else_or(self, yes: T, no: T, el: T) -> T {
        match self {
            None => el,
            Some(true) => yes,
            Some(false) => no,
        }
    }
    fn if_else_map(self, yes: T, no: T) -> Option<T> {
        self.map(|val| if val { yes } else { no })
    }
}
