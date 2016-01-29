use tokenizers::{MathToken, MathTokenizer};
use tokenizers::DelimTokenizer;

#[test]
fn test1() {
    let mut lx = MathTokenizer::from_str("3+4*2/-(1-5)^2^3");
    let expect = [
        MathToken::Number(3.0),
        MathToken::BOp(format!("+")),
        MathToken::Number(4.0),
        MathToken::BOp(format!("*")),
        MathToken::Number(2.0),
        MathToken::BOp(format!("/")),
        MathToken::UOp(format!("-")),
        MathToken::OParen,
        MathToken::Number(1.0),
        MathToken::BOp(format!("-")),
        MathToken::Number(5.0),
        MathToken::CParen,
        MathToken::BOp(format!("^")),
        MathToken::Number(2.0),
        MathToken::BOp(format!("^")),
        MathToken::Number(3.0),
    ];
    for exp_token in expect.iter() {
        let token = lx.next().unwrap();
        assert_eq!(*exp_token, token);
    }
    assert_eq!(lx.next(), None);
}

#[test]
fn test2() {
    let mut lx = MathTokenizer::from_str("3.4e-2 * sin(x)/(7! % -4) * max(2, x)");
    let expect = [
        MathToken::Number(3.4e-2),
        MathToken::BOp(format!("*")),
        MathToken::Function(format!("sin")),
        MathToken::OParen,
        MathToken::Variable(format!("x")),
        MathToken::CParen,
        MathToken::BOp(format!("/")),
        MathToken::OParen,
        MathToken::Number(7.0),
        MathToken::UOp(format!("!")),
        MathToken::BOp(format!("%")),
        MathToken::UOp(format!("-")),
        MathToken::Number(4.0),
        MathToken::CParen,
        MathToken::BOp(format!("*")),
        MathToken::Function(format!("max")),
        MathToken::OParen,
        MathToken::Number(2.0),
        MathToken::Comma,
        MathToken::Variable(format!("x")),
        MathToken::CParen,
    ];
    for exp_token in expect.iter() {
        let token = lx.next().unwrap();
        assert_eq!(*exp_token, token);
    }
    assert_eq!(lx.next(), None);
}

#[test]
fn test3() {
    let mut lx = MathTokenizer::from_str("sqrt(-(1-x^2) / (1 + x^2))");
    let expect = [
        MathToken::Function(format!("sqrt")),
        MathToken::OParen,
        MathToken::UOp(format!("-")),
        MathToken::OParen,
        MathToken::Number(1.0),
        MathToken::BOp(format!("-")),
        MathToken::Variable(format!("x")),
        MathToken::BOp(format!("^")),
        MathToken::Number(2.0),
        MathToken::CParen,
        MathToken::BOp(format!("/")),
        MathToken::OParen,
        MathToken::Number(1.0),
        MathToken::BOp(format!("+")),
        MathToken::Variable(format!("x")),
        MathToken::BOp(format!("^")),
        MathToken::Number(2.0),
        MathToken::CParen,
        MathToken::CParen,
    ];
    for exp_token in expect.iter() {
        let token = lx.next().unwrap();
        assert_eq!(*exp_token, token);
    }
    assert_eq!(lx.next(), None);
}

#[test]
fn test4() {
    let mut lx = MathTokenizer::from_str("x---y");
    let expect = [
        MathToken::Variable(format!("x")),
        MathToken::BOp(format!("-")),
        MathToken::UOp(format!("-")),
        MathToken::UOp(format!("-")),
        MathToken::Variable(format!("y")),
    ];
    for exp_token in expect.iter() {
        let token = lx.next().unwrap();
        assert_eq!(*exp_token, token);
    }
    assert_eq!(lx.next(), None);
}

#[test]
fn test5() {
    let mut lx = MathTokenizer::from_str("max(0, 1, 3)");
    let expect = [
        MathToken::Function(format!("max")),
        MathToken::OParen,
        MathToken::Number(0.0),
        MathToken::Comma,
        MathToken::Number(1.0),
        MathToken::Comma,
        MathToken::Number(3.0),
        MathToken::CParen,
    ];
    for exp_token in expect.iter() {
        let token = lx.next().unwrap();
        assert_eq!(*exp_token, token);
    }
    assert_eq!(lx.next(), None);
}

#[test]
fn test_delim_tokenizer() {
    let mut lx = DelimTokenizer::from_str("this  is a   test ", " ", true);
    let expect = vec!["this", "is", "a", "test"]
        .iter().map(|s| s.to_string()).collect::<Vec<_>>();
    for exp_token in expect.iter() {
        assert_eq!(*exp_token, lx.next().unwrap());
    }
    assert_eq!(lx.next(), None);
}