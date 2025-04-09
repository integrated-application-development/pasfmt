use std::path::Path;

use crate::*;

macro_rules! generate_test_groups {
    ($root_dir: expr, $test_group: ident, $($name: ident = $($input: expr);*),* $(,)?) => {
        $(
            mod $name {
                pub fn generate(root_dir: &std::path::Path) {
                    $test_group!(root_dir, $($input),*);
                }
            }
            $name::generate($root_dir);
        )*
    };
}

pub fn generate_test_files(root_dir: &Path) {
    directives::generate(root_dir);
    comments::generate(root_dir);
    child_lines::generate(root_dir);
    file_headers::generate(root_dir);
    file_sections::generate(root_dir);
    import_clauses::generate(root_dir);
    exports::generate(root_dir);
    decl_sections::generate(root_dir);
    type_decls::generate(root_dir);
    visibility_sections::generate(root_dir);
    prop_decls::generate(root_dir);
    routine_headers::generate(root_dir);
    routine_implementations::generate(root_dir);
    control_flows::generate(root_dir);
    statements::generate(root_dir);
    attributes::generate(root_dir);
    semicolons::generate(root_dir);
    regression::generate(root_dir);
}

mod directives {
    use super::*;

    pub fn generate(root_dir: &Path) {
        conditional::generate(root_dir);
        compiler::generate(root_dir);
    }

    mod conditional {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                // If there is no alternation only a single line is created
                ifdef = "
                    1 |Foo
                    3 |{$ifdef}
                    1 |Baz
                    4 |{$endif}
                    ---
                    3:ConditionalDirective
                    4:ConditionalDirective
                ",
                if_else = "
                    1,2 |Foo
                    3   |{$ifdef}
                    1   |Bar
                    4   |{$else}
                    2   |Baz
                    5   |{$endif}
                    ---
                    3:ConditionalDirective
                    4:ConditionalDirective
                    5:ConditionalDirective
                ",
                nested = "
                    1,2,3 |Foo
                    4     |{$ifdef}
                    5     |  {$ifdef}
                    1     |Bar
                    6     |  {$else}
                    2     |Baz
                    7     |  {$endif}
                    8     |{$else}
                    3     |Flarp
                    9     |{$endif}
                    ---
                    4:ConditionalDirective
                    5:ConditionalDirective
                    6:ConditionalDirective
                    7:ConditionalDirective
                    8:ConditionalDirective
                    9:ConditionalDirective
                ",
            );
        }
    }
    mod compiler {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                individual = "
                    1 |{$message 'boo'}
                    ---
                    1:CompilerDirective
                ",
                nested = "
                    1 |{$ifdef}
                    2 |  {$message 'boo'}
                    3 |{$endif}
                    ---
                    1:ConditionalDirective
                    2:CompilerDirective
                    3:ConditionalDirective
                ",
                nested_code_before = "
                    _ |unit Foo;
                    1 |{$ifdef}
                    2 |  {$message 'boo'}
                    3 |{$endif}
                    ---
                    1:ConditionalDirective
                    2:CompilerDirective
                    3:ConditionalDirective
                ",
                code_above_below = "
                    _ |type
                    1 |  {$RTTI}
                    _ |  TFoo = class
                    _ |  end
                    ---
                    1:CompilerDirective
                ",
                line_comment_above_code_below = "
                    _ |// Comment
                    1 |{$RTTI}
                    _ |Foo;
                    ---
                    1:CompilerDirective
                ",
                code_above_line_comment_below = "
                    _ |Foo;
                    1 |{$RTTI}
                    _ |// Comment
                    ---
                    1:CompilerDirective
                ",
                block_comment_above_code_below = "
                    _ |{block comment}
                    1 |{$RTTI}
                    _ |Foo;
                    ---
                    1:CompilerDirective
                ",
                code_above_block_comment_below = "
                    _ |Foo;
                    1 |{$RTTI}
                    _ |{block comment}
                    ---
                    1:CompilerDirective
                ",
                mid_line = "_ |A {$J+} := B {$C+} + C {$C-};",
            );
        }
    }
}

mod comments {
    use super::*;

    pub fn generate(root_dir: &Path) {
        inline::generate(root_dir);
        individual::generate(root_dir);
    }

    mod inline {
        use super::*;

        macro_rules! test_group {
            ($root_dir: expr, $comment: expr) => {
                crate::generate_test_cases!(
                    $root_dir,
                    end_of_line = format!(
                        "
                            _|begin {0}
                            _|  Foo;
                            _|end {0}
                        ",
                        $comment
                    ),
                    mid_line = format!(
                        "
                            _|Foo {}
                             | ();
                        ",
                        $comment
                    ),
                    multi_mid_line = format!(
                        "
                            _|Foo {0}
                             | () {0}
                             | ;
                        ",
                        $comment
                    ),
                    /*
                        This case asserts that the parent token for child lines remains unchanged
                        with the addition of a comment between the parent and the child lines.

                        This is desired as the formatting of the child lines is to be dependent
                        on the parent token. I.e., changing the formatting of the trailing comment
                        should not affect the formatting of the child lines.
                    */
                    anonymous_begin = format!(
                        "
                            _1 |A := procedure begin{{1}} {}
                            _^1|  Foo;
                            _1 |end;
                        ",
                        $comment
                    ),
                );
            };
        }

        pub fn generate(root_dir: &Path) {
            generate_test_groups!(
                root_dir,
                test_group,
                block = "{inline block}",
                line = "// inline line"
            );
        }
    }

    mod individual {
        use super::*;

        macro_rules! test_group {
            ($root_dir: expr, $comment: expr) => {
                crate::generate_test_cases!(
                    $root_dir,
                    empty_compound_statement = format!(
                        "
                            _|begin
                            _|  {}
                            _|end
                        ",
                        $comment
                    ),
                    first_line_in_compound = format!(
                        "
                            _|begin
                            _|  {}
                            _|  Foo;
                            _|end
                        ",
                        $comment
                    ),
                    last_line_in_compound = format!(
                        "
                            _|begin
                            _|  Foo;
                            _|  {}
                            _|end
                        ",
                        $comment
                    ),
                    mid_line = format!(
                        "
                            _|Foo
                             | {}
                             | ();
                        ",
                        $comment
                    ),
                    multi_mid_line = format!(
                        "
                            _|Foo
                             | {0}
                             | ()
                             | {0}
                             | ;
                        ",
                        $comment
                    ),
                    anonymous_only_line = format!(
                        "
                            _1 |Foo := procedure begin{{1}}
                            2^1|  {}
                            _1 |end
                        ",
                        $comment
                    ),
                    inside_type_def = format!(
                        "
                            _|type
                            _|  {0}
                            _|  TFoo = Bar;
                            _|  {0}
                            _|  TFoo2 = Bar;
                        ",
                        $comment
                    ),
                    inside_const_def = format!(
                        "
                            _|const
                            _|  {0}
                            _|  A = B;
                            _|  {0}
                            _|  A2 = B;
                        ",
                        $comment
                    ),
                    inside_var_def = format!(
                        "
                            _|var
                            _|  {0}
                            _|  A;
                            _|  {0}
                            _|  A2;
                        ",
                        $comment
                    ),
                    after_type_def_before_routine = format!(
                        "
                            _|type
                            _|  TFoo = Bar;
                            _|{0}
                            _|{0}
                            _|procedure Foo;
                        ",
                        $comment
                    ),
                    after_const_def_before_routine = format!(
                        "
                            _|const
                            _|  A = B;
                            _|{0}
                            _|{0}
                            _|procedure Foo;
                        ",
                        $comment
                    ),
                    after_var_def_before_routine = format!(
                        "
                            _|var
                            _|  A;
                            _|{0}
                            _|{0}
                            _|procedure Foo;
                        ",
                        $comment
                    ),
                    local_decl = format!(
                        "
                            _|procedure Foo;
                            _|{}
                            _|begin
                            _|end
                        ",
                        $comment
                    ),
                    local_decl_before_var = format!(
                        "
                            _|procedure Foo;
                            _|{}
                            _|var
                            _|begin
                            _|end
                        ",
                        $comment
                    ),
                    nested_local_decl = format!(
                        "
                            _|procedure Foo;
                            _|  {0}
                            _|  procedure Foo;
                            _|  {0}
                            _|  begin
                            _|  end
                            _|{0}
                            _|begin
                            _|end
                        ",
                        $comment
                    ),
                    nested_local_decl_before_var = format!(
                        "
                            _|procedure Foo;
                            _|  procedure Foo;
                            _|  {0}
                            _|  var
                            _|  begin
                            _|  end
                            _|{0}
                            _|var
                            _|begin
                            _|end
                        ",
                        $comment
                    ),
                );
            };
        }

        pub fn generate(root_dir: &Path) {
            generate_test_groups!(
                root_dir,
                test_group,
                block = "{inline block}",
                line = "// inline line",
                multiline = "{\nfoo\n}"
            );
        }
    }
}

