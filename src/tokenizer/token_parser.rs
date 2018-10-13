use tokenizer::Token::*;
use tokenizer::Token;
use nom::*;
use util::*;
use tokenizer::Input;

named!(lower<char>, one_of!("abcdefghijklmnopqrstuvwxyz"));

named!(upper<char>, one_of!("ABCDEFGHIJKLMNOPQRSTUVWXYZ"));

named!(number<char>, one_of!("0123456789"));

named!(newline<char>, one_of!("\r\n"));

named!(binop_char<char>, one_of!(":~!@#$%^&*-+=<>/?\\._"));

named!(id_char<char>, alt!(lower | upper | number | one_of!("_'")));

named!(read_id<Token>, do_parse!(
    a: lower >>
    b: many0!(id_char) >>
    (from_id(create_vec(a, b).into_iter().collect::<String>()))
));

named!(read_upper_id<Token>, do_parse!(
    a: upper >>
    b: many0!(id_char) >>
    (UpperId(create_vec(a, b).into_iter().collect::<String>()))
));

named!(read_literal<Token>, alt!(
    read_float  |
    read_int    |
    read_string |
    read_char
));

named!(read_int<Token>, do_parse!(
    minus: opt!(char!('-')) >>
    numbers: many1!(number) >>
    (LitInt(parse_int(minus.is_some(), numbers)))
));

named!(read_float<Token>, do_parse!(
    minus: opt!(char!('-')) >>
    integer: many0!(number) >>
    char!('.') >>
    decimal: many1!(number) >>
    (LitFloat(parse_float2(minus.is_some(), integer, decimal)))
));

named!(read_string<Token>, alt!(read_line_string | read_multiline_string));

named!(read_line_string<Token>, do_parse!(
    char!('\"') >>
    c: many0!(string_char) >>
    char!('\"') >>
    (LitString(c.into_iter().collect::<String>()))
));

named!(read_multiline_string<Token>, do_parse!(
    tag!("\"\"\"") >>
    c: many0!(here_doc_char) >>
    tag!("\"\"\"") >>
    (LitString(c.into_iter().collect::<String>()))
));

named!(read_char<Token>, delimited!(char!('\''), char_char, char!('\'')));

named!(char_char<Token>, map!(none_of!("\n\'"), |c| LitChar(c)));

named!(string_char<char>, alt!(
    none_of!("\n\"") | preceded!(char!('\\'), char!('\"'))
));

named!(here_doc_char<char>, alt!(
    none_of!("\"") | terminated!(char!('\"'), not!(tag!("\"\"")))
));

named!(left_paren<Token>, map!(char!('('), |_c| LeftParen));
named!(right_paren<Token>, map!(char!(')'), |_c| RightParen));

named!(left_braket<Token>, map!(char!('['), |_c| LeftBracket));
named!(right_braket<Token>, map!(char!(']'), |_c| RightBracket));

named!(left_brace<Token>, map!(char!('{'), |_c| LeftBrace));
named!(right_brace<Token>, map!(char!('}'), |_c| RightBrace));

named!(colon<Token>, map!(char!(':'), |_c| Colon));

named!(ignore_spaces<()>, do_parse!(
    many0!(alt!(char!(' ') | char!('\t'))) >> (())
));

// the Token::NewLine gets ignored
named!(comment_first_char<()>, not!(alt!(newline | binop_char)));

named!(line_comment<()>, do_parse!(
    tag!("--") >>
    comment_first_char >>
    many0!(none_of!("\n\r")) >>
    newline >>
    ()
));

named!(basic_binop_string<String>, map!(
    many1!(binop_char),
    |v| v.into_iter().collect::<String>()
));

named!(underscore<Token>, map!(char!('_'), |_c| Underscore));

named!(dot<Token>, map!(char!('.'), |_c| Dot));

named!(double_dot<Token>, map!(tag!(".."), |_c| DoubleDot));

named!(comma<Token>, map!(char!(','), |_c| Comma));

named!(equals<Token>, map!(char!('='), |_c| Equals));

named!(pipe<Token>, map!(char!('|'), |_c| Pipe));

named!(left_arrow<Token>, map!(tag!("<-"), |_c| LeftArrow));

named!(right_arrow<Token>, map!(tag!("->"), |_c| RightArrow));

named!(eof_marker<Token>, alt!(map!(eof!(), |_c| Eof) | map!(char!('\0'), |_c| Eof)));

