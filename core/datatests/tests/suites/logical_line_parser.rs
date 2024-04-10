use std::{
    collections::HashMap,
    error::Error,
    fmt::{Debug, Display},
    fs::read_to_string,
    num::IntErrorKind,
    path::Path,
};

use itertools::Itertools;

use super::ErrorString;
use pasfmt_core::{
    defaults::lexer::DelphiLexer, defaults::parser::DelphiLogicalLineParser,
    prelude::LogicalLineType, prelude::*,
};

use LineNumber as LN;

fn run_test(input: &str) -> datatest_stable::Result<()> {
    let DslParserOutput {
        input_str,
        logical_lines: expected_lines,
    } = parse_dsl(input)?;

    let tokens = DelphiLexer {}.lex(&input_str);
    let (actual_lines, tokens) = DelphiLogicalLineParser {}.parse(tokens);

    let actual_lines: Vec<AssertLL> = actual_lines.into_iter().map_into().collect();

    let mut orphan_line_index = actual_lines.len();
    let mut expected_line_map = HashMap::new();
    for (&expected_line_index, expected_line) in &expected_lines {
        if let Some((actual_line_index, _)) = actual_lines
            .iter()
            .enumerate()
            .filter(|(index, _)| !expected_line_map.values().contains(index))
            .find(|(_, actual_line)| actual_line.get_tokens() == expected_line.get_tokens())
        {
            expected_line_map.insert(expected_line_index, actual_line_index);
        } else {
            expected_line_map.insert(expected_line_index, orphan_line_index);
            orphan_line_index += 1;
        }
    }

    // Map the expected lines to the order of the actual lines
    let expected_lines = expected_lines
        .into_iter()
        .sorted_by_key(|(index, _)| expected_line_map.get(index).unwrap())
        .map(|(_, line)| {
            let expected_line = &line.line;
            let mapped_parent = expected_line.get_parent().map(|parent| LineParent {
                line_index: *expected_line_map.get(&parent.line_index).unwrap(),
                global_token_index: parent.global_token_index,
            });
            AssertLL {
                line: LogicalLine::new(
                    mapped_parent,
                    expected_line.get_level(),
                    expected_line.get_tokens().clone(),
                    expected_line.get_line_type(),
                ),
                check_line_type: line.check_line_type,
            }
        })
        .collect_vec();

    assert_lines(
        input,
        &tokens,
        expected_line_map,
        actual_lines,
        expected_lines,
    )
}

fn assert_lines(
    input: &str,
    tokens: &[Token],
    expected_line_map: HashMap<usize, usize>,
    actual_lines: Vec<AssertLL>,
    expected_lines: Vec<AssertLL>,
) -> Result<(), Box<dyn Error>> {
    let orphan_expected = expected_lines
        .iter()
        .enumerate()
        .filter(|(_, expected_line)| !actual_lines.contains(expected_line))
        .collect_vec();
    let orphan_actual = (0..actual_lines.len())
        .filter(|index| {
            !expected_line_map.values().contains(index)
                && actual_lines[*index].line.get_line_type() != LogicalLineType::Eof
        })
        .map(|index| (index, &actual_lines[index]))
        .collect_vec();

    let mut error_message = String::new();
    if !orphan_actual.is_empty() || !orphan_expected.is_empty() {
        error_message.push_str("input:\n");
        error_message.push_str(input);
        error_message.push_str("\nactual:\n");
        actual_lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.line.get_line_type() != LogicalLineType::Eof)
            .for_each(|(index, line)| {
                error_message.push_str(&format!(
                    "{} - line type: {:?}\n",
                    format_line(index, &line.line, tokens),
                    &line.line.get_line_type()
                ))
            });

        error_message.push_str("expected:\n");
        expected_lines.iter().enumerate().for_each(|(index, line)| {
            error_message.push_str(&format!("{}\n", format_line(index, &line.line, tokens)))
        });
    }

    if !orphan_expected.is_empty() {
        error_message.push_str("\nactual lines did not contain:\n");
        for (index, expected_line) in orphan_expected {
            error_message.push_str(&format!(
                "  {} {}\n",
                format_line(index, &expected_line.line, tokens),
                format_line_type(expected_line),
            ))
        }
    }
    if !orphan_actual.is_empty() {
        error_message.push_str("\nexpected lines did not contain:\n");
        for (index, expected_line) in orphan_actual {
            error_message.push_str(&format!(
                "  {} {}\n",
                format_line(index, &expected_line.line, tokens),
                format_line_type(expected_line),
            ))
        }
    }

    match error_message.is_empty() {
        true => Ok(()),
        false => Err(Box::new(ErrorString(error_message))),
    }
}