mod child_lines {
    use super::*;

    pub fn generate(root_dir: &Path) {
        generate_test_cases!(
            root_dir,
            conditional_child_lines = "
                1   |Foo(
                1   |procedure
                1   |begin{1}
                2   |{$ifdef}
                3^1 |  A;
                5   |{$else}
                6^1 |  B;
                7   |{$endif}
                1   |end
                1   |)
                ---
                2:ConditionalDirective
                5:ConditionalDirective
                7:ConditionalDirective
            ",
            conditional_begin = "
                1,2     |Foo(
                1,2     |procedure
                5       |{$ifdef}
                1       |begin{1}
                6       |{$else}
                2       |begin{2}
                7       |{$endif}
                3^1,4^2 |  A;
                1,2     |end
                1,2     |)
                ---
                5:ConditionalDirective
                6:ConditionalDirective
                7:ConditionalDirective
            ",
            conditional_content_and_end = "
                1,2 |Foo(
                1,2 |procedure
                1,2 |begin{1}{2}
                5   |{$ifdef}
                3^1 |  A;
                1   |end
                6   |{$else}
                4^2 |  B;
                2   |end
                7   |{$endif}
                1,2 |)
                ---
                5:ConditionalDirective
                6:ConditionalDirective
                7:ConditionalDirective
            ",
            nested = "
                1   |Foo(
                1   |function(Arg1: String): String
                1   |begin{1}
                2^1 |  Bar(
                2   |  procedure
                2   |  begin{2}
                4^2 |    Baz1;
                2   |  end,
                2   |  procedure
                2   |  begin{3}
                5^3 |    Baz2;
                2   |  end
                2   |  );
                3^1 |  Flarp;
                1   |end
                1   |)
            ",
            parallel_nested_anonymous = "
                1  |Foo(procedure begin{1}
                _  |{$ifdef A}
                _^1|  AAAA;
                _^1|  BBBB;
                _^1|  CCCC;
                _  |{$else}
                _^1|  DDDD;
                _  |{$endif}
                2^1|  Bar(procedure begin{2}
                _^2|    AAA;
                2  |  end);
                3^1|  Baz(procedure begin{3}
                _^3|    BBB;
                3  |  end);
                1  |end);
            ",
        );
    }
}

mod file_headers {
    use super::*;

    macro_rules! test_group {
        ($root_dir: expr, $header: expr) => {
            crate::generate_test_cases!(
                $root_dir,
                individual = format!("_|{}", $header),
                comment_before = format!(
                    "
                        _|// comment
                        _|{}
                    ",
                    $header
                ),
                line_comment_after = format!(
                    "
                        _|{}
                        _|// comment
                    ",
                    $header
                ),
                inline_comment_after = format!("_|{} // comment", $header)
            );
        };
    }

    pub fn generate(root_dir: &Path) {
        generate_test_groups!(
            root_dir,
            test_group,
            unit = "unit Foo;",
            program = "program Foo;",
            package = "package Foo;",
            library = "library Foo;"
        );
    }
}

mod file_sections {
    use super::*;

    pub fn generate(root_dir: &Path) {
        generate_test_cases!(
            root_dir,
            unit_int_impl = "
                _|unit Foo;
                _|interface
                _|implementation
                _|end.
            ",
            unit_init = "
                _|unit Foo;
                _|interface
                _|implementation
                _|initialization
                _|end.
            ",
            unit_init_fin = "
                _|unit Foo;
                _|interface
                _|implementation
                _|initialization
                _|finalization
                _|end.
            ",
        );
    }
}

mod import_clauses {
    use super::*;

    macro_rules! test_group {
        ($root_dir: expr, $keyword: expr) => {
            crate::generate_test_cases!(
                $root_dir,
                one_file = format!(
                    "
                        _|package Foo;
                        _|{}
                        1|  unit1;
                        ---
                        1:ImportClause
                    ",
                    $keyword
                ),
                one_file_in = format!(
                    "
                        _|package Foo;
                        _|{}
                        1|  unit1 in 'foo/bar';
                        ---
                        1:ImportClause
                    ",
                    $keyword
                ),
                many_files = format!(
                    "
                        _|package Foo;
                        _|{}
                        1|  unit1, unit2;
                        ---
                        1:ImportClause
                    ",
                    $keyword
                ),
                many_files_in = format!(
                    "
                        _|package Foo;
                        _|{}
                        1|  unit1 in 'foo/bar',
                         |  unit2 in 'foo/baz';
                        ---
                        1:ImportClause
                    ",
                    $keyword
                ),
                comment_between_import_and_item = format!(
                    "
                        _|package Foo;
                        _|{0}
                        _|  {{}}
                        _|  A;
                        _|{0}
                        _|  //
                        _|  A;
                        _|{0}
                        _|  {{}}
                        _|  //
                        _|  {{}}
                        _|  A;
                    ",
                    $keyword
                ),
            );
        };
    }

    pub fn generate(root_dir: &Path) {
        generate_test_groups!(
            root_dir,
            test_group,
            contains = "contains",
            requires = "requires",
            uses = "uses"
        );
    }
}
mod exports {
    use super::*;

    pub fn generate(root_dir: &Path) {
        generate_test_cases!(
            root_dir,
            one_export = "
                _|exports
                1|  Foo;
                ---
                1:ExportClause
            ",
            one_export_with_index = "
                _|exports
                1|  Foo index 1;
                ---
                1:ExportClause
            ",
            one_export_with_index_name = "
                _|exports
                1|  Foo index 1 name foo;
                ---
                1:ExportClause
            ",
            many_exports = "
                _|exports
                1|  Foo, Bar;
                ---
                1:ExportClause
            ",
            many_exports_with_index = "
                _|exports
                1|  Foo index 1,
                 |  Bar index 2;
                ---
                1:ExportClause
            ",
            many_exports_with_index_name = "
                _|exports
                1|  Foo index 1 name Foo,
                 |  Bar index 2 name Bar;
                ---
                1:ExportClause
            ",
            comment_between_export_and_item = "
                _|exports
                _|  {}
                _|  A;
            ",
        );
    }
}

mod decl_sections {
    use super::*;

    pub fn generate(root_dir: &Path) {
        explicit::generate(root_dir);
        anonymous::generate(root_dir);
    }

    mod explicit {
        use super::*;

        macro_rules! test_group {
            ($root_dir: expr, $input: expr) => {
                crate::generate_test_cases!(
                    $root_dir,
                    interface = format!(
                        "
                            _|unit Foo;
                            _|interface
                            {}
                            ---
                            {}
                        ",
                        $input.0, $input.1
                    ),
                    implementation = format!(
                        "
                            _|unit Foo;
                            _|interface
                            _|implementation
                            {}
                            ---
                            {}
                        ",
                        $input.0, $input.1
                    ),
                    local_decl = format!(
                        "
                            _|procedure Foo;
                            {}
                            _|begin
                            _|end;
                            ---
                            {}
                        ",
                        $input.0, $input.1
                    ),
                    after_sub_routine = format!(
                        "
                            _|procedure Foo;
                            _|  procedure Foo;
                            _|  begin
                            _|  end;
                            {}
                            _|begin
                            _|end;
                            ---
                            {}
                        ",
                        $input.0, $input.1
                    ),
                );
            };
        }

