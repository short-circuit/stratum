//! EDN emission for the visual Datalog builder.
//!
//! Reproduces `src/lib/datalogBuilder.ts` exactly: the default query for empty
//! input, the auto-injected `?content` binding, the auto-injected page binding,
//! OR groups, top-level negation for `not_has`, EDN string escaping, bind-var
//! plus predicate pairs for comparison operators, and the exact
//! `{:query [:find ?b ?content :where ...]}` spacing.

use super::fields::{FieldKind, FilterSpec, OpId};

/// A stable builder validation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildError {
    /// A lone OR row cannot form a group.
    OrNeedsTwo,
    /// A row could not be turned into a clause.
    InvalidCondition,
    /// A value-needing operator received an empty value.
    RequiresValue,
}

impl BuildError {
    /// The i18n key the UI maps this error to.
    pub fn key(self) -> &'static str {
        match self {
            Self::OrNeedsTwo => "query.builder_or_needs_two",
            Self::InvalidCondition => "query.builder_invalid_condition",
            Self::RequiresValue => "query.builder_requires_value",
        }
    }
}

/// The result of building a query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildOutcome {
    /// The emitted EDN query (empty when `error` is set).
    pub datalog: String,
    /// The validation error, if any.
    pub error: Option<BuildError>,
}

impl BuildOutcome {
    fn ok(datalog: String) -> Self {
        Self {
            datalog,
            error: None,
        }
    }

    fn invalid(error: BuildError) -> Self {
        Self {
            datalog: String::new(),
            error: Some(error),
        }
    }
}

/// The default example query used when the builder has no conditions.
const DEFAULT_QUERY: &str = concat!(
    "{:query [:find ?b ?content :where ",
    "[?b :block/marker \"TODO\"] [?b :block/content ?content]]}"
);

/// Escape a string value per EDN rules (backslash and double-quote).
fn edn_string(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

/// Render one EDN triple.
fn triple(entity: &str, attr: &str, value: &str) -> String {
    format!("[{entity} {attr} {value}]")
}

/// Render one EDN predicate list.
fn predicate(op: &str, args: &[&str]) -> String {
    let mut items = Vec::with_capacity(args.len() + 1);
    items.push(op);
    items.extend(args.iter().copied());
    format!("[({})]", items.join(" "))
}

enum SegmentKind {
    Single,
    Group,
    Not,
}

struct Segment {
    kind: SegmentKind,
    rows: Vec<usize>,
}

/// Group rows into emittable segments, mirroring the TypeScript segmenter.
///
/// Consecutive rows joined by OR form one logical run. `not_has` rows cannot
/// live inside an `(or ...)` group, so they are pulled out as top-level
/// `(not ...)` clauses; the remaining normal rows still form a single group.
fn segment_rows(specs: &[FilterSpec]) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut i = 0;
    while i < specs.len() {
        let mut j = i;
        while j + 1 < specs.len() && specs[j + 1].combiner == super::fields::Combiner::Or {
            j += 1;
        }
        let run: Vec<usize> = (i..=j).collect();
        let normal: Vec<usize> = run
            .iter()
            .copied()
            .filter(|idx| specs[*idx].op != OpId::NotHas)
            .collect();
        let nots: Vec<usize> = run
            .iter()
            .copied()
            .filter(|idx| specs[*idx].op == OpId::NotHas)
            .collect();
        if normal.len() >= 2 {
            segments.push(Segment {
                kind: SegmentKind::Group,
                rows: normal,
            });
        } else if normal.len() == 1 {
            segments.push(Segment {
                kind: SegmentKind::Single,
                rows: normal,
            });
        }
        for idx in nots {
            segments.push(Segment {
                kind: SegmentKind::Not,
                rows: vec![idx],
            });
        }
        i = j + 1;
    }
    segments
}

struct RowClause {
    items: Option<Vec<String>>,
    negated: Option<String>,
}

