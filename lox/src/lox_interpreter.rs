#![deny(warnings)]

use lox_scanner::TT;
use lox_parser::{Expr, Stmt};
use lox_environment::Environment;
use lox_native::native_fn_env;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt;


pub trait Callable {
    fn call(&self, &mut LoxInterpreter, &Vec<V>) -> ExecResult;
    fn arity(&self) -> usize;
    fn id(&self) -> String;
}

#[derive(Clone)]
pub enum V {
    Nil,
    Num(f64),
    Bool(bool),
    Str(String),
    Callable(Rc<Callable>),
}

impl V {
    fn is_truthy(&self) -> bool {
        match self {
            &V::Nil => false,
            &V::Bool(ref b) => *b,
            _ => true
        }
    }
    fn num(&self) -> Result<f64, String> {
        match self {
            &V::Num(ref n) => Ok(*n),
            o => Err(format!("expected V::Num, found {:?}", o))
        }
    }
    fn str<'a>(&'a self) -> Result<&'a str, String> {
        match self {
            &V::Str(ref s) => Ok(s),
            o => Err(format!("expected V::Str, found {:?}", o))
        }
    }
    fn call(&self) -> Result<Rc<Callable>, String> {
        match self {
            &V::Callable(ref c) => Ok(c.clone()),
            o => Err(format!("expected V::Callable, found {:?}", o))
        }
    }
}

impl fmt::Debug for V {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &V::Nil => write!(f, "nil"),
            &V::Bool(ref b) => write!(f, "{}", b),
            &V::Num(ref n) => write!(f, "{}", n),
            &V::Str(ref s) => write!(f, "\"{}\"", s),
            &V::Callable(ref c) => write!(f, "\"{}\"", c.id()),
        }
    }
}