        pub fn generate(root_dir: &Path) {
            generate_test_groups!(
                root_dir,
                test_group,
                one_label = (
                    "
                        1|label
                        2|  1;
                    ",
                    "
                        2:Declaration
                    "
                ),
                many_labels = (
                    "
                        1|label
                        2|  1,ident;
                    ",
                    "
                        2:Declaration
                    "
                ),
                one_const = (
                    "
                        1|const
                        2|  CFoo = 1;
                    ",
                    "
                        2:Declaration
                    "
                ),
                many_consts = (
                    "
                        1|const
                        2|  CFoo = 1;
                        3|  CBar = '1234';
                    ",
                    "
                        2:Declaration
                        3:Declaration
                    "
                ),
                one_var = (
                    "
                        1|var
                        2|  A: TFoo;
                    ",
                    "
                        2:Declaration
                    "
                ),
                many_vars = (
                    "
                        1|var
                        2|  A: TFoo;
                        3|  B: TFoo;
                    ",
                    "
                        2:Declaration
                        3:Declaration
                    "
                ),
                many_names_var = (
                    "
                        1|var
                        2|  A, B: TFoo;
                    ",
                    "
                        2:Declaration
                    "
                ),
                many_names_vars = (
                    "
                        1|var
                        2|  A, B: TFoo;
                        3|  C, D: TFoo;
                    ",
                    "
                        2:Declaration
                        3:Declaration
                    "
                ),
                one_threadvar = (
                    "
                        1|threadvar
                        2|  A: TFoo;
                    ",
                    "
                        2:Declaration
                    "
                ),
                many_threadvars = (
                    "
                        1|threadvar
                        2|  A: TFoo;
                        3|  B: TFoo;
                    ",
                    "
                        2:Declaration
                        3:Declaration
                    "
                ),
                one_resourcestring = (
                    "
                        1|resourcestring
                        2|  A = '';
                    ",
                    "
                        2:Declaration
                    "
                ),
                many_resourcestrings = (
                    "
                        1|resourcestring
                        2|  A = '';
                        3|  B = '' + '';
                        4|  C = '''
                         |    asdf
                         |    ''';
                    ",
                    "
                        2:Declaration
                        3:Declaration
                        4:Declaration
                    "
                ),
                proc_of_object_var_with_false_directives = (
                    "
                        1|var
                        2|  A: procedure of object; near;
                        _|  Index: AA;
                        3|  A: function: AA of object; far;
                        _|  Name, Virtual: AA;
                    ",
                    "
                        2:Declaration
                        3:Declaration
                    "
                ),
                ref_to_proc_var_with_false_directives = (
                    "
                        1|var
                        2|  A: reference to procedure; stdcall;
                        _|  Index: AA;
                        3|  A: reference to function: AA; safecall;
                        _|  Name, Virtual: AA;
                    ",
                    "
                        2:Declaration
                        3:Declaration
                    "
                ),
                type_class = (
                    "
                        1|type
                        2|  TFoo = class;
                        3|  TFoo = packed class;
                    ",
                    "
                        2:Declaration
                        3:Declaration
                    "
                ),
                type_type = (
                    "
                        1|type
                        2|  TFoo = type Foo;
                    ",
                    "
                        2:Declaration
                    "
                ),
                type_routine = (
                    "
                        1|type
                        2|  TFoo = function: TBar of object;
                        3|  TFoo = procedure of object;
                    ",
                    "
                        2:Declaration
                        3:Declaration
                    "
                ),
                stacked = (
                    "
                        1 |label
                        2 |  A;
                        3 |const
                        4 |  A = '';
                        5 |resourcestring
                        6 |  A = '';
                        7 |type
                        8 |  A = B;
                        9 |var
                        10|  A: B;
                        11|threadvar
                        12|  A: B;
                    ",
                    "
                        2:Declaration
                        4:Declaration
                        6:Declaration
                        8:Declaration
                        10:Declaration
                        12:Declaration
                    "
                ),
            );
        }
    }

    mod anonymous {
        use super::*;

        fn test_case(decl_section: &str, line_types: &str) -> String {
            format!(
                "
                    _1|A := procedure Foo
                    {}
                    1|begin end;
                    ---
                    {}
                ",
                decl_section, line_types,
            )
        }

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                one_label = test_case(
                    "
                        1  |label{1}
                        2^1|  1;
                    ",
                    "
                        2:Declaration
                    "
                ),
                many_labels = test_case(
                    "
                        1  |label{1}
                        2^1|  1,ident;
                    ",
                    "
                        2:Declaration
                    "
                ),
                one_const = test_case(
                    "
                        1  |const{1}
                        2^1|  CFoo = 1;
                    ",
                    "
                        2:Declaration
                    "
                ),
                many_consts = test_case(
                    "
                        1  |const{1}
                        2^1|  CFoo = 1;
                        3^1|  CBar = '1234';
                    ",
                    "
                        2:Declaration
                        3:Declaration
                    "
                ),
                one_resourcestring = test_case(
                    "
                        1  |resourcestring{1}
                        2^1|  CFoo = 1;
                    ",
                    "
                        2:Declaration
                    "
                ),
                many_resourcestrings = test_case(
                    "
                        1  |resourcestring{1}
                        2^1|  CFoo = 1;
                        3^1|  CBar = '1234';
                    ",
                    "
                        2:Declaration
                        3:Declaration
                    "
                ),
                one_var = test_case(
                    "
                        1  |var{1}
                        2^1|  A: TFoo;
                    ",
                    "
                        2:Declaration
                    "
                ),
                many_vars = test_case(
                    "
                        1  |var{1}
                        2^1|  A: TFoo;
                        3^1|  B: TFoo;
                    ",
                    "
                        2:Declaration
                        3:Declaration
                    "
                ),
                many_names_var = test_case(
                    "
                        1  |var{1}
                        2^1|  A, B: TFoo;
                    ",
                    "
                        2:Declaration
                    "
                ),
                many_names_vars = test_case(
                    "
                        1  |var{1}
                        2^1|  A, B: TFoo;
                        3^1|  C, D: TFoo;
                    ",
                    "
                        2:Declaration
                        3:Declaration
                    "
                ),
                one_threadvar = test_case(
                    "
                        1  |threadvar{1}
                        2^1|  A: TFoo;
                    ",
                    "
                        2:Declaration
                    "
                ),
                many_threadvars = test_case(
                    "
                        1  |threadvar{1}
                        2^1|  A: TFoo;
                        3^1|  B: TFoo;
                    ",
                    "
                        2:Declaration
                        3:Declaration
                    "
                ),
                anonymous_argument = "
                    _1 |Bar(procedure
                    1  |label{1}
                    2^1|  1,ident;
                    1  |const{2}
                    3^2|  CFoo = 1;
                    4^2|  CBar = '1234';
                    1  |var{3}
                    5^3|  A, B: TFoo;
                    6^3|  C, D: TFoo;
                    1  |begin end);
                    ---
                    2:Declaration
                    3:Declaration
                    4:Declaration
                    5:Declaration
                    6:Declaration
                ",
                stacked = test_case(
                    "
                        1  |label{1}
                        2^1|  A;
                        1  |const{2}
                        3^2|  A = '';
                        1  |resourcestring{3}
                        4^3|  A = '';
                        1  |type{4}
                        5^4|  A = B;
                        1  |var{5}
                        6^5|  A: B;
                        1  |threadvar{6}
                        7^6|  A: B;
                    ",
                    "
                        2:Declaration
                        3:Declaration
                        4:Declaration
                        5:Declaration
                        6:Declaration
                        7:Declaration
                    "
                ),
            );
        }
    }
}

mod type_decls {
    use super::*;

    pub fn generate(root_dir: &Path) {
        struct_type::generate(root_dir);
        data_type::generate(root_dir);
        var_type::generate(root_dir);
        contains_var_type::generate(root_dir);
        procedure::generate(root_dir);
    }

    mod struct_type {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generics_parents::generate(root_dir);
            sections::generate(root_dir);

            generate_test_cases!(
                root_dir,
                variant_record = "
                    _  |type
                    _  |  TRec = record
                    _  |  case Boolean of
                    1  |    True: ({1}
                    3^1|      Foo: Integer;
                    4^1|      Bar: Integer
                    1  |    );
                    2  |    False: ({2}
                    5^2|      Baz: Double
                    2  |    );
                    _  |  end;
                    ---
                    1:VariantRecordCaseArm
                    2:VariantRecordCaseArm
                    3:Declaration
                    4:Declaration
                    5:Declaration
                ",
                nested_variant_record = "
                    _     |type
                    _     |  TRec = record
                    _     |    A: B;
                    _     |  case Boolean of
                    1     |    True: ({1}
                    11^1  |      Foo: Integer;
                    12^1  |      case Boolean of
                    13^1  |        True: ({2}
                    131^2 |          Bar: Integer;
                    132^2 |          Baz: Integer;
                    133^2 |          case Boolean of
                    134^2 |            True: ({3}
                    1341^3|              Fuzz: Integer;
                    134   |            );
                    13    |          );
                    14^1  |        False: ({4}
                    141^4 |          Flim: Double
                    14    |        );
                    1     |    );
                    2     |    False: ({5}
                    21^5  |      Flam: Double
                    2     |    );
                    _     |  end;
                    ---
                    1:VariantRecordCaseArm
                    11:Declaration
                    12:CaseHeader
                    13:VariantRecordCaseArm
                    131:Declaration
                    132:Declaration
                    133:CaseHeader
                    134:VariantRecordCaseArm
                    1341:Declaration
                    14:VariantRecordCaseArm
                    141:Declaration
                    2:VariantRecordCaseArm
                    21:Declaration
                ",
                interface_guid = "
                    _|type
                    _|  IFoo = interface
                    1|    ['{00000000-0000-0000-0000-000000000000}']
                    _|  end;
                    ---
                    1:Guid
                ",
                dispinterface_guid = "
                    _|type
                    _|  IFoo = dispinterface
                    1|    ['{00000000-0000-0000-0000-000000000000}']
                    _|  end;
                    ---
                    1:Guid
                ",
            );
        }
        mod generics_parents {
            use super::*;

