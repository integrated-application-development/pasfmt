mod suites;

datatest_stable::harness!(
    suites::logical_line_parser::test_file,
    "generated/logical_line_test",
    r"^.*/*",
    suites::optimising_line_formatter::test_file,
    "generated/optimising_line_formatter",
    r"^.*/*",
);
