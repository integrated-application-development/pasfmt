use std::marker::PhantomData;

use crate::lang::*;
use crate::traits::*;

pub struct Formatter {
    lexer: Box<dyn Lexer + Sync>,
    token_consolidators: Vec<Box<dyn TokenConsolidator + Sync>>,
    logical_line_parser: Box<dyn LogicalLineParser + Sync>,
    logical_line_consolidators: Vec<Box<dyn LogicalLinesConsolidator + Sync>>,
    logical_line_formatters: Vec<FormatterKind>,
    reconstructor: Box<dyn LogicalLinesReconstructor + Sync>,
}
impl Formatter {
    pub fn builder() -> FormatterBuilder<BeforeLexer> {
        FormatterBuilder::default()
    }
    pub fn format(&self, input: &str) -> String {
        let mut tokens = self.lexer.lex(input);
        for token_consolidator in self.token_consolidators.iter() {
            token_consolidator.consolidate(&mut tokens);
        }
        let mut lines = self.logical_line_parser.parse(&tokens);
        for line_consolidator in self.logical_line_consolidators.iter() {
            line_consolidator.consolidate((&mut tokens, &mut lines));
        }
        let mut formatted_tokens = FormattedTokens::new_from_tokens(&tokens);
        for formatter in self.logical_line_formatters.iter() {
            formatter.format(&mut formatted_tokens, &lines);
        }
        self.reconstructor.reconstruct(formatted_tokens)
    }
}

macro_rules! builder_state {
    ($name: ident $(: [ $($trait_name: ident),* ] )?) => {
        #[derive(Default)]
        pub struct $name;
        $(
            $(
                impl $trait_name for $name {}
            )*
        )?
    };
}

trait WithParserMarker {}
trait WithFormattersMarker {}
trait WithReconstructorMarker {}

builder_state!(BeforeLexer);
builder_state!(BeforeTokenConsolidators: [WithParserMarker]);
builder_state!(BeforeParsers: [WithParserMarker]);
builder_state!(BeforeLineConsolidators: [WithFormattersMarker, WithReconstructorMarker]);
builder_state!(BeforeFormatters: [WithFormattersMarker, WithReconstructorMarker]);
builder_state!(BeforeReconstructor: [WithReconstructorMarker]);
builder_state!(BeforeBuild);

trait FormattingBuilderData: Sized {
    fn set_lexer<L: Lexer + Sync + 'static>(self, lexer: L) -> Self;
    fn add_token_consolidator<T: TokenConsolidator + Sync + 'static>(
        self,
        token_consolidator: T,
    ) -> Self;
    fn set_line_parser<T: LogicalLineParser + Sync + 'static>(self, line_parser: T) -> Self;
    fn add_lines_consolidator<T: LogicalLinesConsolidator + Sync + 'static>(
        self,
        lines_consolidator: T,
    ) -> Self;
    fn add_formatter(self, logical_line_formatter: FormatterKind) -> Self;
    fn set_reconstructor<T: LogicalLinesReconstructor + Sync + 'static>(
        self,
        reconstructor: T,
    ) -> Self;

    fn convert_type<U>(self) -> FormatterBuilder<U>;
}

pub trait WithLexer {
    fn lexer<T: Lexer + Sync + 'static>(
        self,
        lexer: T,
    ) -> FormatterBuilder<BeforeTokenConsolidators>;
}
pub trait WithTokenConsolidator {
    fn token_consolidator<T: TokenConsolidator + Sync + 'static>(
        self,
        token_consolidator: T,
    ) -> FormatterBuilder<BeforeTokenConsolidators>;
}
pub trait WithParser {
    fn parser<T: LogicalLineParser + Sync + 'static>(
        self,
        parser: T,
    ) -> FormatterBuilder<BeforeLineConsolidators>;
}
pub trait WithLineConsolidator {
    fn lines_consolidator<T: LogicalLinesConsolidator + Sync + 'static>(
        self,
        lines_consolidator: T,
    ) -> FormatterBuilder<BeforeLineConsolidators>;
}
pub trait WithFormatter {
    fn line_formatter<T: LogicalLineFormatter + Sync + 'static>(
        self,
        formatter: T,
    ) -> FormatterBuilder<BeforeFormatters>;
    fn file_formatter<T: LogicalLineFileFormatter + Sync + 'static>(
        self,
        formatter: T,
    ) -> FormatterBuilder<BeforeFormatters>;
}
pub trait WithReconstructor {
    fn reconstructor<T: LogicalLinesReconstructor + Sync + 'static>(
        self,
        reconstructor: T,
    ) -> FormatterBuilder<BeforeBuild>;
}
pub trait BuildFormatter {
    fn build(self) -> Formatter;
}

