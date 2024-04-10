mod suites;

datatest_stable::harness!(
    suites::logical_line_parser::test_file,
    "generated/logical_line_test",
    r"^.*/*",
);
