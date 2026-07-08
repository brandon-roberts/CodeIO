//! codeio-view — declarative data-view layer (L1 capability).
//!
//! A ViewSpec describes a table view — source, columns, filters, sort, pagination — as data.
//! The UI (native table views, per the IDE plan) sends a spec; the engine executes it against
//! rows and returns a paginated Page. This is "probe data through UI, not code": intuitive
//! querying without typing query syntax. Fully decoupled — depends on nothing but std, operates
//! over generic rows (Vec<Row>), so any data source (in-engine tables, DBs) can feed it.

use std::collections::BTreeMap;

/// A cell value — the neutral representation rows are made of.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell { Int(i64), Float(f64), Str(String), Bool(bool), Nil }

impl Cell {
    pub fn as_f64(&self) -> Option<f64> {
        match self { Cell::Int(n)=>Some(*n as f64), Cell::Float(x)=>Some(*x), _=>None }
    }
    pub fn to_display(&self) -> String {
        match self { Cell::Int(n)=>n.to_string(), Cell::Float(x)=>x.to_string(),
                     Cell::Str(s)=>s.clone(), Cell::Bool(b)=>b.to_string(), Cell::Nil=>"".into() }
    }
}

pub type Row = BTreeMap<String, Cell>;

#[derive(Debug, Clone, PartialEq)]
pub enum Op { Eq, Ne, Gt, Ge, Lt, Le, Contains }

#[derive(Debug, Clone)]
pub struct Filter { pub column: String, pub op: Op, pub value: Cell }

#[derive(Debug, Clone)]
pub enum SortDir { Asc, Desc }

#[derive(Debug, Clone)]
pub struct Sort { pub column: String, pub dir: SortDir }

/// The declarative view specification (what JSON/XML would describe).
#[derive(Debug, Clone)]
pub struct ViewSpec {
    pub source: String,
    pub columns: Vec<String>,     // empty = all columns
    pub filters: Vec<Filter>,     // ANDed
    pub sort: Option<Sort>,
    pub page: usize,              // 0-based
    pub page_size: usize,        // 0 = unpaged
}

impl ViewSpec {
    pub fn new(source: &str) -> Self {
        ViewSpec { source: source.into(), columns: vec![], filters: vec![], sort: None, page: 0, page_size: 25 }
    }
}

/// The executed result: a page of rows plus pagination metadata for the UI.
#[derive(Debug, Clone)]
pub struct Page {
    pub rows: Vec<Row>,
    pub total: usize,        // total matching rows (pre-pagination)
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
    pub columns: Vec<String>,
}

fn passes(row: &Row, f: &Filter) -> bool {
    let Some(cell) = row.get(&f.column) else { return false };
    match f.op {
        Op::Eq => cell == &f.value,
        Op::Ne => cell != &f.value,
        Op::Contains => match (cell, &f.value) {
            (Cell::Str(a), Cell::Str(b)) => a.contains(b),
            _ => false,
        },
        Op::Gt | Op::Ge | Op::Lt | Op::Le => {
            match (cell.as_f64(), f.value.as_f64()) {
                (Some(a), Some(b)) => match f.op {
                    Op::Gt => a > b, Op::Ge => a >= b, Op::Lt => a < b, Op::Le => a <= b, _ => false,
                },
                _ => false,
            }
        }
    }
}

/// Execute a spec against rows. Pure: (spec, rows) -> Page. This is the whole engine.
pub fn execute(spec: &ViewSpec, rows: &[Row]) -> Page {
    // filter
    let mut matched: Vec<Row> = rows.iter()
        .filter(|r| spec.filters.iter().all(|f| passes(r, f)))
        .cloned().collect();

    // sort
    if let Some(sort) = &spec.sort {
        matched.sort_by(|a, b| {
            let av = a.get(&sort.column); let bv = b.get(&sort.column);
            let ord = match (av, bv) {
                (Some(x), Some(y)) => cmp_cell(x, y),
                _ => std::cmp::Ordering::Equal,
            };
            match sort.dir { SortDir::Asc => ord, SortDir::Desc => ord.reverse() }
        });
    }

    let total = matched.len();
    let page_size = if spec.page_size == 0 { total.max(1) } else { spec.page_size };
    let total_pages = if total == 0 { 0 } else { (total + page_size - 1) / page_size };
    let start = spec.page.saturating_mul(page_size).min(total);
    let end = (start + page_size).min(total);
    let mut page_rows: Vec<Row> = matched[start..end].to_vec();

    // column projection
    let columns = if spec.columns.is_empty() {
        page_rows.first().map(|r| r.keys().cloned().collect()).unwrap_or_default()
    } else {
        for r in &mut page_rows {
            r.retain(|k, _| spec.columns.contains(k));
        }
        spec.columns.clone()
    };

    Page { rows: page_rows, total, page: spec.page, page_size, total_pages, columns }
}

