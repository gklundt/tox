#![deny(warnings)]

extern crate lexers;
use grammar::{GrammarBuilder, Grammar};
use parser::EarleyParser;
use self::lexers::{Scanner, DelimTokenizer};
use std::fmt;
use std::iter::FromIterator;
use std::str::FromStr;
use trees::EarleyEvaler;
use util::{Sexpr, Tree};

fn grammar_math() -> Grammar {
    // Sum -> Sum + Mul | Mul
    // Mul -> Mul * Pow | Pow
    // Pow -> Num ^ Pow | Num
    // Num -> Number | ( Sum )
    GrammarBuilder::new()
      .symbol("Sum")
      .symbol("Mul")
      .symbol("Pow")
      .symbol("Num")
      .symbol(("Number", |n: &str| n.chars().all(|c| "1234567890".contains(c))))
      .symbol(("[+-]", |n: &str| n.len() == 1 && "+-".contains(n)))
      .symbol(("[*/]", |n: &str| n.len() == 1 && "*/".contains(n)))
      .symbol(("[^]", |n: &str| { n == "^" }))
      .symbol(("(", |n: &str| { n == "(" }))
      .symbol((")", |n: &str| { n == ")" }))
      .rule("Sum", &["Sum", "[+-]", "Mul"])
      .rule("Sum", &["Mul"])
      .rule("Mul", &["Mul", "[*/]", "Pow"])
      .rule("Mul", &["Pow"])
      .rule("Pow", &["Num", "[^]", "Pow"])
      .rule("Pow", &["Num"])
      .rule("Num", &["(", "Sum", ")"])
      .rule("Num", &["Number"])
      .into_grammar("Sum")
      .expect("Bad Grammar")
}

fn check_trees<T: fmt::Debug>(trees: &Vec<T>, expected: Vec<&str>) {
    use std::collections::HashSet;
    assert_eq!(trees.len(), expected.len());
    let mut expect = HashSet::<&str>::from_iter(expected);
    for t in trees {
        let teststr = format!("{:?}", t);
        eprintln!("{}", teststr);
        assert!(expect.remove(teststr.as_str()));
    }
    assert_eq!(0, expect.len());
}

///////////////////////////////////////////////////////////////////////////////

#[test]
fn grammar_ambiguous() {
    // S -> SS | b
    let grammar = GrammarBuilder::new()
      .symbol("S")
      .symbol(("b", |n: &str| n == "b"))
      .rule("S", &["S", "S"])
      .rule("S", &["b"])
      .into_grammar("S")
      .expect("Bad grammar");
    // Earley's corner case that generates spurious trees for bbb
    let mut input = DelimTokenizer::from_str("b b b", " ", true);
    let p = EarleyParser::new(grammar.clone());
    let ps = p.parse(&mut input).unwrap();
    // check we only get 2 trees
    let trees = Tree::builder(grammar).eval_all(&ps).unwrap();
    check_trees(&trees, vec![
        r#"Node("S -> S S", [Node("S -> S S", [Node("S -> b", [Leaf("b", "b")]), Node("S -> b", [Leaf("b", "b")])]), Node("S -> b", [Leaf("b", "b")])])"#,
        r#"Node("S -> S S", [Node("S -> b", [Leaf("b", "b")]), Node("S -> S S", [Node("S -> b", [Leaf("b", "b")]), Node("S -> b", [Leaf("b", "b")])])])"#,
    ]);
    eprintln!("=== tree ===");
    for t in trees { eprintln!("{:?}", t); }
}

