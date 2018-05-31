use ordered_float::OrderedFloat;
use id;
use syntax::{IntBin, FloatBin, CompBin, Type};
use k_normal::{KNormal, KFundef};
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cls {
    pub entry: id::L,
    pub actual_fv: Box<[String]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Closure {
    Unit,
    Int(i64),
    Float(OrderedFloat<f64>),
    Neg(String),
    IntBin(IntBin, String, String),
    FNeg(String),
    FloatBin(FloatBin, String, String),
    IfComp(CompBin, String, String, Box<Closure>, Box<Closure>),
    Let((String, Type), Box<Closure>, Box<Closure>),
    Var(String),
    MakeCls(String, Type, Cls, Box<Closure>),
    AppCls(String, Box<[String]>),
    AppDir(id::L, Box<[String]>),
    Tuple(Box<[String]>),
    LetTuple(Box<[(String, Type)]>, String, Box<Closure>),
    Get(String, String),
    Put(String, String, String),
    ExtArray(id::L),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fundef {
    pub name: (id::L, Type),
    pub args: Box<[(String, Type)]>,
    pub formal_fv: Box<[(String, Type)]>,
    pub body: Box<Closure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prog(pub Box<[Fundef]>, pub Closure);

// Display
impl Closure {
    fn fmt2(&self, f: &mut fmt::Formatter, level: usize) -> fmt::Result {
        use self::Closure::*;
        match *self {
            Unit => write!(f, "()"),
            Int(v) => write!(f, "{}", v),
            Float(fv) => write!(f, "{}", fv),
            Neg(ref x) => write!(f, "-{}", x),
            IntBin(op, ref x, ref y) => {
                let op_str = match op {
                    self::IntBin::Add => "+",
                    self::IntBin::Sub => "-",
                };
                write!(f, "{} {} {}", x, op_str, y)
            },
            FNeg(ref x) => write!(f, "-.{}", x),
            FloatBin(op, ref x, ref y) => {
                let op_str = match op {
                    self::FloatBin::FAdd => "+.",
                    self::FloatBin::FSub => "-.",
                    self::FloatBin::FMul => "*.",
                    self::FloatBin::FDiv => "/.",
                };
                write!(f, "{} {} {}", x, op_str, y)
            },
            IfComp(op, ref x, ref y, ref e1, ref e2) => {
                let op_str = match op {
                    self::CompBin::Eq => "=",
                    self::CompBin::LE => "<=",
                };
                write!(f, "if {} {} {} then\n", x, op_str, y)?;
                for _ in 0 .. level + 2 {
                    write!(f, " ")?;
                }
                e1.fmt2(f, level + 2)?;
                write!(f, "\n")?;
                for _ in 0 .. level {
                    write!(f, " ")?;
                }
                write!(f, "else\n")?;
                for _ in 0 .. level + 2 {
                    write!(f, " ")?;
                }
                e2.fmt2(f, level + 2)
            },
            Let((ref x, ref t), ref e1, ref e2) => {
                if let Type::Unit = *t {
                    if x.len() >= 6 && &x[0..6] == "_dummy" {
                        // this let expression is actually "e1; e2"
                        e1.fmt2(f, level)?;
                        write!(f, ";\n")?;
                        for _ in 0 .. level {
                            write!(f, " ")?;
                        }
                        return e2.fmt2(f, level);
                    }
                }
                write!(f, "let {}: {} = ", x, t)?;
                e1.fmt2(f, level)?;
                write!(f, " in\n")?;
                for _ in 0 .. level {
                    write!(f, " ")?;
                }
                e2.fmt2(f, level)
            },
            Var(ref x) => write!(f, "{}", x),
            MakeCls(ref x, ref t, ref cls, ref e) => {
                write!(f, "MakeCls {}: {} (", x, t)?;
                let Cls { entry: id::L(ref l), actual_fv: ref fv } = *cls;
                write!(f, "{} {:?}) in\n", l, fv)?;
                for _ in 0 .. level {
                    write!(f, " ")?;
                }
                e.fmt2(f, level)
            },
            AppDir(id::L(ref func), ref args) => {
                write!(f, "{}", func)?;
                for v in args.iter() {
                    write!(f, " {}", v)?;
                }
                Ok(())
            },
            AppCls(ref func, ref args) => {
                write!(f, "[{}]", func)?;
                for v in args.iter() {
                    write!(f, " {}", v)?;
                }
                Ok(())
            },
            Tuple(ref elems) => {
                write!(f, "(")?;
                for i in 0 .. elems.len() {
                    write!(f, "{}", elems[i])?;
                    if i < elems.len() - 1 {
                        write!(f, ", ")?;
                    }
                }
                write!(f, ")")
            },
            LetTuple(ref xts, ref y, ref e) => {
                write!(f, "let (")?;
                for i in 0 .. xts.len() {
                    write!(f, "{}: {}", xts[i].0, xts[i].1)?;
                    if i < xts.len() - 1 {
                        write!(f, ", ")?;
                    }
                }
                write!(f, ") = {} in\n", y)?;
                for _ in 0 .. level {
                    write!(f, " ")?;
                }
                e.fmt2(f, level)
            },
            Get(ref x, ref y) =>
                write!(f, "{}.({})", x, y),
            Put(ref x, ref y, ref z) =>
                write!(f, "{}.({}) <- {}", x, y, z),
            ExtArray(id::L(ref a)) => write!(f, "(extarr:{})", a),
        }
    }
}

impl fmt::Display for Closure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.fmt2(f, 0)
    }
}

impl fmt::Display for Fundef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Fundef { name: (id::L(ref x), ref t), args: ref yts, body: ref e1,
                     formal_fv: ref fv } = *self;
        write!(f, "define ({}: {})", x, t)?;
        for &(ref y, ref t) in yts.iter() {
            write!(f, " ({}: {})", y, t)?;
        }
        if !fv.is_empty() {
            write!(f, " freevar:")?;
            for &(ref x, ref t) in fv.iter() {
                write!(f, " ({}: {})", x, t)?;
            }
        }
        write!(f, " {{\n")?;
        write!(f, "  ")?;
        e1.fmt2(f, 2)?;
        write!(f, "\n}}")
    }
}

