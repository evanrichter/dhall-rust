use crate::*;
use itertools::Itertools;
use std::fmt::{self, Display};

/// Generic instance that delegates to subexpressions
impl<SE: Display + Clone, N, E: Display> Display for ExprF<SE, Label, N, E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use crate::ExprF::*;
        match self {
            Lam(a, b, c) => {
                write!(f, "λ({} : {}) → {}", a, b, c)?;
            }
            BoolIf(a, b, c) => {
                write!(f, "if {} then {} else {}", a, b, c)?;
            }
            Pi(a, b, c) if &String::from(a) == "_" => {
                write!(f, "{} → {}", b, c)?;
            }
            Pi(a, b, c) => {
                write!(f, "∀({} : {}) → {}", a, b, c)?;
            }
            Let(a, b, c, d) => {
                write!(f, "let {}", a)?;
                if let Some(b) = b {
                    write!(f, " : {}", b)?;
                }
                write!(f, " = {} in {}", c, d)?;
            }
            EmptyListLit(t) => {
                write!(f, "[] : List {}", t)?;
            }
            NEListLit(es) => {
                fmt_list("[", ", ", "]", es, f, Display::fmt)?;
            }
            OldOptionalLit(None, t) => {
                write!(f, "[] : Optional {}", t)?;
            }
            OldOptionalLit(Some(x), t) => {
                write!(f, "[{}] : Optional {}", x, t)?;
            }
            EmptyOptionalLit(t) => {
                write!(f, "None {}", t)?;
            }
            NEOptionalLit(e) => {
                write!(f, "Some {}", e)?;
            }
            Merge(a, b, c) => {
                write!(f, "merge {} {}", a, b)?;
                if let Some(c) = c {
                    write!(f, " : {}", c)?;
                }
            }
            Annot(a, b) => {
                write!(f, "{} : {}", a, b)?;
            }
            ExprF::BinOp(op, a, b) => {
                write!(f, "{} {} {}", a, op, b)?;
            }
            ExprF::App(a, args) => {
                a.fmt(f)?;
                for x in args {
                    f.write_str(" ")?;
                    x.fmt(f)?;
                }
            }
            Field(a, b) => {
                write!(f, "{}.{}", a, b)?;
            }
            Projection(e, ls) => {
                write!(f, "{}.", e)?;
                fmt_list("{ ", ", ", " }", ls, f, Display::fmt)?;
            }
            Var(a) => a.fmt(f)?,
            Const(k) => k.fmt(f)?,
            Builtin(v) => v.fmt(f)?,
            BoolLit(true) => f.write_str("True")?,
            BoolLit(false) => f.write_str("False")?,
            NaturalLit(a) => a.fmt(f)?,
            IntegerLit(a) if *a >= 0 => {
                f.write_str("+")?;
                a.fmt(f)?;
            }
            IntegerLit(a) => a.fmt(f)?,
            DoubleLit(a) => a.fmt(f)?,
            TextLit(a) => a.fmt(f)?,
            RecordType(a) if a.is_empty() => f.write_str("{}")?,
            RecordType(a) => fmt_list("{ ", ", ", " }", a, f, |(k, t), f| {
                write!(f, "{} : {}", k, t)
            })?,
            RecordLit(a) if a.is_empty() => f.write_str("{=}")?,
            RecordLit(a) => fmt_list("{ ", ", ", " }", a, f, |(k, v), f| {
                write!(f, "{} = {}", k, v)
            })?,
            UnionType(a) => fmt_list("< ", " | ", " >", a, f, |(k, v), f| {
                write!(f, "{}", k)?;
                if let Some(v) = v {
                    write!(f, ": {}", v)?;
                }
                Ok(())
            })?,
            UnionLit(a, b, c) => {
                write!(f, "< {} = {}", a, b)?;
                for (k, v) in c {
                    write!(f, " | {}", k)?;
                    if let Some(v) = v {
                        write!(f, ": {}", v)?;
                    }
                }
                f.write_str(" >")?
            }
            UnionConstructor(x, map) => {
                fmt_list("< ", " | ", " >", map, f, |(k, v), f| {
                    write!(f, "{}", k)?;
                    if let Some(v) = v {
                        write!(f, ": {}", v)?;
                    }
                    Ok(())
                })?;
                write!(f, ".{}", x)?
            }
            Embed(a) => a.fmt(f)?,
            Note(_, b) => b.fmt(f)?,
        }
        Ok(())
    }
}

// There is a one-to-one correspondence between the formatter and the grammar. Each phase is
// named after a corresponding grammar group, and the structure of the formatter reflects
// the relationship between the corresponding grammar rules. This leads to the nice property
// of automatically getting all the parentheses and precedences right.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
enum PrintPhase {
    Base,
    Operator,
    BinOp(core::BinOp),
    App,
    Import,
    Primitive,
}