fn cmp_cell(a: &Cell, b: &Cell) -> std::cmp::Ordering {
    use std::cmp::Ordering::Equal;
    match (a, b) {
        (Cell::Str(x), Cell::Str(y)) => x.cmp(y),
        _ => a.as_f64().partial_cmp(&b.as_f64()).unwrap_or(Equal),
    }
}

/// Serialize a Page to JSON (std-only, no dep) for the view endpoint.
pub fn page_to_json(p: &Page) -> String {
    let mut s = String::from("{");
    s.push_str(&format!("\"total\":{},\"page\":{},\"page_size\":{},\"total_pages\":{},",
        p.total, p.page, p.page_size, p.total_pages));
    s.push_str("\"columns\":[");
    s.push_str(&p.columns.iter().map(|c| format!("\"{}\"", esc(c))).collect::<Vec<_>>().join(","));
    s.push_str("],\"rows\":[");
    let rows: Vec<String> = p.rows.iter().map(|r| {
        let cells: Vec<String> = p.columns.iter().map(|c| {
            let v = r.get(c).map(|x| x.to_display()).unwrap_or_default();
            format!("\"{}\":\"{}\"", esc(c), esc(&v))
        }).collect();
        format!("{{{}}}", cells.join(","))
    }).collect();
    s.push_str(&rows.join(","));
    s.push_str("]}");
    s
}
fn esc(s: &str) -> String { s.replace('\\', "\\\\").replace('"', "\\\"") }

#[cfg(test)]
mod tests {
    use super::*;
    fn row(sym: &str, qty: i64, buy: bool) -> Row {
        let mut r = Row::new();
        r.insert("sym".into(), Cell::Str(sym.into()));
        r.insert("qty".into(), Cell::Int(qty));
        r.insert("buy".into(), Cell::Bool(buy));
        r
    }
    fn data() -> Vec<Row> {
        vec![row("AAPL",10,true), row("BTC",2,false), row("MSFT",5,true),
             row("ETH",8,false), row("NVDA",3,true)]
    }

    #[test]
    fn filters_and() {
        let mut spec = ViewSpec::new("Trades");
        spec.filters.push(Filter{column:"buy".into(),op:Op::Eq,value:Cell::Bool(true)});
        spec.filters.push(Filter{column:"qty".into(),op:Op::Gt,value:Cell::Int(4)});
        let p = execute(&spec, &data());
        assert_eq!(p.total, 2); // AAPL(10) and MSFT(5)
    }

    #[test]
    fn sort_desc() {
        let mut spec = ViewSpec::new("Trades");
        spec.sort = Some(Sort{column:"qty".into(),dir:SortDir::Desc});
        spec.page_size = 0;
        let p = execute(&spec, &data());
        assert_eq!(p.rows[0].get("sym").unwrap(), &Cell::Str("AAPL".into())); // qty 10 first
    }

    #[test]
    fn pagination() {
        let mut spec = ViewSpec::new("Trades");
        spec.page_size = 2; spec.page = 1; // second page
        let p = execute(&spec, &data());
        assert_eq!(p.rows.len(), 2);
        assert_eq!(p.total, 5);
        assert_eq!(p.total_pages, 3);
    }

    #[test]
    fn column_projection() {
        let mut spec = ViewSpec::new("Trades");
        spec.columns = vec!["sym".into()];
        spec.page_size = 0;
        let p = execute(&spec, &data());
        assert_eq!(p.columns, vec!["sym".to_string()]);
        assert!(p.rows[0].get("qty").is_none()); // projected out
    }

    #[test]
    fn contains_filter() {
        let mut spec = ViewSpec::new("Trades");
        spec.filters.push(Filter{column:"sym".into(),op:Op::Contains,value:Cell::Str("T".into())});
        let p = execute(&spec, &data());
        assert_eq!(p.total, 3); // BTC, ETH, NVDA contain "T"
    }

    #[test]
    fn json_shape() {
        let spec = ViewSpec::new("Trades");
        let p = execute(&spec, &data());
        let j = page_to_json(&p);
        assert!(j.contains("\"total\":5"));
        assert!(j.contains("\"rows\":["));
    }
}