fn format_line(index: usize, line: &LogicalLine, tokens: &[Token]) -> String {
    format!(
        "{} - Level: {} Parent: {:?} Tokens: {}",
        index,
        line.get_level(),
        line.get_parent(),
        line.get_tokens()
            .iter()
            .map(|token| format!(
                "{}:{:?} ",
                *token,
                tokens.get(*token).unwrap().get_content()
            ))
            .join("")
            .trim(),
    )
}
fn format_line_type(assert_line: &AssertLL) -> String {
    match assert_line.check_line_type {
        true => format!("with line type {:?}", assert_line.line.get_line_type()),
        false => "ignoring line type".to_owned(),
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
enum LineNumber {
    Explicit(usize),
    Implicit(usize),
}
impl LineNumber {
    pub fn explicit_num(&self) -> Option<usize> {
        match self {
            LineNumber::Explicit(number) => Some(*number),
            _ => None,
        }
    }
}
#[derive(Debug, Eq)]
struct AssertLL {
    pub line: LogicalLine,
    pub check_line_type: bool,
}
impl AssertLL {
    pub fn get_tokens(&self) -> &Vec<usize> {
        self.line.get_tokens()
    }
}
impl From<LogicalLine> for AssertLL {
    fn from(value: LogicalLine) -> Self {
        AssertLL {
            line: value,
            check_line_type: true,
        }
    }
}
impl PartialEq for AssertLL {
    fn eq(&self, other: &Self) -> bool {
        self.line.get_level() == other.line.get_level()
            && self.line.get_parent() == other.line.get_parent()
            && self.line.get_tokens() == other.line.get_tokens()
            && (!self.check_line_type
                || !other.check_line_type
                || self.line.get_line_type() == other.line.get_line_type())
    }
}

struct DslParserOutput {
    pub input_str: String,
    pub logical_lines: HashMap<usize, AssertLL>,
}

struct RawAssertLL {
    parent: Option<LineParent>,
    level: u16,
    tokens: Vec<usize>,
    check_line_type: bool,
}

/*
    This DSL allows the assertion of multiple aspects of the LogicalLines in a
    terse fashion. These are:
    - logical line contents, e.g.,
        - line numbers don't need to be sequential
        `1|Foo()` - `Foo()` will all be on the same line
        `1,2|Foo()` - `Foo()` will be on 2 lines, labelled 1 and 2
        ```
        1|Foo();
        2|Bar();
        ``` - `Foo();` and `Bar();` will be on separate lines
        ```
        _|Foo();
        _|Bar();
        ``` - `Foo();` and `Bar();` will be on separate lines
            - lines labelled `_` will be unique lines
        ```
        1|Foo();
        2|Bar();
        1|Baz();
        ``` - `Foo(); Baz();` will be the line labelled `1`
            - useful if there are lines in the middle
        ```
        1|Foo();
         |Bar();
        ``` - `Foo(); Bar();` will be the line labelled `1`
            - useful for breaking long lines
        ```
        1|Foo({
          multiline comment});
        ``` - `Foo({...});` will be the line labelled `1`
            - useful if there are multiline tokens
    - logical line types, e.g.,
        ```
        1|procedure Foo();
        2|Foo();
        ---
        1:RoutineHeader
        ``` - line `procedure Foo();` will have line type `RoutineHeader`
            - line `Foo();` will have line type `Unknown`
            - any line that is not explicitly stated will be asserted as Unknown
            - start asserting line types with the `---` separator
            - the line types will be matched to the line number label
        ```
        _1|procedure Foo();
        _ |procedure Bar();
        ``` - ignore testing the logical line type with a `_`
            - these lines can have any type
            - line numbers are optionally specifiable
    - logical line parents, e.g.,
        ```
        1 |Foo(procedure begin{1}
        ^1|  Bar;
        ^1|  Baz;
        1 |end);
        ``` - postfix labelling of `begin` with `{1}` allows token to be referenced
            - lines `Bar;` and `Baz;` will have parent of the `begin` token
            - the `^1` next to the line number references the `{1}` label
        ```
        1,2|Foo(procedure begin{1}{2}
        _^1|  Bar;
        _^2|  Baz;
        1,2|end);
        ``` - labels of `begin` correspond with multiple line numbers
                - label 1 on line 1, label 2 on line 2
            - `Bar;` has parent of line 1 `begin`
            - `Baz;` has parent of line 2 `begin`
            - useful for conditionally compiled code
        ```
        1,2    |Foo(procedure begin{1}{2}
        3^1,4^2|  Bar;
        1,2    |end);
        ``` - multiple lines have different token markers
                - line 3 marker 1, line 4 marker 2
            - useful for conditionally compiled code
    - logical line level, e.g.,
        - based on indentation, 1 level is 2 spaces
        ```
        _|procedure Foo();
        _|  Foo();
        ``` - line `procedure Foo();` will have line level 0
            - line `Foo();` will have line level 1
        ```
        _1 |  Foo(procedure begin
        _^1|    Bar;
        _1 |  end);
        ``` - child lines are relative to the parent indentation
            - `Bar;` will have level 1 as it is 1 greater than its parent
*/
fn parse_dsl(input: &str) -> Result<DslParserOutput, TestParsingError> {
    const LINE_TYPES_SEPARATOR: &str = "---";

    let line_input = input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take_while(|line| line != &LINE_TYPES_SEPARATOR)
        .collect_vec();
    let line_type_input = input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .skip(line_input.len() + 1)
        .collect_vec();

    let line_types: HashMap<usize, LogicalLineType> = parse_line_types(line_type_input)?;
    parse_expected_lines(line_input, line_types)
}

fn parse_expected_lines(
    lines: Vec<&str>,
    line_types: HashMap<usize, LogicalLineType>,
) -> Result<DslParserOutput, TestParsingError> {
    let mut line_data: Vec<(&str, String)> = Vec::new();
    for line in lines {
        let mut append_to_previous_line = |line: &str, content: &str| {
            use std::fmt::Write;
            write!(
                line_data
                    .last_mut()
                    .ok_or_else(|| TestParsingError::InvalidFirstLine(line.to_string()))?
                    .1,
                "\n{content}"
            )
            .map_err(|e| TestParsingError::InvalidFirstLine(e.to_string()))
        };

        match line.split_once('|') {
            None => append_to_previous_line(line, line)?,
            Some((metadata, content)) if metadata.trim().is_empty() => {
                append_to_previous_line(line, content)?
            }
            Some((metadata, content)) => line_data.push((metadata, content.to_owned())),
        };
    }

    let mut token_markers = HashMap::new();
    let mut parsed_lines = HashMap::new();
    let mut included_tokens: usize = 0;

    let mut input_str = String::new();
    let mut implicit_lines = 0;

    // e.g., ("_1,2^1", "foo")
    for (metadata, content) in line_data {
        let line_numbers = parse_line(
            metadata,
            &content,
            &mut implicit_lines,
            &token_markers,
            &mut parsed_lines,
        )?;

        let mut markers_added = 0;
        /*
            Read the tokens in the data for the line and,
                - construct new input string without marker tokens
                - register marker tokens
        */
        for source_token in (DelphiLexer {})
            .lex(content.trim_start())
            .into_iter()
            .filter(|token| token.get_token_type() != RawTokenType::Eof)
        {
            let token_content = source_token.get_content();
            if let RawTokenType::Comment(CommentKind::InlineBlock | CommentKind::IndividualBlock) =
                source_token.get_token_type()
            {
                let interior_end = token_content.len() - 1;
                if let Ok(Some(marker_num)) = parse_number(&token_content[1..interior_end]) {
                    token_markers.insert(
                        marker_num,
                        LineParent {
                            line_index: line_numbers
                                .get(markers_added)
                                .and_then(LineNumber::explicit_num)
                                .ok_or_else(|| {
                                    TestParsingError::ImplicitMarkerLine(format!(
                                        "{metadata} |{content}"
                                    ))
                                })?,
                            global_token_index: included_tokens.checked_sub(1).ok_or_else(
                                || TestParsingError::BadMarker(format!("{metadata} |{content}")),
                            )?,
                        },
                    );
                    markers_added += 1;
                    continue;
                };
            }
            for line_num in &line_numbers {
                parsed_lines
                    .get_mut(line_num)
                    .unwrap()
                    .1
                    .tokens
                    .push(included_tokens);
            }

            input_str.push_str(source_token.get_leading_whitespace());
            input_str.push_str(token_content);
            included_tokens += 1;
        }
        input_str.push('\n');
    }

    Ok(DslParserOutput {
        input_str,
        logical_lines: finalise_logical_lines(parsed_lines, line_types),
    })
}

enum TestParsingError {
    InvalidFirstLine(String),
    InvalidNumber(String),
    InvalidParentReference(usize),
    ImplicitMarkerLine(String),
    BadMarker(String),
    NoLineTypeDelimiter(String),
    InvalidLogicalLineType(String),
}
impl Display for TestParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const PREFIX: &str = "maformed input: ";
        match self {
            TestParsingError::InvalidFirstLine(input) => {
                write!(
                    f,
                    "{PREFIX}first line cannot continue previous line '{input}'"
                )
            }
            TestParsingError::InvalidNumber(input) => write!(f, "{PREFIX}invalid number '{input}'"),
            TestParsingError::InvalidParentReference(parent) => {
                write!(
                    f,
                    "{PREFIX}invalid parent reference for marker '{parent:?}'"
                )
            }
            TestParsingError::ImplicitMarkerLine(input) => write!(
                f,
                "{PREFIX}only explicitly numbered lines can have markers\nline: '{input}'"
            ),
            TestParsingError::BadMarker(input) => {
                write!(f, "{PREFIX}marker must succeed another token '{input}'")
            }
            TestParsingError::NoLineTypeDelimiter(input) => {
                write!(f, "{PREFIX}no logical line type declaration delimiter '{input}' (expected `number:LogicalLineType`)")
            }
            TestParsingError::InvalidLogicalLineType(input) => {
                write!(f, "{PREFIX}invalid logical line type '{input}'")
            }
        }
    }
}
impl Debug for TestParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}
impl Error for TestParsingError {}

