//! Parsing utility for conditional directives. Used for generating a series of "passes" over a slice of tokens.
//!
//! Use [DirectiveTree::parse] to get started.
use std::ops::Range;

use itertools::Itertools;

use crate::prelude::*;
use RawTokenType as TT;

/// Represents a portion of source code as a kind of heterogeneous tree, where each "node" is either
/// a contiguous string of tokens ("flat"), or a series of subtrees representing a complete section
/// of conditionally-defined code ("nested").
///
/// As an example, consider the following code
/// ```delphi
/// A;             // flat section
/// {$ifdef FOO}   // nested section
/// B;             //  | flat section
///   {$ifdef BAR} //  | nested section
/// C;             //  |  | flat section
///   {$endif}     //  |  |
/// {$elseif FOO}  //  |
/// D;             //  | flat section
/// E;             //  |  |
/// {$endif}       //  |
/// F;             // flat section
/// ```
///
/// This tree can be used to iterate simplified versions of the code (without the conditional directives).
/// Each "pass" corresponds to a particular set of choices for each conditional directive (i.e. take
/// branch 1 at directive 1, and branch 2 at directive 2, etc.). You can think of this as iterating
/// "real" versions of the code that the compiler could potential see, given the right configuration.
///
/// Use [DirectiveTree::passes] to create an iterator for the passes.
pub struct DirectiveTree {
    sections: Vec<Section>,
}

enum Section {
    Flat { explored: bool, range: Range<usize> },
    Nested(Vec<DirectiveTree>),
}

impl DirectiveTree {
    /// Creates a [DirectiveTree] representing the provided slice of tokens.
    pub fn parse(tokens: &[RawToken]) -> Self {
        let mut tokens = tokens.iter().map(RawToken::get_token_type).enumerate();
        Self::parse_next(true, &mut tokens).0
    }

    fn parse_next(
        top_level: bool,
        tokens: &mut impl Iterator<Item = (usize, RawTokenType)>,
    ) -> (Self, Option<ConditionalDirectiveKind>) {
        let mut tree = DirectiveTree { sections: vec![] };
        loop {
            let (flat, cdk) = Section::parse_flat(tokens);
            tree.sections.push(flat);
            match cdk {
                Some(cdk) if cdk.is_if() => {
                    let nested = Section::parse_nested(tokens);
                    tree.sections.push(nested);
                }
                // ignore any unmatched directives at the top level
                Some(_) if top_level => {}
                None | Some(_) => return (tree, cdk),
            }
        }
    }
}

impl Section {
    fn parse_flat(
        tokens: &mut impl Iterator<Item = (usize, RawTokenType)>,
    ) -> (Self, Option<ConditionalDirectiveKind>) {
        let mut start = None;
        let mut range = 0..0;
        let mut ending_cdk = None;

        for (idx, tt) in tokens {
            if let TT::ConditionalDirective(cdk) = tt {
                ending_cdk = Some(cdk);
                break;
            };
            range = (*start.get_or_insert(idx))..(idx + 1);
        }

        (
            Section::Flat {
                explored: false,
                range,
            },
            ending_cdk,
        )
    }

    fn parse_nested(tokens: &mut impl Iterator<Item = (usize, RawTokenType)>) -> Self {
        let (if_tree, mut cdk) = DirectiveTree::parse_next(false, tokens);
        let mut sections = vec![if_tree];
        while matches!(cdk, Some(cdk) if cdk.is_else()) {
            let (tree, next) = DirectiveTree::parse_next(false, tokens);
            sections.push(tree);
            cdk = next;
        }
        Section::Nested(sections)
    }
}

impl DirectiveTree {
    fn explored(&self) -> bool {
        self.sections.iter().all(|s| s.explored())
    }

    /// Creates an iterator for the conditional directive passes.
    pub fn passes(self) -> PassIter {
        PassIter {
            tree: self,
            exhausted: false,
        }
    }

    fn pass(&mut self, pass: &mut Vec<usize>) {
        for section in &mut self.sections {
            section.pass(pass);
        }
    }
}

impl Section {
    fn pass(&mut self, pass: &mut Vec<usize>) {
        match self {
            Section::Flat {
                explored,
                range: tokens,
            } => {
                pass.extend(tokens.clone());
                *explored = true;
            }
            Section::Nested(sections) => {
                if let Some(tree) = sections.iter_mut().find_or_last(|g| !g.explored()) {
                    tree.pass(pass);
                }
            }
        }
    }

    fn explored(&self) -> bool {
        match self {
            Section::Flat { explored, .. } => *explored,
            Section::Nested(sections) => sections.iter().all(|s| s.explored()),
        }
    }
}

/// Iterates the conditional directive passes of a [DirectiveTree].
///
/// Each iteration yields a "pass", which is a [Vec] of global token indices included in that
/// version of the source code.
///
/// The order and selection of conditional directive branch paths is implementation-defined and not
/// guaranteed to be stable between releases.
pub struct PassIter {
    tree: DirectiveTree,
    exhausted: bool,
}