named!(pub read_token_forced<Token>, alt!(
    read_binop
    | comma
    | pipe
    | read_id
    | read_upper_id
    | read_literal
    | left_paren
    | right_paren
    | left_braket
    | right_braket
    | left_brace
    | right_brace
    | eof_marker
));


fn read_binop(input: &[u8]) -> IResult<&[u8], Token> {
    let (o, binop): (&[u8], String) = basic_binop_string(input)?;
    let op = from_binop(binop);
    Ok((o, op))
}

fn from_id(id: String) -> Token {
    match id.as_bytes() {
        b"let" => Let,
        b"if" => If,
        b"else" => Else,
        b"then" => Then,
        b"case" => Case,
        b"of" => Of,
        b"in" => In,
        b"module" => ModuleTk,
        b"where" => Where,
        b"exposing" => Exposing,
        b"import" => ImportTk,
        b"as" => As,
        b"type" => TypeTk,
        b"port" => Port,
        b"alias" => Alias,
        _ => Id(id)
    }
}

fn from_binop(id: String) -> Token {
    match id.as_bytes() {
        b"_" => Underscore,
        b":" => Colon,
        b"." => Dot,
        b".." => DoubleDot,
        b"=" => Equals,
        b"<-" => LeftArrow,
        b"->" => RightArrow,
        b"\\" => BackSlash,
        _ => BinaryOperator(id)
    }
}

#[cfg(test)]
mod tests {
    use nom::*;
    use super::*;

    #[test]
    fn check_line_string() {
        assert_ok!(
            read_line_string("\"Hello World\"".as_bytes()),
            LitString("Hello World".s())
        );
    }

    #[test]
    fn check_multiline_string() {
        assert_ok!(
            read_multiline_string(b"\"\"\"Hello\nWorld\"\"\""),
            LitString("Hello\nWorld".s())
        );
    }

    #[test]
    fn check_char() {
        assert_ok!(
            read_char(b"'H'"),
            LitChar('H')
        );
    }

    #[test]
    fn check_int() {
        assert_ok!(read_int(b"0|"), LitInt(0));
        assert_ok!(read_int(b"-1|"), LitInt(-1));
        assert_ok!(read_int(b"1|"), LitInt(1));
        assert_ok!(read_int(b"99999|"), LitInt(99999));
        assert_ok!(read_int(b"-1234|"), LitInt(-1234));
    }

    #[test]
    fn check_float() {
        assert_ok!(read_float(b"0.0|"), LitFloat(0.0));
        assert_ok!(read_float(b"-1.0|"), LitFloat(-1.0));
        assert_ok!(read_float(b".0|"), LitFloat(0.0));
        assert_ok!(read_float(b"-.0|"), LitFloat(0.0));
        assert_ok!(read_float(b"1.2|"), LitFloat(1.2));
        assert_ok!(read_float(b"99999.0|"), LitFloat(99999.0));
        assert_ok!(read_float(b"-1234.0|"), LitFloat(-1234.0));
    }

    #[test]
    fn check_comment() {
        assert_ok!(line_comment(b"-- \n"), ());
        assert_ok!(line_comment(b"-- aghf\n"), ());
        assert_ok!(line_comment(b"--sss\n"), ());
        assert_ok!(line_comment(b"--srtga\n"), ());
        assert_ok!(line_comment(b"-- er ert -- eyr\n"), ());
    }

    #[test]
    fn check_binop() {
        assert_ok!(read_token_forced(b"= "), Equals);
        assert_ok!(read_token_forced(b"== "), BinaryOperator("==".s()));
        assert_ok!(read_token_forced(b"=== "), BinaryOperator("===".s()));
        assert_ok!(read_token_forced(b"- "), BinaryOperator("-".s()));
        assert_ok!(read_token_forced(b"-- "), BinaryOperator("--".s()));
        assert_ok!(read_token_forced(b"--- "), BinaryOperator("---".s()));
        assert_ok!(read_token_forced(b". "), Dot);
        assert_ok!(read_token_forced(b".. "), DoubleDot);
        assert_ok!(read_token_forced(b"... "), BinaryOperator("...".s()));
        assert_ok!(read_token_forced(b"-> "), RightArrow);
        assert_ok!(read_token_forced(b"<- "), LeftArrow);
        assert_ok!(read_token_forced(b"<-- "), BinaryOperator("<--".s()));
    }
}