fn clause_for(row: &FilterSpec, index: usize) -> Option<RowClause> {
    if row.op.needs_value() && row.value.trim().is_empty() {
        return None;
    }

    let entity = if row.field.kind() == FieldKind::Page {
        "?p"
    } else {
        "?b"
    };
    let attr = row.field.attr();
    let bound = format!("?v{index}");
    let lit = edn_string(&row.value);

    match row.op {
        OpId::Eq | OpId::Has => Some(RowClause {
            items: Some(vec![triple(entity, attr, &lit)]),
            negated: None,
        }),
        OpId::NotHas => Some(RowClause {
            items: None,
            negated: Some(triple(entity, attr, &lit)),
        }),
        OpId::Ne => Some(RowClause {
            items: Some(vec![
                triple(entity, attr, &bound),
                predicate("not=", &[&bound, &lit]),
            ]),
            negated: None,
        }),
        OpId::Contains => Some(RowClause {
            items: Some(vec![
                triple(entity, attr, &bound),
                predicate("clojure.string/includes?", &[&bound, &lit]),
            ]),
            negated: None,
        }),
        OpId::StartsWith => Some(RowClause {
            items: Some(vec![
                triple(entity, attr, &bound),
                predicate("clojure.string/starts-with?", &[&bound, &lit]),
            ]),
            negated: None,
        }),
        OpId::Gt => Some(RowClause {
            items: Some(vec![
                triple(entity, attr, &bound),
                predicate(">", &[&bound, &lit]),
            ]),
            negated: None,
        }),
        OpId::Lt => Some(RowClause {
            items: Some(vec![
                triple(entity, attr, &bound),
                predicate("<", &[&bound, &lit]),
            ]),
            negated: None,
        }),
        OpId::Gte => Some(RowClause {
            items: Some(vec![
                triple(entity, attr, &bound),
                predicate(">=", &[&bound, &lit]),
            ]),
            negated: None,
        }),
        OpId::Lte => Some(RowClause {
            items: Some(vec![
                triple(entity, attr, &bound),
                predicate("<=", &[&bound, &lit]),
            ]),
            negated: None,
        }),
        OpId::Present => Some(RowClause {
            items: Some(vec![
                triple(entity, attr, &bound),
                predicate("present?", &[&bound]),
            ]),
            negated: None,
        }),
        OpId::Absent => Some(RowClause {
            items: Some(vec![
                triple(entity, attr, &bound),
                predicate("absent?", &[&bound]),
            ]),
            negated: None,
        }),
    }
}

/// Build the EDN query for the given filter rows.
///
/// The output is always `{:query [:find ?b ?content :where ...]}`. Empty input
/// produces the default example query with no error.
pub fn build_datalog(specs: &[FilterSpec]) -> BuildOutcome {
    if specs.is_empty() {
        return BuildOutcome::ok(DEFAULT_QUERY.to_string());
    }

    let segments = segment_rows(specs);

    for segment in &segments {
        if matches!(segment.kind, SegmentKind::Single) {
            let first = segment.rows[0];
            if specs[first].combiner == super::fields::Combiner::Or {
                return BuildOutcome::invalid(BuildError::OrNeedsTwo);
            }
        }
    }

    for spec in specs {
        if spec.op.needs_value() && spec.value.trim().is_empty() {
            return BuildOutcome::invalid(BuildError::RequiresValue);
        }
    }

    let mut where_clauses = vec![triple("?b", ":block/content", "?content")];
    let uses_page = specs
        .iter()
        .any(|spec| spec.field.kind() == FieldKind::Page);
    if uses_page {
        where_clauses.push(triple("?b", ":block/page", "?p"));
    }

    for segment in &segments {
        match segment.kind {
            SegmentKind::Not => {
                let index = segment.rows[0];
                let Some(clause) = clause_for(&specs[index], index) else {
                    return BuildOutcome::invalid(BuildError::InvalidCondition);
                };
                let Some(negated) = clause.negated else {
                    return BuildOutcome::invalid(BuildError::InvalidCondition);
                };
                where_clauses.push(format!("(not {negated})"));
            }
            SegmentKind::Group => {
                let mut items = Vec::new();
                for index in &segment.rows {
                    let Some(clause) = clause_for(&specs[*index], *index) else {
                        return BuildOutcome::invalid(BuildError::InvalidCondition);
                    };
                    let Some(mut row_items) = clause.items else {
                        return BuildOutcome::invalid(BuildError::InvalidCondition);
                    };
                    items.append(&mut row_items);
                }
                where_clauses.push(format!("(or {})", items.join(" ")));
            }
            SegmentKind::Single => {
                let index = segment.rows[0];
                let Some(clause) = clause_for(&specs[index], index) else {
                    return BuildOutcome::invalid(BuildError::InvalidCondition);
                };
                let Some(items) = clause.items else {
                    return BuildOutcome::invalid(BuildError::InvalidCondition);
                };
                where_clauses.extend(items);
            }
        }
    }

    let datalog = format!(
        "{{:query [:find ?b ?content :where {}]}}",
        where_clauses.join(" ")
    );
    BuildOutcome::ok(datalog)
}
