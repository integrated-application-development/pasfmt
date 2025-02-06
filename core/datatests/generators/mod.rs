pub mod logical_line_parser;
pub mod optimising_line_formatter;

#[allow(dead_code)]
const FILE_SEPARATOR: &str = "!#################################!";

#[macro_export]
macro_rules! get_dir_from_module {
    () => {{
        let module_name = std::path::Path::new(file!())
            .file_stem()
            .unwrap()
            .to_string_lossy();
        module_path!()
            .split("::")
            .skip_while(|module| module != &module_name)
            .skip(1)
            .fold(std::path::PathBuf::new(), |acc, element| acc.join(element))
    }};
}

#[macro_export]
macro_rules! generate_test_cases {
    (# $dir: expr; $name: ident = { $input: expr $(, $output: expr )? $(,)? }) => {
        $crate::generate_test_cases!(# $dir; $name = { $input $(, $output)? ,}, )
    };
    (# $dir: expr; $name: ident = { $input: expr $(, $output: expr )? $(,)? }, $($rest: tt)*) => {
        {
            let dir = $dir;
            std::fs::create_dir_all(&dir)
                .expect(&format!("failed to create all directories: {}", &dir.display()));
            let file_name = dir.join(stringify!($name));
            if file_name.exists() {
                panic!("Test with name {} already exists at {}", stringify!($name), file_name.display());
            }
            let mut contents = String::new();
            contents.push_str(&$input);
            $(
                contents.push_str($crate::generators::FILE_SEPARATOR);
                contents.push_str(&$output);
            )?
            std::fs::write(&file_name, contents)
                .expect(&format!("failed to write file `{}` for test", &file_name.display()));
        }
        $crate::generate_test_cases!(# $dir; $($rest)*)
    };
    (# $dir: expr; $name: ident = $input: expr) => {
        $crate::generate_test_cases!(# $dir; $name = { $input },);
    };
    (# $dir: expr; $name: ident = $input: expr, $($rest: tt)*) => {
        $crate::generate_test_cases!(# $dir; $name = { $input }, $($rest)*);
    };
    (# $dir: expr;) => {};
    ($root_dir: expr, $($rest: tt)*) => {
        $crate::generate_test_cases!(# $root_dir.join($crate::get_dir_from_module!()); $($rest)*);
    };
}
