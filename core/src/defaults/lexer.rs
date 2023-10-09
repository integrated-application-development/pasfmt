use std::ops::RangeInclusive;

use crate::lang::ConditionalDirectiveKind::*;
use crate::lang::KeywordKind::*;
use crate::lang::NumberLiteralKind::*;
use crate::lang::OperatorKind::*;
use crate::lang::TokenType::*;
use crate::lang::*;
use crate::traits::Lexer;

use cfg_if::cfg_if;
use log::*;

pub struct DelphiLexer {}
impl Lexer for DelphiLexer {
    fn lex<'a>(&self, input: &'a str) -> Vec<Token<'a>> {
        lex_complete(input)
    }
}

#[derive(Copy, Clone)]
struct LexState {
    is_first: bool,
    in_asm: bool,
}

struct LexedToken<'a> {
    whitespace_count: usize,
    token_content: &'a str,
    token_type: TokenType,
}

fn lex_complete(input: &str) -> Vec<Token> {
    let (remaining, tokens) = lex(input);

    // Remaining input is always a programming error; invalid input should turn into 'Unknown' tokens.
    assert!(
        remaining.is_empty(),
        "Failed to lex the entire input. Remaining input starts with: {}",
        rounded_prefix(remaining, 100)
    );
    tokens
}