#[derive(Default)]
pub struct FormatterBuilder<T> {
    lexer: Option<Box<dyn Lexer + Sync>>,
    token_consolidators: Vec<Box<dyn TokenConsolidator + Sync + 'static>>,
    logical_line_parser: Option<Box<dyn LogicalLineParser + Sync + 'static>>,
    logical_line_consolidators: Vec<Box<dyn LogicalLinesConsolidator + Sync + 'static>>,
    logical_line_formatters: Vec<FormatterKind>,
    reconstructor: Option<Box<dyn LogicalLinesReconstructor + Sync + 'static>>,
    builder_state: PhantomData<T>,
}
impl<T> FormatterBuilder<T> {
    fn convert_type<U>(self) -> FormatterBuilder<U> {
        FormatterBuilder::<U> {
            lexer: self.lexer,
            token_consolidators: self.token_consolidators,
            logical_line_parser: self.logical_line_parser,
            logical_line_consolidators: self.logical_line_consolidators,
            logical_line_formatters: self.logical_line_formatters,
            reconstructor: self.reconstructor,
            builder_state: PhantomData,
        }
    }
}
impl<T> FormattingBuilderData for FormatterBuilder<T> {
    fn set_lexer<L: Lexer + Sync + 'static>(mut self, lexer: L) -> Self {
        self.lexer = Some(Box::new(lexer));
        self
    }
    fn add_token_consolidator<C: TokenConsolidator + Sync + 'static>(
        mut self,
        token_consolidator: C,
    ) -> Self {
        self.token_consolidators.push(Box::new(token_consolidator));
        self
    }
    fn set_line_parser<P: LogicalLineParser + Sync + 'static>(mut self, line_parser: P) -> Self {
        self.logical_line_parser = Some(Box::new(line_parser));
        self
    }
    fn add_lines_consolidator<C: LogicalLinesConsolidator + Sync + 'static>(
        mut self,
        lines_consolidator: C,
    ) -> Self {
        self.logical_line_consolidators
            .push(Box::new(lines_consolidator));
        self
    }
    fn add_formatter(mut self, logical_line_formatter: FormatterKind) -> Self {
        self.logical_line_formatters.push(logical_line_formatter);
        self
    }
    fn set_reconstructor<R: LogicalLinesReconstructor + Sync + 'static>(
        mut self,
        reconstructor: R,
    ) -> Self {
        self.reconstructor = Some(Box::new(reconstructor));
        self
    }

    fn convert_type<U>(self) -> FormatterBuilder<U> {
        FormatterBuilder::<U> {
            lexer: self.lexer,
            token_consolidators: self.token_consolidators,
            logical_line_parser: self.logical_line_parser,
            logical_line_consolidators: self.logical_line_consolidators,
            logical_line_formatters: self.logical_line_formatters,
            reconstructor: self.reconstructor,
            builder_state: PhantomData,
        }
    }
}