#[test]
fn grammar_ambiguous_epsilon() {
    // S -> SSX | b
    // X -> <e>
    let g = GrammarBuilder::new()
      .symbol("S")
      .symbol("X")
      .symbol(("b", |n: &str| n == "b"))
      .rule("S", &["S", "S", "X"])
      .rule::<_, &str>("X", &[])
      .rule("S", &["b"])
      .into_grammar("S")
      .expect("Bad grammar");
    // Earley's corner case that generates spurious trees for bbb
    let mut input = DelimTokenizer::from_str("b b b", " ", true);
    let ps = EarleyParser::new(g.clone()).parse(&mut input).unwrap();
    let trees = Tree::builder(g).eval_all(&ps).unwrap();
    check_trees(&trees, vec![
        r#"Node("S -> S S X", [Node("S -> S S X", [Node("S -> b", [Leaf("b", "b")]), Node("S -> b", [Leaf("b", "b")]), Node("X -> ", [])]), Node("S -> b", [Leaf("b", "b")]), Node("X -> ", [])])"#,
        r#"Node("S -> S S X", [Node("S -> b", [Leaf("b", "b")]), Node("S -> S S X", [Node("S -> b", [Leaf("b", "b")]), Node("S -> b", [Leaf("b", "b")]), Node("X -> ", [])]), Node("X -> ", [])])"#,
    ]);
}

#[test]
fn math_grammar_test() {
    let grammar = grammar_math();
    let mut input = DelimTokenizer::from_str("1+(2*3-4)", "+*-/()", false);
    let p = EarleyParser::new(grammar.clone());
    let ps = p.parse(&mut input).unwrap();
    let evaler = Tree::builder(grammar);
    let trees = evaler.eval_all(&ps).unwrap();
    check_trees(&trees, vec![
        r#"Node("Sum -> Sum [+-] Mul", [Node("Sum -> Mul", [Node("Mul -> Pow", [Node("Pow -> Num", [Node("Num -> Number", [Leaf("Number", "1")])])])]), Leaf("[+-]", "+"), Node("Mul -> Pow", [Node("Pow -> Num", [Node("Num -> ( Sum )", [Leaf("(", "("), Node("Sum -> Sum [+-] Mul", [Node("Sum -> Mul", [Node("Mul -> Mul [*/] Pow", [Node("Mul -> Pow", [Node("Pow -> Num", [Node("Num -> Number", [Leaf("Number", "2")])])]), Leaf("[*/]", "*"), Node("Pow -> Num", [Node("Num -> Number", [Leaf("Number", "3")])])])]), Leaf("[+-]", "-"), Node("Mul -> Pow", [Node("Pow -> Num", [Node("Num -> Number", [Leaf("Number", "4")])])])]), Leaf(")", ")")])])])])"#,
    ]);
    assert_eq!(evaler.eval(&ps).unwrap(), trees[0]);
}

#[test]
fn left_recurse() {
    // S -> S + N | N
    // N -> [0-9]
    let grammar = GrammarBuilder::new()
      .symbol("S")
      .symbol("N")
      .symbol(("[+]", |n: &str| n == "+"))
      .symbol(("[0-9]", |n: &str| "1234567890".contains(n)))
      .rule("S", &["S", "[+]", "N"])
      .rule("S", &["N"])
      .rule("N", &["[0-9]"])
      .into_grammar("S")
      .expect("Bad grammar");
    let mut input = DelimTokenizer::from_str("1+2", "+", false);
    let p = EarleyParser::new(grammar.clone());
    let ps = p.parse(&mut input).unwrap();
    let tree = Tree::builder(grammar).eval(&ps).unwrap();
    check_trees(&vec![tree], vec![
        r#"Node("S -> S [+] N", [Node("S -> N", [Node("N -> [0-9]", [Leaf("[0-9]", "1")])]), Leaf("[+]", "+"), Node("N -> [0-9]", [Leaf("[0-9]", "2")])])"#,
    ]);
}

#[test]
fn right_recurse() {
    // P -> N ^ P | N
    // N -> [0-9]
    let grammar = GrammarBuilder::new()
      .symbol("P")
      .symbol("N")
      .symbol(("[^]", |n: &str| n == "^"))
      .symbol(("[0-9]", |n: &str| "1234567890".contains(n)))
      .rule("P", &["N", "[^]", "P"])
      .rule("P", &["N"])
      .rule("N", &["[0-9]"])
      .into_grammar("P")
      .expect("Bad grammar");
    let p = EarleyParser::new(grammar.clone());
    let mut input = DelimTokenizer::from_str("1^2", "^", false);
    let ps = p.parse(&mut input).unwrap();
    let tree = Tree::builder(grammar).eval(&ps).unwrap();
    check_trees(&vec![tree], vec![
        r#"Node("P -> N [^] P", [Node("N -> [0-9]", [Leaf("[0-9]", "1")]), Leaf("[^]", "^"), Node("P -> N", [Node("N -> [0-9]", [Leaf("[0-9]", "2")])])])"#,
    ]);
}

