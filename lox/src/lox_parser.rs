#![deny(warnings)]

extern crate lexers;
use self::lexers::Scanner;

use lox_scanner::{Token, TT};


#[derive(Debug)]
pub enum Expr {
    Logical(Box<Expr>, Token, Box<Expr>),
    Binary(Box<Expr>, Token, Box<Expr>),
    Unary(Token, Box<Expr>),
    Bool(bool),
    Nil,
    Num(f64),
    Str(String),
    Grouping(Box<Expr>),
    Var(String),
    Assign(String, Box<Expr>),
}

pub enum Stmt {
    Print(Expr),
    Expr(Expr),
    Var(String, Expr),
    Block(Vec<Stmt>),
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    While(Expr, Box<Stmt>),
}

pub type ExprResult = Result<Expr, String>;
pub type StmtResult = Result<Stmt, String>;

pub struct LoxParser {
    scanner: Scanner<Token>,
    errors: bool,
}

impl LoxParser {
    pub fn new(scanner: Scanner<Token>) -> Self {
        LoxParser{scanner: scanner, errors: false}
    }

    fn accept(&mut self, token_types: Vec<TT>) -> bool {
        let backtrack = self.scanner.pos();
        if let Some(token) = self.scanner.next() {
            let found = token_types.iter().any(|ttype| match &token.token {
                &TT::Str(_) => match ttype { &TT::Str(_) => true, _ => false },
                &TT::Id(_) => match ttype { &TT::Id(_) => true, _ => false },
                &TT::Num(_) => match ttype { &TT::Num(_) => true, _ => false },
                other => other == ttype
            });
            if found { return true; }
        }
        self.scanner.set_pos(backtrack);
        false
    }

    fn consume<S: AsRef<str>>(&mut self, token_types: Vec<TT>,
                              err: S) -> Result<(), String> {
        match self.accept(token_types) {
            true => { self.scanner.ignore(); Ok(()) },
            false => {
                let bad_token = self.scanner.peek();
                Err(self.error(bad_token, err))
            }
        }
    }

    fn error<S: AsRef<str>>(&mut self, token: Option<Token>, msg: S) -> String {
        self.errors = true;
        match token {
            Some(t) => format!("LoxParser error: {:?} at line {}, {}",
                               t.lexeme, t.line, msg.as_ref()),
            _ => format!("LoxParser error: EOF, {}", msg.as_ref()),
        }
    }

    //fn synchronize(&mut self) {
        //// sync on statement boundaries (ie: semicolon)
        //// TODO: check for loops' semicolon
        //while let Some(token) = self.scanner.next() {
            //if token.token == TT::SEMICOLON {
                //return self.scanner.ignore();
            //}
        //}
    //}
}


/* Grammar:
 *
 *  program        := { statement } EOF ;
 *
 *  declaration    := varDecl
 *                  | statement ;
 *
 *  varDecl        := "var" IDENTIFIER [ "=" expression ] ";" ;
 *
 *  statement      := exprStmt
 *                  | ifStmt
 *                  | printStmt
 *                  | whileStmt
 *                  | block ;
 *
 *  exprStmt       := expression ";" ;
 *  ifStmt         := "if" "(" expression ")" statement [ "else" statement ] ;
 *  printStmt      := "print" expression ";" ;
 *  whileStmt      := "while" "(" expression ")" statement ;
 *  forStmt        := "for" "(" varDecl | exprStmt | ";"
 *                            { expression } ";"
 *                            { expression } ")" statement ;
 *  block          := "{" { declaration } "}" ;
 *
 *  expression     := assignment ;
 *  assignment     := identifier "=" assignment
 *                  | logic_or ;
 *  logic_or       := logic_and { "or" logic_and } ;
 *  logic_and      := equality { "and" equality } ;
 *  equality       := comparison { ( "!=" | "==" ) comparison } ;
 *  comparison     := addition { ( ">" | ">=" | "<" | "<=" ) addition } ;
 *  addition       := multiplication { ( "-" | "+" ) multiplication } ;
 *  multiplication := unary { ( "/" | "*" ) unary } ;
 *  unary          := ( "!" | "-" | "$" ) unary
 *                  | primary ;
 *  primary        := NUMBER | STRING | "false" | "true" | "nil"
 *                  | "(" expression ")"
 *                  | IDENTIFIER ;
 */

impl LoxParser {
    fn assignment(&mut self) -> ExprResult {
        let expr = self.logic_or()?;
        if self.accept(vec![TT::ASSIGN]) {
            let maybe_bad = Some(self.scanner.extract().swap_remove(0));
            // recursively parse right-hand-side
            let value = self.assignment()?;
            return match expr {
                // assign to variable, later other lhs possible
                Expr::Var(name) => Ok(Expr::Assign(name, Box::new(value))),
                _ => Err(self.error(maybe_bad, "invalid assignment target"))
            };
        }
        Ok(expr)
    }