impl WithLexer for FormatterBuilder<BeforeLexer> {
    fn lexer<T: Lexer + Sync + 'static>(
        self,
        lexer: T,
    ) -> FormatterBuilder<BeforeTokenConsolidators> {
        self.set_lexer(lexer).convert_type()
    }
}
impl WithTokenConsolidator for FormatterBuilder<BeforeTokenConsolidators> {
    fn token_consolidator<T: TokenConsolidator + Sync + 'static>(
        self,
        token_consolidator: T,
    ) -> FormatterBuilder<BeforeTokenConsolidators> {
        self.add_token_consolidator(token_consolidator)
            .convert_type()
    }
}
impl<U: WithParserMarker> WithParser for FormatterBuilder<U> {
    fn parser<T: LogicalLineParser + Sync + 'static>(
        self,
        parser: T,
    ) -> FormatterBuilder<BeforeLineConsolidators> {
        self.set_line_parser(parser).convert_type()
    }
}
impl WithLineConsolidator for FormatterBuilder<BeforeLineConsolidators> {
    fn lines_consolidator<T: LogicalLinesConsolidator + Sync + 'static>(
        self,
        lines_consolidator: T,
    ) -> FormatterBuilder<BeforeLineConsolidators> {
        self.add_lines_consolidator(lines_consolidator)
            .convert_type()
    }
}
impl<U: WithFormattersMarker> WithFormatter for FormatterBuilder<U> {
    fn line_formatter<T: LogicalLineFormatter + Sync + 'static>(
        self,
        formatter: T,
    ) -> FormatterBuilder<BeforeFormatters> {
        self.add_formatter(FormatterKind::LineFormatter(Box::new(formatter)))
            .convert_type()
    }

    fn file_formatter<T: LogicalLineFileFormatter + Sync + 'static>(
        self,
        formatter: T,
    ) -> FormatterBuilder<BeforeFormatters> {
        self.add_formatter(FormatterKind::FileFormatter(Box::new(formatter)))
            .convert_type()
    }
}
impl<U: WithReconstructorMarker> WithReconstructor for FormatterBuilder<U> {
    fn reconstructor<T: LogicalLinesReconstructor + Sync + 'static>(
        self,
        reconstructor: T,
    ) -> FormatterBuilder<BeforeBuild> {
        self.set_reconstructor(reconstructor).convert_type()
    }
}
impl BuildFormatter for FormatterBuilder<BeforeBuild> {
    fn build(self) -> Formatter {
        Formatter {
            lexer: self.lexer.unwrap(),
            token_consolidators: self.token_consolidators,
            logical_line_parser: self.logical_line_parser.unwrap(),
            logical_line_consolidators: self.logical_line_consolidators,
            logical_line_formatters: self.logical_line_formatters,
            reconstructor: self.reconstructor.unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use indoc::indoc;
    use itertools::Itertools;
    use spectral::prelude::*;

    fn run_test(formatter: Formatter, input: &str, expected_output: &str) {
        let output = formatter.format(input);
        assert_that(&output).is_equal_to(expected_output.to_string());
    }

    struct MakeMultiplySignIdentifier;
    impl TokenConsolidator for MakeMultiplySignIdentifier {
        fn consolidate(&self, tokens: &mut [Token]) {
            for token in tokens.iter_mut() {
                if token.get_token_type() == TokenType::Op(OperatorKind::Star) {
                    token.set_token_type(TokenType::Identifier);
                }
            }
        }
    }

    struct Append1ToAllIdentifiers;
    impl TokenConsolidator for Append1ToAllIdentifiers {
        fn consolidate(&self, tokens: &mut [Token]) {
            (0..tokens.len() - 1).for_each(|token_index| {
                if let Some(token) = tokens.get_mut(token_index) {
                    if token.get_token_type() != TokenType::Identifier {
                        return;
                    }
                    *token = Token::OwningToken(OwningToken::new(
                        token.get_index(),
                        token.get_leading_whitespace().to_owned(),
                        token.get_content().to_owned() + "1",
                        token.get_token_type(),
                    ));
                }
            });
        }
    }

    struct AddSpaceBeforeIdentifier;
    impl LogicalLineFormatter for AddSpaceBeforeIdentifier {
        fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &LogicalLine) {
            for &token in input.get_tokens() {
                if formatted_tokens.get_token_type_for_index(token) == Some(TokenType::Identifier) {
                    if let Some(formatting_data) = formatted_tokens.get_formatting_data_mut(token) {
                        formatting_data.spaces_before = 1;
                    }
                }
            }
        }
    }

    #[test]
    fn no_optional_stages() {
        let formatter = Formatter::builder()
            .lexer(DelphiLexer {})
            .parser(DelphiLogicalLineParser {})
            .reconstructor(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_owned(), "  ".to_owned(), "  ".to_owned()),
            ))
            .build();
        run_test(formatter, "a 1 b 2 c 3", "a 1 b 2 c 3");
    }

    #[test]
    fn single_token_consolidator() {
        let formatter = Formatter::builder()
            .lexer(DelphiLexer {})
            .token_consolidator(MakeMultiplySignIdentifier {})
            .parser(DelphiLogicalLineParser {})
            .line_formatter(AddSpaceBeforeIdentifier {})
            .reconstructor(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_owned(), "  ".to_owned(), "  ".to_owned()),
            ))
            .build();
        run_test(formatter, "1*1*1", "1 *1 *1");
    }

    #[test]
    fn multiple_token_consolidators() {
        let formatter = Formatter::builder()
            .lexer(DelphiLexer {})
            .token_consolidator(MakeMultiplySignIdentifier {})
            .token_consolidator(Append1ToAllIdentifiers {})
            .parser(DelphiLogicalLineParser {})
            .line_formatter(AddSpaceBeforeIdentifier {})
            .reconstructor(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_owned(), "  ".to_owned(), "  ".to_owned()),
            ))
            .build();
        run_test(formatter, "1*1*1", "1 *11 *11");
    }

    struct CombineFirst2Lines;
    impl LogicalLinesConsolidator for CombineFirst2Lines {
        fn consolidate(&self, (_, lines): (&mut [Token<'_>], &mut [LogicalLine])) {
            let (second_non_void_line_index, _) = match lines
                .iter()
                .enumerate()
                .filter(|(_, line)| line.get_line_type() != LogicalLineType::Voided)
                .take(2)
                .last()
            {
                Some(line) => line,
                _ => return,
            };

            let second_line_tokens = match lines.get_mut(second_non_void_line_index) {
                Some(line) => line.void_and_drain().collect_vec(),
                _ => return,
            };

            lines
                .get_mut(0)
                .unwrap()
                .get_tokens_mut()
                .extend(second_line_tokens);
        }
    }

    struct LogicalLinesOnNewLines;
    impl LogicalLineFormatter for LogicalLinesOnNewLines {
        fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &LogicalLine) {
            if let Some(&first_token) = input.get_tokens().first() {
                if first_token != 0 && first_token != formatted_tokens.get_tokens().len() - 1 {
                    if let Some(formatting_data) =
                        formatted_tokens.get_formatting_data_mut(first_token)
                    {
                        formatting_data.spaces_before = 0;
                        formatting_data.newlines_before = 1;
                    }
                }
            }
        }
    }

    #[test]
    fn single_line_consolidator() {
        let formatter = Formatter::builder()
            .lexer(DelphiLexer {})
            .token_consolidator(MakeMultiplySignIdentifier {})
            .parser(DelphiLogicalLineParser {})
            .lines_consolidator(CombineFirst2Lines {})
            .line_formatter(LogicalLinesOnNewLines {})
            .reconstructor(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_owned(), "  ".to_owned(), "  ".to_owned()),
            ))
            .build();
        run_test(
            formatter,
            "a; b; c; d;",
            indoc! {"
                a; b;
                c;
                d;"
            },
        );
    }

    #[test]
    fn multiple_line_consolidators() {
        let formatter = Formatter::builder()
            .lexer(DelphiLexer {})
            .parser(DelphiLogicalLineParser {})
            .lines_consolidator(CombineFirst2Lines {})
            .lines_consolidator(CombineFirst2Lines {})
            .line_formatter(LogicalLinesOnNewLines {})
            .reconstructor(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_owned(), "  ".to_owned(), "  ".to_owned()),
            ))
            .build();
        run_test(
            formatter,
            "a; b; c; d;",
            indoc! {"
                a; b; c;
                d;"
            },
        );
    }

    struct SpaceBeforeSemiColon;
    impl LogicalLineFormatter for SpaceBeforeSemiColon {
        fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &LogicalLine) {
            let semicolon_indices: Vec<_> = input
                .get_tokens()
                .iter()
                .filter(|&&token_index| {
                    formatted_tokens.get_token_type_for_index(token_index)
                        == Some(TokenType::Op(OperatorKind::Semicolon))
                })
                .collect();

            semicolon_indices.iter().for_each(|&&semicolon_index| {
                if let Some(semicolon_formatting_data) =
                    formatted_tokens.get_formatting_data_mut(semicolon_index)
                {
                    semicolon_formatting_data.spaces_before = 1;
                }
            });
        }
    }

    #[test]
    fn single_line_formatter() {
        let formatter = Formatter::builder()
            .lexer(DelphiLexer {})
            .parser(DelphiLogicalLineParser {})
            .line_formatter(LogicalLinesOnNewLines {})
            .reconstructor(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_owned(), "  ".to_owned(), "  ".to_owned()),
            ))
            .build();
        run_test(
            formatter,
            "a; b; c; d;",
            indoc! {"
                a;
                b;
                c;
                d;"
            },
        );
    }
    #[test]
    fn multiple_line_formatters() {
        let formatter = Formatter::builder()
            .lexer(DelphiLexer {})
            .token_consolidator(MakeMultiplySignIdentifier {})
            .parser(DelphiLogicalLineParser {})
            .line_formatter(LogicalLinesOnNewLines {})
            .line_formatter(SpaceBeforeSemiColon {})
            .reconstructor(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_owned(), "  ".to_owned(), "  ".to_owned()),
            ))
            .build();
        run_test(
            formatter,
            "a; b; c; d;",
            indoc! {"
                a ;
                b ;
                c ;
                d ;"
            },
        );
    }

    struct IndentBasedOnLineNumber;
    impl LogicalLineFileFormatter for IndentBasedOnLineNumber {
        fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &[LogicalLine]) {
            for (index, line) in input
                .iter()
                .enumerate()
                .filter(|(_, line)| !matches!(line.get_line_type(), LogicalLineType::Eof))
            {
                let &first_line_token = line.get_tokens().first().unwrap();
                let formatting_data = formatted_tokens
                    .get_formatting_data_mut(first_line_token)
                    .unwrap();
                formatting_data.spaces_before = 0;
                formatting_data.indentations_before = index;
            }
        }
    }

    #[test]
    fn single_file_formatter() {
        let formatter = Formatter::builder()
            .lexer(DelphiLexer {})
            .parser(DelphiLogicalLineParser {})
            .file_formatter(IndentBasedOnLineNumber {})
            .reconstructor(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_owned(), " ".to_owned(), " ".to_owned()),
            ))
            .build();
        run_test(
            formatter,
            "a; b; c; d;",
            indoc! {"
                a; b;  c;   d;"
            },
        );
    }

    struct IndentSecondLine3SpacesIfNoNewLine;
    impl LogicalLineFileFormatter for IndentSecondLine3SpacesIfNoNewLine {
        fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &[LogicalLine]) {
            let &first_line_token = input[1].get_tokens().first().unwrap();
            let formatting_data = formatted_tokens
                .get_formatting_data_mut(first_line_token)
                .unwrap();
            if formatting_data.newlines_before == 0 {
                formatting_data.spaces_before = 3;
            }
            formatting_data.indentations_before = 0;
        }
    }

    #[test]
    fn multiple_file_formatter() {
        let formatter = Formatter::builder()
            .lexer(DelphiLexer {})
            .token_consolidator(MakeMultiplySignIdentifier {})
            .parser(DelphiLogicalLineParser {})
            .file_formatter(IndentBasedOnLineNumber {})
            .file_formatter(IndentSecondLine3SpacesIfNoNewLine {})
            .reconstructor(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_owned(), " ".to_owned(), " ".to_owned()),
            ))
            .build();
        run_test(
            formatter,
            "a; b; c; d;",
            indoc! {"
                a;   b;  c;   d;"
            },
        );
    }

    struct RetainSpacesLogcialLinesOnNewLines;
    impl LogicalLineFormatter for RetainSpacesLogcialLinesOnNewLines {
        fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &LogicalLine) {
            let first_token = *input.get_tokens().first().unwrap();
            if first_token != 0 && first_token != formatted_tokens.get_tokens().len() - 1 {
                if let Some(formatting_data) = formatted_tokens.get_formatting_data_mut(first_token)
                {
                    formatting_data.newlines_before = 1;
                }
            }
        }
    }

    #[test]
    fn line_then_file_formatter() {
        let formatter = Formatter::builder()
            .lexer(DelphiLexer {})
            .parser(DelphiLogicalLineParser {})
            .line_formatter(RetainSpacesLogcialLinesOnNewLines {})
            .file_formatter(IndentSecondLine3SpacesIfNoNewLine {})
            .reconstructor(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_owned(), "  ".to_owned(), "  ".to_owned()),
            ))
            .build();
        run_test(
            formatter,
            "a;b;c;d;",
            indoc! {"
                a;
                b;
                c;
                d;"
            },
        );
    }
    #[test]
    fn file_then_line_formatter() {
        let formatter = Formatter::builder()
            .lexer(DelphiLexer {})
            .parser(DelphiLogicalLineParser {})
            .file_formatter(IndentSecondLine3SpacesIfNoNewLine {})
            .line_formatter(RetainSpacesLogcialLinesOnNewLines {})
            .reconstructor(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_owned(), " ".to_owned(), " ".to_owned()),
            ))
            .build();
        run_test(
            formatter,
            "a;b;c;d;",
            indoc! {"
                a;
                   b;
                c;
                d;"
            },
        );
    }
}