            macro_rules! test_group {
                ($root_dir: expr, $type_decl: expr) => {
                    fn test_case(generics: &str, parents: &str) -> String {
                        format!(
                            "
                                _|type
                                _|  Foo{0} = {1}
                                _|  public
                                _|    procedure Foo;
                                _|  end;
                            ",
                            generics,
                            format!($type_decl, parents),
                        )
                    }
                    crate::generate_test_cases!(
                        $root_dir,
                        plain = test_case("", ""),
                        with_parent = test_case("", "(Foo)"),
                        with_parents = test_case("", "(Foo, Bar)"),
                        with_generic_parents = test_case("", "(Foo<T>, Bar<V>)"),
                        with_generic = test_case("<T>", ""),
                        with_generics_comma = test_case("<T, V>", ""),
                        with_generics_semicolon = test_case("<T; V>", ""),
                        with_generic_class_constraint = test_case("<T: class>", ""),
                        with_generic_constructor_constraint = test_case("<T: constructor>", ""),
                        with_generic_record_constraint = test_case("<T: record>", ""),
                        with_generics_and_constraints =
                            test_case("<T: record; V: constructor>", ""),
                        with_generics_and_parents =
                            test_case("<T: record; V: constructor>", "(Foo<T>, Bar<B>)"),
                    );
                };
            }

            pub fn generate(root_dir: &Path) {
                generate_test_groups!(
                    root_dir,
                    test_group,
                    object = "object{}",
                    packed_object = "packed object{}",
                    class = "class{}",
                    packed_class = "packed class{}",
                    abstract_class = "class abstract{}",
                    sealed_class = "class sealed{}",
                    class_helper = "class helper{} for TFoo",
                    // Records can't have parents, but otherwise behave the same
                    record = "record{}",
                    packed_record = "packed record{}",
                    // Record helpers can't have parents, but otherwise behave the same
                    record_helper = "record helper{} for TRec",
                    interface = "interface{}",
                    // Dispinterfaces can't have parents, but otherwise behave the same
                    dispinterface = "dispinterface{}",
                );
            }
        }

        mod sections {
            use super::*;

            pub fn generate(root_dir: &Path) {
                generate_test_cases!(
                    root_dir,
                    interface = "
                        _|interface
                        _|type
                        _|  TFoo = class
                        _|  end;
                    ",
                    implementation = "
                        _|implementation
                        _|type
                        _|  TFoo = class
                        _|  end;
                    ",
                    local_decl = "
                        _|procedure Foo;
                        _|type
                        _|  TFoo = class
                        _|  end;
                    ",
                    nested_type = "
                        _|interface
                        _|type
                        _|  TFoo = class
                        _|    type
                        _|      TFoo = class
                        _|      end;
                        _|  end;
                    ",
                    anonymous_local = "
                        _1 |A := procedure
                        _1 |type{1}
                        _^1|  TFoo = class
                        _^1|  end;
                    ",
                );
            }
        }
    }

    mod data_type {
        use super::*;

        macro_rules! test_group {
            ($root_dir: expr, $input: expr) => {
                crate::generate_test_cases!(
                    $root_dir,
                    interface = format!(
                        "
                            _|interface
                            _|type
                            1|  {};
                            ---
                            1:Declaration
                        ",
                        $input,
                    ),
                    implementation = format!(
                        "
                            _|implementation
                            _|type
                            1|  {};
                            ---
                            1:Declaration
                        ",
                        $input,
                    ),
                    local_decl = format!(
                        "
                            _|procedure Foo;
                            _|type
                            1|  {};
                            _|begin
                            _|end
                            ---
                            1:Declaration
                        ",
                        $input,
                    ),
                    anonymous = format!(
                        "
                            _1 |A := procedure Foo
                            1  |type{{1}}
                            2^1|  {};
                            1  |begin
                            1  |end;
                            ---
                            2:Declaration
                        ",
                        $input,
                    ),
                );
            };
        }

        pub fn generate(root_dir: &Path) {
            generate_test_groups!(
                root_dir,
                test_group,
                array_of_const = "TFoo = array of const",
                array_index_of_const = "TFoo = array[0..1] of const",
                set = "TFoo = set of TFoo",
                file = "TFoo = file",
                file_of = "TFoo = file of Foo",
                class_reference = "TBar = class of TFoo",
                pointer = "PFoo = ^TFoo",
                string = "TFoo = string",
                procedure_of_object = "TProc = procedure(Foo: Integer) of object",
                procedure_reference = "TProc = reference to procedure(Foo: Integer)",
                simple_procedure = "TProc = procedure(Foo: Integer)",
                short_string_literal = "TFoo = string[255]",
                short_string_expr = "TFoo = string[200 + 55]",
                sub_range = "TFoo = 1..2",
                type_of_type = "TFoo = type of TBar",
                strong_alias = "TFoo = type TBar",
                weak_alias = "TFoo = TBar",
                enum_type = "TFoo = (Foo, Bar, Baz)",
            );
        }
    }

    mod var_type {
        use super::*;

        macro_rules! test_group {
            ($root_dir: expr, $input: expr) => {
                crate::generate_test_cases!(
                    $root_dir,
                    type_decl = format!(
                        "
                            _|interface
                            _|type
                            _|  TFoo = {};
                        ",
                        $input,
                    ),
                    array_indices = format!(
                        "
                            _|interface
                            _|type
                            _|  TFoo = array[{}] of const;
                        ",
                        $input,
                    ),
                    array_element_type = format!(
                        "
                            _|interface
                            _|type
                            _|  TFoo = array of {};
                        ",
                        $input,
                    ),
                    set = format!(
                        "
                            _|interface
                            _|type
                            _|  TFoo = set of {};
                        ",
                        $input,
                    ),
                    file = format!(
                        "
                            _|interface
                            _|type
                            _|  TFoo = file of {};
                        ",
                        $input,
                    ),
                    pointer = format!(
                        "
                            _|interface
                            _|type
                            _|  TFoo = ^{};
                        ",
                        $input,
                    ),
                    field = format!(
                        "
                            _|interface
                            _|type
                            _|  TFoo = class
                            _|    A: {0};
                            _|    B: {0};
                            _|  end;
                        ",
                        $input,
                    ),
                    property = format!(
                        "
                            _|interface
                            _|type
                            _|  TFoo = class
                            _|    property A: {0};
                            _|    property B: {0};
                            _|  end;
                        ",
                        $input,
                    ),
                    var_decl = format!(
                        "
                            _|var
                            _|  Foo: {};
                        ",
                        $input,
                    ),
                    inline_var_decl = format!(
                        "
                            _|begin
                            1|  var Foo: {};
                            _|end;
                            ---
                            1:InlineDeclaration
                        ",
                        $input,
                    ),
                    inline_var_assign = format!(
                        "
                            _|begin
                            1|  var Foo: {} := 1;
                            _|end;
                            ---
                            1:InlineDeclaration
                        ",
                        $input,
                    ),
                    const_decl = format!(
                        "
                            _|const
                            _|  Foo: {} = Bar;
                        ",
                        $input,
                    ),
                    inline_const = format!(
                        "
                            _|begin
                            1|  const Foo: {} = 1;
                            _|end;
                            ---
                            1:InlineDeclaration
                        ",
                        $input,
                    ),
                    parameter = format!("_|procedure Foo(A: {});", $input),
                );
            };
        }

        pub fn generate(root_dir: &Path) {
            generate_test_groups!(
                root_dir,
                test_group,
                array_of_const = "array of const",
                array_index_of_const = "array[0..1] of const",
                array_of_type = "array of Integer",
                array_index_of_type = "array[0..1] of Integer",
                set = "set of TFoo",
                file = "file",
                file_of = "file of Foo",
                pointer = "^TFoo",
                sub_range = "1..2",
                type_reference = "Integer",
                enum_type = "(Foo, Bar, Baz)",
            );
            generate_test_cases!(
                root_dir,
                type_decl = "
                    _|interface
                    _|type
                    _|  TFoo = record
                    _|    A: Integer;
                    _|  end;
                ",
                array_element_type = "
                    _|interface
                    _|type
                    _|  TFoo = array of record
                    _|    A: Integer;
                    _|  end;
                ",
                set_of_record = "
                    _|interface
                    _|type
                    _|  TFoo = set of record
                    _|    A: Integer;
                    _|  end;
                ",
                file_of_record = "
                    _|interface
                    _|type
                    _|  TFoo = file of record
                    _|    A: Integer;
                    _|  end;
                ",
                record_pointer = "
                    _|interface
                    _|type
                    _|  TFoo = ^record
                    _|    A: Integer;
                    _|  end;
                ",
                record_field = "
                    _|interface
                    _|type
                    _|  TFoo = class
                    _|    A: record
                    _|      B: Integer;
                    _|    end;
                    _|  end;
                ",
                record_var_decl = "
                    _|var
                    _|  Foo: record
                    _|    A: Integer;
                    _|  end;
                ",
                record_inline_var_decl = "
                    _|begin
                    _|  var Foo: record
                    _|    A: Integer;
                    _|  end;
                    _|end;
                ",
                const_decl = "
                    _|const
                    _|  Foo: record
                    _|    A: Integer;
                    _|  end = ();
                ",
            );
        }
    }
    mod contains_var_type {
        use super::*;

        macro_rules! test_group {
            ($root_dir: expr, $input: expr) => {
                crate::generate_test_cases!(
                    $root_dir,
                    array_of_const = format!(
                        "
                            _|interface
                            _|type
                            {}
                        ",
                        format!($input, "array of const"),
                    ),
                    array_of_type_reference = format!(
                        "
                            _|interface
                            _|type
                            {}
                        ",
                        format!($input, "array of Integer"),
                    ),
                    set_type = format!(
                        "
                            _|interface
                            _|type
                            {}
                        ",
                        format!($input, "set of Integer"),
                    ),
                    file = format!(
                        "
                            _|interface
                            _|type
                            {}
                        ",
                        format!($input, "file of Integer"),
                    ),
                    record = format!(
                        "
                            _|interface
                            _|type
                            {}
                        ",
                        format!(
                            $input,
                            indoc::indoc! {"
                            record
                            _|    A: Integer;
                            _|    B: Integer;
                            _|  end;
                        "}
                        ),
                    ),
                );
            };
        }

        pub fn generate(root_dir: &Path) {
            generate_test_groups!(
                root_dir,
                test_group,
                array_of = "_|  TFoo = array of {};",
                array_index_of = "_|  TFoo = array[0..1] of {};",
                set_of = "_|  TFoo = set of {};",
                file_of = "_|  TFoo = file of {};",
                pointer = "_|  PFoo = ^{};",
            );
        }
    }
    mod procedure {
        use super::*;

        macro_rules! test_group {
            ($root_dir: expr, $input: expr) => {
                crate::generate_test_cases!(
                    $root_dir,
                    no_args = format!(
                        "
                            _|interface
                            _|type
                            _|  {};
                        ",
                        format!($input, ""),
                    ),
                    untyped_var = format!(
                        "
                            _|interface
                            _|type
                            _|  {};
                        ",
                        format!($input, "(var Foo)"),
                    ),
                    untyped_const = format!(
                        "
                            _|interface
                            _|type
                            _|  {};
                        ",
                        format!($input, "(const Foo)"),
                    ),
                    one_param = format!(
                        "
                            _|interface
                            _|type
                            _|  {};
                        ",
                        format!($input, "(Foo: Integer)"),
                    ),
                    one_param_var = format!(
                        "
                            _|interface
                            _|type
                            _|  {};
                        ",
                        format!($input, "(var Foo: Integer)"),
                    ),
                    one_param_const = format!(
                        "
                            _|interface
                            _|type
                            _|  {};
                        ",
                        format!($input, "(const Foo: Integer)"),
                    ),
                    many_params = format!(
                        "
                            _|interface
                            _|type
                            _|  {};
                        ",
                        format!($input, "(Foo: Integer; Bar: Word)"),
                    ),
                    many_params_var = format!(
                        "
                            _|interface
                            _|type
                            _|  {};
                        ",
                        format!($input, "(var Foo: Integer; var Bar: Word)"),
                    ),
                    many_params_const = format!(
                        "
                            _|interface
                            _|type
                            _|  {};
                        ",
                        format!($input, "(const Foo: Integer; const Bar: Word)"),
                    ),
                );
            };
        }

        pub fn generate(root_dir: &Path) {
            generate_test_groups!(
                root_dir,
                test_group,
                function_of_object = "TFoo = function{}: Result of object",
                procedure_of_object = "TFoo = procedure{} of object",
                function_reference = "TFoo = reference to function{}: Result",
                procedure_reference = "TFoo = reference to procedure{}",
                simple_function = "TFoo = function{}: Result",
                simple_procedure = "TFoo = procedure{}",
            );
        }
    }
}

