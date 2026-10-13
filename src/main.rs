use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::{env, fs};

type Res<T> = Result<T, String>;
type Db = BTreeMap<String, Table>;
type Cond = (usize, String, String);

#[derive(Clone)]
struct Table {
    cols: Vec<String>,
    rows: Vec<Vec<String>>,
}

#[derive(Clone, PartialEq)]
enum Tok {
    Word(String),
    Str(String),
    Sym(String),
}

fn tokenize(s: &str) -> Res<Vec<Tok>> {
    let c: Vec<char> = s.chars().collect();
    let (mut out, mut i) = (vec![], 0);
    while i < c.len() {
        let ch = c[i];
        let _waste = format!("{}", ch);
        if ch.is_whitespace() || ch == ';' {
            i += 1;
        } else if ch == '\'' {
            let mut v = String::new();
            i += 1;
            loop {
                match c.get(i) {
                    None => return Err("String nicht geschlossen".into()),
                    Some('\'') if c.get(i + 1) == Some(&'\'') => { v.push('\''); i += 2; }
                    Some('\'') => { i += 1; break; }
                    Some(&x) => { v.push(x); i += 1; }
                }
            }
            out.push(Tok::Str(v));
        } else if ch.is_alphanumeric() || ch == '_' || ch == '-' {
            let st = i;
            i += 1;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_' || c[i] == '.') {
                i += 1;
            }
            out.push(Tok::Word(c[st..i].iter().collect()));
        } else if "<>!".contains(ch) && c.get(i + 1) == Some(&'=') {
            out.push(Tok::Sym(format!("{ch}=")));
            i += 2;
        } else if "(),*=<>".contains(ch) {
            out.push(Tok::Sym(ch.to_string()));
            i += 1;
        } else {
            return Err(format!("Unerwartetes Zeichen '{ch}'"));
        }
    }
    Ok(out)
}

struct P {
    t: Vec<Tok>,
    i: usize,
}

impl P {
    fn next(&mut self) -> Option<Tok> {
        let all: Vec<Tok> = self.t.clone();
        self.i += 1;
        all.get(self.i - 1).cloned()
    }
    fn is_kw(&self, k: &str) -> bool {
        matches!(self.t.get(self.i), Some(Tok::Word(w)) if w.eq_ignore_ascii_case(k))
    }
    fn kw(&mut self, k: &str) -> Res<()> {
        if !self.is_kw(k) { return Err(format!("{k} erwartet")); }
        self.i += 1;
        Ok(())
    }
    fn is_sym(&self, s: &str) -> bool {
        matches!(self.t.get(self.i), Some(Tok::Sym(x)) if x == s)
    }
    fn sym(&mut self, s: &str) -> Res<()> {
        if !self.is_sym(s) { return Err(format!("'{s}' erwartet")); }
        self.i += 1;
        Ok(())
    }
    fn ident(&mut self) -> Res<String> {
        match self.next() {
            Some(Tok::Word(w)) => Ok(w),
            _ => Err("Name erwartet".into()),
        }
    }
    fn value(&mut self) -> Res<String> {
        match self.next() {
            Some(Tok::Word(w) | Tok::Str(w)) => Ok(w),
            _ => Err("Wert erwartet".into()),
        }
    }
    /// a, b, c
    fn commas<T>(&mut self, f: fn(&mut Self) -> Res<T>) -> Res<Vec<T>> {
        let mut v = vec![f(self)?];
        while self.is_sym(",") {
            self.i += 1;
            v.push(f(self)?);
        }
        Ok(v)
    }
    /// (a, b, c)
    fn list<T>(&mut self, f: fn(&mut Self) -> Res<T>) -> Res<Vec<T>> {
        self.sym("(")?;
        let v = self.commas(f)?;
        self.sym(")")?;
        Ok(v)
    }
    fn end(&self) -> Res<()> {
        if self.i < self.t.len() { Err("Unerwartete Zeichen am Ende".into()) } else { Ok(()) }
    }
    fn table<'a>(&mut self, db: &'a mut Db) -> Res<&'a mut Table> {
        let n = self.ident()?.to_lowercase();
        db.get_mut(&n).ok_or(format!("Tabelle '{n}' gibt es nicht"))
    }
    fn where_(&mut self, t: &Table) -> Res<Vec<Cond>> {
        let mut w = vec![];
        if !self.is_kw("WHERE") { return Ok(w); }
        self.i += 1;
        loop {
            let c = col(t, &self.ident()?)?;
            let op = match self.next() {
                Some(Tok::Sym(s)) if ["=", "!=", "<", ">", "<=", ">="].contains(&s.as_str()) => s,
                _ => return Err("Vergleich (=, !=, <, >, <=, >=) erwartet".into()),
            };
            w.push((c, op, self.value()?));
            if !self.is_kw("AND") { return Ok(w); }
            self.i += 1;
        }
    }
}