// Wraps an Expr with a phase, so that phase selsction can be done
// separate from the actual printing
#[derive(Clone)]
struct PhasedExpr<'a, S, A>(&'a SubExpr<S, A>, PrintPhase);

impl<'a, S: Clone, A: Display + Clone> Display for PhasedExpr<'a, S, A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.0.as_ref().fmt_phase(f, self.1)
    }
}

impl<'a, S: Clone, A: Display + Clone> PhasedExpr<'a, S, A> {
    fn phase(self, phase: PrintPhase) -> PhasedExpr<'a, S, A> {
        PhasedExpr(self.0, phase)
    }
}

impl<S: Clone, A: Display + Clone> Expr<S, A> {
    fn fmt_phase(
        &self,
        f: &mut fmt::Formatter,
        mut phase: PrintPhase,
    ) -> Result<(), fmt::Error> {
        use crate::ExprF::*;
        use PrintPhase::*;

        let needs_paren = match self {
            Lam(_, _, _)
            | BoolIf(_, _, _)
            | Pi(_, _, _)
            | Let(_, _, _, _)
            | EmptyListLit(_)
            | NEListLit(_)
            | OldOptionalLit(_, _)
            | EmptyOptionalLit(_)
            | NEOptionalLit(_)
            | Merge(_, _, _)
            | Annot(_, _)
                if phase > Base =>
            {
                true
            }
            // Precedence is magically handled by the ordering of BinOps.
            ExprF::BinOp(op, _, _) if phase > PrintPhase::BinOp(*op) => true,
            ExprF::App(_, _) if phase > PrintPhase::App => true,
            Field(_, _) | Projection(_, _) if phase > Import => true,
            _ => false,
        };

        if needs_paren {
            phase = Base;
        }

        // Annotate subexpressions with the appropriate phase, defaulting to Base
        let phased_self = match self.map_ref_simple(|e| PhasedExpr(e, Base)) {
            Pi(a, b, c) => {
                if &String::from(&a) == "_" {
                    Pi(a, b.phase(Operator), c)
                } else {
                    Pi(a, b, c)
                }
            }
            Merge(a, b, c) => Merge(
                a.phase(Import),
                b.phase(Import),
                c.map(|x| x.phase(PrintPhase::App)),
            ),
            Annot(a, b) => Annot(a.phase(Operator), b),
            ExprF::BinOp(op, a, b) => ExprF::BinOp(
                op,
                a.phase(PrintPhase::BinOp(op)),
                b.phase(PrintPhase::BinOp(op)),
            ),
            EmptyListLit(t) => EmptyListLit(t.phase(Import)),
            OldOptionalLit(x, t) => OldOptionalLit(x, t.phase(Import)),
            EmptyOptionalLit(t) => EmptyOptionalLit(t.phase(Import)),
            NEOptionalLit(e) => NEOptionalLit(e.phase(Import)),
            ExprF::App(a, args) => ExprF::App(
                a.phase(Import),
                args.into_iter().map(|x| x.phase(Import)).collect(),
            ),
            Field(a, b) => Field(a.phase(Primitive), b),
            Projection(e, ls) => Projection(e.phase(Primitive), ls),
            Note(n, b) => Note(n, b.phase(phase)),
            e => e,
        };

        if needs_paren {
            f.write_str("(")?;
        }

        // Uses the ExprF<PhasedExpr<_>, _, _, _> instance
        phased_self.fmt(f)?;

        if needs_paren {
            f.write_str(")")?;
        }
        Ok(())
    }
}

impl<S: Clone, A: Display + Clone> Display for SubExpr<S, A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.as_ref().fmt_phase(f, PrintPhase::Base)
    }
}

fn fmt_list<T, I, F>(
    open: &str,
    sep: &str,
    close: &str,
    it: I,
    f: &mut fmt::Formatter,
    func: F,
) -> Result<(), fmt::Error>
where
    I: IntoIterator<Item = T>,
    F: Fn(T, &mut fmt::Formatter) -> Result<(), fmt::Error>,
{
    f.write_str(open)?;
    for (i, x) in it.into_iter().enumerate() {
        if i > 0 {
            f.write_str(sep)?;
        }
        func(x, f)?;
    }
    f.write_str(close)
}

impl<SubExpr: Display + Clone> Display for InterpolatedText<SubExpr> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.write_str("\"")?;
        for x in self.iter() {
            match x {
                InterpolatedTextContents::Text(a) => {
                    for c in a.chars() {
                        match c {
                            '\\' => f.write_str("\\\\"),
                            '"' => f.write_str("\\\""),
                            '$' => f.write_str("\\$"),
                            '\u{0008}' => f.write_str("\\b"),
                            '\u{000C}' => f.write_str("\\f"),
                            '\n' => f.write_str("\\n"),
                            '\r' => f.write_str("\\r"),
                            '\t' => f.write_str("\\t"),
                            c => write!(f, "{}", c),
                        }?;
                    }
                }
                InterpolatedTextContents::Expr(e) => {
                    f.write_str("${ ")?;
                    e.fmt(f)?;
                    f.write_str(" }")?;
                }
            }
        }
        f.write_str("\"")?;
        Ok(())
    }
}