#[test]
fn bogus_empty() {
    // A -> <empty> | B
    // B -> A
    let grammar = GrammarBuilder::new()
      .symbol("A")
      .symbol("B")
      .rule::<_, &str>("A", &[])
      .rule("A", &vec!["B"])
      .rule("B", &vec!["A"])
      .into_grammar("A")
      .expect("Bad grammar");
    let p = EarleyParser::new(grammar.clone());
    let mut input = DelimTokenizer::from_str("", "-", false);
    let ps = p.parse(&mut input).unwrap();
    // this generates an infinite number of parse trees, don't check/print them all
    let tree = Tree::builder(grammar).eval(&ps).unwrap();
    check_trees(&vec![tree], vec![r#"Node("A -> ", [])"#]);
}

#[test]
fn bogus_epsilon() {
    // Grammar for balanced parenthesis
    // P  -> '(' P ')' | P P | <epsilon>
    let grammar = GrammarBuilder::new()
      .symbol("P")
      .symbol(("(", |l: &str| l == "("))
      .symbol((")", |l: &str| l == ")"))
      .rule("P", &["(", "P", ")"])
      .rule("P", &["P", "P"])
      .rule::<_, &str>("P", &[])
      .into_grammar("P")
      .expect("Bad grammar");
    let p = EarleyParser::new(grammar.clone());
    let mut input = Scanner::from_buf("".split_whitespace()
                                      .map(|s| s.to_string()));
    let ps = p.parse(&mut input).unwrap();
    // this generates an infinite number of parse trees, don't check/print them all
    let tree = Tree::builder(grammar).eval(&ps).unwrap();
    check_trees(&vec![tree], vec![r#"Node("P -> ", [])"#]);
}

#[test]
fn grammar_example() {
    // Grammar for all words containing 'main'
    // Program   -> Letters 'm' 'a' 'i' 'n' Letters
    // Letters   -> oneletter Letters | <epsilon>
    // oneletter -> [a-zA-Z]
    let grammar = GrammarBuilder::new()
      .symbol("Program")
      .symbol("Letters")
      .symbol(("oneletter", |l: &str| l.len() == 1 &&
               l.chars().next().unwrap().is_alphabetic()))
      .symbol(("m", |l: &str| l == "m"))
      .symbol(("a", |l: &str| l == "a"))
      .symbol(("i", |l: &str| l == "i"))
      .symbol(("n", |l: &str| l == "n"))
      .rule("Program", &["Letters", "m", "a", "i", "n", "Letters"])
      .rule("Letters", &["oneletter", "Letters"])
      .rule::<_, &str>("Letters", &[])
      .into_grammar("Program")
      .expect("Bad grammar");
    let p = EarleyParser::new(grammar);
    let mut input = Scanner::from_buf("containsmainword".chars().map(|c| c.to_string()));
    assert!(p.parse(&mut input).is_ok());
}

#[test]
fn math_ambiguous() {
    // E -> E + E | E * E | n
    let grammar = GrammarBuilder::new()
      .symbol("E")
      .symbol(("+", |n: &str| n == "+"))
      .symbol(("*", |n: &str| n == "*"))
      .symbol(("n", |n: &str|
          n.chars().all(|c| "1234567890".contains(c))))
      .rule("E", &["E", "+", "E"])
      .rule("E", &["E", "*", "E"])
      .rule("E", &["n"])
      .into_grammar("E")
      .expect("Bad grammar");
    // number of trees here should match Catalan numbers if same operator
    let mut input = DelimTokenizer::from_str("0*1*2*3*4*5", "*", false);
    let p = EarleyParser::new(grammar.clone());
    let ps = p.parse(&mut input).unwrap();
    let trees = Tree::builder(grammar).eval_all(&ps).unwrap();
    check_trees(&trees, vec![
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "1")])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "4")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])])])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "4")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "2")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "4")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "2")])])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "4")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "1")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "2")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "4")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])"#,
        r#"Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "2")])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "3")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])])"#,
        r#"Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "2")])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "3")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "4")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])])])"#,
        r#"Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "4")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])])])"#,
        r#"Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "3")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])])])"#,
        r#"Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "3")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "4")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])])])])"#,
        r#"Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])])"#,
        r#"Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "3")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])])"#,
        r#"Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])"#,
        r#"Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "3")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])])])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])"#,
        r#"Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])"#,
        r#"Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "2")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])"#,
        r#"Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "2")])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "3")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])"#,
        r#"Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "4")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])])"#,
        r#"Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "2")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "4")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "2")])])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "3")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "1")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "2")])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "3")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "1")])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])])])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "2")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "2")])])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "1")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "2")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "1")])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "1")])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "3")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])])])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])])])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "3")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])])])])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "2")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "2")])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "3")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])])])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "1")])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "4")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "1")])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "3")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "1")])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "3")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "4")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "1")])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "3")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "1")])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "2")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "3")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "2")])])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "3")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "1")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "2")])])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "3")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "4")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "1")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "2")])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "3")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "4")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])"#,
        r#"Node("E -> E * E", [Node("E -> E * E", [Node("E -> E * E", [Node("E -> n", [Leaf("n", "0")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "1")])]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "2")])]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "3")]), Leaf("*", "*"), Node("E -> E * E", [Node("E -> n", [Leaf("n", "4")]), Leaf("*", "*"), Node("E -> n", [Leaf("n", "5")])])])])"#,
    ]);
}