impl fmt::Display for Prog {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Prog(ref fundefs, ref e) = *self;
        for fundef in fundefs.iter() {
            write!(f, "{}\n", fundef)?;
        }
        write!(f, "{}", e)
    }
}

fn fv(e: &Closure) -> HashSet<String> {
    use self::Closure::*;
    macro_rules! invoke {
        ($e:expr) => (fv($e));
    }
    match *e {
        Unit | Int(_) | Float(_) | ExtArray(_) => HashSet::new(),
        Neg(ref x) | FNeg(ref x) => build_set!(x),
        IntBin(_, ref x, ref y) | FloatBin(_, ref x, ref y) |
        Get(ref x, ref y) =>
            build_set!(x, y),
        IfComp(_, ref x, ref y, ref e1, ref e2) => {
            let h = build_set!(x, y);
            let s1 = invoke!(e1);
            let s2 = invoke!(e2);
            &(&h | &s1) | &s2
        },
        Let((ref x, _), ref e1, ref e2) => {
            let s1 = invoke!(e1);
            let s2 = &invoke!(e2) - &build_set!(x);
            &s1 | &s2
        }
        Var(ref x) => build_set!(x),
        MakeCls(ref x, _, Cls { entry: _, actual_fv: ref ys }, ref e) =>
            &(&ys.iter().cloned().collect() | &invoke!(e)) - &build_set!(x),
        AppCls(ref x, ref ys) =>
            &build_set!(x) | &ys.iter().cloned().collect::<HashSet<_>>(),
        AppDir(_, ref xs) | Tuple(ref xs) => xs.iter().cloned().collect(),
        LetTuple(ref xs, ref y, ref e) => {
            let tmp: HashSet<String> = xs.iter().map(|x| x.0.clone())
                .collect(); // S.of_list (List.map fst xs)
            &build_set!(y) | &(&invoke!(e) - &tmp)
        },
        Put(ref x, ref y, ref z) => build_set!(x, y, z),
    }
}