impl fmt::Display for V {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl PartialEq for V {
    fn eq(&self, other: &V) -> bool {
        match (self, other) {
            (&V::Nil, &V::Nil) => true,
            (&V::Num(ref a), &V::Num(ref b)) => a == b,
            (&V::Bool(ref a), &V::Bool(ref b)) => a == b,
            (&V::Str(ref a), &V::Str(ref b)) => a == b,
            (&V::Callable(ref a), &V::Callable(ref b)) => a.id() == b.id(),
            _ => false,
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

struct LoxFunction {
    name: String,
    params: Vec<String>,
    body: Vec<Stmt>,
    closure: Option<Rc<RefCell<Environment>>>,
}

impl Callable for LoxFunction {
    fn call(&self, interp: &mut LoxInterpreter, args: &Vec<V>) -> ExecResult {
        let mut environ = Environment::new(self.closure.clone());
        for (i, param) in self.params.iter().enumerate() {
            environ.define(param.to_string(), args[i].clone());
        }
        // keep track of return boundaries
        let depth = interp.func_depth;
        interp.func_depth += 1;
        let retval =
            interp.exec_block(&self.body, Rc::new(RefCell::new(environ)));
        interp.func_depth = depth;
        interp.funreturn = false;
        retval
    }
    fn arity(&self) -> usize {
        self.params.len()
    }
    fn id(&self) -> String {
        format!("<fn {}({})>", self.name, self.params.join(","))
    }
}

///////////////////////////////////////////////////////////////////////////////

type EvalResult = Result<V, String>;
pub type ExecResult = Result<V, String>;

pub struct LoxInterpreter {
    environ: Rc<RefCell<Environment>>,
    break_loops: usize,
    funreturn: bool,
    break_depth: usize,
    func_depth: usize,
}

impl LoxInterpreter {
    pub fn new() -> Self {
        LoxInterpreter{
            environ: Rc::new(RefCell::new(native_fn_env())),
            break_loops: 0,
            funreturn: false,
            break_depth: 0,
            func_depth: 0,
        }
    }

    fn eval(&mut self, expr: &Expr) -> EvalResult {
        match expr {
            &Expr::Nil => Ok(V::Nil),
            &Expr::Num(n) => Ok(V::Num(n)),
            &Expr::Str(ref s) => Ok(V::Str(s.to_string())),
            &Expr::Bool(ref b) => Ok(V::Bool(*b)),
            &Expr::Grouping(ref expr) => self.eval(&*expr),
            &Expr::Unary(ref op, ref expr) => {
                let expr = self.eval(expr)?;
                match op.token {
                    TT::MINUS => Ok(V::Num(-expr.num()?)),
                    TT::BANG => Ok(V::Bool(!expr.is_truthy())),
                    TT::DOLLAR => self.environ.borrow().get(expr.str()?),
                    _ => unreachable!("LoxIntepreter: bad Unary op {:?}", op)
                }
            },
            &Expr::Binary(ref lhs, ref op, ref rhs) => {
                let lhs = self.eval(lhs)?;
                let rhs = self.eval(rhs)?;
                match op.token {
                    TT::SLASH => Ok(V::Num(lhs.num()? / rhs.num()?)),
                    TT::STAR => Ok(V::Num(lhs.num()? * rhs.num()?)),
                    TT::MINUS => Ok(V::Num(lhs.num()? - rhs.num()?)),
                    TT::PLUS => match (&lhs, &rhs) {
                        (&V::Num(ref l), &V::Num(ref r)) => Ok(V::Num(l + r)),
                        (&V::Str(ref l), &V::Str(ref r)) =>
                            Ok(V::Str(format!("{}{}", l, r))),
                        (&V::Str(ref l), ref other) =>
                            Ok(V::Str(format!("{}{}", l, other))),
                        (ref other, &V::Str(ref r)) =>
                            Ok(V::Str(format!("{}{}", other, r))),
                        _ => Err(format!("can't {:?} + {:?}", lhs, rhs))
                    },
                    TT::GT => Ok(V::Bool(lhs.num()? > rhs.num()?)),
                    TT::GE => Ok(V::Bool(lhs.num()? >= rhs.num()?)),
                    TT::LT => Ok(V::Bool(lhs.num()? < rhs.num()?)),
                    TT::LE => Ok(V::Bool(lhs.num()? <= rhs.num()?)),
                    TT::EQ => Ok(V::Bool(lhs == rhs)),
                    TT::NE => Ok(V::Bool(lhs != rhs)),
                    _ => unreachable!("LoxIntepreter: bad binop {:?} {:?} {:?}",
                                      lhs, op, rhs)
                }
            },
            &Expr::Logical(ref lhs, ref op, ref rhs) => {
                let lhs = self.eval(lhs)?;
                match op.token {
                    TT::OR if lhs.is_truthy() => Ok(lhs),
                    TT::AND if !lhs.is_truthy() => Ok(lhs),
                    _ => self.eval(rhs)
                }
            },
            &Expr::Var(ref var) => self.environ.borrow().get(&var.lexeme),
            &Expr::Assign(ref var, ref expr) => {
                let value = self.eval(expr)?;
                self.environ.borrow_mut().assign(var.lexeme.clone(), value)
            },
            &Expr::Call(ref callee, ref args) => {
                let callee = self.eval(callee)?.call()?;
                if callee.arity() != args.len() {
                    return Err(format!("wrong arity for {} expected {} not {}",
                                       callee.id(), callee.arity(), args.len()))
                }
                let mut arguments = Vec::new();
                for arg in args {
                    arguments.push(self.eval(arg)?);
                }
                callee.call(self, &arguments)
            }
        }
    }

    fn exec_block(&mut self, statements: &Vec<Stmt>,
                  env: Rc<RefCell<Environment>>) -> ExecResult {
        let prev_env = self.environ.clone();
        self.environ = env;
        let mut retval = Ok(V::Nil);
        for stmt in statements {
            retval = self.execute(stmt);
            if retval.is_err() || self.funreturn || self.break_loops > 0 {
                break;
            }
        }
        // restore interpreter's env
        self.environ = prev_env;
        retval
    }

    fn execute(&mut self, stmt: &Stmt) -> ExecResult {
        match stmt {
            &Stmt::Expr(ref expr) => self.eval(expr),
            &Stmt::Print(ref expr) => {
                println!("{}", self.eval(expr)?);
                Ok(V::Nil)
            }
            &Stmt::Var(ref name, ref init) => {
                let value = self.eval(init)?;
                self.environ.borrow_mut().define(name.to_string(), value);
                Ok(V::Nil)
            },
            &Stmt::Block(ref stmts) => {
                let curenv = Environment::new(Some(self.environ.clone()));
                self.exec_block(stmts, Rc::new(RefCell::new(curenv)))
            },
            &Stmt::If(ref expr, ref then_branch, ref else_branch) => {
                let condition = self.eval(expr)?;
                match condition.is_truthy() {
                    true => self.execute(then_branch),
                    _ => match else_branch {
                        &Some(ref else_branch) => self.execute(else_branch),
                        _ => Ok(V::Nil)
                    }
                }
            },
            &Stmt::While(ref condition, ref body) => {
                let depth = self.break_depth;
                self.break_depth += 1;
                let mut retval = Ok(V::Nil);
                loop {
                    // check if we're trying to break out of loops
                    if self.break_loops > 0 {
                        self.break_loops -= 1; // we just got out of one
                        break;
                    }
                    retval = self.eval(condition);
                    if retval.is_err() { break; }
                    if let Ok(ref cond) = retval {
                        if !cond.is_truthy() { break; }
                    }
                    retval = self.execute(body);
                    if retval.is_err() { break; }
                }
                self.break_depth = depth;
                retval
            },
            &Stmt::Break(num_breaks) => {
                if self.break_depth < num_breaks {
                    return Err(format!("can't break {} times, depth {}",
                                       num_breaks, self.break_depth));
                }
                self.break_loops = num_breaks;
                Ok(V::Nil)
            },
            &Stmt::Function(ref name, ref params, ref body) => {
                let function = LoxFunction{
                    name: name.to_string(),
                    params: params.clone(),
                    body: body.clone(),
                    closure: Some(self.environ.clone())
                };
                self.environ.borrow_mut().define(
                    name.to_string(), V::Callable(Rc::new(function)));
                Ok(V::Nil)
            },
            &Stmt::Return(ref expr) => {
                if self.func_depth < 1 {
                    return Err("can't return outside of function".to_string());
                }
                let retval = self.eval(expr)?;
                self.funreturn = true;
                Ok(retval)
            }
        }
    }

    pub fn interpret(&mut self, statements: &Vec<Stmt>) -> ExecResult {
        for stmt in statements {
            self.execute(stmt)?;
        }
        Ok(V::Nil)
    }
}