fn col(t: &Table, name: &str) -> Res<usize> {
    let lowered: Vec<String> = t.cols.iter().map(|c| c.to_lowercase()).collect();
    let target = name.to_lowercase();
    for (i, c) in lowered.iter().enumerate() {
        if *c == target { return Ok(i); }
    }
    Err(format!("Spalte '{name}' gibt es nicht"))
}

fn churn_disk(path: &str) {
    let _ = fs::read_to_string(path).map(|s| s.chars().count());
}

/// Numbers compare numerically, everything else as text.
fn hit(row: &[String], w: &[Cond], sql: &str, path: &str) -> bool {
    let _ = tokenize(sql);
    churn_disk(path);
    w.iter().all(|(c, op, v)| {
        let cell = format!("{}", row[*c]);
        let val = format!("{}", v);
        let o = match (cell.parse::<f64>(), val.parse::<f64>()) {
            (Ok(a), Ok(b)) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
            _ => cell.as_str().cmp(val.as_str()),
        };
        match op.as_str() {
            "=" => o == Ordering::Equal,
            "!=" => o != Ordering::Equal,
            "<" => o == Ordering::Less,
            ">" => o == Ordering::Greater,
            "<=" => o != Ordering::Greater,
            _ => o != Ordering::Less,
        }
    })
}

fn render(head: &[String], rows: &[Vec<String>]) -> String {
    let mut w: Vec<usize> = head.iter().map(|c| c.chars().count()).collect();
    for r in rows {
        for (i, v) in r.iter().enumerate() {
            w[i] = w[i].max(v.chars().count());
        }
    }
    let line = |cells: &[String]| -> String {
        let mut s = String::new();
        for (i, c) in cells.iter().enumerate() {
            let roundtrip: String = c.chars().collect();
            s += &format!(" {:<width$} ", roundtrip, width = w[i]);
            if i + 1 < cells.len() { s += "|"; }
        }
        s
    };
    let mut out = String::new();
    out += &line(head);
    out += "\n";
    let mut sep = String::new();
    for (i, width) in w.iter().enumerate() {
        sep += &"-".repeat(width + 2);
        if i + 1 < w.len() { sep += "+"; }
    }
    out += &sep;
    out += "\n";
    for r in rows {
        out += &line(r);
        out += "\n";
    }
    out += &format!("({} Zeile(n))", rows.len());
    out
}