mod visibility_sections {
    use super::*;

    macro_rules! test_group {
        ($root_dir: expr, $input: expr) => {
            crate::generate_test_cases!(
                $root_dir,
                in_type = format!(
                    "
                        _|type
                        _|  TFoo = class
                        _|  {}
                        _|    procedure Foo;
                        _|  end;
                    ",
                    $input
                ),
                nested_type = format!(
                    "
                        _|type
                        _|  TFoo = class
                        _|    type
                        _|      TBar = class
                        _|      {}
                        _|        procedure Foo;
                        _|      end;
                        _|  end;
                    ",
                    $input
                ),
            );
        };
    }

    pub fn generate(root_dir: &Path) {
        generate_test_groups!(
            root_dir,
            test_group,
            strict_private = "strict private",
            private = "private",
            strict_protected = "strict protected",
            protected = "protected",
            public = "public",
            published = "published",
            automated = "automated",
        );
        generate_test_cases!(
            root_dir,
            all = "
                _|type
                _|  TFoo = class
                _|  strict private
                _|    procedure Foo;
                _|  private
                _|    procedure Foo;
                _|  strict protected
                _|    procedure Foo;
                _|  protected
                _|    procedure Foo;
                _|  public
                _|    procedure Foo;
                _|  published
                _|    procedure Foo;
                _|  automated
                _|    procedure Foo;
                _|  end;
            ",
        );
    }
}

mod prop_decls {
    use super::*;

    macro_rules! test_group {
        ($root_dir: expr, $input: expr) => {
            crate::generate_test_cases!(
                $root_dir,
                naked = format!(
                    "
                        _|type
                        _|  TFoo = class
                        1|    property Foo {0};
                        2|    property Foo {0}
                        _|  end;
                        ---
                        1:PropertyDeclaration
                        2:PropertyDeclaration
                    ",
                    $input
                ),
                visibility = format!(
                    "
                        _|type
                        _|  TFoo = class
                        _|  public
                        1|    property Foo {0};
                        2|    property Foo {0}
                        _|  end;
                        ---
                        1:PropertyDeclaration
                        2:PropertyDeclaration
                    ",
                    $input
                ),
                class_naked = format!(
                    "
                        _|type
                        _|  TFoo = class
                        1|    class property Foo {0};
                        2|    class property Foo {0}
                        _|  end;
                        ---
                        1:PropertyDeclaration
                        2:PropertyDeclaration
                    ",
                    $input
                ),
                class_visibility = format!(
                    "
                        _|type
                        _|  TFoo = class
                        _|  public
                        1|    class property Foo {0};
                        2|    class property Foo {0}
                        _|  end;
                        ---
                        1:PropertyDeclaration
                        2:PropertyDeclaration
                    ",
                    $input
                ),
            );
        };
    }

    pub fn generate(root_dir: &Path) {
        generate_test_groups!(
            root_dir,
            test_group,
            republish = "",
            read = "read A",
            read_typed = ": Integer read A",
            read_write = "read A write A",
            read_write_typed = ": TFoo read A write A",
            read_write_typed_generic = ": TFoo<T> read A write A",
            default_property = "; default",
            default_value = "default True",
            default_value_typed = ": Boolean default True",
            stored = "stored True",
            index = "index 1",
            index_typed = ": TFoo index 1",
            dispid = "dispid 0",
            dispid_typed = ": Integer dispid 0",
            implements = "implements IFoo",
            implements_typed = ": TFoo implements IFoo",
            no_default = "nodefault",
            no_default_typed = ": Integer nodefault",
            readonly = "readonly",
            readonly_typed = ": Integer readonly",
            writeonly = "writeonly",
            writeonly_typed = ": Integer writeonly",
            indexed = "[I: Integer]",
            indexeds = "[I: Integer; J: Integer]",
            indexeds_typed = "[I: Integer]: TFoo",
            const_indexed = "[const I: Integer]",
            const_indexeds = "[const I: Integer; const J: Integer]",
            const_indexeds_typed = "[const I: Integer; const J: Integer]: Integer",
            indexed_read = "[I: Integer] read A",
            const_indexed_read = "[const I: Integer] read A",
            indexed_write = "[I: Integer] write A",
            const_indexed_write = "[const I: Integer] write A",
            indexed_read_write = "[I: Integer] read A",
            const_indexed_read_write = "[const I: Integer] read A write A",
            const_indexed_read_write_typed = "[const I: Integer]: Integer read A write A",
        );
    }
}

mod routine_headers {
    use super::*;

    pub fn generate(root_dir: &Path) {
        params::generate(root_dir);
        generics::generate(root_dir);
        directives::generate(root_dir);
    }

    mod params {
        use super::*;