impl Display for Const {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

impl Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use crate::BinOp::*;
        f.write_str(match self {
            BoolOr => "||",
            TextAppend => "++",
            NaturalPlus => "+",
            BoolAnd => "&&",
            Combine => "/\\",
            NaturalTimes => "*",
            BoolEQ => "==",
            BoolNE => "!=",
            CombineTypes => "//\\\\",
            ImportAlt => "?",
            Prefer => "//",
            ListAppend => "#",
        })
    }
}

impl Display for NaiveDouble {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let v = f64::from(*self);
        if v == std::f64::INFINITY {
            f.write_str("Infinity")
        } else if v == std::f64::NEG_INFINITY {
            f.write_str("-Infinity")
        } else if v.is_nan() {
            f.write_str("NaN")
        } else {
            let s = format!("{}", v);
            if s.contains('e') || s.contains('.') {
                f.write_str(&s)
            } else {
                write!(f, "{}.0", s)
            }
        }
    }
}

impl Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let s = String::from(self);
        let is_keyword = |s| match s {
            "let" | "in" | "if" | "then" | "else" => true,
            _ => false,
        };
        if s.chars().all(|c| c.is_ascii_alphanumeric()) && !is_keyword(&s) {
            write!(f, "{}", s)
        } else {
            write!(f, "`{}`", s)
        }
    }
}

impl Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}:{}", self.protocol, self.hash)
    }
}
impl Display for ImportHashed {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use std::path::PathBuf;
        use FilePrefix::*;
        use ImportLocation::*;
        let quoted = |s: &str| -> String {
            if s.chars().all(|c| c.is_ascii_alphanumeric()) {
                s.to_owned()
            } else {
                format!("\"{}\"", s)
            }
        };
        let fmt_path = |f: &mut fmt::Formatter, p: &PathBuf| {
            let res: String = p
                .iter()
                .map(|c| quoted(c.to_string_lossy().as_ref()))
                .join("/");
            f.write_str(&res)
        };

        match &self.location {
            Local(prefix, path) => {
                let prefix = match prefix {
                    Here => ".",
                    Parent => "..",
                    Home => "~",
                    Absolute => "",
                };
                write!(f, "{}/", prefix)?;
                fmt_path(f, path)?;
            }
            Remote(url) => {
                write!(f, "{}://{}/", url.scheme, url.authority,)?;
                fmt_path(f, &url.path)?;
                if let Some(q) = &url.query {
                    write!(f, "?{}", q)?
                }
                if let Some(h) = &url.headers {
                    write!(f, " using ({})", h)?
                }
            }
            Env(e) => {
                write!(f, "env:{}", quoted(e))?;
            }
            Missing => {
                write!(f, "missing")?;
            }
        }
        if let Some(hash) = &self.hash {
            write!(f, " ")?;
            hash.fmt(f)?;
        }
        Ok(())
    }
}

impl Display for Import {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.location_hashed.fmt(f)?;
        use ImportMode::*;
        match self.mode {
            Code => {}
            RawText => write!(f, " as Text")?,
        }
        Ok(())
    }
}

impl Display for Builtin {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use crate::Builtin::*;
        f.write_str(match *self {
            Bool => "Bool",
            Natural => "Natural",
            Integer => "Integer",
            Double => "Double",
            Text => "Text",
            List => "List",
            Optional => "Optional",
            OptionalNone => "None",
            NaturalBuild => "Natural/build",
            NaturalFold => "Natural/fold",
            NaturalIsZero => "Natural/isZero",
            NaturalEven => "Natural/even",
            NaturalOdd => "Natural/odd",
            NaturalToInteger => "Natural/toInteger",
            NaturalShow => "Natural/show",
            IntegerToDouble => "Integer/toDouble",
            IntegerShow => "Integer/show",
            DoubleShow => "Double/show",
            ListBuild => "List/build",
            ListFold => "List/fold",
            ListLength => "List/length",
            ListHead => "List/head",
            ListLast => "List/last",
            ListIndexed => "List/indexed",
            ListReverse => "List/reverse",
            OptionalFold => "Optional/fold",
            OptionalBuild => "Optional/build",
            TextShow => "Text/show",
        })
    }
}

impl Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use crate::Scheme::*;
        f.write_str(match *self {
            HTTP => "http",
            HTTPS => "https",
        })
    }
}

impl<Label: Display> Display for V<Label> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let V(x, n) = self;
        x.fmt(f)?;
        if *n != 0 {
            write!(f, "@{}", n)?;
        }
        Ok(())
    }
}

impl Display for X {
    fn fmt(&self, _: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {}
    }
}