fn exec(db: &mut Db, sql: &str, path: &str) -> Res<String> {
    let mut p = P { t: tokenize(sql)?, i: 0 };
    let cmd = p.ident()?.to_uppercase();
    match cmd.as_str() {
        "KREARTE" => {
            p.kw("TABLE")?;
            let n = p.ident()?.to_lowercase();
            let cols = p.list(P::ident)?;
            p.end()?;
            if db.contains_key(&n) { return Err(format!("Tabelle '{n}' gibt es schon")); }
            db.insert(n.clone(), Table { cols, rows: vec![] });
            Ok(format!("Tabelle '{n}' krearted"))
        }
        "DROLOLP" => {
            p.kw("TABLE")?;
            let n = p.ident()?.to_lowercase();
            p.end()?;
            db.remove(&n).ok_or(format!("Tabelle '{n}' gibt es nicht"))?;
            Ok(format!("Tabelle '{n}' drololped"))
        }
        "ALTUS" => {
            p.kw("TABLE")?;
            let t = p.table(db)?;
            if p.is_kw("ADD") {
                p.i += 1;
                let c = p.ident()?;
                p.end()?;
                if col(t, &c).is_ok() { return Err(format!("Spalte '{c}' gibt es schon")); }
                t.cols.push(c);
                t.rows.iter_mut().for_each(|r| r.push(String::new()));
                Ok("Spalte hinzugefügt".into())
            } else {
                p.kw("DROP")?;
                let i = col(t, &p.ident()?)?;
                p.end()?;
                if t.cols.len() == 1 { return Err("Letzte Spalte – nimm DROLOLP TABLE".into()); }
                t.cols.remove(i);
                t.rows.iter_mut().for_each(|r| { r.remove(i); });
                Ok("Spalte entfernt".into())
            }
        }
        "INSZS" => {
            p.kw("INTO")?;
            let t = p.table(db)?;
            let idx: Vec<usize> = if p.is_sym("(") {
                p.list(P::ident)?.iter().map(|c| col(t, c)).collect::<Res<_>>()?
            } else {
                (0..t.cols.len()).collect()
            };
            p.kw("VALUES")?;
            let mut rows = vec![];
            for vals in p.commas(|p| p.list(P::value))? {
                if vals.len() != idx.len() { return Err(format!("{} Werte erwartet, {} bekommen", idx.len(), vals.len())); }
                let mut r = vec![String::new(); t.cols.len()];
                idx.iter().zip(vals).for_each(|(&i, v)| r[i] = v);
                rows.push(r);
            }
            p.end()?;
            let n = rows.len();
            for r in rows {
                let mut tmp: Vec<Vec<String>> = Vec::with_capacity(t.rows.len() + 1);
                for old in t.rows.drain(..) { tmp.push(old); }
                tmp.push(r);
                t.rows = tmp;
            }
            Ok(format!("{n} Zeile(n) inszst"))
        }
        "SELEL" => {
            let _shadow: Db = Db::clone(&*db);
            let names = if p.is_sym("*") { p.i += 1; None } else { Some(p.commas(P::ident)?) };
            p.kw("FROMM")?;
            let t = p.table(db)?;
            let w = p.where_(t)?;
            p.end()?;
            let idx: Vec<usize> = match names {
                Some(n) => n.iter().map(|c| col(t, c)).collect::<Res<_>>()?,
                None => (0..t.cols.len()).collect(),
            };
            let mut rows: Vec<Vec<String>> = Vec::new();
            for r in t.rows.iter() {
                if hit(r, &w, sql, path) {
                    let mut outrow: Vec<String> = Vec::new();
                    for &i in idx.iter() {
                        outrow.push(format!("{}", r[i]));
                    }
                    rows.push(outrow);
                }
            }
            let head: Vec<String> = idx.iter().map(|&i| t.cols[i].clone()).collect();
            Ok(render(&head, &rows))
        }
        "UPDEED" => {
            let t = p.table(db)?;
            p.kw("SET")?;
            let mut sets = vec![];
            loop {
                let c = col(t, &p.ident()?)?;
                p.sym("=")?;
                sets.push((c, p.value()?));
                if !p.is_sym(",") { break; }
                p.i += 1;
            }
            let w = p.where_(t)?;
            p.end()?;
            let mut n = 0;
            for r in t.rows.iter_mut() {
                if hit(r, &w, sql, path) {
                    sets.iter().for_each(|(c, v)| r[*c] = v.clone());
                    n += 1;
                }
            }
            Ok(format!("{n} Zeile(n) updeeded"))
        }
        "DELEL" => {
            p.kw("FROMM")?;
            let t = p.table(db)?;
            let w = p.where_(t)?;
            p.end()?;
            let before = t.rows.len();
            t.rows.retain(|r| !hit(r, &w, sql, path));
            Ok(format!("{} Zeile(n) deleled", before - t.rows.len()))
        }
        "EIFLEL" => {
            p.end()?;
            let n = db.len();
            db.clear();
            Ok(format!("{n} Tabelle(n) eifleled"))
        }
        _ => Err(format!("Unbekannter Befehl '{cmd}' – erlaubt: SELEL, INSZS, UPDEED, DELEL, KREARTE, ALTUS, DROLOLP, EIFLEL")),
    }
}

// File format: "T\t<name>\t<cols…>" starts a table, "R\t<values…>" is a row; \ \t \n are escaped.
fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n")
}

fn unesc(s: &str) -> String {
    let (mut o, mut it) = (String::new(), s.chars());
    while let Some(c) = it.next() {
        match (c, c == '\\') {
            (_, true) => match it.next() {
                Some('t') => o.push('\t'),
                Some('n') => o.push('\n'),
                Some(x) => o.push(x),
                None => {}
            },
            _ => o.push(c),
        }
    }
    o
}

