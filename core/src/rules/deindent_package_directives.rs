use crate::prelude::*;

pub struct DeindentPackageDirectives {}

impl LogicalLinesConsolidator for DeindentPackageDirectives {
    fn consolidate(&self, (tokens, lines): (&mut [Token], &mut [LogicalLine])) {
        let Some(TokenType::Keyword(KeywordKind::Package)) =
            (tokens.iter().map(TokenData::get_token_type)).find(|tt| !tt.is_comment_or_directive())
        else {
            return;
        };

        for line in lines {
            if let LogicalLineType::CompilerDirective | LogicalLineType::ConditionalDirective =
                line.get_line_type()
            {
                // must run before the call to `void_and_drain`
                let line_type = line.get_line_type();
                let tmp_tokens = line.void_and_drain().collect();
                let mut new_line = LogicalLine::new(line.get_parent(), 0, tmp_tokens, line_type);
                std::mem::swap(line, &mut new_line);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    struct IndentAsLevel;
    impl LogicalLineFileFormatter for IndentAsLevel {
        fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, lines: &[LogicalLine]) {
            for line in lines {
                dbg!(line.get_line_type(), line.get_level());
                for (idx, &tok) in line.get_tokens().iter().enumerate() {
                    let fmt = formatted_tokens.get_formatting_data_mut(tok).unwrap();
                    if idx == 0 {
                        fmt.indentations_before = line.get_level();
                        fmt.spaces_before = 0;
                    } else {
                        fmt.indentations_before = 0;
                    }
                    fmt.continuations_before = 0;
                }
            }
        }
    }

    fn formatter() -> Formatter {
        Formatter::builder()
            .lexer(DelphiLexer {})
            .parser(DelphiLogicalLineParser {})
            .lines_consolidator(DeindentPackageDirectives {})
            .file_formatter(IndentAsLevel)
            .reconstructor(default_test_reconstructor())
            .build()
    }

    formatter_test_group!(
        deindent,
        implicit_build_section = {
            indoc! {"
                package Foo;
                {$IFDEF IMPLICITBUILDING}
                  {$ALIGN 8}
                  {$IMAGEBASE $400000}
                  {$DEFINE RELEASE}
                {$ENDIF IMPLICITBUILDING}
                {$IMPLICITBUILD ON}
                end.
            "},
            indoc! {"
                package Foo;
                {$IFDEF IMPLICITBUILDING}
                {$ALIGN 8}
                {$IMAGEBASE $400000}
                {$DEFINE RELEASE}
                {$ENDIF IMPLICITBUILDING}
                {$IMPLICITBUILD ON}
                end.
            "},
        },
        outside_implicit_build_section = {
            indoc! {"
                {$IFDEF FOO}
                  {$DEFINE OOF}
                  {$IFDEF OOF}
                    {$DEFINE FOO}
                  {$ENDIF}
                {$ENDIF}
                package Foo;
                {$IFDEF IMPLICITBUILDING}
                {$ENDIF IMPLICITBUILDING}
                end.
                {$IFDEF FOO}
                  {$DEFINE OOF}
                {$ENDIF}
            "},
            indoc! {"
                {$IFDEF FOO}
                {$DEFINE OOF}
                {$IFDEF OOF}
                {$DEFINE FOO}
                {$ENDIF}
                {$ENDIF}
                package Foo;
                {$IFDEF IMPLICITBUILDING}
                {$ENDIF IMPLICITBUILDING}
                end.
                {$IFDEF FOO}
                {$DEFINE OOF}
                {$ENDIF}
            "},
        },
        program_not_affected = {
            indoc! {"
                program Foo;
                {$IFDEF IMPLICITBUILDING}
                  {$ALIGN 8}
                  {$IMAGEBASE $400000}
                  {$DEFINE RELEASE}
                {$ENDIF IMPLICITBUILDING}
                {$IMPLICITBUILD ON}
                end.
            "},
            indoc! {"
                program Foo;
                {$IFDEF IMPLICITBUILDING}
                  {$ALIGN 8}
                  {$IMAGEBASE $400000}
                  {$DEFINE RELEASE}
                {$ENDIF IMPLICITBUILDING}
                {$IMPLICITBUILD ON}
                end.
            "},
        },
        library_not_affected = {
            indoc! {"
                library Foo;
                {$IFDEF IMPLICITBUILDING}
                  {$ALIGN 8}
                  {$IMAGEBASE $400000}
                  {$DEFINE RELEASE}
                {$ENDIF IMPLICITBUILDING}
                {$IMPLICITBUILD ON}
                end.
            "},
            indoc! {"
                library Foo;
                {$IFDEF IMPLICITBUILDING}
                  {$ALIGN 8}
                  {$IMAGEBASE $400000}
                  {$DEFINE RELEASE}
                {$ENDIF IMPLICITBUILDING}
                {$IMPLICITBUILD ON}
                end.
            "},
        },
        unit_not_affected = {
            indoc! {"
                unit Foo;
                {$IFDEF IMPLICITBUILDING}
                  {$ALIGN 8}
                  {$IMAGEBASE $400000}
                  {$DEFINE RELEASE}
                {$ENDIF IMPLICITBUILDING}
                {$IMPLICITBUILD ON}
                end.
            "},
            indoc! {"
                unit Foo;
                {$IFDEF IMPLICITBUILDING}
                  {$ALIGN 8}
                  {$IMAGEBASE $400000}
                  {$DEFINE RELEASE}
                {$ENDIF IMPLICITBUILDING}
                {$IMPLICITBUILD ON}
                end.
            "},
        },
    );
}
