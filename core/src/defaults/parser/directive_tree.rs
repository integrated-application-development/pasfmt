use std::ops::Range;

use itertools::Itertools;

use crate::prelude::*;
use RawTokenType as TT;

pub struct DirectiveTree {
    sections: Vec<Section>,
}

enum Section {
    Flat { explored: bool, range: Range<usize> },
    Nested(Vec<DirectiveTree>),
}

impl DirectiveTree {
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