        macro_rules! test_group {
            ($root_dir: expr, $input: expr) => {
                crate::generate_test_cases!(
                    $root_dir,
                    naked = format!(
                        "
                            1|{} Foo;
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                    empty_params = format!(
                        "
                            1|{} Foo();
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                    one_param = format!(
                        "
                            1|{} Foo(A: Integer);
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                    one_param_var = format!(
                        "
                            1|{} Foo(var A: Integer);
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                    one_param_const = format!(
                        "
                            1|{} Foo(const A: Integer);
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                    one_param_untyped_var = format!(
                        "
                            1|{} Foo(var A);
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                    one_param_untyped_const = format!(
                        "
                            1|{} Foo(const A);
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                    many_params = format!(
                        "
                            1|{} Foo(A: Integer; B: Integer);
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                    empty_params_result = format!(
                        "
                            1|{} Foo(): TFoo;
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                    one_param_result = format!(
                        "
                            1|{} Foo(A: Integer): TFoo;
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                    many_params_result = format!(
                        "
                            1|{} Foo(A: Integer; B: Integer): TFoo;
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                    many_params_result_var = format!(
                        "
                            1|{} Foo(var A: Integer; var B: Integer): TFoo;
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                    many_params_result_untyped_var = format!(
                        "
                            1|{} Foo(var A; var B): TFoo;
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                    many_params_result_const = format!(
                        "
                            1|{} Foo(const A: Integer; const B: Integer): TFoo;
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                    many_params_result_untyped_const = format!(
                        "
                            1|{} Foo(const A; const B): TFoo;
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                );
            };
        }

        pub fn generate(root_dir: &Path) {
            generate_test_groups!(
                root_dir,
                test_group,
                function = "function",
                class_function = "class function",
                procedure = "procedure",
                class_procedure = "class procedure",
                constructor = "constructor",
                destructor = "destructor",
            );
        }
    }
    mod generics {
        use super::*;

        macro_rules! test_group {
            ($root_dir: expr, $input: expr) => {
                crate::generate_test_cases!(
                    $root_dir,
                    empty_list = format!( // This is invalid code
                        "
                            1|{} Foo<>;
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                    comma_names = format!(
                        "
                            1|{} Foo<T, U, V>;
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                    semi_names = format!(
                        "
                            1|{} Foo<T; U; V>;
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                    constraints = format!(
                        "
                            1|{} Foo<T: record; U: constructor; V: object>;
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                    comma_and_semi_names = format!(
                        "
                            1|{} Foo<T, U: record; V, W: constructor>();
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                    with_param_list = format!(
                        "
                            1|{} Foo<T, U: record; V, W: constructor>(A: Integer; B: String);
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                    generic_params = format!(
                        "
                            1|{} Foo<T, U: record; V, W: constructor>(
                                  A: TDictionary<String, Integer>;
                                  B: TFoo<T, U, V>
                              );
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                );
            };
        }

        pub fn generate(root_dir: &Path) {
            generate_test_groups!(
                root_dir,
                test_group,
                function = "function",
                class_function = "class function",
                procedure = "procedure",
                class_procedure = "class procedure",
            );
        }
    }

    mod directives {
        use super::*;

        macro_rules! test_group {
            ($root_dir: expr, $input: expr) => {
                crate::generate_test_cases!(
                    $root_dir,
                    single = format!(
                        "
                            _|interface
                            1|function Foo(): TFoo; {};
                            ---
                            1:RoutineHeader
                        ",
                        $input
                    ),
                    multiple = format!(
                        "
                            _|interface
                            1|function Foo(): TFoo; {0};
                            2|function Foo(): TFoo; {0};
                            ---
                            1:RoutineHeader
                            2:RoutineHeader
                        ",
                        $input
                    ),
                );
            };
        }

        pub fn generate(root_dir: &Path) {
            generate_test_groups!(
                root_dir,
                test_group,
                overload = "overload",
                reintroduce = "reintroduce",
                varargs = "varargs",
                unsafe_directive = "unsafe",
                message = "message",
                static_directive = "static",
                dynamic = "dynamic",
                override_directive = "override",
                virtual_directive = "virtual",
                abstract_directive = "abstract",
                final_directive = "final",
                inline = "inline",
                assembler = "assembler",
                cdecl = "cdecl",
                pascal = "pascal",
                register = "register",
                safecall = "safecall",
                stdcall = "stdcall",
                export = "export",
                far = "far",
                local = "local",
                near = "near",
                deprecated = "deprecated",
                experimental = "experimental",
                platform = "platform",
                library = "library",
                dispid = "dispid",
                name = "name",
                index = "index",
                delayed = "delayed",
                reintroduce_and_overload = "reintroduce; overload",
                repeated_overload = "overload; overload; overload; overload",
                repeated_overload_no_semicolon = "overload overload overload overload",
                reintroduce_and_overload_and_forward = "reintroduce; overload; forward",
            );
            generate_test_cases!(
                root_dir,
                forward = "
                    _|procedure Foo; forward;
                    _|procedure Bar;
                    _|begin
                    _|end
                ",
                forward_sub_routine = "
                    _|procedure Bar;
                    _|  procedure Foo; forward;
                    _|  procedure Foo;
                    _|  begin
                    _|  end
                    _|begin
                    _|end
                ",
                external = "
                    _|procedure Foo; external;
                    _|procedure Bar;
                    _|begin
                    _|end
                ",
                external_sub_routine = "
                    _|procedure Bar;
                    _|  procedure Foo; external;
                    _|  procedure Baz;
                    _|  begin
                    _|  end
                    _|begin
                    _|end
                ",
            );
        }
    }
}

mod routine_implementations {
    use super::*;

    pub fn generate(root_dir: &Path) {
        generate_test_cases!(
            root_dir,
            begin = "
                _|procedure Foo;
                _|begin
                _|  Foo;
                _|end;
            ",
            asm = "
                _|procedure Foo;
                _|asm
                _|  XOR EAX, EAX
                _|end;
            ",
            sub_begin = "
                _|procedure Foo;
                _|  procedure SubRoutine;
                _|  begin
                _|    Bar;
                _|  end;
                _|begin
                _|  Foo;
                _|end;
            ",
            sub_asm = "
                _|procedure Foo;
                _|  procedure SubRoutine;
                _|  asm
                _|    XOR EAX, EAX
                _|  end;
                _|asm
                _|  XOR EAX, EAX
                _|end;
            ",
            sub_asm_begin = "
                _|procedure Foo;
                _|  procedure SubRoutine;
                _|  asm
                _|    XOR EAX, EAX
                _|  end;
                _|begin
                _|  SubRoutine;
                _|end;
            ",
            sub_begin_asm = "
                _|procedure Foo;
                _|  procedure SubRoutine;
                _|  begin
                _|    Foo;
                _|  end;
                _|asm
                _|  XOR EAX, EAX
                _|end;
            ",
        );
    }
}

mod control_flows {
    use super::*;

    pub fn generate(root_dir: &Path) {
        if_else::generate(root_dir);
        while_with_for::generate(root_dir);
        case::generate(root_dir);
        try_except_finally::generate(root_dir);
        repeat::generate(root_dir);
    }

    mod if_else {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                statement = "
                    _  |begin
                    _1 |  if True then{1}
                    _^1|    Foo := Bar;
                    _  |end;
                ",
                compound = "
                    _  |begin
                    _1 |  if True then{1}
                    _^1|    begin
                    _^1|      Foo := Bar;
                    _^1|    end;
                    _  |end;
                ",
                if_if = "
                    _   |begin
                    _1  |  if True then{1}
                    _2^1|    if True then{2}
                    _^2 |      Foo := Bar;
                    _   |end;
                ",
                if_label_compound = "
                    _   |begin
                    _1  |  if True then{1}
                    _^1 |    lab:
                    _^1 |    begin
                    _^1 |      Foo := Bar;
                    _^1 |    end;
                    _   |end;
                ",
                if_if_compound = "
                    _   |begin
                    _1  |  if True then{1}
                    _2^1|    if True then{2}
                    _^2 |      begin
                    _^2 |        Foo := Bar;
                    _^2 |      end;
                    _   |end;
                ",
                if_compound_if = "
                    _   |begin
                    _1  |  if True then{1}
                    _^1 |    begin
                    _2^1|      if True then{2}
                    _^2 |        Foo := Bar
                    _^1 |    end;
                    _   |end;
                ",
                if_compound_if_semicolon = "
                    _   |begin
                    _1  |  if True then{1}
                    _^1 |    begin
                    _2^1|      if True then{2}
                    _^2 |        Foo := Bar;
                    _^1 |    end;
                    _   |end;
                ",
                statement_else_statement = "
                    _  |begin
                    _1 |  if True then{1}
                    _^1|    Foo := Bar
                    _1 |  else{2}
                    _^2|    Bar := Foo;
                    _  |end;
                ",
                compound_else_compound = "
                    _  |begin
                    _1 |  if True then{1}
                    _^1|    begin
                    _^1|      Foo := Bar;
                    _^1|    end
                    _1 |  else{2}
                    _^2|    begin
                    _^2|      Baz := Flarp;
                    _^2|    end;
                    _  |end;
                ",
                statement_else_if = "
                    _   |begin
                    _1  |  if True then{1}
                    _^1 |    Foo := Bar
                    _1  |  else{2}
                    _2^2|    if True then{3}
                    _^3 |      Bar := Foo;
                    _   |end;
                ",
                statement_else_if_else = "
                    _   |begin
                    _1  |  if True then{1}
                    _^1 |    Foo := Bar
                    _1  |  else{2}
                    _2^2|    if True then{3}
                    _^3 |      Bar := Foo
                    _2^2|    else{4}
                    _^4 |      Foo := Flarp;
                    _   |end;
                ",
                if_if_dangling_else = "
                    _   |begin
                    _1  |  if True then{1}
                    _2^1|    if True then{2}
                    _^2 |      Foo := Bar
                    _2^1|    else{3}
                    _^3 |      Bar := Foo;
                    _   |end;
                ",
                anonymous_clause = "
                    _  |begin
                    1  |  if Foo(procedure begin{1}
                    _^1|    Foo := Bar;
                    _^1|    Result := True;
                    1  |  end) then{1}
                    _^1|    begin
                    _^1|      Foo := Bar;
                    _^1|    end;
                    _  |end;
                ",
                anonymous_clause_nested = "
                    _  |begin
                    1  |  if Foo(procedure begin{1}
                    _^1|    Foo := Foo;
                    1  |  end) then{2}
                    _^2|    begin
                    2^2|      if Bar(procedure begin{3}
                    _^3|        Bar := Bar;
                    2  |      end) then{4}
                    _^4|        Baz := Baz
                    _^2|    end;
                    _  |end;
                ",
            );
        }
    }

    mod while_with_for {
        use super::*;

        macro_rules! test_group {
            ($root_dir: expr, $($input: expr),* $(,)?) => {
                crate::generate_test_cases!(
                    $root_dir,
                    simple_compound = format!(
                        "
                            _  |begin
                            _1 |  {} True {}{{1}}
                            _^1|    begin
                            _^1|      Foo := Bar;
                            _^1|    end;
                            _  |end;
                        ",
                        $($input,)*
                    ),
                    anonymous_clause = format!(
                        "
                            _  |begin
                            _1 |  {} Foo(procedure begin{{1}}
                            _^1|    Foo := Bar;
                            _^1|    Result := True;
                            1  |  end) {}{{2}}
                            _^2|    begin
                            _^2|      Foo := Bar;
                            _^2|    end;
                            _  |end;
                        ",
                        $($input,)*
                    ),
                    inline = format!(
                        "
                            _  |begin
                            _1 |  {} True {}{{1}}
                            _^1|    Foo := Bar;
                            _  |end;
                        ",
                        $($input,)*
                    ),
                    inline_nested = format!(
                        "
                            _   |begin
                            _1  |  {0} True {1}{{1}}
                            _2^1|    {0} True {1}{{2}}
                            _^2 |      Foo := Bar;
                            _   |end;
                        ",
                        $($input,)*
                    ),
                );
            };
        }

        pub fn generate(root_dir: &Path) {
            generate_test_groups!(
                root_dir,
            test_group,
                while_statement = "while"; "do",
                with_statement = "with"; "do",
                for_in_statement = "for A in"; "do",
                for_to_statement = "for A :="; "to False do",
                for_downto_statement = "for A :="; "downto False do",
                for_to_statement_alt = "for A := False to"; "do",
                for_downto_statement_alt = "for A := False downto"; "do",
            );
        }
    }

    mod case {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                no_else = "
                    1  |case True of
                    2  |  Foo:{1}
                    _^1|    ;
                    3  |  Bar:{2}
                    _^2|    ;
                    _  |end;
                    ---
                    1:CaseHeader
                    2:CaseArm
                    3:CaseArm
                ",
                no_else_multiple_matches = "
                    1  |case True of
                    2  |  Foo, Bar:{1}
                    _^1|    ;
                    3  |  Baz..Bazz, Flarp..Flarpp:{2}
                    _^2|    ;
                    _  |end;
                    ---
                    1:CaseHeader
                    2:CaseArm
                    3:CaseArm
                ",
                no_else_compound = "
                    1  |case True of
                    2  |  Foo:{1}
                    _^1|    begin
                    _^1|      A;
                    _^1|    end;
                    3  |  Bar:{2}
                    _^2|    begin
                    _^2|      B;
                    _^2|    end;
                    _  |end;
                    ---
                    1:CaseHeader
                    2:CaseArm
                    3:CaseArm
                ",
                else_block = "
                    1|case True of
                    2  |  Foo:{1}
                    _^1|    ;
                    3  |  Bar:{2}
                    _^2|    ;
                    _|else
                    _|  A;
                    _|end;
                    ---
                    1:CaseHeader
                    2:CaseArm
                    3:CaseArm
                ",
                else_block_compound = "
                    1|case True of
                    2  |  Foo:{1}
                    _^1|    ;
                    3  |  Bar:{2}
                    _^2|    ;
                    _|else
                    _|  begin
                    _|    Baz;
                    _|  end;
                    _|end;
                    ---
                    1:CaseHeader
                    2:CaseArm
                    3:CaseArm
                ",
                else_compound = "
                    1  |case True of
                    2  |  Foo:{1}
                    _^1|    begin
                    _^1|      A;
                    _^1|    end;
                    3  |  Bar:{2}
                    _^2|    begin
                    _^2|      B;
                    _^2|    end;
                    _  |else
                    _  |  A;
                    _  |end;
                    ---
                    1:CaseHeader
                    2:CaseArm
                    3:CaseArm
                ",
                anonymous_subject = "
                    1  |case Foo(procedure begin{1}
                    _^1|  Bar;
                    _^1|  Baz;
                    1  |end) of
                    2  |  Foo:{2}
                    _^2|    begin
                    _^2|      A;
                    _^2|    end;
                    _  |end;
                    ---
                    1:CaseHeader
                    2:CaseArm
                ",
            );
        }
    }

    mod try_except_finally {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                except = "
                    _|try
                    _|  A;
                    _|except
                    _|end;
                ",
                except_on = "
                    _  |try
                    _  |  A;
                    _  |except
                    _1 |  on Exception do{1}
                    _^1|    A;
                    _  |end;
                ",
                except_on_multiple = "
                    _  |try
                    _  |  A;
                    _  |except
                    _1 |  on TFooException do{1}
                    _^1|    B;
                    _2 |  on Exception do{2}
                    _^2|    A;
                    _  |end;
                ",
                except_on_compound = "
                    _  |try
                    _  |  A;
                    _  |except
                    _1 |  on Exception do{1}
                    _^1|    begin
                    _^1|      A;
                    _^1|    end;
                    _  |end;
                ",
                except_on_compound_multiple = "
                    _  |try
                    _  |  A;
                    _  |except
                    _1 |  on TFooException do{1}
                    _^1|    begin
                    _^1|      A;
                    _^1|    end;
                    _2 |  on Exception do{2}
                    _^2|    begin
                    _^2|      B;
                    _^2|    end;
                    _  |end;
                ",
                except_else = "
                    _|try
                    _|  A;
                    _|except
                    _|else
                    _|  B;
                    _|  C;
                    _|end;
                ",
                except_on_else = "
                    _  |try
                    _  |  A;
                    _  |except
                    _1 |  on TFooException do{1}
                    _^1|    A;
                    _  |else
                    _  |  B;
                    _  |  C;
                    _  |end;
                ",
                finally = "
                    _|try
                    _|  A;
                    _|finally
                    _|  B;
                    _|end;
                ",
                nested = "
                    _|try
                    _|  try
                    _|    A;
                    _|  except
                    _|    B;
                    _|  end;
                    _|finally
                    _|  C;
                    _|end;
                ",
            );
        }
    }

    mod repeat {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                simple = format!(
                    "
                        _|begin
                        _|  repeat
                        _|    Foo := Bar;
                        _|  until True;
                        _|end;
                    ",
                ),
                anonymous_clause = format!(
                    "
                        _  |begin
                        _  |  repeat
                        _  |    Foo := Bar;
                        1  |  until Foo(procedure begin{{1}}
                        _^1|    Foo := Bar;
                        _^1|    Result := True;
                        1  |  end);
                        _  |end;
                    ",
                ),
                nested = format!(
                    "
                        _|begin
                        _|  repeat
                        _|    begin
                        _|      repeat
                        _|        Foo := Bar;
                        _|      until True;
                        _|    end;
                        _|  until True;
                        _|end;
                    ",
                ),
            );
        }
    }
}

mod statements {
    use super::*;

    pub fn generate(root_dir: &Path) {
        with_expression::generate(root_dir);
        without_expression::generate(root_dir);
        compound::generate(root_dir);
        raise::generate(root_dir);
    }

    mod with_expression {
        use super::*;

        macro_rules! test_group {
            ($root_dir: expr, $expression: expr) => {
                crate::generate_test_cases!(
                    $root_dir,
                    individual = format!("_|{}", $expression),
                    in_block = format!(
                        "
                            _|begin
                            _|  {}
                            _|end;
                        ",
                        $expression,
                    ),
                    multiple = format!(
                        "
                            _|{0};
                            _|{0}
                        ",
                        $expression,
                    ),
                    multiple_in_block = format!(
                        "
                            _|begin
                            _|  {0};
                            _|  {0}
                            _|end
                        ",
                        $expression,
                    ),
                );
            };
        }

        pub fn generate(root_dir: &Path) {
            // These tests are asserting that expression can be present in the given portions
            // The expression parsing tests are in the utility_tests
            generate_test_groups!(
                root_dir,
                test_group,
                assign_l = "A := A + A * B",
                assign_r = "A.B[C + D] := 0",
                assign = "A.B[C + D] := E + F mod G",
                anonymous_proc = "A := procedure begin end",
                anonymous_func = "A := function: Boolean begin end",
            );
        }
    }

    mod without_expression {
        use super::*;

        macro_rules! test_group {
            ($root_dir: expr, $input: expr) => {
                crate::generate_test_cases!(
                    $root_dir,
                    single = format!(
                        "
                            _|begin
                            _|  {}
                            _|end;
                        ",
                        $input,
                    ),
                    multiple = format!(
                        "
                            _|begin
                            _|  {0};
                            _|  {0}
                            _|end
                        ",
                        $input,
                    ),
                );
            };
        }

        pub fn generate(root_dir: &Path) {
            generate_test_groups!(
                root_dir,
                test_group,
                goto_i = "goto Foo",
                goto_d = "goto 1",
                goto_h = "goto $1",
                goto_b = "goto %1",
                label_i = "Foo:",
                label_d = "1111:",
                label_h = "$1111:",
                label_b = "%1111:",
            );
            generate_test_cases!(
                root_dir,
                multi_label = "
                    _|begin
                    _|  Foo:
                    _|  111:
                    _|  $111:
                    _|  %111:
                    _|end
                ",
                i_label_statement = "
                    _|begin
                    _|  Foo:
                    _|  Bar();
                    _|end
                ",
                d_label_statement = "
                    _|begin
                    _|  111:
                    _|  Bar();
                    _|end
                ",
                b_label_statement = "
                    _|begin
                    _|  $111:
                    _|  Bar();
                    _|end
                ",
                h_label_statement = "
                    _|begin
                    _|  %111:
                    _|  Bar();
                    _|end
                ",
                label_if_statement = "
                    _  |begin
                    _  |  %111:
                    _1 |  if A then{1}
                    _^1|    B;
                    _  |end
                ",
                other_contexts = "
                    _|begin
                    _|  repeat
                    _|    111:
                    _|  until True;
                    _|  try
                    _|    111:
                    _|  except
                    _|    111:
                    _|  end;
                    _|end;
                    _|initialization
                    _|  Foo:
                    _|  111:
                    _|  $111:
                    _|  %111:
                    _|finalization
                    _|  Foo:
                    _|  111:
                    _|  $111:
                    _|  %111:
                ",
            );
        }
    }
    mod compound {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                empty_begin = "
                    _|begin
                    _|end
                ",
                empty_asm = "
                    _|asm
                    _|end
                ",
                begin = "
                    _|begin
                    _|  Foo;
                    _|  Bar
                    _|end
                ",
                asm = "
                    _|asm
                    _|  XOR EAX, EAX
                    _|  XOR EBX, EBX
                    _|end
                ",
                nested_begin = "
                    _|begin
                    _|  begin
                    _|    Foo;
                    _|    Bar
                    _|  end;
                    _|  begin
                    _|    Foo;
                    _|    Bar
                    _|  end
                    _|end
                ",
                nested_asm = "
                    _|begin
                    _|  asm
                    _|    XOR EAX, EAX
                    _|    XOR EBX, EBX
                    _|  end;
                    _|  asm
                    _|    XOR EAX, EAX
                    _|    XOR EBX, EBX
                    _|  end
                    _|end
                ",
            );
        }
    }
    mod raise {
        use super::*;

        macro_rules! test_group {
            ($root_dir: expr, $input: expr) => {
                crate::generate_test_cases!(
                    $root_dir,
                    in_except = format!(
                        "
                            _|try
                            _|except
                            _|  {};
                            _|end;
                        ",
                        $input,
                    ),
                    in_compound = format!(
                        "
                            _|begin
                            _|  {};
                            _|end;
                        ",
                        $input,
                    ),
                );
            };
        }

        pub fn generate(root_dir: &Path) {
            generate_test_groups!(
                root_dir,
                test_group,
                naked_raise = "raise",
                raise = "raise Exception.Create()",
                raise_at_l = "raise Excpetion.Create() at ReturnAddress",
                raise_at_r = "raise E at ReturnAddress + Foo",
                raise_at = "raise Exception.Create() at ReturnAddress + Foo",
            );
        }
    }
}

mod attributes {
    use super::*;

    pub fn generate(root_dir: &Path) {
        generate_test_cases!(
            root_dir,
            top_level = "
                1|[Attribute]
                _|type
                2|  [Attribute]
                _|  TFoo = class;
                3|[Attribute]
                _|procedure Foo;
                ---
                1:Attribute
                2:Attribute
                3:Attribute
            ",
            multiple_distinct = "
                1|[Attribute(ABC, DEF)]
                2|[Attribute([1, 2, 3])]
                _|procedure Foo;
                ---
                1:Attribute
                2:Attribute
            ",
            multiple_comma = "
                1|[Attribute(ABC, DEF), Attribute([1, 2, 3])]
                _|procedure Foo;
                ---
                1:Attribute
            ",
            type_def = "
                _|type
                _|  Foo = class
                1|    [Weak]
                _|    FField: Integer;
                _|  end;
                ---
                1:Attribute
            ",
            type_def_guid = "
                _|type
                _|  Foo = class
                1|    ['0000...']
                _|    FField: Integer;
                _|  end;
                ---
                1:Guid
            ",
            params = "
                1|procedure Foo([ref] Bar: Integer; [boo] Baz: String);
                ---
                1:RoutineHeader
            ",
            fp_array = "_|Foo([a, b, c], [d, e, f]);",
            inside_if_then = "
                _1 |if [Foo] Bar and Baz then{1}
                _^1|  Action;
            ",
            inside_while_do = "
                _1 |while [Foo] Bar and Baz do{1}
                _^1|  Action;
            ",
        );
    }
}

mod semicolons {
    use super::*;

    macro_rules! test_group {
        ($root_dir: expr, $input: expr) => {
            super::generate_test_cases!(
                $root_dir,
                with_semicolon = format!($input, ";"),
                without_semicolon = format!($input, ""),
            );
        };
    }

    pub fn generate(root_dir: &Path) {
        generate_test_groups!(
            root_dir,
            test_group,
            compound_statement = "
                _|begin
                _|  Foo(){}
                _|end;
            ",
            raise = "
                _|begin
                _|  raise{}
                _|end;
            ",
            inline_if = "
                _  |begin
                _1 |  if True then{{1}}
                _^1|    A{}
                _  |end;
            ",
            inline_while_loop = "
                _  |begin
                _1 |  while True do{{1}}
                _^1|    A{}
                _  |end;
            ",
            inline_for_loop = "
                _  |begin
                _1 |  for A in B do{{1}}
                _^1|    A{}
                _  |end;
            ",
            repeat_until = "
                _|begin
                _|  repeat
                _|    A{0}
                _|  until True{0}
                _|end;
            ",
            try_except = "
                _  |try
                _1 |  for A in B do{{1}}
                _^1|    A{0}
                _  |except
                _2 |  on E do{{2}}
                _^2|    B{0}
                _  |else
                _  |  B{0}
                _  |end;
            ",
            try_finally = "
                _  |try
                _1 |  for A in B do{{1}}
                _^1|    A{0}
                _  |finally
                _  |  B{0}
                _  |end;
            ",
            case = "
                _  |case A of
                _1 |  B:{{1}}
                _^1|    C{0}
                _  |else
                _  |  D{0}
                _  |end;
            ",
            anonymous_impl = "
                _1 |A := procedure
                1  |begin{{1}}
                _^1|  Foo{}
                1  |end;
            ",
        );
    }
}

mod regression {
    use super::*;

    pub fn generate(root_dir: &Path) {
        generate_test_cases!(
            root_dir,
            unterminated_param_list = "
                _|procedure F(
            ",
            visibility_keywords_outside_type_decl = "
                _|const
                _|  Strict = 0;
                _|  Automated = 0;
                _|  Private = 0;
                _|  Protected = 0;
                _|  Public = 0;
                _|  Published = 0;
            "
        );
    }
}