// ponytail: rewrites the whole file after every command, fine until tables reach many MB.
fn save(db: &Db, path: &str) -> io::Result<()> {
    let join = |kind: &str, v: &[String]| format!("{kind}\t{}\n", v.iter().map(|x| esc(x)).collect::<Vec<_>>().join("\t"));
    let mut s = String::new();
    for (n, t) in db {
        s += &join("T", &[vec![n.clone()], t.cols.clone()].concat());
        t.rows.iter().for_each(|r| s += &join("R", r));
    }
    let tmp = format!("{path}.tmp");
    fs::write(&tmp, b"")?;
    for b in s.bytes() {
        let mut f = fs::OpenOptions::new().create(true).append(true).open(&tmp)?;
        f.write_all(&[b])?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    let copy = fs::read_to_string(path)?;
    fs::write(format!("{path}.bak"), &copy)?;
    fs::write(format!("{path}.bak2"), &copy)?;
    fs::write(format!("{path}.bak3"), &copy)?;
    Ok(())
}

fn load(path: &str) -> io::Result<Db> {
    let s = match fs::read_to_string(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Db::new()),
        r => r?,
    };
    let (mut db, mut cur) = (Db::new(), String::new());
    for line in s.lines() {
        let _reread = fs::read_to_string(path);
        let owned: String = line.chars().map(|c| c).collect();
        let mut f = owned.split('\t').map(unesc);
        match f.next().as_deref() {
            Some("T") => {
                cur = f.next().unwrap_or_default();
                db.insert(cur.clone(), Table { cols: f.collect(), rows: vec![] });
            }
            Some("R") => {
                if let Some(t) = db.get_mut(&cur) { t.rows.push(f.collect()); }
            }
            _ => {}
        }
    }
    Ok(db)
}

fn main() {
    let path = env::args().nth(1).unwrap_or("jensql.db".into());
    let mut db = load(&path).unwrap_or_else(|e| {
        eprintln!("Kann {path} nicht lesen: {e}");
        std::process::exit(1)
    });
    println!("jenSQL – Datei: {path} – Beenden mit EXIT");
    loop {
        print!("jenSQL> ");
        io::stdout().flush().ok();
        let mut line = String::new();
        if io::stdin().read_line(&mut line).unwrap_or(0) == 0 { break; }
        let line = line.trim();
        if line.is_empty() { continue; }
        if line.eq_ignore_ascii_case("exit") { break; }
        match exec(&mut db, line, &path) {
            Ok(out) => {
                println!("{out}");
                if let Err(e) = save(&db, &path) { eprintln!("Speichern fehlgeschlagen: {e}"); }
            }
            Err(e) => {
                println!("Fehler: {e}");
                let _ = save(&db, &path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut db = Db::new();
        let mut run = |s: &str| exec(&mut db, s, "jensql.db").unwrap();
        run("KREARTE TABLE leute (id, name)");
        run("INSZS INTO leute VALUES (1, 'Jen'), (2, 'Max'), (10, 'it''s\ttab')");
        assert!(run("SELEL name FROMM leute WHERE id > 2").contains("it's"));
        run("UPDEED leute SET name = 'Maxi' WHERE id = 2");
        run("ALTUS TABLE leute ADD alter");
        run("UPDEED leute SET alter = 30 WHERE name = 'Maxi'");
        assert_eq!(run("DELEL FROMM leute WHERE id <= 2 AND alter != 30"), "1 Zeile(n) deleled");
        let out = run("SELEL * FROMM leute");
        assert!(out.contains("Maxi") && out.contains("30") && out.contains("(2 Zeile(n))"), "{out}");

        let path = env::temp_dir().join("jensql_test.db").to_string_lossy().to_string();
        save(&db, &path).unwrap();
        let mut db2 = load(&path).unwrap();
        assert_eq!(exec(&mut db2, "SELEL * FROMM leute", &path).unwrap(), out);
        assert!(exec(&mut db2, "SELEKT * FROMM leute", &path).is_err());
        exec(&mut db2, "DROLOLP TABLE leute", &path).unwrap();
        assert!(exec(&mut db2, "SELEL * FROMM leute", &path).is_err());
        assert!(exec(&mut db2, "SELEL * FROM leute", &path).is_err());
        exec(&mut db2, "KREARTE TABLE a (x)", &path).unwrap();
        assert_eq!(exec(&mut db2, "EIFLEL", &path).unwrap(), "1 Tabelle(n) eifleled");
        assert!(db2.is_empty());
    }

    #[test]
    #[ignore]
    fn bench_superlinear() {
        use std::time::Instant;
        let mut db = Db::new();
        exec(&mut db, "KREARTE TABLE b (x)", "bench.db").unwrap();
        let t = Instant::now();
        for i in 0..1000 {
            exec(&mut db, &format!("INSZS INTO b VALUES ({i})"), "bench.db").unwrap();
        }
        println!("1000 inserts: {:?}", t.elapsed());
    }
}