    fn expression(&mut self) -> ExprResult {
        self.assignment()
    }

    fn logic_and(&mut self) -> ExprResult {
        let mut expr = self.equality()?;
        while self.accept(vec![TT::AND]) {
            let op = self.scanner.extract().swap_remove(0);
            let rhs = self.equality()?;
            expr = Expr::Logical(Box::new(expr), op, Box::new(rhs));
        }
        Ok(expr)
    }

    fn logic_or(&mut self) -> ExprResult {
        let mut expr = self.logic_and()?;
        while self.accept(vec![TT::OR]) {
            let op = self.scanner.extract().swap_remove(0);
            let rhs = self.logic_and()?;
            expr = Expr::Logical(Box::new(expr), op, Box::new(rhs));
        }
        Ok(expr)
    }

    fn equality(&mut self) -> ExprResult {
        let mut expr = self.comparison()?;
        while self.accept(vec![TT::EQ, TT::NE]) {
            let op = self.scanner.extract().swap_remove(0);
            let rhs = self.comparison()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
        }
        Ok(expr)
    }

    fn comparison(&mut self ) -> ExprResult {
        let mut expr = self.addition()?;
        while self.accept(vec![TT::GT, TT::GE, TT::LT, TT::LE]) {
            let op = self.scanner.extract().swap_remove(0);
            let rhs = self.addition()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
        }
        Ok(expr)
    }

    fn addition(&mut self) -> ExprResult {
        let mut expr = self.multiplication()?;
        while self.accept(vec![TT::MINUS, TT::PLUS]) {
            let op = self.scanner.extract().swap_remove(0);
            let rhs = self.multiplication()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
        }
        Ok(expr)
    }

    fn multiplication(&mut self) -> ExprResult {
        let mut expr = self.unary()?;
        while self.accept(vec![TT::SLASH, TT::STAR]) {
            let op = self.scanner.extract().swap_remove(0);
            let rhs = self.unary()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
        }
        Ok(expr)
    }

    fn unary(&mut self) -> ExprResult {
        if self.accept(vec![TT::BANG, TT::MINUS, TT::DOLLAR]) {
            let op = self.scanner.extract().swap_remove(0);
            let rhs = self.unary()?;
            return Ok(Expr::Unary(op, Box::new(rhs)));
        }
        self.primary()
    }

    fn primary(&mut self) -> ExprResult {
        if self.accept(vec![TT::FALSE, TT::TRUE]) {
            return Ok(match self.scanner.extract().swap_remove(0).token {
                TT::TRUE => Expr::Bool(true),
                _ => Expr::Bool(false),
            });
        }
        if self.accept(vec![TT::NIL]) {
            self.scanner.ignore();
            return Ok(Expr::Nil);
        }
        if self.accept(vec![TT::Num(0.0)]) {
            return Ok(match self.scanner.extract().swap_remove(0).token {
                TT::Num(n) => Expr::Num(n),
                o => panic!("LoxParser Bug! unexpected token: {:?}", o),
            });
        }
        if self.accept(vec![TT::Str("".to_string())]) {
            return Ok(match self.scanner.extract().swap_remove(0).token {
                TT::Str(s) => Expr::Str(s),
                o => panic!("LoxParser Bug! unexpected token: {:?}", o),
            });
        }
        if self.accept(vec![TT::Id("".to_string())]) {
            return Ok(match self.scanner.extract().swap_remove(0).token {
                TT::Id(v) => Expr::Var(v),
                o => panic!("LoxParser Bug! unexpected token: {:?}", o),
            });
        }
        if self.accept(vec![TT::OPAREN]) {
            self.scanner.ignore(); // skip OPAREN
            let expr = self.expression()?;
            self.consume(vec![TT::CPAREN], "expect ')' after expression")?;
            return Ok(Expr::Grouping(Box::new(expr)));
        }
        let bad_token = self.scanner.peek();
        Err(self.error(bad_token, "expected expression"))
    }

    fn print_stmt(&mut self) -> StmtResult {
        let expr = self.expression()?;
        self.consume(vec![TT::SEMICOLON], "expect ';' after value")?;
        Ok(Stmt::Print(expr))
    }

    fn expr_stmt(&mut self) -> StmtResult {
        let expr = self.expression()?;
        self.consume(vec![TT::SEMICOLON], "expect ';' after value")?;
        Ok(Stmt::Expr(expr))
    }