fn lex(mut input: &str) -> (&str, Vec<Token>) {
    // Experimentally it was determined that this linear regression on input length is best on average.
    // The performance difference from the default capacity is minor, but measurable.
    let mut tokens = Vec::with_capacity(input.len() / 8);

    let mut lex_state = LexState {
        in_asm: false,
        is_first: true,
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
) -> Token<'_> {
    let whitespace_count: u32 = whitespace_count
        .try_into()
        .unwrap_or_else(|_| truncate_whitespace(&mut token_content, whitespace_count));
    Token::RefToken(RefToken::new(token_content, whitespace_count, token_type))
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

fn count_leading_whitespace(input: &str) -> usize {
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

type OffsetAndTokenType = (usize, TokenType);

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
        (ByteSet::List(b"&"), Some(ampersand_number_literal_or_ident)),
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
        (ByteSet::Range(0x80..=0xFF), Some(lex_unicode)),
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
fn lex_token_with_map(map: [LexerFn; 256], args: LexArgs) -> Option<(usize, TokenType)> {
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
const KEYWORDS: [(&str, TokenType); 123] = [
    ("absolute", IdentifierOrKeyword(Absolute)),
    ("abstract", IdentifierOrKeyword(Abstract)),
    ("add", IdentifierOrKeyword(Add)),
    ("align", IdentifierOrKeyword(Align)),
    ("and", Keyword(And)),
    ("array", Keyword(Array)),
    ("as", Keyword(As)),
    ("asm", Keyword(Asm)),
    ("assembler", IdentifierOrKeyword(Assembler)),
    ("at", IdentifierOrKeyword(At)),
    ("automated", IdentifierOrKeyword(Automated)),
    ("begin", Keyword(Begin)),
    ("case", Keyword(Case)),
    ("cdecl", IdentifierOrKeyword(Cdecl)),
    ("class", Keyword(Class)),
    ("const", Keyword(Const)),
    ("constructor", Keyword(Constructor)),
    ("contains", IdentifierOrKeyword(Contains)),
    ("default", IdentifierOrKeyword(Default)),
    ("delayed", IdentifierOrKeyword(Delayed)),
    ("deprecated", IdentifierOrKeyword(Deprecated)),
    ("destructor", Keyword(Destructor)),
    ("dispid", IdentifierOrKeyword(DispId)),
    ("dispinterface", Keyword(DispInterface)),
    ("div", Keyword(Div)),
    ("do", Keyword(Do)),
    ("downto", Keyword(Downto)),
    ("dynamic", IdentifierOrKeyword(Dynamic)),
    ("else", Keyword(KeywordKind::Else)),
    ("end", Keyword(End)),
    ("except", Keyword(Except)),
    ("experimental", IdentifierOrKeyword(Experimental)),
    ("export", IdentifierOrKeyword(Export)),
    ("exports", Keyword(Exports)),
    ("external", IdentifierOrKeyword(External)),
    ("far", IdentifierOrKeyword(Far)),
    ("file", Keyword(File)),
    ("final", IdentifierOrKeyword(Final)),
    ("finalization", Keyword(Finalization)),
    ("finally", Keyword(Finally)),
    ("for", Keyword(For)),
    ("forward", IdentifierOrKeyword(Forward)),
    ("function", Keyword(Function)),
    ("goto", Keyword(Goto)),
    ("helper", IdentifierOrKeyword(Helper)),
    ("if", Keyword(KeywordKind::If)),
    ("implementation", Keyword(Implementation)),
    ("implements", IdentifierOrKeyword(Implements)),
    ("in", Keyword(In)),
    ("index", IdentifierOrKeyword(Index)),
    ("inherited", Keyword(Inherited)),
    ("initialization", Keyword(Initialization)),
    ("inline", Keyword(Inline)),
    ("interface", Keyword(Interface)),
    ("is", Keyword(Is)),
    ("label", Keyword(Label)),
    ("library", Keyword(Library)),
    ("local", IdentifierOrKeyword(Local)),
    ("message", IdentifierOrKeyword(Message)),
    ("mod", Keyword(Mod)),
    ("name", IdentifierOrKeyword(Name)),
    ("near", IdentifierOrKeyword(Near)),
    ("nil", Keyword(Nil)),
    ("nodefault", IdentifierOrKeyword(NoDefault)),
    ("not", Keyword(Not)),
    ("object", Keyword(Object)),
    ("of", Keyword(Of)),
    ("on", IdentifierOrKeyword(On)),
    ("operator", IdentifierOrKeyword(Operator)),
    ("or", Keyword(Or)),
    ("out", IdentifierOrKeyword(Out)),
    ("overload", IdentifierOrKeyword(Overload)),
    ("override", IdentifierOrKeyword(Override)),
    ("package", IdentifierOrKeyword(Package)),
    ("packed", Keyword(Packed)),
    ("pascal", IdentifierOrKeyword(Pascal)),
    ("platform", IdentifierOrKeyword(Platform)),
    ("private", IdentifierOrKeyword(Private)),
    ("procedure", Keyword(Procedure)),
    ("program", Keyword(Program)),
    ("property", Keyword(Property)),
    ("protected", IdentifierOrKeyword(Protected)),
    ("public", IdentifierOrKeyword(Public)),
    ("published", IdentifierOrKeyword(Published)),
    ("raise", Keyword(Raise)),
    ("read", IdentifierOrKeyword(Read)),
    ("readonly", IdentifierOrKeyword(ReadOnly)),
    ("record", Keyword(Record)),
    ("reference", IdentifierOrKeyword(Reference)),
    ("register", IdentifierOrKeyword(Register)),
    ("reintroduce", IdentifierOrKeyword(Reintroduce)),
    ("remove", IdentifierOrKeyword(Remove)),
    ("repeat", Keyword(Repeat)),
    ("requires", IdentifierOrKeyword(Requires)),
    ("resident", IdentifierOrKeyword(Resident)),
    ("resourcestring", Keyword(ResourceString)),
    ("safecall", IdentifierOrKeyword(SafeCall)),
    ("sealed", IdentifierOrKeyword(Sealed)),
    ("set", Keyword(Set)),
    ("shl", Keyword(Shl)),
    ("shr", Keyword(Shr)),
    ("static", IdentifierOrKeyword(Static)),
    ("stdcall", IdentifierOrKeyword(StdCall)),
    ("stored", IdentifierOrKeyword(Stored)),
    ("strict", IdentifierOrKeyword(Strict)),
    ("then", Keyword(Then)),
    ("threadvar", Keyword(ThreadVar)),
    ("to", Keyword(To)),
    ("try", Keyword(Try)),
    ("type", Keyword(Type)),
    ("unit", Keyword(Unit)),
    ("unsafe", IdentifierOrKeyword(Unsafe)),
    ("until", Keyword(Until)),
    ("uses", Keyword(Uses)),
    ("var", Keyword(Var)),
    ("varargs", IdentifierOrKeyword(VarArgs)),
    ("variant", IdentifierOrKeyword(Variant)),
    ("virtual", IdentifierOrKeyword(Virtual)),
    ("while", Keyword(While)),
    ("with", Keyword(With)),
    ("write", IdentifierOrKeyword(Write)),
    ("writeonly", IdentifierOrKeyword(WriteOnly)),
    ("xor", Keyword(Xor)),
];

fn get_word_token_type(input: &str) -> TokenType {
    const fn make_keyword_lookup_table<'a, const N: usize>(
        keywords: &[(&'a str, TokenType)],
    ) -> [Option<(&'a str, TokenType)>; N] {
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
        240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240,
        240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240,
        240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240,
        240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 13, 66, 33, 2, 0, 37, 48, 74, 31,
        10, 240, 28, 53, 20, 1, 64, 1, 7, 0, 5, 70, 97, 104, 94, 61, 240, 240, 240, 240, 240, 240,
        240, 13, 66, 33, 2, 0, 37, 48, 74, 31, 10, 240, 28, 53, 20, 1, 64, 1, 7, 0, 5, 70, 97, 104,
        94, 61, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240,
        240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240,
        240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240,
        240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240,
        240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240,
        240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240,
        240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240, 240,
        240, 240, 240, 240, 240, 240, 240, 240, 240, 240,
    ];

    #[allow(clippy::len_zero)]
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

    const KEYWORD_LOOKUP_TABLE: [Option<(&'static str, TokenType)>;
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

    Identifier
}
// endregion: keywords

// region: identifiers/keywords

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
// SAFETY: callers must ensure avx2 intrinsics are supported.
unsafe fn find_identifier_end_avx2(input: &str, mut offset: usize) -> usize {
    use core::mem::size_of;
    use std::arch::x86_64::*;

    unsafe fn range_mask(x: __m256i, range: RangeInclusive<u8>) -> __m256i {
        unsafe {
            let lower = _mm256_cmpgt_epi8(_mm256_set1_epi8(*range.end() as i8 + 1), x);
            let upper = _mm256_cmpgt_epi8(x, _mm256_set1_epi8(*range.start() as i8 - 1));
            _mm256_and_si256(upper, lower)
        }
    }

    unsafe fn any_non_ascii(chunk: std::arch::x86_64::__m256i) -> bool {
        unsafe { _mm256_testz_si256(_mm256_set1_epi8(i8::MIN), chunk) == 0 }
    }

    while (offset + size_of::<__m256>()) <= input.len() {
        // SAFETY: requires that a 32-byte load from `input.as_ptr() + offset` does not touch uninitialised memory.
        // The above length check guarantees this.
        let ident_mask = unsafe {
            let chunk = _mm256_loadu_si256(core::mem::transmute(input.as_ptr().add(offset)));
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
        offset += size_of::<__m256>();
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
        unsafe { fun(input, offset) }
    }

    unsafe {
        let fun = FN.load(Ordering::Relaxed);
        core::mem::transmute::<Fn, RealFn>(fun)(input, offset)
    }
}

fn find_identifier_end(input: &str, offset: usize) -> usize {
    cfg_if! {
        if #[cfg(target_arch = "x86_64")] {
            find_identifier_end_x86_64(input, offset)
        } else {
            find_identifier_end_generic(input, offset)
        }
    }
}

fn count_bytes_in_set(input: &str, offset: usize, set: &[bool; 256]) -> usize {
    count_matching(input, offset, |b| set[*b as usize])
}

fn lex_unicode(mut args: LexArgs) -> OffsetAndTokenType {
    while !args.input.is_char_boundary(args.offset) {
        args.offset += 1;
    }
    identifier(args)
}

fn identifier(args: LexArgs) -> OffsetAndTokenType {
    (find_identifier_end(args.input, args.offset), Identifier)
}

fn identifier_or_keyword(args: LexArgs) -> OffsetAndTokenType {
    let end_offset = find_identifier_end(args.input, args.offset);
    let word = &args.input[(args.offset - 1)..end_offset];
    let token_type = get_word_token_type(word);
    args.lex_state.in_asm = token_type == Keyword(Asm);
    (end_offset, token_type)
}

fn escaped_identifier(
    LexArgs {
        input, mut offset, ..
    }: LexArgs,
) -> OffsetAndTokenType {
    offset += count_matching(input, offset, |b| *b == b'&');
    offset = find_identifier_end(input, offset);
    (offset, Identifier)
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
        Identifier,
    )
}

fn asm_identifier(args: LexArgs) -> OffsetAndTokenType {
    let end_offset = find_identifier_end(args.input, args.offset);

    if args.input[(args.offset - 1)..end_offset].eq_ignore_ascii_case("end") {
        args.lex_state.in_asm = false;
        (end_offset, Keyword(End))
    } else {
        (end_offset, Identifier)
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
            return ParseState::Stop;
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
        (offset, TextLiteral)
    };

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

    (offset, TextLiteral)
}

fn asm_text_literal(mut args: LexArgs) -> OffsetAndTokenType {
    let start_offset = args.offset;

    loop {
        match args.next_byte() {
            Some(b'\\') => {
                args.offset += 1;
            }
            Some(b'\"') => {
                return (args.offset + 1, TextLiteral);
            }
            None | Some(b'\n' | b'\r') => {
                break;
            }
            _ => {}
        }
        args.offset += 1;
    }

    warn_unterminated("asm text literal", args.input, start_offset);
    (args.offset, TextLiteral)
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
        Some(b'O' | b'o') => (args.offset + 1, NumberLiteral(Octal)),
        Some(b'H' | b'h') => (args.offset + 1, NumberLiteral(Hex)),
        _ => {
            // if the literal ended in a 'b' it would have been consumed as hex
            match args.prev_byte() {
                Some(b'B' | b'b') => (args.offset, NumberLiteral(Binary)),
                _ => (args.offset, NumberLiteral(Decimal)),
            }
        }
    }
}

fn dec_number_literal(mut args: LexArgs) -> OffsetAndTokenType {
    args.offset += count_decimal(args.input, args.offset);
    if args.next_byte() == Some(&b'.') {
        args.offset += 1;
        args.offset += count_full_decimal(args.input, args.offset);
    }
    if matches!(args.next_byte(), Some(&b'e' | b'E')) {
        args.offset += 1;
        if matches!(args.next_byte(), Some(&b'+' | b'-')) {
            args.offset += 1;
        }
        args.offset += count_full_decimal(args.input, args.offset);
    }

    (args.offset, NumberLiteral(Decimal))
}

fn hex_number_literal(args: LexArgs) -> OffsetAndTokenType {
    (
        args.offset + count_hex(args.input, args.offset),
        NumberLiteral(Hex),
    )
}
fn binary_number_literal(args: LexArgs) -> OffsetAndTokenType {
    (
        args.offset + count_binary(args.input, args.offset),
        NumberLiteral(Binary),
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
fn consume_to_eof(input: &str, token_type: TokenType) -> (usize, TokenType) {
    let trim_count = count_unicode_whitespace(input.chars().rev());
    (input.len() - trim_count, token_type)
}

// region: directives/comments

fn compiler_directive_type(input: &str, offset: usize) -> TokenType {
    let count = count_matching(input, offset, |b| b.is_ascii_alphabetic());
    // `to_lowercase` here is slow, but this code isn't hit very often.
    match input[offset..(offset + count)].to_lowercase().as_str() {
        "if" => ConditionalDirective(ConditionalDirectiveKind::If),
        "ifdef" => ConditionalDirective(Ifdef),
        "ifndef" => ConditionalDirective(Ifndef),
        "ifopt" => ConditionalDirective(Ifopt),
        "elseif" => ConditionalDirective(Elseif),
        "else" => ConditionalDirective(ConditionalDirectiveKind::Else),
        "ifend" => ConditionalDirective(Ifend),
        "endif" => ConditionalDirective(Endif),
        _ => CompilerDirective,
    }
}

fn _compiler_directive<const START_LEN: usize>(
    input: &str,
    start_offset: usize,
    end_offset: Option<usize>,
) -> OffsetAndTokenType {
    let token_type = compiler_directive_type(input, start_offset);
    if let Some(pos) = end_offset {
        (pos, token_type)
    } else {
        warn_unterminated("compiler directive", input, start_offset - START_LEN);
        consume_to_eof(input, token_type)
    }
}

fn compiler_directive_alt(args: LexArgs) -> OffsetAndTokenType {
    let end_offset = memchr::memmem::find(args.input.as_bytes(), b"*)").map(|o| o + 2);
    _compiler_directive::<2>(args.input, args.offset, end_offset)
}

fn compiler_directive(args: LexArgs) -> OffsetAndTokenType {
    let end_offset = memchr::memchr(b'}', args.input.as_bytes()).map(|o| o + 1);
    _compiler_directive::<1>(args.input, args.offset, end_offset)
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

fn _block_comment<const START_LEN: usize>(
    LexArgs {
        input,
        offset,
        lex_state,
    }: LexArgs,
    end_offset: Option<usize>,
) -> OffsetAndTokenType {
    if let Some(end_offset) = end_offset {
        let nl_offset =
            memchr::memchr(b'\n', &input.as_bytes()[..end_offset]).unwrap_or(input.len());
        let comment_kind = block_comment_kind(nl_offset, offset, end_offset, lex_state);
        (end_offset, Comment(comment_kind))
    } else {
        warn_unterminated("block comment", input, offset - START_LEN);
        consume_to_eof(input, Comment(CommentKind::MultilineBlock))
    }
}

fn block_comment_alt(args: LexArgs) -> OffsetAndTokenType {
    let end_offset = memchr::memmem::find(args.input.as_bytes(), b"*)").map(|pos| pos + 2);
    _block_comment::<2>(args, end_offset)
}

fn block_comment(args: LexArgs) -> OffsetAndTokenType {
    let end_offset = memchr::memchr(b'}', args.input.as_bytes()).map(|pos| pos + 1);
    _block_comment::<1>(args, end_offset)
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
        Comment(kind),
    )
}

fn compiler_directive_or_comment_alt(args: LexArgs) -> OffsetAndTokenType {
    match args.next_byte() {
        Some(b'$') => compiler_directive_alt(args.consume(1)),
        _ => block_comment_alt(args),
    }
}

fn compiler_directive_or_comment(args: LexArgs) -> OffsetAndTokenType {
    match args.next_byte() {
        Some(b'$') => compiler_directive(args.consume(1)),
        _ => block_comment(args),
    }
}

// endregion: directives/comments

fn ampersand_number_literal_or_ident(args: LexArgs) -> OffsetAndTokenType {
    match args.next_byte() {
        Some(b'$') => hex_number_literal(args.consume(1)),
        Some(b'%') => binary_number_literal(args.consume(1)),
        Some(b'0'..=b'9') => dec_number_literal(args.consume(1)),
        Some(b'a'..=b'z' | b'A'..=b'Z' | b'_') => identifier(args.consume(1)),
        Some(b'&') => escaped_identifier(args.consume(1)),
        _ => unknown(args),
    }
}

// region: operators

macro_rules! basic_op {
    ($name: ident, $typ: ident) => {
        fn $name(args: LexArgs) -> OffsetAndTokenType {
            (args.offset, TokenType::Op($typ))
        }
    };
}

// All of these operators are dead simple, because the meaning does not depend on what comes after.
// The other 'operators' are more complicated and require looking ahead.
basic_op!(plus, Plus);
basic_op!(minus, Minus);
basic_op!(star, Star);
basic_op!(comma, Comma);
basic_op!(semicolon, Semicolon);
basic_op!(equal, Equal);
basic_op!(pointer, Pointer);
basic_op!(address_of, AddressOf);
basic_op!(l_brack, LBrack);
basic_op!(r_brack, RBrack);
basic_op!(r_paren, RParen);

fn l_paren(args: LexArgs) -> OffsetAndTokenType {
    match args.next_byte() {
        Some(b'*') => compiler_directive_or_comment_alt(args.consume(1)),
        Some(b'.') => (args.offset + 1, TokenType::Op(LBrack)),
        _ => (args.offset, TokenType::Op(LParen)),
    }
}

fn l_brace(args: LexArgs) -> OffsetAndTokenType {
    compiler_directive_or_comment(args)
}

fn slash(args: LexArgs) -> OffsetAndTokenType {
    match args.next_byte() {
        Some(b'/') => line_comment(args.consume(1)),
        _ => (args.offset, TokenType::Op(Slash)),
    }
}

fn colon(args: LexArgs) -> OffsetAndTokenType {
    match args.next_byte() {
        Some(b'=') => (args.offset + 1, TokenType::Op(Assign)),
        _ => (args.offset, TokenType::Op(Colon)),
    }
}

fn l_angle(args: LexArgs) -> OffsetAndTokenType {
    match args.next_byte() {
        Some(b'=') => (args.offset + 1, TokenType::Op(LessEqual)),
        Some(b'>') => (args.offset + 1, TokenType::Op(NotEqual)),
        _ => (args.offset, TokenType::Op(LessThan)),
    }
}

fn r_angle(args: LexArgs) -> OffsetAndTokenType {
    match args.next_byte() {
        Some(b'=') => (args.offset + 1, TokenType::Op(GreaterEqual)),
        _ => (args.offset, TokenType::Op(GreaterThan)),
    }
}

fn dot(args: LexArgs) -> OffsetAndTokenType {
    match args.next_byte() {
        Some(b'.') => (args.offset + 1, TokenType::Op(DotDot)),
        Some(b')') => (args.offset + 1, TokenType::Op(RBrack)),
        _ => (args.offset, TokenType::Op(Dot)),
    }
}

// endregion: operators

// region: special tokens

#[cold]
fn unknown(args: LexArgs) -> OffsetAndTokenType {
    const MAX_LEN: usize = 100;
    if args.offset <= MAX_LEN {
        warn!("Found unknown token: {}", &args.input[..args.offset]);
    } else {
        warn!(
            "Found unknown token starting with: {}",
            rounded_prefix(args.input, MAX_LEN)
        );
    }
    (args.offset, Unknown)
}

fn eof(input: &str) -> (&str, LexedToken) {
    let whitespace_count = count_leading_whitespace(input);
    let (token_content, remaining) = input.split_at(whitespace_count);
    (
        remaining,
        LexedToken {
            whitespace_count,
            token_content,
            token_type: Eof,
        },
    )
}

// endregion: special tokens

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use spectral::prelude::*;

    type ContentAndTokenType<'a> = (&'a str, TokenType);

    fn run_test(input: &str, expected_token_types: Vec<ContentAndTokenType>) {
        let lexer = DelphiLexer {};
        let tokens = lexer.lex(input);
        let token_types: Vec<_> = tokens
            .iter()
            .filter(|token| token.get_token_type() != Eof)
            .map(|token| (token.get_content(), token.get_token_type()))
            .collect();

        assert_that(&token_types).is_equal_to(&expected_token_types);
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
            vec![
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
            vec![
                ("{block comment}", Comment(CommentKind::IndividualBlock)),
                (
                    "{.$fake compiler directive}",
                    Comment(CommentKind::InlineBlock),
                ),
                ("(*star block comment*)", Comment(CommentKind::InlineBlock)),
                (
                    "(*.$fake compiler star directive*)",
                    Comment(CommentKind::InlineBlock),
                ),
                ("{*)}", Comment(CommentKind::InlineBlock)),
                ("(*{*)", Comment(CommentKind::InlineBlock)),
                (
                    indoc! {"
                        {
                            Multiline block comment
                        }"
                    },
                    Comment(CommentKind::MultilineBlock),
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
            vec![
                ("{individual block}", Comment(CommentKind::IndividualBlock)),
                ("{individual block}", Comment(CommentKind::IndividualBlock)),
                (";", Op(Semicolon)),
                ("{inline block}", Comment(CommentKind::InlineBlock)),
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
            vec![(
                "{individual block\n// other comment\nFoo;",
                Comment(CommentKind::MultilineBlock),
            )],
        );
        run_test(
            indoc! {"
                (*individual block
                // other comment
                Foo;

                "
            },
            vec![(
                "(*individual block\n// other comment\nFoo;",
                Comment(CommentKind::MultilineBlock),
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
            vec![
                (
                    "// Individual line comment 1",
                    Comment(CommentKind::IndividualLine),
                ),
                (
                    "// Individual line comment 2",
                    Comment(CommentKind::IndividualLine),
                ),
                (";", Op(Semicolon)),
                ("// Inline line comment", Comment(CommentKind::InlineLine)),
            ],
        );
    }

    #[test]
    fn lex_compiler_directives() {
        run_test(
            "(*$message*) {$foo *) } (*$bar aa {}*)",
            vec![
                ("(*$message*)", CompilerDirective),
                ("{$foo *) }", CompilerDirective),
                ("(*$bar aa {}*)", CompilerDirective),
            ],
        );
        [
            ("{$if}", ConditionalDirective(ConditionalDirectiveKind::If)),
            ("{$ifdef}", ConditionalDirective(Ifdef)),
            ("{$ifndef}", ConditionalDirective(Ifndef)),
            ("{$ifopt}", ConditionalDirective(Ifopt)),
            ("{$elseif}", ConditionalDirective(Elseif)),
            (
                "{$else}",
                ConditionalDirective(ConditionalDirectiveKind::Else),
            ),
            ("{$ifend}", ConditionalDirective(Ifend)),
            ("{$endif}", ConditionalDirective(Endif)),
            (
                "(*$if*)",
                ConditionalDirective(ConditionalDirectiveKind::If),
            ),
            ("(*$ifdef*)", ConditionalDirective(Ifdef)),
            ("(*$ifndef*)", ConditionalDirective(Ifndef)),
            ("(*$ifopt*)", ConditionalDirective(Ifopt)),
            ("(*$elseif*)", ConditionalDirective(Elseif)),
            (
                "(*$else*)",
                ConditionalDirective(ConditionalDirectiveKind::Else),
            ),
            ("(*$ifend*)", ConditionalDirective(Ifend)),
            ("(*$endif*)", ConditionalDirective(Endif)),
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
            vec![(
                "{$if\n// other comment\nFoo;",
                ConditionalDirective(ConditionalDirectiveKind::If),
            )],
        );
        run_test(
            indoc! {"
                (*$if
                // other comment
                Foo;

                "
            },
            vec![(
                "(*$if\n// other comment\nFoo;",
                ConditionalDirective(ConditionalDirectiveKind::If),
            )],
        );
    }

    #[test]
    fn lex_string_literals() {
        run_test(
            "'' 'string' 'string''part2' 'ab''''cd' 'abc'#13#10 'after escaped stuff' 'a'#1'b' 'a'#1#0'b' 'a'#$017F #%010 #%0_1 #%_0 #%_ #$F7F #$F_7 #$_F #$_ #123 #_",
            vec![
                ("''", TextLiteral),
                ("'string'", TextLiteral),
                ("'string''part2'", TextLiteral),
                ("'ab''''cd'", TextLiteral),
                ("'abc'#13#10", TextLiteral),
                ("'after escaped stuff'", TextLiteral),
                ("'a'#1'b'", TextLiteral),
                ("'a'#1#0'b'", TextLiteral),
                ("'a'#$017F", TextLiteral),
                ("#%010", TextLiteral),
                ("#%0_1", TextLiteral),
                ("#%_0", TextLiteral),
                ("#%_", TextLiteral),
                ("#$F7F", TextLiteral),
                ("#$F_7", TextLiteral),
                ("#$_F", TextLiteral),
                ("#$_", TextLiteral),
                ("#123", TextLiteral),
                ("#_", TextLiteral),
            ],
        );
    }

    #[test]
    fn lex_unterminated_string_literals() {
        run_test(
            "'string\n' + '';\n'a'#\n'a'##",
            vec![
                ("'string", TextLiteral),
                ("' + '';", TextLiteral),
                ("'a'#", TextLiteral),
                ("'a'#", TextLiteral),
                ("#", TextLiteral),
            ],
        );
    }

    #[test]
    fn lex_decimal_number_literals() {
        run_test(
            "0 0.0 10 1_000 1_000.00 1_111_111.11 1.111_1 0e-1 0e+1 0E+1 0.5e+21",
            vec![
                ("0", NumberLiteral(Decimal)),
                ("0.0", NumberLiteral(Decimal)),
                ("10", NumberLiteral(Decimal)),
                ("1_000", NumberLiteral(Decimal)),
                ("1_000.00", NumberLiteral(Decimal)),
                ("1_111_111.11", NumberLiteral(Decimal)),
                ("1.111_1", NumberLiteral(Decimal)),
                ("0e-1", NumberLiteral(Decimal)),
                ("0e+1", NumberLiteral(Decimal)),
                ("0E+1", NumberLiteral(Decimal)),
                ("0.5e+21", NumberLiteral(Decimal)),
            ],
        );
    }

    #[test]
    fn lex_invalid_decimal_number_literals() {
        run_test(
            "0._0 0.e5 0.e-5 0.e+5 0.e-",
            vec![
                ("0.", NumberLiteral(Decimal)),
                ("_0", Identifier),
                ("0.e5", NumberLiteral(Decimal)),
                ("0.e-5", NumberLiteral(Decimal)),
                ("0.e+5", NumberLiteral(Decimal)),
                ("0.e-", NumberLiteral(Decimal)),
            ],
        );
    }

    #[test]
    fn lex_hex_number_literal() {
        run_test(
            "$ $00 $FF $0_0 $_ $_1",
            vec![
                ("$", NumberLiteral(Hex)),
                ("$00", NumberLiteral(Hex)),
                ("$FF", NumberLiteral(Hex)),
                ("$0_0", NumberLiteral(Hex)),
                ("$_", NumberLiteral(Hex)),
                ("$_1", NumberLiteral(Hex)),
            ],
        );
    }

    #[test]
    fn lex_binary_number_literals() {
        run_test(
            "% %0 %1 %1111_0000 %_ %_1",
            vec![
                ("%", NumberLiteral(Binary)),
                ("%0", NumberLiteral(Binary)),
                ("%1", NumberLiteral(Binary)),
                ("%1111_0000", NumberLiteral(Binary)),
                ("%_", NumberLiteral(Binary)),
                ("%_1", NumberLiteral(Binary)),
            ],
        );
    }

    #[test]
    fn lex_ampersand_integer_literals() {
        // Only the &0 case is valid according to our compiler, not that it makes any sense, but we figure that
        // the other cases should be lexed in the same way (even if they are invalid).
        run_test(
            "&$FF &%0 &0",
            vec![
                ("&$FF", NumberLiteral(Hex)),
                ("&%0", NumberLiteral(Binary)),
                ("&0", NumberLiteral(Decimal)),
            ],
        );
    }

    #[test]
    fn lex_identifiers() {
        run_test(
            "Foo _Foo _1Foo &begin &&op_Addition &&&Foo &&&&Foo &&",
            vec![
                ("Foo", Identifier),
                ("_Foo", Identifier),
                ("_1Foo", Identifier),
                ("&begin", Identifier),
                ("&&op_Addition", Identifier),
                // These aren't valid identifiers, but they're most valid as such.
                ("&&&Foo", Identifier),
                ("&&&&Foo", Identifier),
                // You can't actually use this as an identifier, but in some contexts it's valid yet ignored.
                ("&&", Identifier),
            ],
        );
    }

    #[test]
    fn lex_operators() {
        run_test(
            "+-*/:=,;=:<><<=>=>[](..)()^@...",
            vec![
                ("+", Op(Plus)),
                ("-", Op(Minus)),
                ("*", Op(Star)),
                ("/", Op(Slash)),
                (":=", Op(Assign)),
                (",", Op(Comma)),
                (";", Op(Semicolon)),
                ("=", Op(Equal)),
                (":", Op(Colon)),
                ("<>", Op(NotEqual)),
                ("<", Op(LessThan)),
                ("<=", Op(LessEqual)),
                (">=", Op(GreaterEqual)),
                (">", Op(GreaterThan)),
                ("[", Op(LBrack)),
                ("]", Op(RBrack)),
                ("(.", Op(LBrack)),
                (".)", Op(RBrack)),
                ("(", Op(LParen)),
                (")", Op(RParen)),
                ("^", Op(Pointer)),
                ("@", Op(AddressOf)),
                ("..", Op(DotDot)),
                (".", Op(Dot)),
            ],
        );
    }

    #[test]
    fn lex_keywords() {
        [
            ("absolute", IdentifierOrKeyword(Absolute)),
            ("abstract", IdentifierOrKeyword(Abstract)),
            ("add", IdentifierOrKeyword(Add)),
            ("align", IdentifierOrKeyword(Align)),
            ("and", Keyword(And)),
            ("array", Keyword(Array)),
            ("as", Keyword(As)),
            ("assembler", IdentifierOrKeyword(Assembler)),
            ("at", IdentifierOrKeyword(At)),
            ("automated", IdentifierOrKeyword(Automated)),
            ("begin", Keyword(Begin)),
            ("case", Keyword(Case)),
            ("cdecl", IdentifierOrKeyword(Cdecl)),
            ("class", Keyword(Class)),
            ("const", Keyword(Const)),
            ("constructor", Keyword(Constructor)),
            ("contains", IdentifierOrKeyword(Contains)),
            ("default", IdentifierOrKeyword(Default)),
            ("delayed", IdentifierOrKeyword(Delayed)),
            ("deprecated", IdentifierOrKeyword(Deprecated)),
            ("destructor", Keyword(Destructor)),
            ("dispid", IdentifierOrKeyword(DispId)),
            ("dispinterface", Keyword(DispInterface)),
            ("div", Keyword(Div)),
            ("do", Keyword(Do)),
            ("downto", Keyword(Downto)),
            ("dynamic", IdentifierOrKeyword(Dynamic)),
            ("else", Keyword(KeywordKind::Else)),
            ("end", Keyword(End)),
            ("except", Keyword(Except)),
            ("experimental", IdentifierOrKeyword(Experimental)),
            ("export", IdentifierOrKeyword(Export)),
            ("exports", Keyword(Exports)),
            ("external", IdentifierOrKeyword(External)),
            ("far", IdentifierOrKeyword(Far)),
            ("file", Keyword(File)),
            ("final", IdentifierOrKeyword(Final)),
            ("finalization", Keyword(Finalization)),
            ("finally", Keyword(Finally)),
            ("for", Keyword(For)),
            ("forward", IdentifierOrKeyword(Forward)),
            ("function", Keyword(Function)),
            ("goto", Keyword(Goto)),
            ("helper", IdentifierOrKeyword(Helper)),
            ("if", Keyword(KeywordKind::If)),
            ("implementation", Keyword(Implementation)),
            ("implements", IdentifierOrKeyword(Implements)),
            ("in", Keyword(In)),
            ("index", IdentifierOrKeyword(Index)),
            ("inherited", Keyword(Inherited)),
            ("initialization", Keyword(Initialization)),
            ("inline", Keyword(Inline)),
            ("interface", Keyword(Interface)),
            ("is", Keyword(Is)),
            ("label", Keyword(Label)),
            ("library", Keyword(Library)),
            ("local", IdentifierOrKeyword(Local)),
            ("message", IdentifierOrKeyword(Message)),
            ("mod", Keyword(Mod)),
            ("name", IdentifierOrKeyword(Name)),
            ("near", IdentifierOrKeyword(Near)),
            ("nil", Keyword(Nil)),
            ("nodefault", IdentifierOrKeyword(NoDefault)),
            ("not", Keyword(Not)),
            ("object", Keyword(Object)),
            ("of", Keyword(Of)),
            ("on", IdentifierOrKeyword(On)),
            ("operator", IdentifierOrKeyword(Operator)),
            ("or", Keyword(Or)),
            ("out", IdentifierOrKeyword(Out)),
            ("overload", IdentifierOrKeyword(Overload)),
            ("override", IdentifierOrKeyword(Override)),
            ("package", IdentifierOrKeyword(Package)),
            ("packed", Keyword(Packed)),
            ("pascal", IdentifierOrKeyword(Pascal)),
            ("platform", IdentifierOrKeyword(Platform)),
            ("private", IdentifierOrKeyword(Private)),
            ("procedure", Keyword(Procedure)),
            ("program", Keyword(Program)),
            ("property", Keyword(Property)),
            ("protected", IdentifierOrKeyword(Protected)),
            ("public", IdentifierOrKeyword(Public)),
            ("published", IdentifierOrKeyword(Published)),
            ("raise", Keyword(Raise)),
            ("read", IdentifierOrKeyword(Read)),
            ("readonly", IdentifierOrKeyword(ReadOnly)),
            ("record", Keyword(Record)),
            ("reference", IdentifierOrKeyword(Reference)),
            ("register", IdentifierOrKeyword(Register)),
            ("reintroduce", IdentifierOrKeyword(Reintroduce)),
            ("remove", IdentifierOrKeyword(Remove)),
            ("repeat", Keyword(Repeat)),
            ("requires", IdentifierOrKeyword(Requires)),
            ("resident", IdentifierOrKeyword(Resident)),
            ("resourcestring", Keyword(ResourceString)),
            ("safecall", IdentifierOrKeyword(SafeCall)),
            ("sealed", IdentifierOrKeyword(Sealed)),
            ("set", Keyword(Set)),
            ("shl", Keyword(Shl)),
            ("shr", Keyword(Shr)),
            ("static", IdentifierOrKeyword(Static)),
            ("stdcall", IdentifierOrKeyword(StdCall)),
            ("stored", IdentifierOrKeyword(Stored)),
            ("strict", IdentifierOrKeyword(Strict)),
            ("then", Keyword(Then)),
            ("threadvar", Keyword(ThreadVar)),
            ("to", Keyword(To)),
            ("try", Keyword(Try)),
            ("type", Keyword(Type)),
            ("unit", Keyword(Unit)),
            ("unsafe", IdentifierOrKeyword(Unsafe)),
            ("until", Keyword(Until)),
            ("uses", Keyword(Uses)),
            ("var", Keyword(Var)),
            ("varargs", IdentifierOrKeyword(VarArgs)),
            ("variant", IdentifierOrKeyword(Variant)),
            ("virtual", IdentifierOrKeyword(Virtual)),
            ("while", Keyword(While)),
            ("with", Keyword(With)),
            ("write", IdentifierOrKeyword(Write)),
            ("writeonly", IdentifierOrKeyword(WriteOnly)),
            ("xor", Keyword(Xor)),
        ]
        .into_iter()
        .for_each(run_casing_test);
    }

    #[test]
    fn lex_function_declaration() {
        run_test(
            "function Foo(Arg1:String;Arg2:Bar);stdcall;",
            vec![
                ("function", Keyword(Function)),
                ("Foo", Identifier),
                ("(", Op(LParen)),
                ("Arg1", Identifier),
                (":", Op(Colon)),
                ("String", Identifier),
                (";", Op(Semicolon)),
                ("Arg2", Identifier),
                (":", Op(Colon)),
                ("Bar", Identifier),
                (")", Op(RParen)),
                (";", Op(Semicolon)),
                ("stdcall", IdentifierOrKeyword(StdCall)),
                (";", Op(Semicolon)),
            ],
        );
    }

    #[test]
    fn lex_invalid_code() {
        run_test(
            "? ? ?",
            vec![("?", Unknown), ("?", Unknown), ("?", Unknown)],
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
            vec![
                ("asm", Keyword(Asm)),
                ("@@end", Identifier),
                (":", Op(Colon)),
                ("XOR", Identifier),
                ("RBX", Identifier),
                (",", Op(Comma)),
                ("RBX", Identifier),
                ("end", Keyword(End)),
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
            vec![
                ("asm", Keyword(Asm)),
                ("XOR", Identifier),
                ("RBX", Identifier),
                (",", Op(Comma)),
                ("RBX", Identifier),
                ("{$ifdef End}", ConditionalDirective(Ifdef)),
                ("end", Keyword(End)),
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
            vec![
                ("asm", Keyword(Asm)),
                ("XOR", Identifier),
                ("RBX", Identifier),
                (",", Op(Comma)),
                ("RBX", Identifier),
                ("// End", Comment(CommentKind::InlineLine)),
                ("end", Keyword(End)),
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
            vec![
                ("asm", Keyword(Asm)),
                ("XOR", Identifier),
                ("RBX", Identifier),
                (",", Op(Comma)),
                ("IfEnd", Identifier),
                ("end", Keyword(End)),
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
            vec![
                ("asm", Keyword(Asm)),
                ("@@A", Identifier),
                (":", Op(Colon)),
                ("@A", Identifier),
                (":", Op(Colon)),
                ("@A@a", Identifier),
                (":", Op(Colon)),
                ("@_", Identifier),
                (":", Op(Colon)),
                ("@0", Identifier),
                (":", Op(Colon)),
                ("end", Keyword(End)),
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
            vec![
                ("asm", Keyword(Asm)),
                ("CMP", Identifier),
                ("AL", Identifier),
                (",", Op(Comma)),
                ("\"'\"", TextLiteral),
                ("XOR", Identifier),
                ("RBX", Identifier),
                (",", Op(Comma)),
                ("RBX", Identifier),
                ("end", Keyword(End)),
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
            vec![
                ("asm", Keyword(Asm)),
                ("CMP", Identifier),
                ("AL", Identifier),
                (",", Op(Comma)),
                (r#""\"""#, TextLiteral),
                ("end", Keyword(End)),
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
            vec![
                ("asm", Keyword(Asm)),
                ("CMP", Identifier),
                ("AL", Identifier),
                (",", Op(Comma)),
                ("\"a", TextLiteral),
                ("end", Keyword(End)),
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
            vec![
                ("asm", Keyword(Asm)),
                ("MOV", Identifier),
                ("RAX", Identifier),
                (",", Op(Comma)),
                ("0", NumberLiteral(Decimal)),
                ("// comment", Comment(CommentKind::InlineLine)),
                ("XOR", Identifier),
                ("RBX", Identifier),
                (",", Op(Comma)),
                ("RBX", Identifier),
                ("end", Keyword(End)),
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
            vec![
                ("asm", Keyword(Asm)),
                ("MOV", Identifier),
                ("RAX", Identifier),
                (",", Op(Comma)),
                ("0", NumberLiteral(Decimal)),
                ("XOR", Identifier),
                ("RBX", Identifier),
                (",", Op(Comma)),
                ("RBX", Identifier),
                ("end", Keyword(End)),
            ],
        );
    }

    #[test]
    fn identifier_starting_with_asm() {
        run_test(
            "begin var asmA := 0; end;",
            vec![
                ("begin", Keyword(Begin)),
                ("var", Keyword(Var)),
                ("asmA", Identifier),
                (":=", Op(Assign)),
                ("0", NumberLiteral(Decimal)),
                (";", Op(Semicolon)),
                ("end", Keyword(End)),
                (";", Op(Semicolon)),
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
            vec![
                ("asm", Keyword(Asm)),
                ("0", NumberLiteral(Decimal)),
                ("0O", NumberLiteral(Octal)),
                ("0o", NumberLiteral(Octal)),
                ("0B", NumberLiteral(Binary)),
                ("0b", NumberLiteral(Binary)),
                ("0H", NumberLiteral(Hex)),
                ("0h", NumberLiteral(Hex)),
                ("$0", NumberLiteral(Hex)),
                ("0AH", NumberLiteral(Hex)),
                ("0FH", NumberLiteral(Hex)),
                ("end", Keyword(End)),
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
            vec![
                ("asm", Keyword(Asm)),
                ("8O", NumberLiteral(Octal)),
                ("2B", NumberLiteral(Binary)),
                ("$0", NumberLiteral(Hex)),
                ("H", Identifier),
                ("0A", NumberLiteral(Decimal)),
                ("0", NumberLiteral(Decimal)),
                ("GH", Identifier),
                ("$A_A", NumberLiteral(Hex)),
                ("00_11B", NumberLiteral(Binary)),
                ("9_5", NumberLiteral(Decimal)),
                ("end", Keyword(End)),
            ],
        );
    }

    #[test]
    fn unicode() {
        run_test(
            "böb öb bö ö",
            vec![
                ("böb", Identifier),
                ("öb", Identifier),
                ("bö", Identifier),
                ("ö", Identifier),
            ],
        );

        // Japanese
        run_test("クールなテスト", vec![("クールなテスト", Identifier)]);
        // Chinese (Traditional)
        run_test("酷測試", vec![("酷測試", Identifier)]);
        // Korean
        run_test("멋진테스트", vec![("멋진테스트", Identifier)]);

        /*
            Codepoints above U+FFFF are all surrogate pairs, and don't seem to be allowed in identifiers.
            However, the documentation hints that with the right compiler CodePage setting you could get
            this to work, so we are treating all of these codepoints as valid identifiers.  There's
            no harm in doing so.
        */
        // Emojis
        run_test("🚀🦀🚀🦀", vec![("🚀🦀🚀🦀", Identifier)]);
        // Enclosed Alphanumeric
        run_test("🄲🄾🄾🄻🅃🄴🅂🅃", vec![("🄲🄾🄾🄻🅃🄴🅂🅃", Identifier)]);
    }

    #[test]
    fn unicode_fullwidth_chars() {
        // Fullwidth characters - valid identifier chars
        run_test("ｃｏｏｌｔｅｓｔ", vec![("ｃｏｏｌｔｅｓｔ", Identifier)]);

        // Fullwidth spaces - invalid identifier char (but valid as whitespace)
        run_test("a　b", vec![("a", Identifier), ("b", Identifier)]);

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
            vec![
                ("０", Identifier),
                ("１", Identifier),
                ("２", Identifier),
                ("３", Identifier),
                ("４", Identifier),
                ("５", Identifier),
                ("６", Identifier),
                ("７", Identifier),
                ("８", Identifier),
                ("９", Identifier),
            ],
        );
    }

    #[test]
    fn unicode_whitespace() {
        // Exotic whitespace characters are not generally considered whitespace, but it appears that an exception was
        // made for 'ideographic space'. We presume this is because it is used in Japanese text.
        run_test(
            "NBSP\u{A0} EN_QUAD\u{2000} THIN_SPACE\u{2009} ZERO_WIDTH_NBSP\u{FEFF} IDEOGRAPHIC_SPACE\u{3000}",
            vec![
                ("NBSP\u{A0}", Identifier),
                ("EN_QUAD\u{2000}", Identifier),
                ("THIN_SPACE\u{2009}", Identifier),
                ("ZERO_WIDTH_NBSP\u{FEFF}", Identifier),
                // note, does not contain the U+3000 character
                ("IDEOGRAPHIC_SPACE", Identifier),
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
            vec![
                ("NUL", Identifier),
                ("SOH", Identifier),
                ("STX", Identifier),
                ("ETX", Identifier),
                ("EOT", Identifier),
                ("ENQ", Identifier),
                ("ACK", Identifier),
                ("BEL", Identifier),
                ("BS", Identifier),
                ("HT", Identifier),
                ("LF", Identifier),
                ("VT", Identifier),
                ("FF", Identifier),
                ("CR", Identifier),
                ("SO", Identifier),
                ("SI", Identifier),
                ("DLE", Identifier),
                ("DC1", Identifier),
                ("DC2", Identifier),
                ("DC3", Identifier),
                ("DC4", Identifier),
                ("NAK", Identifier),
                ("SYN", Identifier),
                ("ETB", Identifier),
                ("CAN", Identifier),
                ("EM", Identifier),
                ("SUB", Identifier),
                ("ESC", Identifier),
                ("FS", Identifier),
                ("GS", Identifier),
                ("RS", Identifier),
                ("US", Identifier),
                ("Space", Identifier),
            ],
        )
    }
}
