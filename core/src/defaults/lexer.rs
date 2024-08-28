use std::ops::RangeInclusive;

use crate::lang::ConditionalDirectiveKind as CDK;
use crate::lang::KeywordKind as KK;
use crate::lang::NumberLiteralKind as NLK;
use crate::lang::OperatorKind as OK;
use crate::lang::RawTokenType as TT;
use crate::lang::TextLiteralKind as TLK;
use crate::lang::*;
use crate::traits::Lexer;

use log::*;

pub struct DelphiLexer {}
impl Lexer for DelphiLexer {
    fn lex<'a>(&self, input: &'a str) -> Vec<RawToken<'a>> {
        lex_complete(input)
    }
}

struct LexState {
    is_first: bool,
    in_asm: bool,
    prev_real_token: Option<RawTokenType>,
}

struct LexedToken<'a> {
    whitespace_count: usize,
    token_content: &'a str,
    token_type: RawTokenType,
}

fn lex_complete(input: &str) -> Vec<RawToken> {
    let (remaining, tokens) = lex(input);

    // Remaining input is always a programming error; invalid input should turn into 'Unknown' tokens.
    assert!(
        remaining.is_empty(),
        "Failed to lex the entire input. Remaining input starts with: {}",
        rounded_prefix(remaining, 100)
    );
    tokens
}

fn lex(mut input: &str) -> (&str, Vec<RawToken>) {
    // Experimentally it was determined that this linear regression on input length is best on average.
    // The performance difference from the default capacity is minor, but measurable.
    let mut tokens = Vec::with_capacity(input.len() / 8);

    let mut lex_state = LexState {
        in_asm: false,
        is_first: true,
        prev_real_token: None,
    };
    while let Some((remaining, token)) = whitespace_and_token(input, &mut lex_state) {
        tokens.push(to_final_token(token));
        input = remaining;
    }

    let (input, eof_token) = eof(input);
    tokens.push(to_final_token(eof_token));

    (input, tokens)
}

fn to_final_token(
    LexedToken {
        whitespace_count,
        mut token_content,
        token_type,
    }: LexedToken,
) -> RawToken<'_> {
    let whitespace_count: u32 = whitespace_count
        .try_into()
        .unwrap_or_else(|_| truncate_whitespace(&mut token_content, whitespace_count));
    RawToken::new(token_content, whitespace_count, token_type)
}

fn whitespace_and_token<'a>(
    input: &'a str,
    lex_state: &mut LexState,
) -> Option<(&'a str, LexedToken<'a>)> {
    let whitespace_count = count_leading_whitespace(input);

    let args = LexArgs {
        input,
        offset: whitespace_count,
        lex_state,
    };
    let (end_exclusive, token_type) = if args.lex_state.in_asm {
        lex_asm_token(args)?
    } else {
        lex_token(args)?
    };
    lex_state.is_first = false;

    if !token_type.is_comment_or_directive() {
        lex_state.prev_real_token = Some(token_type);
    }

    let (token_content, remaining) = input.split_at(end_exclusive);

    Some((
        remaining,
        LexedToken {
            whitespace_count,
            token_content,
            token_type,
        },
    ))
}

#[cold]
fn truncate_whitespace(content: &mut &str, whitespace: usize) -> u32 {
    /*
        To save space in the Token enum, we limit the whitespace count to 32 bits.
        This gives a measurable improvement in performance, but we have to handle the case
        where the whitespace is too long. We simply truncate it, which seems acceptable given
        the whitespace count is literally overflowing a 32-bit integer.
    */
    warn!(
        "Truncating whitespace before token to avoid overflow. Token starts with: {}",
        rounded_prefix(content, 50)
    );
    *content = &content[(whitespace - u32::MAX as usize)..];
    u32::MAX
}

pub(crate) fn count_leading_whitespace(input: &str) -> usize {
    let mut count = 0;
    for &b in input.as_bytes() {
        // All codepoints in [U+0, U+20] are considered 'blank'.
        // - see http://docwiki.embarcadero.com/RADStudio/en/Special:Search/Fundamental%20Syntactic%20Elements%20(Delphi)#The_Delphi_Character_Set
        if b > 0x20 {
            if b > 0x7F {
                return count + count_unicode_whitespace(input[count..].chars());
            }
            break;
        }
        count += 1;
    }

    count
}

#[cold]
fn count_unicode_whitespace(input: impl Iterator<Item = char>) -> usize {
    input
        // As above for [U+0, U+20].
        // The special case for U+3000 (ideographic space, for Chinese/Japanese/Korean) isn't documented.
        .take_while(|c| *c <= '\u{20}' || *c == '\u{3000}')
        .map(|c| c.len_utf8())
        .sum()
}

type OffsetAndTokenType = (usize, RawTokenType);

struct LexArgs<'a, 'b> {
    input: &'a str,
    offset: usize,
    lex_state: &'b mut LexState,
}
impl LexArgs<'_, '_> {
    fn consume(self, bytes: usize) -> Self {
        Self {
            offset: self.offset + bytes,
            ..self
        }
    }

    fn next_byte(&self) -> Option<&u8> {
        self.input.as_bytes().get(self.offset)
    }

    fn prev_byte(&self) -> Option<&u8> {
        self.offset
            .checked_sub(1)
            .and_then(|o| self.input.as_bytes().get(o))
    }
}

// I can't seem to write a function signature for 'cloning' this type that the borrow checker is happy with.
// It can't be cloned in the usual way because it contains a mutable reference.
// The borrow checker is smart enough to see that the mutable references don't overlap when you construct the clone
// in place, and this macro just makes it more convenient to do that.
macro_rules! lex_args_copy {
    ($args: ident) => {
        LexArgs {
            input: $args.input,
            offset: $args.offset,
            lex_state: $args.lex_state,
        }
    };
}

type LexerFn = fn(LexArgs) -> OffsetAndTokenType;

const COMMON_LEXER_MAP: [Option<LexerFn>; 256] = make_byte_map(
    &[
        (ByteSet::List(b"("), Some(l_paren)),
        (ByteSet::List(b"{"), Some(l_brace)),
        (ByteSet::List(b"/"), Some(slash)),
        (ByteSet::List(b":"), Some(colon)),
        (ByteSet::List(b"<"), Some(l_angle)),
        (ByteSet::List(b">"), Some(r_angle)),
        (ByteSet::List(b"."), Some(dot)),
        (ByteSet::List(b"+"), Some(plus)),
        (ByteSet::List(b"-"), Some(minus)),
        (ByteSet::List(b"*"), Some(star)),
        (ByteSet::List(b","), Some(comma)),
        (ByteSet::List(b";"), Some(semicolon)),
        (ByteSet::List(b"="), Some(equal)),
        (ByteSet::List(b"^"), Some(pointer)),
        (ByteSet::List(b"@"), Some(address_of)),
        (ByteSet::List(b"["), Some(l_brack)),
        (ByteSet::List(b"]"), Some(r_brack)),
        (ByteSet::List(b")"), Some(r_paren)),
        //
        (ByteSet::List(b"'#"), Some(text_literal)),
        (ByteSet::List(b"&"), Some(ampersand)),
        (ByteSet::List(b"%"), Some(binary_number_literal)),
        (ByteSet::List(b"$"), Some(hex_number_literal)),
        (ByteSet::Range(b'0'..=b'9'), Some(dec_number_literal)),
        (ByteSet::Range(b'a'..=b'z'), Some(identifier_or_keyword)),
        (ByteSet::Range(b'A'..=b'Z'), Some(identifier_or_keyword)),
        (ByteSet::List(b"_"), Some(identifier)),
        // In Delphi every non-ascii utf-8 character (except U+3000) is considered an identifier character.
        // U+3000 is treated as whitespace, so this code should never see it.
        // We need to delegate to a unicode-specific sub-lexer though, so it can ensure that the next
        // lexer is positioned on a valid char boundary.
        (ByteSet::Range(0x80..=0xFF), Some(unicode_identifier)),
    ],
    None,
);

const LEXER_MAP: [LexerFn; 256] = merge_byte_maps(&[COMMON_LEXER_MAP], unknown);

const ASM_LEXER_MAP: [LexerFn; 256] = merge_byte_maps(
    &[
        COMMON_LEXER_MAP,
        make_byte_map(
            &[
                (ByteSet::List(b"@"), Some(asm_label)),
                (ByteSet::List(b"\""), Some(asm_text_literal)),
                (ByteSet::Range(b'0'..=b'9'), Some(asm_number_literal)),
                // The only keyword that can occur in an asm block is 'end'; anything else is an identifier.
                (ByteSet::Range(b'a'..=b'z'), Some(identifier)),
                (ByteSet::Range(b'A'..=b'Z'), Some(identifier)),
                (ByteSet::List(b"eE"), Some(asm_identifier)),
            ],
            None,
        ),
    ],
    unknown,
);

fn lex_token(args: LexArgs) -> Option<OffsetAndTokenType> {
    lex_token_with_map(LEXER_MAP, args)
}

#[cold]
fn lex_asm_token(args: LexArgs) -> Option<OffsetAndTokenType> {
    lex_token_with_map(ASM_LEXER_MAP, args)
}

/*
    This jump-table approach of dynamic dispatch is measurably faster than the equivalent match
    statement, according to our benchmarks on our hardware. The difference is not huge though
    (<10%), since the rust compiler does a very good job of dynamically jumping to addresses
    computed from the byte value.

    This approach also lends itself well to efficient inheritance of the different sub-lexers.
*/
fn lex_token_with_map(map: [LexerFn; 256], args: LexArgs) -> Option<(usize, RawTokenType)> {
    args.input
        .as_bytes()
        .get(args.offset)
        .map(|b| map[*b as usize](args.consume(1)))
}

// region: byte-set
enum ByteSet<'a> {
    Range(RangeInclusive<u8>),
    List(&'a [u8]),
}

const fn merge_byte_maps<T: Copy>(maps: &[[Option<T>; 256]], default: T) -> [T; 256] {
    let mut out: [T; 256] = [default; 256];

    let mut i = 0;
    while i < maps.len() {
        let map = &maps[i];
        let mut j = 0;
        while j < map.len() {
            if let Some(elm) = map[j] {
                out[j] = elm;
            }
            j += 1;
        }
        i += 1;
    }

    out
}

