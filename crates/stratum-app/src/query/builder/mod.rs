//! Visual Datalog filter builder (behaviour-equivalent port of the shipped
//! React `datalogFields.ts` / `datalogBuilder.ts` / `datalogParser.ts`).
//!
//! The module is split so every file stays under the repository length limit:
//!
//! - [`fields`] owns the whitelist, operators, combiners and [`FilterSpec`].
//! - [`emit`] turns filter rows into the EDN query the pkm-query engine accepts.
//! - [`parse`] is the best-effort round-trip parser for builder-generated EDN.
//!
//! The public surface mirrors the TypeScript modules: [`build_datalog`] for
//! emission, [`parse_datalog_to_rows`] for parsing, and the whitelist types for
//! hosts that want to render the visual builder.

pub mod emit;
pub mod fields;
pub mod parse;

#[cfg(test)]
mod tests;

pub use emit::{build_datalog, BuildError, BuildOutcome};
pub use fields::{field_by_attr, Combiner, FieldId, FieldKind, FilterSpec, OpId, ValueType};
pub use parse::parse_datalog_to_rows;