fn parse_line(
    metadata: &str,
    content: &str,
    implicit_lines: &mut usize,
    token_markers: &HashMap<usize, LineParent>,
    parsed_lines: &mut HashMap<LineNumber, (u16, RawAssertLL)>,
) -> Result<Vec<LineNumber>, TestParsingError> {
    let metadata = parse_line_metadata(metadata)?;
    let indentation_level = ((content.len() - content.trim_start().len()) / 2) as u16;
    let mut line_numbers = Vec::new();
    for LineMetadata {
        line_number,
        parent_marker,
        check_line_type,
    } in metadata
    {
        let mut level = indentation_level;
        let line_number = match line_number {
            Some(line_number) => LN::Explicit(line_number),
            _ => LN::Implicit({
                *implicit_lines += 1;
                *implicit_lines - 1
            }),
        };

        let parent = if let Some(parent_marker) = parent_marker {
            let line_parent = token_markers
                .get(&parent_marker)
                .ok_or(TestParsingError::InvalidParentReference(parent_marker))?;
            let &(parent_indentation, _) = parsed_lines
                .get(&LN::Explicit(line_parent.line_index))
                .ok_or(TestParsingError::InvalidParentReference(parent_marker))?;
            level -= parent_indentation + 1;
            Some(line_parent)
        } else {
            None
        };
        parsed_lines.entry(line_number).or_insert_with(|| {
            (
                indentation_level,
                RawAssertLL {
                    parent: parent.cloned(),
                    level,
                    tokens: vec![],
                    check_line_type,
                },
            )
        });
        line_numbers.push(line_number);
    }
    Ok(line_numbers)
}