const fn make_byte_map<T: Copy>(map: &[(ByteSet<'_>, T)], default: T) -> [T; 256] {
    let mut out: [T; 256] = [default; 256];

    let mut i = 0;
    while i < map.len() {
        let elm = &map[i];
        match &elm.0 {
            ByteSet::List(list) => {
                let mut j = 0;
                while j < list.len() {
                    out[list[j] as usize] = elm.1;
                    j += 1;
                }
            }
            ByteSet::Range(range) => {
                let mut j = *range.start() as usize;
                while j <= *range.end() as usize {
                    out[j] = elm.1;
                    j += 1;
                }
            }
        }

        i += 1;
    }

    out
}
// endregion: byte-set

// region: keywords
const KEYWORDS: [(&str, RawTokenType); 122] = [
    ("absolute", TT::IdentifierOrKeyword(KK::Absolute)),
    ("abstract", TT::IdentifierOrKeyword(KK::Abstract)),
    ("align", TT::IdentifierOrKeyword(KK::Align)),
    ("and", TT::Keyword(KK::And)),
    ("array", TT::Keyword(KK::Array)),
    ("as", TT::Keyword(KK::As)),
    ("asm", TT::Keyword(KK::Asm)),
    ("assembler", TT::IdentifierOrKeyword(KK::Assembler)),
    ("at", TT::IdentifierOrKeyword(KK::At)),
    ("automated", TT::IdentifierOrKeyword(KK::Automated)),
    ("begin", TT::Keyword(KK::Begin)),
    ("case", TT::Keyword(KK::Case)),
    ("cdecl", TT::IdentifierOrKeyword(KK::Cdecl)),
    ("class", TT::Keyword(KK::Class)),
    ("const", TT::Keyword(KK::Const)),
    ("constructor", TT::Keyword(KK::Constructor)),
    ("contains", TT::IdentifierOrKeyword(KK::Contains)),
    ("default", TT::IdentifierOrKeyword(KK::Default)),
    ("delayed", TT::IdentifierOrKeyword(KK::Delayed)),
    ("deprecated", TT::IdentifierOrKeyword(KK::Deprecated)),
    ("destructor", TT::Keyword(KK::Destructor)),
    ("dispid", TT::IdentifierOrKeyword(KK::DispId)),
    ("dispinterface", TT::Keyword(KK::DispInterface)),
    ("div", TT::Keyword(KK::Div)),
    ("do", TT::Keyword(KK::Do)),
    ("downto", TT::Keyword(KK::Downto)),
    ("dynamic", TT::IdentifierOrKeyword(KK::Dynamic)),
    ("else", TT::Keyword(KK::Else)),
    ("end", TT::Keyword(KK::End)),
    ("except", TT::Keyword(KK::Except)),
    ("experimental", TT::IdentifierOrKeyword(KK::Experimental)),
    ("export", TT::IdentifierOrKeyword(KK::Export)),
    ("exports", TT::Keyword(KK::Exports)),
    ("external", TT::IdentifierOrKeyword(KK::External)),
    ("far", TT::IdentifierOrKeyword(KK::Far)),
    ("file", TT::Keyword(KK::File)),
    ("final", TT::IdentifierOrKeyword(KK::Final)),
    ("finalization", TT::Keyword(KK::Finalization)),
    ("finally", TT::Keyword(KK::Finally)),
    ("for", TT::Keyword(KK::For)),
    ("forward", TT::IdentifierOrKeyword(KK::Forward)),
    ("function", TT::Keyword(KK::Function)),
    ("goto", TT::Keyword(KK::Goto)),
    ("helper", TT::IdentifierOrKeyword(KK::Helper)),
    ("if", TT::Keyword(KK::If)),
    ("implementation", TT::Keyword(KK::Implementation)),
    ("implements", TT::IdentifierOrKeyword(KK::Implements)),
    ("in", TT::Keyword(KK::In)),
    ("index", TT::IdentifierOrKeyword(KK::Index)),
    ("inherited", TT::Keyword(KK::Inherited)),
    ("initialization", TT::Keyword(KK::Initialization)),
    ("inline", TT::Keyword(KK::Inline)),
    ("interface", TT::Keyword(KK::Interface)),
    ("is", TT::Keyword(KK::Is)),
    ("label", TT::Keyword(KK::Label)),
    ("library", TT::Keyword(KK::Library)),
    ("local", TT::IdentifierOrKeyword(KK::Local)),
    ("message", TT::IdentifierOrKeyword(KK::Message)),
    ("mod", TT::Keyword(KK::Mod)),
    ("name", TT::IdentifierOrKeyword(KK::Name)),
    ("near", TT::IdentifierOrKeyword(KK::Near)),
    ("nil", TT::Keyword(KK::Nil)),
    ("nodefault", TT::IdentifierOrKeyword(KK::NoDefault)),
    ("not", TT::Keyword(KK::Not)),
    ("object", TT::Keyword(KK::Object)),
    ("of", TT::Keyword(KK::Of)),
    ("on", TT::IdentifierOrKeyword(KK::On)),
    ("operator", TT::IdentifierOrKeyword(KK::Operator)),
    ("or", TT::Keyword(KK::Or)),
    ("out", TT::IdentifierOrKeyword(KK::Out)),
    ("overload", TT::IdentifierOrKeyword(KK::Overload)),
    ("override", TT::IdentifierOrKeyword(KK::Override)),
    ("package", TT::IdentifierOrKeyword(KK::Package)),
    ("packed", TT::Keyword(KK::Packed)),
    ("pascal", TT::IdentifierOrKeyword(KK::Pascal)),
    ("platform", TT::IdentifierOrKeyword(KK::Platform)),
    ("private", TT::IdentifierOrKeyword(KK::Private)),
    ("procedure", TT::Keyword(KK::Procedure)),
    ("program", TT::Keyword(KK::Program)),
    ("property", TT::Keyword(KK::Property)),
    ("protected", TT::IdentifierOrKeyword(KK::Protected)),
    ("public", TT::IdentifierOrKeyword(KK::Public)),
    ("published", TT::IdentifierOrKeyword(KK::Published)),
    ("raise", TT::Keyword(KK::Raise)),
    ("read", TT::IdentifierOrKeyword(KK::Read)),
    ("readonly", TT::IdentifierOrKeyword(KK::ReadOnly)),
    ("record", TT::Keyword(KK::Record)),
    ("reference", TT::IdentifierOrKeyword(KK::Reference)),
    ("register", TT::IdentifierOrKeyword(KK::Register)),
    ("reintroduce", TT::IdentifierOrKeyword(KK::Reintroduce)),
    ("repeat", TT::Keyword(KK::Repeat)),
    ("requires", TT::IdentifierOrKeyword(KK::Requires)),
    ("resident", TT::IdentifierOrKeyword(KK::Resident)),
    ("resourcestring", TT::Keyword(KK::ResourceString)),
    ("safecall", TT::IdentifierOrKeyword(KK::SafeCall)),
    ("sealed", TT::IdentifierOrKeyword(KK::Sealed)),
    ("set", TT::Keyword(KK::Set)),
    ("shl", TT::Keyword(KK::Shl)),
    ("shr", TT::Keyword(KK::Shr)),
    ("static", TT::IdentifierOrKeyword(KK::Static)),
    ("stdcall", TT::IdentifierOrKeyword(KK::StdCall)),
    ("stored", TT::IdentifierOrKeyword(KK::Stored)),
    ("strict", TT::IdentifierOrKeyword(KK::Strict)),
    ("string", TT::Keyword(KK::String)),
    ("then", TT::Keyword(KK::Then)),
    ("threadvar", TT::Keyword(KK::ThreadVar)),
    ("to", TT::Keyword(KK::To)),
    ("try", TT::Keyword(KK::Try)),
    ("type", TT::Keyword(KK::Type)),
    ("unit", TT::Keyword(KK::Unit)),
    ("unsafe", TT::IdentifierOrKeyword(KK::Unsafe)),
    ("until", TT::Keyword(KK::Until)),
    ("uses", TT::Keyword(KK::Uses)),
    ("var", TT::Keyword(KK::Var)),
    ("varargs", TT::IdentifierOrKeyword(KK::VarArgs)),
    ("virtual", TT::IdentifierOrKeyword(KK::Virtual)),
    ("while", TT::Keyword(KK::While)),
    ("winapi", TT::IdentifierOrKeyword(KK::WinApi)),
    ("with", TT::Keyword(KK::With)),
    ("write", TT::IdentifierOrKeyword(KK::Write)),
    ("writeonly", TT::IdentifierOrKeyword(KK::WriteOnly)),
    ("xor", TT::Keyword(KK::Xor)),
];

fn get_word_token_type(input: &str) -> RawTokenType {
    const fn make_keyword_lookup_table<'a, const N: usize>(
        keywords: &[(&'a str, RawTokenType)],
    ) -> [Option<(&'a str, RawTokenType)>; N] {
        let mut out = [None; N];
        let mut i = 0;
        while i < keywords.len() {
            let (keyword, token_type) = &keywords[i];
            let h = hash_keyword(keyword) as usize;
            if h < out.len() && out[h].is_none() {
                out[h] = Some((*keyword, *token_type));
            } else {
                panic!("Failed to construct keyword table.");
            }
            i += 1;
        }

        out
    }

    /*
        Generated using gperf, see `misc/generate_keyword_hash_fn.sh`.

        To update this (if another keyword is added in a later Delphi version):
        - install a recent version of gperf and add it to the path
        - run the above script and copy the generated block of constants from the output
            into the array below.

        This is pretty foolproof, because it will fail to compile if it doesn't form a perfect hash
        function over the domain of Delphi keywords. Furthermore, it's fully-covered by tests.
    */
    const KEYWORD_ASSO_VALUES: [u8; 256] = [
        244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244,
        244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244,
        244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244,
        244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 29, 76, 45, 8, 6, 31, 142, 82, 18,
        9, 244, 41, 49, 14, 8, 64, 8, 6, 6, 5, 47, 91, 119, 83, 74, 244, 244, 244, 244, 244, 244,
        244, 29, 76, 45, 8, 6, 31, 142, 82, 18, 9, 244, 41, 49, 14, 8, 64, 8, 6, 6, 5, 47, 91, 119,
        83, 74, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244,
        244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244,
        244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244,
        244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244,
        244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244,
        244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244,
        244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244, 244,
        244, 244, 244, 244, 244, 244, 244, 244, 244, 244,
    ];

    #[expect(clippy::len_zero)]
    const fn hash_keyword(input: &str) -> u16 {
        let bytes = input.as_bytes();
        let mut sum = bytes.len() as u16;
        if bytes.len() >= 3 {
            sum += KEYWORD_ASSO_VALUES[bytes[2] as usize] as u16;
        }

        if bytes.len() >= 2 {
            sum += KEYWORD_ASSO_VALUES[bytes[1] as usize] as u16;
        }

        if bytes.len() >= 1 {
            sum += KEYWORD_ASSO_VALUES[bytes[0] as usize] as u16;
            sum += KEYWORD_ASSO_VALUES[bytes[bytes.len() - 1] as usize] as u16;
        }

        sum
    }

    const KEYWORD_LOOKUP_TABLE: [Option<(&'static str, RawTokenType)>;
        KEYWORD_ASSO_VALUES[0] as usize] = make_keyword_lookup_table(&KEYWORDS);

    const MAX_WORD_LENGTH: usize = {
        let mut i = 0;
        let mut max = 0;
        while i < KEYWORDS.len() {
            let len = KEYWORDS[i].0.len();
            if len > max {
                max = len;
            }

            i += 1;
        }

        max
    };

    if input.len() <= MAX_WORD_LENGTH {
        let key = hash_keyword(input) as usize;
        if let Some(Some((candidate, keyword))) = KEYWORD_LOOKUP_TABLE.get(key) {
            if input.eq_ignore_ascii_case(candidate) {
                return *keyword;
            }
        }
    }

    TT::Identifier
}
// endregion: keywords

// region: identifiers/keywords

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
// SAFETY: callers must ensure avx2 intrinsics are supported.
unsafe fn find_identifier_end_avx2(input: &str, mut offset: usize) -> usize {
    use core::mem::size_of;
    use std::arch::x86_64::*;

    type Chunk = __m256i;

    // SAFETY: callers must ensure avx2 intrinsics are supported.
    unsafe fn range_mask(x: Chunk, range: RangeInclusive<u8>) -> Chunk {
        unsafe {
            let lower = _mm256_cmpgt_epi8(_mm256_set1_epi8(*range.end() as i8 + 1), x);
            let upper = _mm256_cmpgt_epi8(x, _mm256_set1_epi8(*range.start() as i8 - 1));
            _mm256_and_si256(upper, lower)
        }
    }

    // SAFETY: callers must ensure avx2 intrinsics are supported.
    unsafe fn any_non_ascii(chunk: Chunk) -> bool {
        unsafe { _mm256_testz_si256(_mm256_set1_epi8(i8::MIN), chunk) == 0 }
    }

    while (offset + size_of::<Chunk>()) <= input.len() {
        // SAFETY: `ptr::const_ptr::add` requires that
        // * the input ptr and the offset ptr are in-bounds for the same object.
        // * the resulting ptr doesn't overflow an isize
        //
        // The input ptr is valid by because it comes from a rust reference, and the length check
        // above guarantees that the offset ptr is in bounds and points into the same allocation.
        let chunk_ptr: *const Chunk = unsafe {
            // the `loadu` variant of this intrinsic doesn't require aligned addresses
            #[expect(clippy::cast_ptr_alignment)]
            input.as_ptr().add(offset).cast::<Chunk>()
        };

        // SAFETY: this load intrinsic requires that
        // * `chunk_ptr` can be safely dereferenced
        // * avx2 intrinsics are supported
        //
        // The above length check guarantees that the ptr produced by the above offset is no more
        // than `size_of::<Chunk>` from the end of the allocation, and requirement for avx2 is
        // forwarded to the caller.
        let chunk: Chunk = unsafe { _mm256_loadu_si256(chunk_ptr) };

        // SAFETY: requires that avx2 intrinsics are supported
        // Requirement forwarded to the caller.
        let ident_mask = unsafe {
            if any_non_ascii(chunk) {
                break;
            };

            let lower_alpha = range_mask(chunk, b'a'..=b'z');
            let upper_alpha = range_mask(chunk, b'A'..=b'Z');
            let digit = range_mask(chunk, b'0'..=b'9');
            let underscore = _mm256_cmpeq_epi8(chunk, _mm256_set1_epi8(b'_' as i8));

            let alnum_mask = _mm256_or_si256(upper_alpha, _mm256_or_si256(lower_alpha, digit));
            let ident_mask = _mm256_or_si256(underscore, alnum_mask);

            _mm256_movemask_epi8(ident_mask)
        };

        // -1 in a two's-complement integer is all bits set to 1
        let any_zeros = ident_mask != -1i32;
        if any_zeros {
            offset += ident_mask.trailing_ones() as usize;
            return offset;
        }
        offset += size_of::<Chunk>();
    }

    find_identifier_end_generic(input, offset)
}

fn find_identifier_end_generic(input: &str, offset: usize) -> usize {
    offset
        + count_matching_char_bytes(
            input,
            offset,
            // We treat all unicode characters except ideographic space as identifiers.
            |c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '\u{80}'.. if c != &'\u{3000}'),
        )
}

// This strategy of amortised-cost platform-detection is borrowed from the memchr crate.
#[cfg(target_arch = "x86_64")]
fn find_identifier_end_x86_64(input: &str, offset: usize) -> usize {
    use core::sync::atomic::{AtomicPtr, Ordering};

    type Fn = *mut ();
    type RealFn = unsafe fn(&str, usize) -> usize;
    static FN: AtomicPtr<()> = AtomicPtr::new(detect as Fn);

    fn detect(input: &str, offset: usize) -> usize {
        let fun: RealFn = {
            if is_x86_feature_detected!("avx2") {
                debug!("chose AVX2 for find_identifier_end");
                find_identifier_end_avx2 as RealFn
            } else {
                debug!("chose fallback for find_identifier_end");
                find_identifier_end_generic as RealFn
            }
        };
        FN.store(fun as Fn, Ordering::Relaxed);
        // SAFETY: we just checked that the required intrinsics are supported.
        unsafe { fun(input, offset) }
    }

    let fun = FN.load(Ordering::Relaxed);

    // SAFETY: requires that
    // * `fun` is a valid instance of both types
    // * the safety requirements of the unsafe `fun` instance to be upheld
    //
    // We only ever store instances of `RealFn` inside the `FN` atomic, so it is valid to
    // interpret it as a `RealFn`, and any `RealFn` (i.e. a valid function pointer) is safe to
    // interpret as `*mut ()` (indeed a safe `as` cast above is enough).
    //
    // The safety requirements of `fun` are checked above before the store in `FN`, and the
    // initial value inside `FN` has no safety requirements.
    unsafe { core::mem::transmute::<Fn, RealFn>(fun)(input, offset) }
}

fn find_identifier_end(input: &str, offset: usize) -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        find_identifier_end_x86_64(input, offset)
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        find_identifier_end_generic(input, offset)
    }
}

fn count_bytes_in_set(input: &str, offset: usize, set: &[bool; 256]) -> usize {
    count_matching(input, offset, |b| set[*b as usize])
}

fn unicode_identifier(mut args: LexArgs) -> OffsetAndTokenType {
    while !args.input.is_char_boundary(args.offset) {
        args.offset += 1;
    }
    identifier(args)
}

fn identifier(args: LexArgs) -> OffsetAndTokenType {
    (find_identifier_end(args.input, args.offset), TT::Identifier)
}

fn identifier_or_keyword(args: LexArgs) -> OffsetAndTokenType {
    let end_offset = find_identifier_end(args.input, args.offset);

    let token_type = if args.lex_state.prev_real_token == Some(TT::Op(OK::Dot)) {
        TT::Identifier
    } else {
        let word = &args.input[(args.offset - 1)..end_offset];
        get_word_token_type(word)
    };

    args.lex_state.in_asm = token_type == TT::Keyword(KK::Asm);
    (end_offset, token_type)
}

fn asm_label(LexArgs { input, offset, .. }: LexArgs) -> OffsetAndTokenType {
    // The same as the ascii subset of valid regular identifier characters, with the addition of '@'.
    const ASM_IDENT_CHAR_SET: [bool; 256] = make_byte_map(
        &[
            (ByteSet::Range(b'a'..=b'z'), true),
            (ByteSet::Range(b'A'..=b'Z'), true),
            (ByteSet::Range(b'0'..=b'9'), true),
            (ByteSet::List(b"_@"), true),
        ],
        false,
    );

    (
        offset + count_bytes_in_set(input, offset, &ASM_IDENT_CHAR_SET),
        TT::Identifier,
    )
}

fn asm_identifier(args: LexArgs) -> OffsetAndTokenType {
    let end_offset = find_identifier_end(args.input, args.offset);

    if args.input[(args.offset - 1)..end_offset].eq_ignore_ascii_case("end") {
        args.lex_state.in_asm = false;
        (end_offset, TT::Keyword(KK::End))
    } else {
        (end_offset, TT::Identifier)
    }
}

// endregion: identifiers/keywords

// region: literals

fn text_literal(
    LexArgs {
        input, mut offset, ..
    }: LexArgs,
) -> OffsetAndTokenType {
    enum ParseState {
        Continue,
        Stop,
        Unterminated,
    }

    fn consume_pascal_str(input: &str, offset: &mut usize) -> ParseState {
        let bytes = input.as_bytes();

        if bytes.get(*offset) != Some(&b'\'') {
            return ParseState::Stop;
        }
        *offset += 1;

        if *offset >= bytes.len() {
            return ParseState::Unterminated;
        }

        if let Some(pos) = memchr::memchr3(b'\'', b'\n', b'\r', &bytes[*offset..]) {
            *offset += pos;
            // escaped quotes are handled by the calling of this function in a loop
            if let Some(b'\'') = bytes.get(*offset) {
                *offset += 1;
                return ParseState::Continue;
            } else {
                return ParseState::Unterminated;
            }
        }

        *offset = bytes.len();
        ParseState::Unterminated
    }

    fn consume_escaped_chars(input: &str, offset: &mut usize) -> ParseState {
        loop {
            let bytes = input.as_bytes();
            if bytes.get(*offset) != Some(&b'#') {
                return ParseState::Continue;
            }
            *offset += 1;

            match bytes.get(*offset) {
                Some(b'0'..=b'9' | b'_') => {
                    *offset += 1;
                    *offset += count_decimal(input, *offset);
                }
                Some(b'$') => {
                    *offset += 1;
                    match count_hex(input, *offset) {
                        0 => {
                            return ParseState::Unterminated;
                        }
                        count => *offset += count,
                    }
                }
                // As of Delphi 11 this isn't valid, but there's no reason it shouldn't be.
                Some(b'%') => {
                    *offset += 1;
                    match count_binary(input, *offset) {
                        0 => {
                            return ParseState::Unterminated;
                        }
                        count => *offset += count,
                    }
                }
                _ => {
                    return ParseState::Unterminated;
                }
            }
        }
    }

    offset -= 1;
    let orig_offset = offset;

    let unterminated = |offset: usize| {
        warn_unterminated("text literal", input, orig_offset);
        (offset, TT::TextLiteral(TLK::Unterminated))
    };

    let quote_count = input
        .bytes()
        .skip(offset)
        .take_while(|b| b == &b'\'')
        .count();

    if quote_count >= 3
        && quote_count % 2 == 1
        && matches!(
            input.as_bytes().get(offset + quote_count),
            Some(b'\r' | b'\n')
        )
    {
        let start_of_contents = offset + quote_count;
        let quote_used = &input.as_bytes()[offset..start_of_contents];
        return memchr::memmem::find(&input.as_bytes()[start_of_contents..], quote_used)
            .map(|pos| {
                (
                    start_of_contents + pos + quote_count,
                    TT::TextLiteral(TLK::MultiLine),
                )
            })
            .unwrap_or_else(|| unterminated(input.len()));
    }

    loop {
        match consume_escaped_chars(input, &mut offset) {
            ParseState::Continue => {}
            ParseState::Stop => break,
            ParseState::Unterminated => return unterminated(offset),
        }
        match consume_pascal_str(input, &mut offset) {
            ParseState::Continue => {}
            ParseState::Stop => break,
            ParseState::Unterminated => return unterminated(offset),
        }
    }

    (offset, TT::TextLiteral(TLK::SingleLine))
}

fn asm_text_literal(mut args: LexArgs) -> OffsetAndTokenType {
    let start_offset = args.offset;

    loop {
        match args.next_byte() {
            Some(b'\\') => {
                args.offset += 1;
                if args.next_byte().is_some() {
                    args.offset += 1;
                }
            }
            Some(b'\"') => {
                return (args.offset + 1, TT::TextLiteral(TLK::Asm));
            }
            None | Some(b'\n' | b'\r') => {
                break;
            }
            _ => {
                args.offset += 1;
            }
        }
    }

    warn_unterminated("asm text literal", args.input, start_offset);
    (args.offset, TT::TextLiteral(TLK::Unterminated))
}

fn asm_number_literal(mut args: LexArgs) -> OffsetAndTokenType {
    /*
        Counting all hex characters (and underscore) doesn't match the compiler's understanding
        of asm number literals, but it never leads to correct code being incorrectly lexed;
        including hex characters in non-hex literals (or underscores in any literal) leads to
        'E2115 Error in numeric constant'.

        We use the integer suffix to determine the integer type, which might be invalid.
    */
    args.offset += count_hex(args.input, args.offset);

    match args.next_byte() {
        Some(b'O' | b'o') => (args.offset + 1, TT::NumberLiteral(NLK::Octal)),
        Some(b'H' | b'h') => (args.offset + 1, TT::NumberLiteral(NLK::Hex)),
        _ => {
            // if the literal ended in a 'b' it would have been consumed as hex
            match args.prev_byte() {
                Some(b'B' | b'b') => (args.offset, TT::NumberLiteral(NLK::Binary)),
                _ => (args.offset, TT::NumberLiteral(NLK::Decimal)),
            }
        }
    }
}

fn dec_number_literal(mut args: LexArgs) -> OffsetAndTokenType {
    args.offset += count_decimal(args.input, args.offset);
    if args.next_byte() == Some(&b'.') {
        let frac_count = count_full_decimal(args.input, args.offset + 1);
        if frac_count > 0 {
            args.offset += 1 + frac_count;
        }
    }
    if matches!(args.next_byte(), Some(&b'e' | b'E')) {
        args.offset += 1;
        if matches!(args.next_byte(), Some(&b'+' | b'-')) {
            args.offset += 1;
        }
        args.offset += count_full_decimal(args.input, args.offset);
    }

    (args.offset, TT::NumberLiteral(NLK::Decimal))
}

fn hex_number_literal(args: LexArgs) -> OffsetAndTokenType {
    (
        args.offset + count_hex(args.input, args.offset),
        TT::NumberLiteral(NLK::Hex),
    )
}
fn binary_number_literal(args: LexArgs) -> OffsetAndTokenType {
    (
        args.offset + count_binary(args.input, args.offset),
        TT::NumberLiteral(NLK::Binary),
    )
}

fn count_matching<F: Fn(&u8) -> bool>(input: &str, offset: usize, f: F) -> usize {
    input.bytes().skip(offset).take_while(|b| f(b)).count()
}

fn count_matching_char_bytes<F: Fn(&char) -> bool>(input: &str, offset: usize, f: F) -> usize {
    input[offset..]
        .chars()
        .take_while(|c| f(c))
        .map(|c| c.len_utf8())
        .sum()
}

fn count_full_decimal(input: &str, offset: usize) -> usize {
    if input.as_bytes().get(offset) != Some(&b'_') {
        count_decimal(input, offset)
    } else {
        0
    }
}

fn count_decimal(input: &str, offset: usize) -> usize {
    count_matching(input, offset, |b| matches!(b, b'_' | b'0'..=b'9'))
}

fn count_hex(input: &str, offset: usize) -> usize {
    count_matching(
        input,
        offset,
        |b| matches!(b, b'_' | b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F'),
    )
}

fn count_binary(input: &str, offset: usize) -> usize {
    count_matching(input, offset, |b| matches!(b, b'_' | b'0' | b'1'))
}

// endregion: literals

fn rounded_prefix(s: &str, mut n: usize) -> &str {
    n = n.min(s.len());
    while n < s.len() && !s.is_char_boundary(n) {
        n += 1;
    }
    &s[..n]
}

#[cold]
fn warn_unterminated(description: &str, input: &str, start_offset: usize) {
    warn!(
        "Unterminated {description} found starting with: {}",
        rounded_prefix(&input[start_offset..], 50)
    );
}

#[cold]
fn consume_to_eof(input: &str, token_type: RawTokenType) -> (usize, RawTokenType) {
    let trim_count = count_unicode_whitespace(input.chars().rev());
    (input.len() - trim_count, token_type)
}

// region: directives/comments

fn conditional_directive_type(
    input: &str,
    offset: usize,
) -> (usize, Option<ConditionalDirectiveKind>) {
    let end_offset = offset
        + count_matching(
            input,
            offset,
            // Compiler directive names are ASCII only.
            // They can't start with '_' or a number, but there's no harm in including them.
            |b| matches!(b, b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'_'),
        );

    let directive = &input[offset..end_offset];

    let kind = {
        if directive.eq_ignore_ascii_case("if") {
            Some(CDK::If)
        } else if directive.eq_ignore_ascii_case("ifdef") {
            Some(CDK::Ifdef)
        } else if directive.eq_ignore_ascii_case("ifndef") {
            Some(CDK::Ifndef)
        } else if directive.eq_ignore_ascii_case("ifopt") {
            Some(CDK::Ifopt)
        } else if directive.eq_ignore_ascii_case("elseif") {
            Some(CDK::Elseif)
        } else if directive.eq_ignore_ascii_case("else") {
            Some(CDK::Else)
        } else if directive.eq_ignore_ascii_case("ifend") {
            Some(CDK::Ifend)
        } else if directive.eq_ignore_ascii_case("endif") {
            Some(CDK::Endif)
        } else {
            None
        }
    };

    (end_offset, kind)
}

#[derive(Eq, PartialEq, Copy, Clone)]
enum BlockCommentKind {
    ParenStar,
    Brace,
}

fn parse_directive_expr(
    mut args: LexArgs,
    kind: BlockCommentKind,
) -> (RawTokenType, Option<usize>) {
    let (offset, cdk) = conditional_directive_type(args.input, args.offset);
    args.offset = offset;

    match cdk {
        Some(cdk @ (CDK::If | CDK::Elseif)) => (
            TT::ConditionalDirective(cdk),
            find_directive_expr_end(args, kind),
        ),
        Some(cdk) => (
            TT::ConditionalDirective(cdk),
            find_block_comment_end(args, kind),
        ),
        None => (TT::CompilerDirective, find_block_comment_end(args, kind)),
    }
}

fn find_directive_expr_end(mut args: LexArgs, kind: BlockCommentKind) -> Option<usize> {
    let input = args.input.as_bytes();
    loop {
        match (
            input.get(args.offset),
            input.get(args.offset + 1),
            input.get(args.offset + 2),
        ) {
            // end alt block comment or directive
            (Some(b'*'), Some(b')'), _) if kind == BlockCommentKind::ParenStar => {
                return Some(args.offset + 2);
            }
            // end block comment or directive
            (Some(b'}'), _, _) if kind == BlockCommentKind::Brace => {
                return Some(args.offset + 1);
            }
            // start alt directive
            (Some(b'('), Some(b'*'), Some(b'$')) => {
                args.offset += 3;
                args.offset =
                    parse_directive_expr(lex_args_copy!(args), BlockCommentKind::ParenStar).1?;
            }
            // start directive
            (Some(b'{'), Some(b'$'), _) => {
                args.offset += 2;
                args.offset =
                    parse_directive_expr(lex_args_copy!(args), BlockCommentKind::Brace).1?;
            }
            // start alt block
            (Some(b'('), Some(b'*'), _) => {
                args.offset += 2;
                args.offset = block_comment_alt(lex_args_copy!(args)).0;
            }
            // start block
            (Some(b'{'), _, _) => {
                args.offset += 1;
                args.offset = block_comment(lex_args_copy!(args)).0;
            }
            // start string
            (Some(b'\''), _, _) => {
                args.offset += 1;
                args.offset = text_literal(lex_args_copy!(args)).0;
            }
            // start line comment
            (Some(b'/'), Some(b'/'), _) => {
                args.offset += 2;
                args.offset = line_comment(lex_args_copy!(args)).0;
            }
            (None, _, _) => {
                return None;
            }
            _ => {
                args.offset += 1;
            }
        }
    }
}

fn find_block_comment_end(
    LexArgs { input, offset, .. }: LexArgs,
    kind: BlockCommentKind,
) -> Option<usize> {
    match kind {
        BlockCommentKind::ParenStar => {
            memchr::memmem::find(&input.as_bytes()[offset..], b"*)").map(|o| offset + o + 2)
        }
        BlockCommentKind::Brace => {
            memchr::memchr(b'}', &input.as_bytes()[offset..]).map(|o| offset + o + 1)
        }
    }
}

fn compiler_directive(args: LexArgs, kind: BlockCommentKind) -> OffsetAndTokenType {
    let (token_type, end_offset) = parse_directive_expr(lex_args_copy!(args), kind);

    if let Some(pos) = end_offset {
        (pos, token_type)
    } else {
        let start_len = match kind {
            BlockCommentKind::ParenStar => 2,
            BlockCommentKind::Brace => 1,
        };
        warn_unterminated("compiler directive", args.input, args.offset - start_len);
        consume_to_eof(args.input, token_type)
    }
}

fn block_comment_kind(
    nl_offset: usize,
    start_offset: usize,
    end_offset: usize,
    lex_state: &LexState,
) -> CommentKind {
    if nl_offset >= start_offset && nl_offset < end_offset {
        CommentKind::MultilineBlock
    } else if nl_offset < start_offset || lex_state.is_first {
        CommentKind::IndividualBlock
    } else {
        CommentKind::InlineBlock
    }
}

fn _block_comment(
    LexArgs {
        input,
        offset,
        lex_state,
    }: LexArgs,
    start_len: usize,
    end_offset: Option<usize>,
) -> OffsetAndTokenType {
    if let Some(end_offset) = end_offset {
        let nl_offset =
            memchr::memchr(b'\n', &input.as_bytes()[..end_offset]).unwrap_or(input.len());
        let comment_kind = block_comment_kind(nl_offset, offset, end_offset, lex_state);
        (end_offset, TT::Comment(comment_kind))
    } else {
        warn_unterminated("block comment", input, offset - start_len);
        consume_to_eof(input, TT::Comment(CommentKind::MultilineBlock))
    }
}

fn block_comment_alt(args: LexArgs) -> OffsetAndTokenType {
    let end_offset = find_block_comment_end(lex_args_copy!(args), BlockCommentKind::ParenStar);
    _block_comment(args, 2, end_offset)
}

fn block_comment(args: LexArgs) -> OffsetAndTokenType {
    let end_offset = find_block_comment_end(lex_args_copy!(args), BlockCommentKind::Brace);
    _block_comment(args, 1, end_offset)
}

fn line_comment(
    LexArgs {
        input,
        offset,
        lex_state,
    }: LexArgs,
) -> OffsetAndTokenType {
    let kind = if input[..offset].contains('\n') || lex_state.is_first {
        CommentKind::IndividualLine
    } else {
        CommentKind::InlineLine
    };
    (
        memchr::memchr2(b'\n', b'\r', &input.as_bytes()[offset..])
            .map(|o| o + offset)
            .unwrap_or(input.len()),
        TT::Comment(kind),
    )
}

fn compiler_directive_or_comment_alt(args: LexArgs) -> OffsetAndTokenType {
    match args.next_byte() {
        Some(b'$') => compiler_directive(args.consume(1), BlockCommentKind::ParenStar),
        _ => block_comment_alt(args),
    }
}

fn compiler_directive_or_comment(args: LexArgs) -> OffsetAndTokenType {
    match args.next_byte() {
        Some(b'$') => compiler_directive(args.consume(1), BlockCommentKind::Brace),
        _ => block_comment(args),
    }
}

// endregion: directives/comments

fn ampersand(mut args: LexArgs) -> OffsetAndTokenType {
    args.offset += count_matching(args.input, args.offset, |b| *b == b'&');

    match args.next_byte() {
        Some(b'$') => hex_number_literal(args.consume(1)),
        Some(b'%') => binary_number_literal(args.consume(1)),
        Some(b'0'..=b'9') => dec_number_literal(args.consume(1)),
        Some(b'a'..=b'z' | b'A'..=b'Z' | b'_') => identifier(args.consume(1)),
        Some(0x80..) => unicode_identifier(args.consume(1)),
        _ => unknown(args),
    }
}

// region: operators

macro_rules! basic_op {
    ($name: ident, $typ: expr) => {
        fn $name(args: LexArgs) -> OffsetAndTokenType {
            (args.offset, RawTokenType::Op($typ))
        }
    };
}

// All of these operators are dead simple, because the meaning does not depend on what comes after.
// The other 'operators' are more complicated and require looking ahead.
basic_op!(plus, OK::Plus);
basic_op!(minus, OK::Minus);
basic_op!(star, OK::Star);
basic_op!(comma, OK::Comma);
basic_op!(semicolon, OK::Semicolon);
basic_op!(equal, OK::Equal(EqKind::Comp));
basic_op!(pointer, OK::Pointer);
basic_op!(address_of, OK::AddressOf);
basic_op!(l_brack, OK::LBrack);
basic_op!(r_brack, OK::RBrack);
basic_op!(r_paren, OK::RParen);

fn l_paren(args: LexArgs) -> OffsetAndTokenType {
    match args.next_byte() {
        Some(b'*') => compiler_directive_or_comment_alt(args.consume(1)),
        Some(b'.') => (args.offset + 1, TT::Op(OK::LBrack)),
        _ => (args.offset, TT::Op(OK::LParen)),
    }
}

fn l_brace(args: LexArgs) -> OffsetAndTokenType {
    compiler_directive_or_comment(args)
}

fn slash(args: LexArgs) -> OffsetAndTokenType {
    match args.next_byte() {
        Some(b'/') => line_comment(args.consume(1)),
        _ => (args.offset, TT::Op(OK::Slash)),
    }
}

fn colon(args: LexArgs) -> OffsetAndTokenType {
    match args.next_byte() {
        Some(b'=') => (args.offset + 1, TT::Op(OK::Assign)),
        _ => (args.offset, TT::Op(OK::Colon)),
    }
}

fn l_angle(args: LexArgs) -> OffsetAndTokenType {
    match args.next_byte() {
        Some(b'=') => (args.offset + 1, TT::Op(OK::LessEqual)),
        Some(b'>') => (args.offset + 1, TT::Op(OK::NotEqual)),
        _ => (args.offset, TT::Op(OK::LessThan(ChevronKind::Comp))),
    }
}

fn r_angle(args: LexArgs) -> OffsetAndTokenType {
    match args.next_byte() {
        Some(b'=') => (args.offset + 1, TT::Op(OK::GreaterEqual)),
        _ => (args.offset, TT::Op(OK::GreaterThan(ChevronKind::Comp))),
    }
}

fn dot(args: LexArgs) -> OffsetAndTokenType {
    match args.next_byte() {
        Some(b'.') => (args.offset + 1, TT::Op(OK::DotDot)),
        Some(b')') => (args.offset + 1, TT::Op(OK::RBrack)),
        _ => (args.offset, TT::Op(OK::Dot)),
    }
}

// endregion: operators

// region: special tokens

#[cold]
fn unknown(args: LexArgs) -> OffsetAndTokenType {
    warn!(
        "Found unexpected character: {}. Creating `Unknown` token.",
        *args.prev_byte().unwrap() as char
    );
    (args.offset, TT::Unknown)
}

fn eof(input: &str) -> (&str, LexedToken) {
    let whitespace_count = count_leading_whitespace(input);
    let (token_content, remaining) = input.split_at(whitespace_count);
    (
        remaining,
        LexedToken {
            whitespace_count,
            token_content,
            token_type: TT::Eof,
        },
    )
}

// endregion: special tokens

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use spectral::prelude::*;

    type ContentAndTokenType<'a> = (&'a str, RawTokenType);

    fn run_test(input: &str, expected_token_types: &[ContentAndTokenType]) {
        let lexer = DelphiLexer {};
        let tokens = lexer.lex(input);
        let token_types: Vec<_> = tokens
            .iter()
            .filter(|token| token.get_token_type() != TT::Eof)
            .map(|token| (token.get_content(), token.get_token_type()))
            .collect();

        assert_that(&token_types.as_slice()).is_equal_to(expected_token_types);
    }

    fn alternating_case(input: &str) -> String {
        input
            .char_indices()
            .map(|(index, char)| match index % 2 {
                0 => char.to_ascii_uppercase(),
                _ => char.to_ascii_lowercase(),
            })
            .collect::<String>()
    }

    fn run_casing_test((input, expected_token_type): ContentAndTokenType) {
        let lowercase = input.to_ascii_lowercase();
        let uppercase = input.to_ascii_uppercase();
        let alternating = alternating_case(input);
        let input = format!("{} {} {}", lowercase, uppercase, alternating);
        run_test(
            input.as_str(),
            &[
                (lowercase.as_str(), expected_token_type),
                (uppercase.as_str(), expected_token_type),
                (alternating.as_str(), expected_token_type),
            ],
        )
    }

    #[test]
    fn lex_block_comment_types() {
        run_test(
            indoc! {"
                {block comment} {.$fake compiler directive} \
                (*star block comment*) (*.$fake compiler star directive*) {*)} (*{*) \
                {
                    Multiline block comment
                }"
            },
            &[
                ("{block comment}", TT::Comment(CommentKind::IndividualBlock)),
                (
                    "{.$fake compiler directive}",
                    TT::Comment(CommentKind::InlineBlock),
                ),
                (
                    "(*star block comment*)",
                    TT::Comment(CommentKind::InlineBlock),
                ),
                (
                    "(*.$fake compiler star directive*)",
                    TT::Comment(CommentKind::InlineBlock),
                ),
                ("{*)}", TT::Comment(CommentKind::InlineBlock)),
                ("(*{*)", TT::Comment(CommentKind::InlineBlock)),
                (
                    indoc! {"
                        {
                            Multiline block comment
                        }"
                    },
                    TT::Comment(CommentKind::MultilineBlock),
                ),
            ],
        );
    }

    #[test]
    fn lex_block_comments() {
        run_test(
            indoc! {"
                {individual block}
                {individual block}
                ; {inline block}"
            },
            &[
                (
                    "{individual block}",
                    TT::Comment(CommentKind::IndividualBlock),
                ),
                (
                    "{individual block}",
                    TT::Comment(CommentKind::IndividualBlock),
                ),
                (";", TT::Op(OK::Semicolon)),
                ("{inline block}", TT::Comment(CommentKind::InlineBlock)),
            ],
        );
    }

    #[test]
    fn lex_unterminated_block_comments() {
        run_test(
            indoc! {"
                {individual block
                // other comment
                Foo;

                "
            },
            &[(
                "{individual block\n// other comment\nFoo;",
                TT::Comment(CommentKind::MultilineBlock),
            )],
        );
        run_test(
            indoc! {"
                (*individual block
                // other comment
                Foo;

                "
            },
            &[(
                "(*individual block\n// other comment\nFoo;",
                TT::Comment(CommentKind::MultilineBlock),
            )],
        );
    }

    #[test]
    fn lex_line_comments() {
        run_test(
            indoc! {"
                // Individual line comment 1
                // Individual line comment 2
                ; // Inline line comment"
            },
            &[
                (
                    "// Individual line comment 1",
                    TT::Comment(CommentKind::IndividualLine),
                ),
                (
                    "// Individual line comment 2",
                    TT::Comment(CommentKind::IndividualLine),
                ),
                (";", TT::Op(OK::Semicolon)),
                (
                    "// Inline line comment",
                    TT::Comment(CommentKind::InlineLine),
                ),
            ],
        );
    }

    const IF_DIRECTIVE: TT = TT::ConditionalDirective(CDK::If);
    const IFDEF_DIRECTIVE: TT = TT::ConditionalDirective(CDK::Ifdef);
    const IFNDEF_DIRECTIVE: TT = TT::ConditionalDirective(CDK::Ifndef);
    const IFOPT_DIRECTIVE: TT = TT::ConditionalDirective(CDK::Ifopt);
    const ELSEIF_DIRECTIVE: TT = TT::ConditionalDirective(CDK::Elseif);
    const ELSE_DIRECTIVE: TT = TT::ConditionalDirective(CDK::Else);
    const IFEND_DIRECTIVE: TT = TT::ConditionalDirective(CDK::Ifend);
    const ENDIF_DIRECTIVE: TT = TT::ConditionalDirective(CDK::Endif);

    #[test]
    fn lex_compiler_directives() {
        run_test(
            "
            (*$message*)
            {$foo *) }
            (*$bar aa {}*)
            {$if1}
            {$if9}
            {$if_}
            ",
            &[
                ("(*$message*)", TT::CompilerDirective),
                ("{$foo *) }", TT::CompilerDirective),
                ("(*$bar aa {}*)", TT::CompilerDirective),
                ("{$if1}", TT::CompilerDirective),
                ("{$if9}", TT::CompilerDirective),
                ("{$if_}", TT::CompilerDirective),
            ],
        );
        [
            ("{$if}", IF_DIRECTIVE),
            ("{$ifdef}", IFDEF_DIRECTIVE),
            ("{$ifndef}", IFNDEF_DIRECTIVE),
            ("{$ifopt}", IFOPT_DIRECTIVE),
            ("{$elseif}", ELSEIF_DIRECTIVE),
            ("{$else}", ELSE_DIRECTIVE),
            ("{$ifend}", IFEND_DIRECTIVE),
            ("{$endif}", ENDIF_DIRECTIVE),
            ("(*$if*)", IF_DIRECTIVE),
            ("(*$ifdef*)", IFDEF_DIRECTIVE),
            ("(*$ifndef*)", IFNDEF_DIRECTIVE),
            ("(*$ifopt*)", IFOPT_DIRECTIVE),
            ("(*$elseif*)", ELSEIF_DIRECTIVE),
            ("(*$else*)", ELSE_DIRECTIVE),
            ("(*$ifend*)", IFEND_DIRECTIVE),
            ("(*$endif*)", ENDIF_DIRECTIVE),
        ]
        .into_iter()
        .for_each(run_casing_test);
    }

    #[test]
    fn lex_unterminated_compiler_directives() {
        run_test(
            indoc! {"
                {$if
                // other comment
                Foo;

                "
            },
            &[("{$if\n// other comment\nFoo;", IF_DIRECTIVE)],
        );
        run_test(
            indoc! {"
                (*$if
                // other comment
                Foo;

                "
            },
            &[("(*$if\n// other comment\nFoo;", IF_DIRECTIVE)],
        );
        // nested unterminated block comment
        run_test("{$if (*if } //", &[("{$if (*if } //", IF_DIRECTIVE)]);
        // nested unterminated directive
        run_test("{$if (*$if } //", &[("{$if (*$if } //", IF_DIRECTIVE)]);
        // nested line comment
        run_test("{$if // } //", &[("{$if // } //", IF_DIRECTIVE)]);
        // nested string literal
        run_test("{$if '} //", &[("{$if '} //", IF_DIRECTIVE)]);
    }

    #[test]
    fn lex_complex_directive_expressions() {
        /*
            Since Delphi conditional directives contain expressions, they can also contain comments
            and other conditional directives (to a limited and buggy extent). This means that we
            can't just treat them as block comments when finding the bounds of the token for the
            directive.

            The only directives that can contain these expressions are `if` and `elseif`, all the
            others can safely be lexed as a simple block comment.
        */

        run_test(
            indoc! {"
            {$if {$i foo} = 0}
            {$if (*$i foo*) = 0}
            (*$if (*$i foo*) = 0*)
            (*$if {$i foo} = 0*)

            {$if {(*} (*{*) {(* {$if }}

            {$if {{} }
            {$if {$if {}} }

            {$if {$i foo} = 0*) }
            (*$if {$i foo} = 0} *)

            {$if
                {$if True}
                    FOO
                (*$elseif {$if True}FOO{$else}BAR{$endif} *)
                    BAR
                {$endif}
                = 0
            }

            {$ifdef {$i inc}
            {$if {$ifdef {}}

            {$if a = '}'#10#13''''}
            {$if a = #10}

            {$if a = '''
            }
            '''
            }

            "},
            &[
                // Ensure directives and comments can be mixed in any order
                ("{$if {$i foo} = 0}", IF_DIRECTIVE),
                ("{$if (*$i foo*) = 0}", IF_DIRECTIVE),
                ("(*$if (*$i foo*) = 0*)", IF_DIRECTIVE),
                ("(*$if {$i foo} = 0*)", IF_DIRECTIVE),
                ("{$if {(*} (*{*) {(* {$if }}", IF_DIRECTIVE),
                // Ensure that block comments cannot be started from within nested block comments
                ("{$if {{} }", IF_DIRECTIVE),
                // Ensure that block comments can be started from within nested conditional directives
                ("{$if {$if {}} }", IF_DIRECTIVE),
                // Ensure that the nested directives are closed with the right half of the pair
                ("{$if {$i foo} = 0*) }", IF_DIRECTIVE),
                ("(*$if {$i foo} = 0} *)", IF_DIRECTIVE),
                // Ensure nesting works recursively
                (
                    indoc! {"
                        {$if
                            {$if True}
                                FOO
                            (*$elseif {$if True}FOO{$else}BAR{$endif} *)
                                BAR
                            {$endif}
                            = 0
                        }"
                    },
                    IF_DIRECTIVE,
                ),
                // Ensure that the nesting doesn't work with non-expression directives
                ("{$ifdef {$i inc}", IFDEF_DIRECTIVE),
                // Ensure that the nesting doesn't work within a nested non-expression directive
                ("{$if {$ifdef {}}", IF_DIRECTIVE),
                // Ensure that nested text literals work
                ("{$if a = '}'#10#13''''}", IF_DIRECTIVE),
                ("{$if a = #10}", IF_DIRECTIVE),
                ("{$if a = '''\n}\n'''\n}", IF_DIRECTIVE),
            ],
        )
    }

    #[test]
    fn lex_string_literals() {
        run_test(
            "'' 'string' 'string''part2' 'ab''''cd' 'abc'#13#10 'after escaped stuff' 'a'#1'b' 'a'#1#0'b' 'a'#$017F #%010 #%0_1 #%_0 #%_ #$F7F #$F_7 #$_F #$_ #123 #_",
            &[
                ("''", TT::TextLiteral(TLK::SingleLine)),
                ("'string'", TT::TextLiteral(TLK::SingleLine)),
                ("'string''part2'", TT::TextLiteral(TLK::SingleLine)),
                ("'ab''''cd'", TT::TextLiteral(TLK::SingleLine)),
                ("'abc'#13#10", TT::TextLiteral(TLK::SingleLine)),
                ("'after escaped stuff'", TT::TextLiteral(TLK::SingleLine)),
                ("'a'#1'b'", TT::TextLiteral(TLK::SingleLine)),
                ("'a'#1#0'b'", TT::TextLiteral(TLK::SingleLine)),
                ("'a'#$017F", TT::TextLiteral(TLK::SingleLine)),
                ("#%010", TT::TextLiteral(TLK::SingleLine)),
                ("#%0_1", TT::TextLiteral(TLK::SingleLine)),
                ("#%_0", TT::TextLiteral(TLK::SingleLine)),
                ("#%_", TT::TextLiteral(TLK::SingleLine)),
                ("#$F7F", TT::TextLiteral(TLK::SingleLine)),
                ("#$F_7", TT::TextLiteral(TLK::SingleLine)),
                ("#$_F", TT::TextLiteral(TLK::SingleLine)),
                ("#$_", TT::TextLiteral(TLK::SingleLine)),
                ("#123", TT::TextLiteral(TLK::SingleLine)),
                ("#_", TT::TextLiteral(TLK::SingleLine)),
            ],
        );
    }

    #[test]
    fn lex_multiline_string_literals() {
        run_test(
            indoc! {
            "
            '''
            a
            '''

            '''
            ' ''
            '''

            '''''
            '''
            '''''

            '''''''
            '''
            '''''''
            "},
            &[
                ("'''\na\n'''", TT::TextLiteral(TLK::MultiLine)),
                ("'''\n' ''\n'''", TT::TextLiteral(TLK::MultiLine)),
                ("'''''\n'''\n'''''", TT::TextLiteral(TLK::MultiLine)),
                ("'''''''\n'''\n'''''''", TT::TextLiteral(TLK::MultiLine)),
            ],
        )
    }

    #[test]
    fn lex_invalid_multiline_string_literals() {
        run_test(
            indoc! {
            "
            ''' '
            ''''' '
            ''''''
            ''' '''

            '''
            a'''

            '''a
            '''
            //
            "},
            &[
                // unusual, but valid single-line text literals
                ("''' '", TT::TextLiteral(TLK::SingleLine)),
                ("''''' '", TT::TextLiteral(TLK::SingleLine)),
                ("''''''", TT::TextLiteral(TLK::SingleLine)),
                ("''' '''", TT::TextLiteral(TLK::SingleLine)),
                // invalid text before closing quote
                ("'''\na'''", TT::TextLiteral(TLK::MultiLine)),
                // incomplete single-line text literal
                ("'''a", TT::TextLiteral(TLK::Unterminated)),
                // incomplete multiline-line text literal
                ("'''\n//\n", TT::TextLiteral(TLK::Unterminated)),
            ],
        )
    }

    #[test]
    fn lex_unterminated_string_literals() {
        run_test(
            "
            'string
            ' + '';
            'a'#
            'a'##
            #$ #% #$Z #%A
            ",
            &[
                ("'string", TT::TextLiteral(TLK::Unterminated)),
                ("' + '';", TT::TextLiteral(TLK::Unterminated)),
                ("'a'#", TT::TextLiteral(TLK::Unterminated)),
                ("'a'#", TT::TextLiteral(TLK::Unterminated)),
                ("#", TT::TextLiteral(TLK::Unterminated)),
                ("#$", TT::TextLiteral(TLK::Unterminated)),
                ("#%", TT::TextLiteral(TLK::Unterminated)),
                ("#$", TT::TextLiteral(TLK::Unterminated)),
                ("Z", TT::Identifier),
                ("#%", TT::TextLiteral(TLK::Unterminated)),
                ("A", TT::Identifier),
            ],
        );
        run_test(
            "'''\nasdf",
            &[("'''\nasdf", TT::TextLiteral(TLK::Unterminated))],
        );
        run_test("'", &[("'", TT::TextLiteral(TLK::Unterminated))]);
    }

    #[test]
    fn lex_decimal_number_literals() {
        run_test(
            "0 0.0 10 1_000 1_000.00 1_111_111.11 1.111_1 0e-1 0e+1 0E+1 0.5e+21",
            &[
                ("0", TT::NumberLiteral(NLK::Decimal)),
                ("0.0", TT::NumberLiteral(NLK::Decimal)),
                ("10", TT::NumberLiteral(NLK::Decimal)),
                ("1_000", TT::NumberLiteral(NLK::Decimal)),
                ("1_000.00", TT::NumberLiteral(NLK::Decimal)),
                ("1_111_111.11", TT::NumberLiteral(NLK::Decimal)),
                ("1.111_1", TT::NumberLiteral(NLK::Decimal)),
                ("0e-1", TT::NumberLiteral(NLK::Decimal)),
                ("0e+1", TT::NumberLiteral(NLK::Decimal)),
                ("0E+1", TT::NumberLiteral(NLK::Decimal)),
                ("0.5e+21", TT::NumberLiteral(NLK::Decimal)),
            ],
        );
    }

    #[test]
    fn lex_ambiguous_dotdot() {
        run_test(
            "0..1",
            &[
                ("0", TT::NumberLiteral(NLK::Decimal)),
                ("..", TT::Op(OK::DotDot)),
                ("1", TT::NumberLiteral(NLK::Decimal)),
            ],
        );
    }

    #[test]
    fn lex_fake_decimal_number_literals() {
        // These might look like floats, but they are actually member accesses on integers
        // (accessing routines defined in helper classes).
        run_test(
            "0._0 0.e5 0.e-5",
            &[
                ("0", TT::NumberLiteral(NLK::Decimal)),
                (".", TT::Op(OK::Dot)),
                ("_0", TT::Identifier),
                //
                ("0", TT::NumberLiteral(NLK::Decimal)),
                (".", TT::Op(OK::Dot)),
                ("e5", TT::Identifier),
                //
                ("0", TT::NumberLiteral(NLK::Decimal)),
                (".", TT::Op(OK::Dot)),
                ("e", TT::Identifier),
                ("-", TT::Op(OK::Minus)),
                ("5", TT::NumberLiteral(NLK::Decimal)),
            ],
        );
    }

    #[test]
    fn lex_invalid_decimal_number_literals() {
        run_test(
            "0e 0.0e 0e+ 0e- 0.0e+ 0.0e-",
            &[
                ("0e", TT::NumberLiteral(NLK::Decimal)),
                ("0.0e", TT::NumberLiteral(NLK::Decimal)),
                ("0e+", TT::NumberLiteral(NLK::Decimal)),
                ("0e-", TT::NumberLiteral(NLK::Decimal)),
                ("0.0e+", TT::NumberLiteral(NLK::Decimal)),
                ("0.0e-", TT::NumberLiteral(NLK::Decimal)),
            ],
        );
    }

    #[test]
    fn lex_hex_number_literal() {
        run_test(
            "$ $00 $FF $0_0 $_ $_1",
            &[
                ("$", TT::NumberLiteral(NLK::Hex)),
                ("$00", TT::NumberLiteral(NLK::Hex)),
                ("$FF", TT::NumberLiteral(NLK::Hex)),
                ("$0_0", TT::NumberLiteral(NLK::Hex)),
                ("$_", TT::NumberLiteral(NLK::Hex)),
                ("$_1", TT::NumberLiteral(NLK::Hex)),
            ],
        );
    }

    #[test]
    fn lex_binary_number_literals() {
        run_test(
            "% %0 %1 %1111_0000 %_ %_1",
            &[
                ("%", TT::NumberLiteral(NLK::Binary)),
                ("%0", TT::NumberLiteral(NLK::Binary)),
                ("%1", TT::NumberLiteral(NLK::Binary)),
                ("%1111_0000", TT::NumberLiteral(NLK::Binary)),
                ("%_", TT::NumberLiteral(NLK::Binary)),
                ("%_1", TT::NumberLiteral(NLK::Binary)),
            ],
        );
    }

    #[test]
    fn lex_ampersand_integer_literals() {
        // Nonsensically, with the Delphi 11 compiler, only the `&0` and `&&0` cases are valid.
        // We figure that the other cases should be lexed in the same way (even if they are invalid).
        run_test(
            "&$FF &&$FF &%0 &&%0 &0 &&0",
            &[
                ("&$FF", TT::NumberLiteral(NLK::Hex)),
                ("&&$FF", TT::NumberLiteral(NLK::Hex)),
                ("&%0", TT::NumberLiteral(NLK::Binary)),
                ("&&%0", TT::NumberLiteral(NLK::Binary)),
                ("&0", TT::NumberLiteral(NLK::Decimal)),
                ("&&0", TT::NumberLiteral(NLK::Decimal)),
            ],
        );
    }

    #[test]
    fn lex_ampersand_identifiers() {
        run_test(
            "&begin &&op_Addition &&&Foo &&&&Foo &_ && &£ &\0",
            &[
                ("&begin", TT::Identifier),
                ("&&op_Addition", TT::Identifier),
                // These aren't valid identifiers, but they're most valid as such.
                ("&&&Foo", TT::Identifier),
                ("&&&&Foo", TT::Identifier),
                ("&_", TT::Identifier),
                // You can't actually use this as an identifier, but in some contexts it's valid yet ignored.
                ("&&", TT::Unknown),
                ("&£", TT::Identifier),
                ("&", TT::Unknown),
            ],
        );
    }

    #[test]
    fn lex_unknown_ampersand() {
        run_test(
            "&{} &+ &!",
            &[
                ("&", TT::Unknown),
                ("{}", TT::Comment(CommentKind::InlineBlock)),
                ("&", TT::Unknown),
                ("+", TT::Op(OK::Plus)),
                ("&", TT::Unknown),
                ("!", TT::Unknown),
            ],
        );
    }

    #[test]
    fn lex_identifiers() {
        run_test(
            "Foo _Foo _1Foo",
            &[
                ("Foo", TT::Identifier),
                ("_Foo", TT::Identifier),
                ("_1Foo", TT::Identifier),
            ],
        );
    }

    #[test]
    fn lex_operators() {
        run_test(
            "+-*/:=,;=:<><<=>=>[](..)()^@...",
            &[
                ("+", TT::Op(OK::Plus)),
                ("-", TT::Op(OK::Minus)),
                ("*", TT::Op(OK::Star)),
                ("/", TT::Op(OK::Slash)),
                (":=", TT::Op(OK::Assign)),
                (",", TT::Op(OK::Comma)),
                (";", TT::Op(OK::Semicolon)),
                ("=", TT::Op(OK::Equal(EqKind::Comp))),
                (":", TT::Op(OK::Colon)),
                ("<>", TT::Op(OK::NotEqual)),
                ("<", TT::Op(OK::LessThan(ChevronKind::Comp))),
                ("<=", TT::Op(OK::LessEqual)),
                (">=", TT::Op(OK::GreaterEqual)),
                (">", TT::Op(OK::GreaterThan(ChevronKind::Comp))),
                ("[", TT::Op(OK::LBrack)),
                ("]", TT::Op(OK::RBrack)),
                ("(.", TT::Op(OK::LBrack)),
                (".)", TT::Op(OK::RBrack)),
                ("(", TT::Op(OK::LParen)),
                (")", TT::Op(OK::RParen)),
                ("^", TT::Op(OK::Pointer)),
                ("@", TT::Op(OK::AddressOf)),
                ("..", TT::Op(OK::DotDot)),
                (".", TT::Op(OK::Dot)),
            ],
        );
    }

    #[test]
    fn lex_keywords() {
        [
            ("absolute", TT::IdentifierOrKeyword(KK::Absolute)),
            ("abstract", TT::IdentifierOrKeyword(KK::Abstract)),
            ("align", TT::IdentifierOrKeyword(KK::Align)),
            ("and", TT::Keyword(KK::And)),
            ("array", TT::Keyword(KK::Array)),
            ("as", TT::Keyword(KK::As)),
            ("assembler", TT::IdentifierOrKeyword(KK::Assembler)),
            ("at", TT::IdentifierOrKeyword(KK::At)),
            ("automated", TT::IdentifierOrKeyword(KK::Automated)),
            ("begin", TT::Keyword(KK::Begin)),
            ("case", TT::Keyword(KK::Case)),
            ("cdecl", TT::IdentifierOrKeyword(KK::Cdecl)),
            ("class", TT::Keyword(KK::Class)),
            ("const", TT::Keyword(KK::Const)),
            ("constructor", TT::Keyword(KK::Constructor)),
            ("contains", TT::IdentifierOrKeyword(KK::Contains)),
            ("default", TT::IdentifierOrKeyword(KK::Default)),
            ("delayed", TT::IdentifierOrKeyword(KK::Delayed)),
            ("deprecated", TT::IdentifierOrKeyword(KK::Deprecated)),
            ("destructor", TT::Keyword(KK::Destructor)),
            ("dispid", TT::IdentifierOrKeyword(KK::DispId)),
            ("dispinterface", TT::Keyword(KK::DispInterface)),
            ("div", TT::Keyword(KK::Div)),
            ("do", TT::Keyword(KK::Do)),
            ("downto", TT::Keyword(KK::Downto)),
            ("dynamic", TT::IdentifierOrKeyword(KK::Dynamic)),
            ("else", TT::Keyword(KK::Else)),
            ("end", TT::Keyword(KK::End)),
            ("except", TT::Keyword(KK::Except)),
            ("experimental", TT::IdentifierOrKeyword(KK::Experimental)),
            ("export", TT::IdentifierOrKeyword(KK::Export)),
            ("exports", TT::Keyword(KK::Exports)),
            ("external", TT::IdentifierOrKeyword(KK::External)),
            ("far", TT::IdentifierOrKeyword(KK::Far)),
            ("file", TT::Keyword(KK::File)),
            ("final", TT::IdentifierOrKeyword(KK::Final)),
            ("finalization", TT::Keyword(KK::Finalization)),
            ("finally", TT::Keyword(KK::Finally)),
            ("for", TT::Keyword(KK::For)),
            ("forward", TT::IdentifierOrKeyword(KK::Forward)),
            ("function", TT::Keyword(KK::Function)),
            ("goto", TT::Keyword(KK::Goto)),
            ("helper", TT::IdentifierOrKeyword(KK::Helper)),
            ("if", TT::Keyword(KK::If)),
            ("implementation", TT::Keyword(KK::Implementation)),
            ("implements", TT::IdentifierOrKeyword(KK::Implements)),
            ("in", TT::Keyword(KK::In)),
            ("index", TT::IdentifierOrKeyword(KK::Index)),
            ("inherited", TT::Keyword(KK::Inherited)),
            ("initialization", TT::Keyword(KK::Initialization)),
            ("inline", TT::Keyword(KK::Inline)),
            ("interface", TT::Keyword(KK::Interface)),
            ("is", TT::Keyword(KK::Is)),
            ("label", TT::Keyword(KK::Label)),
            ("library", TT::Keyword(KK::Library)),
            ("local", TT::IdentifierOrKeyword(KK::Local)),
            ("message", TT::IdentifierOrKeyword(KK::Message)),
            ("mod", TT::Keyword(KK::Mod)),
            ("name", TT::IdentifierOrKeyword(KK::Name)),
            ("near", TT::IdentifierOrKeyword(KK::Near)),
            ("nil", TT::Keyword(KK::Nil)),
            ("nodefault", TT::IdentifierOrKeyword(KK::NoDefault)),
            ("not", TT::Keyword(KK::Not)),
            ("object", TT::Keyword(KK::Object)),
            ("of", TT::Keyword(KK::Of)),
            ("on", TT::IdentifierOrKeyword(KK::On)),
            ("operator", TT::IdentifierOrKeyword(KK::Operator)),
            ("or", TT::Keyword(KK::Or)),
            ("out", TT::IdentifierOrKeyword(KK::Out)),
            ("overload", TT::IdentifierOrKeyword(KK::Overload)),
            ("override", TT::IdentifierOrKeyword(KK::Override)),
            ("package", TT::IdentifierOrKeyword(KK::Package)),
            ("packed", TT::Keyword(KK::Packed)),
            ("pascal", TT::IdentifierOrKeyword(KK::Pascal)),
            ("platform", TT::IdentifierOrKeyword(KK::Platform)),
            ("private", TT::IdentifierOrKeyword(KK::Private)),
            ("procedure", TT::Keyword(KK::Procedure)),
            ("program", TT::Keyword(KK::Program)),
            ("property", TT::Keyword(KK::Property)),
            ("protected", TT::IdentifierOrKeyword(KK::Protected)),
            ("public", TT::IdentifierOrKeyword(KK::Public)),
            ("published", TT::IdentifierOrKeyword(KK::Published)),
            ("raise", TT::Keyword(KK::Raise)),
            ("read", TT::IdentifierOrKeyword(KK::Read)),
            ("readonly", TT::IdentifierOrKeyword(KK::ReadOnly)),
            ("record", TT::Keyword(KK::Record)),
            ("reference", TT::IdentifierOrKeyword(KK::Reference)),
            ("register", TT::IdentifierOrKeyword(KK::Register)),
            ("reintroduce", TT::IdentifierOrKeyword(KK::Reintroduce)),
            ("repeat", TT::Keyword(KK::Repeat)),
            ("requires", TT::IdentifierOrKeyword(KK::Requires)),
            ("resident", TT::IdentifierOrKeyword(KK::Resident)),
            ("resourcestring", TT::Keyword(KK::ResourceString)),
            ("safecall", TT::IdentifierOrKeyword(KK::SafeCall)),
            ("sealed", TT::IdentifierOrKeyword(KK::Sealed)),
            ("set", TT::Keyword(KK::Set)),
            ("shl", TT::Keyword(KK::Shl)),
            ("shr", TT::Keyword(KK::Shr)),
            ("static", TT::IdentifierOrKeyword(KK::Static)),
            ("stdcall", TT::IdentifierOrKeyword(KK::StdCall)),
            ("stored", TT::IdentifierOrKeyword(KK::Stored)),
            ("strict", TT::IdentifierOrKeyword(KK::Strict)),
            ("string", TT::Keyword(KK::String)),
            ("then", TT::Keyword(KK::Then)),
            ("threadvar", TT::Keyword(KK::ThreadVar)),
            ("to", TT::Keyword(KK::To)),
            ("try", TT::Keyword(KK::Try)),
            ("type", TT::Keyword(KK::Type)),
            ("unit", TT::Keyword(KK::Unit)),
            ("unsafe", TT::IdentifierOrKeyword(KK::Unsafe)),
            ("until", TT::Keyword(KK::Until)),
            ("uses", TT::Keyword(KK::Uses)),
            ("var", TT::Keyword(KK::Var)),
            ("varargs", TT::IdentifierOrKeyword(KK::VarArgs)),
            ("virtual", TT::IdentifierOrKeyword(KK::Virtual)),
            ("while", TT::Keyword(KK::While)),
            ("winapi", TT::IdentifierOrKeyword(KK::WinApi)),
            ("with", TT::Keyword(KK::With)),
            ("write", TT::IdentifierOrKeyword(KK::Write)),
            ("writeonly", TT::IdentifierOrKeyword(KK::WriteOnly)),
            ("xor", TT::Keyword(KK::Xor)),
        ]
        .into_iter()
        .for_each(run_casing_test);
    }

    #[test]
    fn lex_qualified_keyword() {
        // prefixing a keyword with a dot '.' should make it an identifier
        run_test(
            "
            System.String
            A.if
            A.{}if
            A.{}{}if
            A.{$if}if
            A.{$define B}if
            A.//
            if
            .if
            if
            ",
            &[
                ("System", TT::Identifier),
                (".", TT::Op(OK::Dot)),
                ("String", TT::Identifier),
                //
                ("A", TT::Identifier),
                (".", TT::Op(OK::Dot)),
                ("if", TT::Identifier),
                //
                ("A", TT::Identifier),
                (".", TT::Op(OK::Dot)),
                ("{}", TT::Comment(CommentKind::InlineBlock)),
                ("if", TT::Identifier),
                //
                ("A", TT::Identifier),
                (".", TT::Op(OK::Dot)),
                ("{}", TT::Comment(CommentKind::InlineBlock)),
                ("{}", TT::Comment(CommentKind::InlineBlock)),
                ("if", TT::Identifier),
                //
                ("A", TT::Identifier),
                (".", TT::Op(OK::Dot)),
                ("{$if}", TT::ConditionalDirective(CDK::If)),
                ("if", TT::Identifier),
                //
                ("A", TT::Identifier),
                (".", TT::Op(OK::Dot)),
                ("{$define B}", TT::CompilerDirective),
                ("if", TT::Identifier),
                //
                ("A", TT::Identifier),
                (".", TT::Op(OK::Dot)),
                ("//", TT::Comment(CommentKind::InlineLine)),
                ("if", TT::Identifier),
                //
                (".", TT::Op(OK::Dot)),
                ("if", TT::Identifier),
                //
                ("if", TT::Keyword(KK::If)),
            ],
        );
    }

    #[test]
    fn lex_function_declaration() {
        run_test(
            "function Foo(Arg1:String;Arg2:Bar);stdcall;",
            &[
                ("function", TT::Keyword(KK::Function)),
                ("Foo", TT::Identifier),
                ("(", TT::Op(OK::LParen)),
                ("Arg1", TT::Identifier),
                (":", TT::Op(OK::Colon)),
                ("String", TT::Keyword(KK::String)),
                (";", TT::Op(OK::Semicolon)),
                ("Arg2", TT::Identifier),
                (":", TT::Op(OK::Colon)),
                ("Bar", TT::Identifier),
                (")", TT::Op(OK::RParen)),
                (";", TT::Op(OK::Semicolon)),
                ("stdcall", TT::IdentifierOrKeyword(KK::StdCall)),
                (";", TT::Op(OK::Semicolon)),
            ],
        );
    }

    #[test]
    fn lex_invalid_code() {
        run_test(
            "? ? ?",
            &[("?", TT::Unknown), ("?", TT::Unknown), ("?", TT::Unknown)],
        );
    }

    #[test]
    fn inline_assembly_with_end_in_label() {
        run_test(
            indoc! {"
            asm
            @@end:
                XOR RBX, RBX
            end
            "},
            &[
                ("asm", TT::Keyword(KK::Asm)),
                ("@@end", TT::Identifier),
                (":", TT::Op(OK::Colon)),
                ("XOR", TT::Identifier),
                ("RBX", TT::Identifier),
                (",", TT::Op(OK::Comma)),
                ("RBX", TT::Identifier),
                ("end", TT::Keyword(KK::End)),
            ],
        );
    }

    #[test]
    fn inline_assembly_unterminated() {
        run_test(
            indoc! {"
            asm
                XOR RBX, RBX
            "},
            &[
                ("asm", TT::Keyword(KK::Asm)),
                ("XOR", TT::Identifier),
                ("RBX", TT::Identifier),
                (",", TT::Op(OK::Comma)),
                ("RBX", TT::Identifier),
            ],
        );
    }

    #[test]
    fn inline_assembly_with_end_in_ifdef() {
        run_test(
            indoc! {"
            asm
                XOR RBX, RBX {$ifdef End}
            end
            "},
            &[
                ("asm", TT::Keyword(KK::Asm)),
                ("XOR", TT::Identifier),
                ("RBX", TT::Identifier),
                (",", TT::Op(OK::Comma)),
                ("RBX", TT::Identifier),
                ("{$ifdef End}", IFDEF_DIRECTIVE),
                ("end", TT::Keyword(KK::End)),
            ],
        );
    }
    #[test]
    fn inline_assembly_with_end_in_comment() {
        run_test(
            indoc! {"
            asm
                XOR RBX, RBX // End
            end
            "},
            &[
                ("asm", TT::Keyword(KK::Asm)),
                ("XOR", TT::Identifier),
                ("RBX", TT::Identifier),
                (",", TT::Op(OK::Comma)),
                ("RBX", TT::Identifier),
                ("// End", TT::Comment(CommentKind::InlineLine)),
                ("end", TT::Keyword(KK::End)),
            ],
        );
    }
    #[test]
    fn inline_assembly_with_end_in_keyword() {
        run_test(
            indoc! {"
            asm
                XOR RBX, IfEnd
            end
            "},
            &[
                ("asm", TT::Keyword(KK::Asm)),
                ("XOR", TT::Identifier),
                ("RBX", TT::Identifier),
                (",", TT::Op(OK::Comma)),
                ("IfEnd", TT::Identifier),
                ("end", TT::Keyword(KK::End)),
            ],
        );
    }
    #[test]
    fn inline_assembly_with_labels() {
        run_test(
            indoc! {"
            asm
                @@A:
                @A:
                @A@a:
                @_:
                @0:
            end
            "},
            &[
                ("asm", TT::Keyword(KK::Asm)),
                ("@@A", TT::Identifier),
                (":", TT::Op(OK::Colon)),
                ("@A", TT::Identifier),
                (":", TT::Op(OK::Colon)),
                ("@A@a", TT::Identifier),
                (":", TT::Op(OK::Colon)),
                ("@_", TT::Identifier),
                (":", TT::Op(OK::Colon)),
                ("@0", TT::Identifier),
                (":", TT::Op(OK::Colon)),
                ("end", TT::Keyword(KK::End)),
            ],
        );
    }
    #[test]
    fn inline_assembly_with_double_quotes() {
        run_test(
            indoc! {"
            asm
                CMP AL,\"'\"
                XOR RBX, RBX
            end
            "},
            &[
                ("asm", TT::Keyword(KK::Asm)),
                ("CMP", TT::Identifier),
                ("AL", TT::Identifier),
                (",", TT::Op(OK::Comma)),
                ("\"'\"", TT::TextLiteral(TLK::Asm)),
                ("XOR", TT::Identifier),
                ("RBX", TT::Identifier),
                (",", TT::Op(OK::Comma)),
                ("RBX", TT::Identifier),
                ("end", TT::Keyword(KK::End)),
            ],
        );
    }

    #[test]
    fn inline_assembly_with_escaped_double_quotes() {
        run_test(
            indoc! {r#"
            asm
                CMP AL,"\""
            end
            "#},
            &[
                ("asm", TT::Keyword(KK::Asm)),
                ("CMP", TT::Identifier),
                ("AL", TT::Identifier),
                (",", TT::Op(OK::Comma)),
                (r#""\"""#, TT::TextLiteral(TLK::Asm)),
                ("end", TT::Keyword(KK::End)),
            ],
        );
    }

    #[test]
    fn unterminated_asm_text_literal() {
        run_test(
            indoc! {"
            asm
                CMP AL,\"a
            end
            "},
            &[
                ("asm", TT::Keyword(KK::Asm)),
                ("CMP", TT::Identifier),
                ("AL", TT::Identifier),
                (",", TT::Op(OK::Comma)),
                ("\"a", TT::TextLiteral(TLK::Unterminated)),
                ("end", TT::Keyword(KK::End)),
            ],
        );
    }

    #[test]
    fn unterminated_asm_text_literal_at_eof() {
        run_test(
            r#"asm"\"#,
            &[
                ("asm", TT::Keyword(KK::Asm)),
                ("\"\\", TT::TextLiteral(TLK::Unterminated)),
            ],
        );
    }

    #[test]
    fn inline_assembly_with_comments() {
        run_test(
            indoc! {"
            asm
                MOV RAX, 0 // comment
                XOR RBX, RBX
            end
            "},
            &[
                ("asm", TT::Keyword(KK::Asm)),
                ("MOV", TT::Identifier),
                ("RAX", TT::Identifier),
                (",", TT::Op(OK::Comma)),
                ("0", TT::NumberLiteral(NLK::Decimal)),
                ("// comment", TT::Comment(CommentKind::InlineLine)),
                ("XOR", TT::Identifier),
                ("RBX", TT::Identifier),
                (",", TT::Op(OK::Comma)),
                ("RBX", TT::Identifier),
                ("end", TT::Keyword(KK::End)),
            ],
        );
    }
    #[test]
    fn inline_assembly() {
        run_test(
            indoc! {"
            asm
                MOV RAX, 0
                XOR RBX, RBX
            end
            "},
            &[
                ("asm", TT::Keyword(KK::Asm)),
                ("MOV", TT::Identifier),
                ("RAX", TT::Identifier),
                (",", TT::Op(OK::Comma)),
                ("0", TT::NumberLiteral(NLK::Decimal)),
                ("XOR", TT::Identifier),
                ("RBX", TT::Identifier),
                (",", TT::Op(OK::Comma)),
                ("RBX", TT::Identifier),
                ("end", TT::Keyword(KK::End)),
            ],
        );
    }

    #[test]
    fn identifier_starting_with_asm() {
        run_test(
            "begin var asmA := 0; end;",
            &[
                ("begin", TT::Keyword(KK::Begin)),
                ("var", TT::Keyword(KK::Var)),
                ("asmA", TT::Identifier),
                (":=", TT::Op(OK::Assign)),
                ("0", TT::NumberLiteral(NLK::Decimal)),
                (";", TT::Op(OK::Semicolon)),
                ("end", TT::Keyword(KK::End)),
                (";", TT::Op(OK::Semicolon)),
            ],
        );
    }

    #[test]
    fn inline_assembly_numeric_constants() {
        run_test(
            indoc! {"
            asm
                0 0O 0o 0B 0b 0H 0h $0 0AH 0FH
            end
            "},
            &[
                ("asm", TT::Keyword(KK::Asm)),
                ("0", TT::NumberLiteral(NLK::Decimal)),
                ("0O", TT::NumberLiteral(NLK::Octal)),
                ("0o", TT::NumberLiteral(NLK::Octal)),
                ("0B", TT::NumberLiteral(NLK::Binary)),
                ("0b", TT::NumberLiteral(NLK::Binary)),
                ("0H", TT::NumberLiteral(NLK::Hex)),
                ("0h", TT::NumberLiteral(NLK::Hex)),
                ("$0", TT::NumberLiteral(NLK::Hex)),
                ("0AH", TT::NumberLiteral(NLK::Hex)),
                ("0FH", TT::NumberLiteral(NLK::Hex)),
                ("end", TT::Keyword(KK::End)),
            ],
        );
    }

    #[test]
    fn inline_assembly_invalid_numeric_constants() {
        run_test(
            indoc! {"
            asm
                8O 2B $0H 0A 0GH
                $A_A 00_11B 9_5
            end
            "},
            &[
                ("asm", TT::Keyword(KK::Asm)),
                ("8O", TT::NumberLiteral(NLK::Octal)),
                ("2B", TT::NumberLiteral(NLK::Binary)),
                ("$0", TT::NumberLiteral(NLK::Hex)),
                ("H", TT::Identifier),
                ("0A", TT::NumberLiteral(NLK::Decimal)),
                ("0", TT::NumberLiteral(NLK::Decimal)),
                ("GH", TT::Identifier),
                ("$A_A", TT::NumberLiteral(NLK::Hex)),
                ("00_11B", TT::NumberLiteral(NLK::Binary)),
                ("9_5", TT::NumberLiteral(NLK::Decimal)),
                ("end", TT::Keyword(KK::End)),
            ],
        );
    }

    #[test]
    fn unicode() {
        run_test(
            "böb öb bö ö",
            &[
                ("böb", TT::Identifier),
                ("öb", TT::Identifier),
                ("bö", TT::Identifier),
                ("ö", TT::Identifier),
            ],
        );

        // Japanese
        run_test("クールなテスト", &[("クールなテスト", TT::Identifier)]);
        // Chinese (Traditional)
        run_test("酷測試", &[("酷測試", TT::Identifier)]);
        // Korean
        run_test("멋진테스트", &[("멋진테스트", TT::Identifier)]);

        /*
            Codepoints above U+FFFF are all surrogate pairs, and don't seem to be allowed in identifiers.
            However, the documentation hints that with the right compiler CodePage setting you could get
            this to work, so we are treating all of these codepoints as valid identifiers.  There's
            no harm in doing so.
        */
        // Emojis
        run_test("🚀🦀🚀🦀", &[("🚀🦀🚀🦀", TT::Identifier)]);
        // Enclosed Alphanumeric
        run_test("🄲🄾🄾🄻🅃🄴🅂🅃", &[("🄲🄾🄾🄻🅃🄴🅂🅃", TT::Identifier)]);
    }

    #[test]
    fn unicode_fullwidth_chars() {
        // Fullwidth characters - valid identifier chars
        run_test("ｃｏｏｌｔｅｓｔ", &[("ｃｏｏｌｔｅｓｔ", TT::Identifier)]);

        // Fullwidth spaces - invalid identifier char (but valid as whitespace)
        run_test("a　b", &[("a", TT::Identifier), ("b", TT::Identifier)]);

        /*
            Fullwidth numerals [U+FF10, U+FF19]

            To match the Delphi compiler exactly, these shouldn't be allowed at the start of identifiers, which suggests
            that they are treated like regular numerals. However, these numerals are not allowed in integer literals.

            One explanation that might make sense is the characters are normalised only in identifiers (perhaps in
            performing some kind of case-folding & normalisation like NFKC_Casefold).

            We do NOT handle this case correctly, and we're fine with that because it should never break anyone's code;
            something lexed as one token instead of two shouldn't cause valid code to be formatted to something invalid.
        */
        run_test(
            "０ １ ２ ３ ４ ５ ６ ７ ８ ９",
            &[
                ("０", TT::Identifier),
                ("１", TT::Identifier),
                ("２", TT::Identifier),
                ("３", TT::Identifier),
                ("４", TT::Identifier),
                ("５", TT::Identifier),
                ("６", TT::Identifier),
                ("７", TT::Identifier),
                ("８", TT::Identifier),
                ("９", TT::Identifier),
            ],
        );
    }

    #[test]
    fn unicode_whitespace() {
        // Exotic whitespace characters are not generally considered whitespace, but it appears that an exception was
        // made for 'ideographic space'. We presume this is because it is used in Japanese text.
        run_test(
            "NBSP\u{A0} EN_QUAD\u{2000} THIN_SPACE\u{2009} ZERO_WIDTH_NBSP\u{FEFF} IDEOGRAPHIC_SPACE\u{3000}",
            &[
                ("NBSP\u{A0}", TT::Identifier),
                ("EN_QUAD\u{2000}", TT::Identifier),
                ("THIN_SPACE\u{2009}", TT::Identifier),
                ("ZERO_WIDTH_NBSP\u{FEFF}", TT::Identifier),
                // note, does not contain the U+3000 character
                ( "IDEOGRAPHIC_SPACE", TT::Identifier),
            ],
        )
    }

    #[test]
    fn ascii_whitespace() {
        // Placing the whitespace after the identifiers ensures that it's not included as part of the preceding
        // identifier AND it is identified to be leading whitespace for the next identifier.
        run_test(
            indoc! {
                "
                NUL\x00
                SOH\x01
                STX\x02
                ETX\x03
                EOT\x04
                ENQ\x05
                ACK\x06
                BEL\x07
                BS\x08
                HT\x09
                LF\x0A
                VT\x0B
                FF\x0C
                CR\x0D
                SO\x0E
                SI\x0F
                DLE\x10
                DC1\x11
                DC2\x12
                DC3\x13
                DC4\x14
                NAK\x15
                SYN\x16
                ETB\x17
                CAN\x18
                EM\x19
                SUB\x1A
                ESC\x1B
                FS\x1C
                GS\x1D
                RS\x1E
                US\x1F
                Space\x20
                "
            },
            &[
                ("NUL", TT::Identifier),
                ("SOH", TT::Identifier),
                ("STX", TT::Identifier),
                ("ETX", TT::Identifier),
                ("EOT", TT::Identifier),
                ("ENQ", TT::Identifier),
                ("ACK", TT::Identifier),
                ("BEL", TT::Identifier),
                ("BS", TT::Identifier),
                ("HT", TT::Identifier),
                ("LF", TT::Identifier),
                ("VT", TT::Identifier),
                ("FF", TT::Identifier),
                ("CR", TT::Identifier),
                ("SO", TT::Identifier),
                ("SI", TT::Identifier),
                ("DLE", TT::Identifier),
                ("DC1", TT::Identifier),
                ("DC2", TT::Identifier),
                ("DC3", TT::Identifier),
                ("DC4", TT::Identifier),
                ("NAK", TT::Identifier),
                ("SYN", TT::Identifier),
                ("ETB", TT::Identifier),
                ("CAN", TT::Identifier),
                ("EM", TT::Identifier),
                ("SUB", TT::Identifier),
                ("ESC", TT::Identifier),
                ("FS", TT::Identifier),
                ("GS", TT::Identifier),
                ("RS", TT::Identifier),
                ("US", TT::Identifier),
                ("Space", TT::Identifier),
            ],
        )
    }
}