#[test]
fn math_various() {
    let p = EarleyParser::new(grammar_math());
    let inputs = vec![
        "1+2^3^4*5/6+7*8^9",
        "(1+2^3)^4*5/6+7*8^9",
        "1+2^3^4*5",
        "(1+2)*3",
    ];
    for input in inputs.iter() {
        let mut input = DelimTokenizer::from_str(input, "+*-/()^", false);
        assert!(p.parse(&mut input).is_ok());
    }
}

#[test]
fn chained_terminals() {
    // E -> X + +  (and other variants)
    // X -> <epsilon>
    let rule_variants = vec![
        vec!["X", "+"],
        vec!["+", "X"],
        vec!["X", "+", "+"],
        vec!["+", "+", "X"],
        vec!["+", "X", "+"],
    ];
    for variant in rule_variants {
        let tokens = match variant.len() {
            2 => "+", 3 => "++", _ => unreachable!()
        };
        let g = GrammarBuilder::new()
          .symbol("E")
          .symbol("X")
          .symbol(("+", |n: &str| n == "+"))
          .rule("E", &variant)
          .rule::<_, &str>("X", &[])
          .into_grammar("E")
          .expect("Bad grammar");
        let p = EarleyParser::new(g);
        let mut input = DelimTokenizer::from_str(tokens, "+", false);
        assert!(p.parse(&mut input).is_ok());
    }
}

#[test]
fn natural_lang() {
    let g = GrammarBuilder::new()
      .symbol(("N", |n: &str| {
        n == "time" || n == "flight" || n == "banana" ||
        n == "flies" || n == "boy" || n == "telescope"
      }))
      .symbol(("D", |n: &str| { n == "the" || n == "a" || n == "an" }))
      .symbol(("V", |n: &str| {
        n == "book" || n == "eat" || n == "sleep" || n == "saw"
      }))
      .symbol(("P", |n: &str| {
        n == "with" || n == "in" || n == "on" || n == "at" || n == "through"
      }))
      .symbol(("[name]", |n: &str| n == "john" || n == "houston"))
      .symbol("PP")
      .symbol("NP")
      .symbol("VP")
      .symbol("S")
      .rule("NP", &["D", "N"])
      .rule("NP", &["[name]"])
      .rule("NP", &["NP", "PP"])
      .rule("PP", &["P", "NP"])
      .rule("VP", &["V", "NP"])
      .rule("VP", &["VP", "PP"])
      .rule("S", &["NP", "VP"])
      .rule("S", &["VP"])
      .into_grammar("S")
      .expect("Bad grammar");
    let p = EarleyParser::new(g);
    let inputs = vec![
        "book the flight through houston",
        "john saw the boy with the telescope",
    ];
    for input in inputs.iter() {
        let mut input = DelimTokenizer::from_str(input, " ", true);
        assert!(p.parse(&mut input).is_ok());
    }
}