fn finalise_logical_lines(
    parsed_lines: HashMap<LineNumber, (u16, RawAssertLL)>,
    line_types: HashMap<usize, LogicalLineType>,
) -> HashMap<usize, AssertLL> {
    // The generated line numbers will start after the last explicit line number
    let first_implicit_line = parsed_lines
        .keys()
        .flat_map(|key| key.explicit_num())
        .max()
        .unwrap_or(0)
        + 1;

    parsed_lines
        .into_iter()
        .map(|(line_number, (_, assert_line))| {
            (
                match line_number {
                    LN::Implicit(num) => num + first_implicit_line,
                    LN::Explicit(num) => num,
                },
                AssertLL {
                    line: LogicalLine::new(
                        assert_line.parent,
                        assert_line.level,
                        assert_line.tokens,
                        *line_number
                            .explicit_num()
                            .and_then(|line_number| line_types.get(&line_number))
                            .unwrap_or(&LogicalLineType::Unknown),
                    ),
                    check_line_type: assert_line.check_line_type,
                },
            )
        })
        .collect()
}

struct LineMetadata {
    line_number: Option<usize>,
    parent_marker: Option<usize>,
    check_line_type: bool,
}

fn parse_line_metadata(input: &str) -> Result<Vec<LineMetadata>, TestParsingError> {
    /*
        Comma separated definitions with optional parts:
            _   ignore logical line type/line_number
            1   the logical line number
            ^1  parent reference to token marker
    */
    let mut result = Vec::new();
    for mut line_number in input.split(',') {
        line_number = line_number.trim();
        let check_line_type = !line_number.starts_with('_');
        if !check_line_type {
            line_number = line_number.trim_start_matches('_');
        }

        let (line_number, parent_marker) = if let Some((line, parent)) = line_number.split_once('^')
        {
            (parse_number(line)?, parse_number(parent)?)
        } else {
            (parse_number(line_number)?, None)
        };
        result.push(LineMetadata {
            line_number,
            parent_marker,
            check_line_type,
        })
    }
    Ok(result)
}

fn parse_number(input: &str) -> Result<Option<usize>, TestParsingError> {
    input.trim().parse::<usize>().map_or_else(
        |e| match e.kind() {
            IntErrorKind::Empty => Ok(None),
            _ => Err(TestParsingError::InvalidNumber(input.to_string())),
        },
        |num| Ok(Some(num)),
    )
}

fn parse_line_types(lines: Vec<&str>) -> Result<HashMap<usize, LogicalLineType>, TestParsingError> {
    let mut result = HashMap::new();
    for line in lines {
        let (line_number, line_type) = line
            .trim()
            .split_once(':')
            .ok_or_else(|| TestParsingError::NoLineTypeDelimiter(line.to_string()))?;

        let line_number = parse_number(line_number)?
            .ok_or_else(|| TestParsingError::InvalidNumber(line_number.to_string()))?;
        let line_type = LogicalLineType::try_from(line_type.trim())
            .map_err(|_| TestParsingError::InvalidLogicalLineType(line_type.to_string()))?;
        result.insert(line_number, line_type);
    }
    Ok(result)
}

pub fn test_file(path: &Path) -> datatest_stable::Result<()> {
    let input = read_to_string(path)?;
    run_test(&input)
}
