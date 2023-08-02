#![cfg(test)]

macro_rules! formatter_test_group {
    ($i:ident, $($case_name: ident = {$($e: expr),* $(,)?}),* $(,)?) => {
        #[yare::parameterized(
          $($case_name = { $($e),* }),*
        )]
        fn $i(input: &str, output: &str) {
            let formatter: Formatter = formatter();
            let formatted_output = formatter.format(input);
            assert_that(&formatted_output).is_equal_to(output.to_string());
        }
    };
}

pub(crate) use formatter_test_group;
