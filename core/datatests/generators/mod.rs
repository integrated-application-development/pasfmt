pub mod logical_line_parser;

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
    ($root_dir: expr, $($name: ident = $input: expr),* $(,)?) => {
        let dir = $root_dir.join($crate::get_dir_from_module!());
        $(
            {
                std::fs::create_dir_all(&dir)
                    .expect(&format!("failed to create all directories: {}", &dir.display()));
                let file_name = dir.join(stringify!($name));
                if file_name.exists() {
                    panic!("Test with name {} already exists at {}", stringify!($name), file_name.display());
                }
                std::fs::write(&file_name, $input)
                    .expect(&format!("failed to write file `{}` for test", &file_name.display()));
            }
        )*
    };
}