fn g(env: &HashMap<String, Type>, known: &HashSet<String>,
     e: KNormal, toplevel: &mut Vec<Fundef>) -> Closure {
    use self::Closure::*;
    macro_rules! invoke {
        ($e: expr) => { Box::new(g(env, known, *$e, toplevel)) }
    }
    match e {
        KNormal::Unit => Unit,
        KNormal::Int(i) => Int(i),
        KNormal::Float(f) => Float(f),
        KNormal::Neg(x) => Neg(x),
        KNormal::IntBin(op, x, y) => IntBin(op, x, y),
        KNormal::FNeg(x) => FNeg(x),
        KNormal::FloatBin(op, x, y) => FloatBin(op, x, y),
        KNormal::IfComp(op, x, y, e1, e2) =>
            IfComp(op, x, y, invoke!(e1), invoke!(e2)),
        KNormal::Let((x, t), e1, e2) => {
            let mut cp_env = env.clone();
            cp_env.insert(x.clone(), t.clone());
            Let((x, t), invoke!(e1), Box::new(g(&cp_env, known, *e2, toplevel)))
        },
        KNormal::Var(x) => Var(x),
        KNormal::LetRec(KFundef { name: (x, t), args: yts, body: e1}, e2) => {
            /* Follow the original code */
            let mut toplevel_cp = toplevel.clone();
            let mut env_p = env.clone();
            env_p.insert(x.clone(), t.clone());
            let mut known_p = known.clone();
            known_p.insert(x.clone());
            let mut env_p2 = env_p.clone();
            for &(ref y, ref t) in yts.iter() {
                env_p2.insert(y.clone(), t.clone());
            }
            let e1p = g(&env_p2, &known_p, (*e1).clone(), &mut toplevel_cp);
            /* Check if e1p contains free variables */
            let zs =
                &fv(&e1p) - &yts.iter().map(|&(ref y, _)| y.clone()).collect();
            let (known_p, e1p) = if zs.is_empty() {
                *toplevel = toplevel_cp;
                (&known_p, e1p)
            } else {
                eprintln!("free variables(s) {:?} found in function {}",
                          zs, x);
                eprintln!("function {} cannot be directly applied in fact.", x);
                let e1p = g(&env_p2, known, *e1, toplevel);
                (known, e1p)
            };
            let zs: Vec<String> = (&zs - &build_set!(x)).into_iter().collect();
            let zts: Vec<(String, Type)> = zs.iter().map(|&ref z| (z.clone(), env.get(z).unwrap().clone())).collect();
            toplevel.push(Fundef { name: (id::L(x.clone()), t.clone()),
                                   args: yts,
                                   formal_fv: zts.into_boxed_slice(),
                                   body: Box::new(e1p) });
            let e2p = g(&env_p, known_p, *e2, toplevel);
            if fv(&e2p).contains(&x) {
                MakeCls(x.clone(), t,
                        Cls { entry: id::L(x), actual_fv: zs.into_boxed_slice() },
                        Box::new(e2p))
            } else {
                eprintln!("eliminating closure {}", x);
                e2p
            }
        },
        KNormal::App(x, ys) => {
            if known.contains(&x) {
                AppDir(id::L(x), ys)
            } else {
                AppCls(x, ys)
            }
        },
        KNormal::Tuple(xs) => Tuple(xs),
        KNormal::LetTuple(xts, y, e) => {
            let mut cp_env = env.clone();
            for &(ref x, ref t) in xts.iter() {
                cp_env.insert(x.clone(), t.clone());
            }
            LetTuple(xts, y, Box::new(g(&cp_env, known, *e, toplevel)))
        },
        KNormal::Get(x, y) => Get(x, y),
        KNormal::Put(x, y, z) => Put(x, y, z),
        KNormal::ExtArray(x) => ExtArray(id::L(x)),
        KNormal::ExtFunApp(x, ys) =>
            AppDir(id::L(format!("min_caml_{}", x)), ys),
    }
}


pub fn f(e: KNormal) -> Prog {
    let mut toplevel = Vec::new();
    let e = g(&HashMap::new(), &HashSet::new(), e, &mut toplevel);
    Prog(toplevel.into_boxed_slice(), e)
}

#[cfg(test)]
mod tests {
    use closure::*;
    #[test]
    fn test_g_if() {
        let known = HashSet::new();
        let mut toplevel = Vec::new();
        // IfComp(Eq, x, x, y, z)
        // ==> IfComp(Eq, x, x, y, z)
        let x = || "x".to_string();
        let y = || "y".to_string();
        let z = || "z".to_string();
        let env = vec![(x(), Type::Int),
                       (y(), Type::Int),
                       (z(), Type::Int)].into_iter().collect();
        let expr = KNormal::IfComp(CompBin::Eq,
                                   x(), x(),
                                   Box::new(KNormal::Var(y())),
                                   Box::new(KNormal::Var(z())));
        assert_eq!(g(&env, &known, expr, &mut toplevel),
                   Closure::IfComp(CompBin::Eq,
                                   x(), x(),
                                   Box::new(Closure::Var(y())),
                                   Box::new(Closure::Var(z()))));
    }
    #[test]
    fn test_g_appdir() {
        let known = HashSet::new();
        let mut toplevel = Vec::new();
        // LetRec({(f,Fun([Int],Int)),[(x,Int)],Var(x)}, App(f, [y]))
        // ==> AppDir(L(f), y)
        let x = || "x".to_string();
        let y = || "y".to_string();
        let ff = || "f".to_string();
        let env = vec![(y(), Type::Int)].into_iter().collect();
        let expr = KNormal::LetRec(KFundef {
            name: (ff(), Type::Fun(Box::new([Type::Int]), Box::new(Type::Int))),
            args: Box::new([(x(), Type::Int)]),
            body: Box::new(KNormal::Var(x())),
        },
                                   Box::new(KNormal::App(ff(), Box::new([y()]))));
        assert_eq!(g(&env, &known, expr, &mut toplevel),
                   Closure::AppDir(id::L(ff()), Box::new([y()])));
        assert_eq!(toplevel, vec![Fundef {
            name: (id::L(ff()), Type::Fun(Box::new([Type::Int]), Box::new(Type::Int))),
            args: Box::new([(x(), Type::Int)]),
            formal_fv: Box::new([]),
            body: Box::new(Closure::Var(x())),
        }]);
    }
}