    fn block_stmt(&mut self) -> Result<Vec<Stmt>, String> {
        let mut statements = Vec::new();
        while let Some(maybe_cbrace) = self.scanner.peek() {
            if maybe_cbrace.token == TT::CBRACE { break; }
            statements.push(self.declaration()?);
        }
        self.consume(vec![TT::CBRACE], "expect '}' after value")?;
        Ok(statements)
    }

    fn if_stmt(&mut self) -> StmtResult {
        self.consume(vec![TT::OPAREN], "expect '(' after 'if'")?;
        let condition = self.expression()?;
        self.consume(vec![TT::CPAREN], "expect ')' after 'if' condition")?;
        let then_branch = self.statement()?;
        if self.accept(vec![TT::ELSE]) {
            self.scanner.ignore(); // skip else
            let else_branch = Some(Box::new(self.statement()?));
            return Ok(Stmt::If(condition, Box::new(then_branch), else_branch));
        }
        Ok(Stmt::If(condition, Box::new(then_branch), None))
    }

    fn while_stmt(&mut self) -> StmtResult {
        self.consume(vec![TT::OPAREN], "expect '(' after 'while'")?;
        let condition = self.expression()?;
        self.consume(vec![TT::CPAREN], "expect ')' after 'if' condition")?;
        let body = self.statement()?;
        Ok(Stmt::While(condition, Box::new(body)))
    }

    fn for_stmt(&mut self) -> StmtResult {
        self.consume(vec![TT::OPAREN], "expect '(' after 'for'")?;
        let init = if self.accept(vec![TT::SEMICOLON]) {
            self.scanner.ignore(); // skip ';'
            None
        } else if self.accept(vec![TT::VAR]) {
            self.scanner.ignore(); // skip var
            Some(self.var_declaration()?)
        } else {
            Some(self.expr_stmt()?)
        };
        // parse loop condition
        let condition = match self.scanner.peek() {
            Some(ref t) if t.token != TT::SEMICOLON => self.expression()?,
            _ => Expr::Bool(true)
        };
        self.consume(vec![TT::SEMICOLON], "expect ';' loop condition")?;
        // parse loop increment
        let increment = match self.scanner.peek() {
            Some(ref t) if t.token != TT::CPAREN => Some(self.expression()?),
            _ => None
        };
        self.consume(vec![TT::CPAREN], "expect ')' after 'for' clause")?;
        // desugar forStmt into WhileStmt
        let body = Stmt::While(condition, Box::new(match increment {
            Some(inc) => Stmt::Block(vec![self.statement()?, Stmt::Expr(inc)]),
            _ => self.statement()?
        }));
        Ok(match init {Some(init) => Stmt::Block(vec![init, body]), _ => body})
    }

    fn statement(&mut self) -> StmtResult {
        if self.accept(vec![TT::PRINT]) {
            self.scanner.ignore(); // skip print
            return self.print_stmt();
        }
        if self.accept(vec![TT::OBRACE]) {
            self.scanner.ignore(); // skip obrace
            return Ok(Stmt::Block(self.block_stmt()?));
        }
        if self.accept(vec![TT::IF]) {
            self.scanner.ignore(); // skip if
            return self.if_stmt();
        }
        if self.accept(vec![TT::WHILE]) {
            self.scanner.ignore(); // skip while
            return self.while_stmt();
        }
        if self.accept(vec![TT::FOR]) {
            self.scanner.ignore(); // skip for
            return self.for_stmt();
        }
        self.expr_stmt()
    }

    fn var_declaration(&mut self) -> StmtResult {
        if !self.accept(vec![TT::Id("".to_string())]) {
            let bad_token = self.scanner.peek();
            return Err(self.error(bad_token, "expect variable name"));
        }
        let name = self.scanner.extract().swap_remove(0).lexeme;
        let mut init = Expr::Nil;
        if self.accept(vec![TT::ASSIGN]) {
            self.scanner.ignore(); // skip assign
            init = self.expression()?;
        }
        self.consume(vec![TT::SEMICOLON], "expect ';' after variable decl")?;
        Ok(Stmt::Var(name, init))
    }

    fn declaration(&mut self) -> StmtResult {
        if self.accept(vec![TT::VAR]) {
            self.scanner.ignore(); // skip var
            return self.var_declaration();
        }
        self.statement()
    }

    pub fn parse(&mut self) -> Result<Vec<Stmt>, String> {
        let mut statements = Vec::new();
        while self.scanner.peek().is_some() {
            let stmt = self.declaration()?;
            statements.push(stmt);
        }
        Ok(statements)
    }
}