///////////////////////////////////////////////////////////////////////////////

fn small_math() -> Grammar {
    // S -> S + E | E
    // E -> n ^ E | n
    GrammarBuilder::new()
      .symbol("E")
      .symbol(("+", |n: &str| n == "+"))
      .symbol(("*", |n: &str| n == "*"))
      .symbol(("n", |n: &str| "1234567890".contains(n)))
      .rule("E", &["E", "*", "E"])
      .rule("E", &["E", "+", "E"])
      .rule("E", &["n"])
      .into_grammar("E")
      .expect("Bad grammar")
}

#[test]
fn eval_actions() {
    let mut input = DelimTokenizer::from_str("3+4*2", "+*", false);
    let ps = EarleyParser::new(small_math()).parse(&mut input).unwrap();
    let mut ev = EarleyEvaler::new(|symbol, token| {
        match symbol {"n" => f64::from_str(token).unwrap(), _ => 0.0}
    });
    ev.action("E -> E + E", |nodes| nodes[0] + nodes[2]);
    ev.action("E -> E * E", |nodes| nodes[0] * nodes[2]);
    ev.action("E -> n", |nodes| nodes[0]);
    let trees = ev.eval_all(&ps).unwrap();
    eprintln!("{:?}", trees);
    assert_eq!(trees.len(), 2);
    assert!(trees.contains(&11.0));
    assert!(trees.contains(&14.0));
}

#[test]
fn build_ast() {
    let mut input = DelimTokenizer::from_str("3+4*2", "+*", false);
    let ps = EarleyParser::new(small_math()).parse(&mut input).unwrap();

    #[derive(Clone, Debug)]
    enum MathAST {
        BinOP(Box<MathAST>, String, Box<MathAST>),
        Num(f64),
    }
    let mut ev = EarleyEvaler::new(|symbol, token| {
        match symbol {
            "n" => MathAST::Num(f64::from_str(token).unwrap()),
            _ => MathAST::Num(0.0)
        }
    });
    ev.action("E -> E * E", |nodes| MathAST::BinOP(
        Box::new(nodes[0].clone()), format!("*"), Box::new(nodes[2].clone())));
    ev.action("E -> E + E", |nodes| MathAST::BinOP(
        Box::new(nodes[0].clone()), format!("+"), Box::new(nodes[2].clone())));
    ev.action("E -> n", |nodes| nodes[0].clone());

    let trees = ev.eval_all(&ps).unwrap();
    check_trees(&trees, vec![
        r#"BinOP(BinOP(Num(3), "+", Num(4)), "*", Num(2))"#,
        r#"BinOP(Num(3), "+", BinOP(Num(4), "*", Num(2)))"#,
    ]);
}

#[test]
fn build_sexpr() {
    let mut input = DelimTokenizer::from_str("3+4*2", "+*", false);
    let ps = EarleyParser::new(small_math()).parse(&mut input).unwrap();

    let mut ev = EarleyEvaler::new(|_, tok| Sexpr::Atom(tok.to_string()));
    ev.action("E -> E + E", |nodes| Sexpr::List(nodes.clone()));
    ev.action("E -> E * E", |nodes| Sexpr::List(nodes.clone()));
    ev.action("E -> n", |nodes| nodes[0].clone());
    let trees = ev.eval_all(&ps).unwrap();
    eprintln!("{:?}", trees);
    check_trees(&trees, vec![
        r#"List([List([Atom("3"), Atom("+"), Atom("4")]), Atom("*"), Atom("2")])"#,
        r#"List([Atom("3"), Atom("+"), List([Atom("4"), Atom("*"), Atom("2")])])"#,
    ]);
}