impl Iterator for PassIter {
    type Item = Vec<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }

        let mut pass = vec![];
        self.tree.pass(&mut pass);

        self.exhausted = self.tree.explored();
        Some(pass)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token_indices_to_string(tokens: &[RawToken], indices: &[usize]) -> String {
        let mut result = String::new();
        for &index in indices {
            if let Some(token) = tokens.get(index) {
                result.push_str(token.get_leading_whitespace());
                result.push_str(token.get_content());
            }
        }
        result
    }

    const IF: &str = "{$ifdef}";
    const ELSEIF: &str = "{$elseif}";
    const ELSE: &str = "{$else}";
    const END: &str = "{$endif}";

    #[yare::parameterized(
        no_directives = {
            "foo(a, b, c);".to_owned(),
            &["foo(a, b, c);"],
        },
        if_around_all = {
            format!("{IF}foo(a, b, c);{END}"),
            &["foo(a, b, c);"],
        },
        if_else_around_all = {
            format!("{IF}foo(a, b, c){ELSEIF}bar(d, e, f){END}"),
            &[
                "foo(a, b, c)",
                "bar(d, e, f)",
            ],
        },
        if_else_else_around_all = {
            format!("{IF}foo(a, b, c){ELSEIF}bar(d, e, f){ELSEIF}baz(g, h, i){END}"),
            &[
                "foo(a, b, c)",
                "bar(d, e, f)",
                "baz(g, h, i)",
            ],
        },
        if_internal = {
            format!("foo({IF}a{END});"),
            &["foo(a);"],
        },
        if_else_internal = {
            format!("foo({IF}a{ELSEIF}b{END});"),
            &[
                "foo(a);",
                "foo(b);",
            ],
        },
        if_else_else_internal = {
            format!("foo({IF}a{ELSEIF}b{ELSEIF}c{END});"),
            &[
                "foo(a);",
                "foo(b);",
                "foo(c);",
            ],
        },
        max_branch = {
            format!("foo({IF}a{ELSEIF}b{END}, {IF}d{ELSEIF}e{ELSEIF}f{END});"),
            &[
                "foo(a,d);",
                "foo(b,e);",
                "foo(b,f);",
            ],
        },
        max_branch_nested = {
            indoc::formatdoc!("
                foo(
                  {IF}
                    {IF}a{ELSEIF}b{END}, {IF}d{ELSEIF}e{ELSEIF}f{END},
                  {END}
                  {IF}
                    {IF}g{ELSEIF}h{END}, {IF}i{ELSEIF}j{ELSEIF}k{END}
                  {ELSEIF}
                    {IF}l{ELSEIF}m{END}, {IF}n{ELSEIF}o{ELSEIF}p{END}
                  {END});"
            ),
            &[
                "foo(a,d,g,i);",
                "foo(b,e,h,j);",
                "foo(b,f,h,k);",
                "foo(b,f,l,n);",
                "foo(b,f,m,o);",
                "foo(b,f,m,p);",
            ],
        },
        manual_elseif = {
            indoc::formatdoc!("
                {IF}a
                {ELSE}{IF}b
                {ELSE}{IF}c
                {ELSE}{IF}d
                {ELSE}{IF}e
                {ELSE}{IF}f
                {ELSE}{IF}g
                {ELSE}{IF}h
                {ELSE}{IF}i
                {ELSE}{IF}j
                {ELSE}{IF}k
                {ELSE}{IF}l
                {ELSE}{IF}m
                {ELSE}{IF}n
                {ELSE}{IF}o
                {ELSE}{IF}p
                {ELSE}{IF}q
                {ELSE}{IF}r
                {ELSE}{IF}s
                {ELSE}{IF}t
                {ELSE}{IF}u
                {ELSE}{IF}v
                {ELSE}{IF}w
                {ELSE}{IF}x
                {ELSE}{IF}y
                {ELSE}z
                {END}{END}{END}{END}{END}{END}{END}{END}{END}{END}{END}{END}{END}{END}{END}{END}{END}{END}{END}{END}{END}{END}{END}{END}{END}"
            ),
            &[
                "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q",
                "r", "s", "t", "u", "v", "w", "x", "y", "z",
            ],
        },
        first_branch_explored_only_once = {
            indoc::formatdoc!("
                {IF}
                  {IF}
                  {ELSE}a
                  {END}
                {END}

                {IF}b
                {ELSE}c
                {END}"
            ),
            // Note that there is no "AB" parse; each 'flat' section is only walked once when it's not the last branch.
            &["b", "ac"],
        },
        unmatched_end = {
            indoc::formatdoc!("
                {END}
                {ELSE}

                {IF}a
                {ELSE}b
                {END}

                {END}
                {ELSE}
                {END}

                {IF}c
                {ELSE}d
                {END}

                {END}"
            ),
            &["ac", "bd"],
        },
    )]
    fn token_views(input: String, expected_pass_strings: &[&str]) {
        let raw_tokens = DelphiLexer {}.lex(&input);
        let pass_strings = DirectiveTree::parse(&raw_tokens)
            .passes()
            .map(|pass| token_indices_to_string(&raw_tokens, &pass))
            .collect_vec();
        let pass_strings = pass_strings.iter().map(String::as_str).collect_vec();
        pretty_assertions::assert_eq!(&expected_pass_strings, &pass_strings);
    }
}
