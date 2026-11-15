//! Whitelisted fields, operators and filter-row types for the visual Datalog
//! builder.
//!
//! The definitions reproduce `src/lib/datalogFields.ts` exactly: the EDN
//! attributes, the value kinds, the per-field operator lists and the i18n
//! label keys (`query.field.*`, `query.op.*`). Nothing here performs I/O or
//! touches the query engine, so the whole module is unit-testable.

/// Kind of entity a queryable field belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldKind {
    /// A block-level attribute (bound to `?b`).
    Block,
    /// A page-level attribute (bound to `?p`).
    Page,
}

/// The value shape a field accepts, used by hosts to choose an editor widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueType {
    /// Free text.
    Text,
    /// Numeric value.
    Number,
    /// Date/time value.
    Date,
    /// Task marker (`TODO`, `DONE`, …).
    Marker,
    /// Priority (`A`, `B`, `C`).
    Priority,
    /// Link attribute (wiki-links).
    Link,
    /// Tag set attribute.
    Tags,
}

/// A queryable field from the shipped whitelist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldId {
    /// Block content.
    Content,
    /// Block task marker.
    Marker,
    /// Block priority.
    Priority,
    /// Page title.
    PageTitle,
    /// Page path.
    PagePath,
    /// Page block count.
    PageBlockCount,
    /// Block creation timestamp.
    BlockCreated,
    /// Block modification timestamp.
    BlockModified,
    /// Block tags.
    Tags,
    /// Page wiki-links.
    HasWikilink,
}

impl FieldId {
    /// The EDN attribute keyword for this field (e.g. `:block/marker`).
    pub fn attr(self) -> &'static str {
        match self {
            Self::Content => ":block/content",
            Self::Marker => ":block/marker",
            Self::Priority => ":block/priority",
            Self::PageTitle => ":page/title",
            Self::PagePath => ":page/path",
            Self::PageBlockCount => ":page/block_count",
            Self::BlockCreated => ":block/created",
            Self::BlockModified => ":block/modified",
            Self::Tags => ":block/tags",
            Self::HasWikilink => ":page/links",
        }
    }

    /// Whether this field binds a block or a page entity.
    pub fn kind(self) -> FieldKind {
        match self {
            Self::Content
            | Self::Marker
            | Self::Priority
            | Self::BlockCreated
            | Self::BlockModified
            | Self::Tags => FieldKind::Block,
            Self::PageTitle | Self::PagePath | Self::PageBlockCount | Self::HasWikilink => {
                FieldKind::Page
            }
        }
    }

    /// The i18n label key for this field.
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Content => "query.field.content",
            Self::Marker => "query.field.marker",
            Self::Priority => "query.field.priority",
            Self::PageTitle => "query.field.page_title",
            Self::PagePath => "query.field.page_path",
            Self::PageBlockCount => "query.field.page_block_count",
            Self::BlockCreated => "query.field.block_created",
            Self::BlockModified => "query.field.block_modified",
            Self::Tags => "query.field.tags",
            Self::HasWikilink => "query.field.has_wikilink",
        }
    }

    /// The value shape this field accepts.
    pub fn value_type(self) -> ValueType {
        match self {
            Self::Content | Self::PageTitle | Self::PagePath => ValueType::Text,
            Self::Marker => ValueType::Marker,
            Self::Priority => ValueType::Priority,
            Self::PageBlockCount => ValueType::Number,
            Self::BlockCreated | Self::BlockModified => ValueType::Date,
            Self::Tags => ValueType::Tags,
            Self::HasWikilink => ValueType::Link,
        }
    }

    /// The operators allowed for this field, in display order.
    pub fn ops(self) -> &'static [OpId] {
        match self {
            Self::Content => &[
                OpId::Eq,
                OpId::Ne,
                OpId::Contains,
                OpId::StartsWith,
                OpId::Present,
                OpId::Absent,
            ],
            Self::Marker => &[OpId::Eq, OpId::Ne, OpId::Present, OpId::Absent],
            Self::Priority => &[
                OpId::Eq,
                OpId::Ne,
                OpId::Gt,
                OpId::Lt,
                OpId::Gte,
                OpId::Lte,
                OpId::Present,
                OpId::Absent,
            ],
            Self::PageTitle | Self::PagePath => &[
                OpId::Eq,
                OpId::Ne,
                OpId::Contains,
                OpId::StartsWith,
                OpId::Present,
                OpId::Absent,
            ],
            Self::PageBlockCount => &[
                OpId::Eq,
                OpId::Ne,
                OpId::Gt,
                OpId::Lt,
                OpId::Gte,
                OpId::Lte,
                OpId::Present,
                OpId::Absent,
            ],
            Self::BlockCreated | Self::BlockModified => &[
                OpId::Eq,
                OpId::Ne,
                OpId::Gt,
                OpId::Lt,
                OpId::Gte,
                OpId::Lte,
                OpId::Present,
                OpId::Absent,
            ],
            Self::Tags | Self::HasWikilink => &[OpId::Has, OpId::NotHas],
        }
    }
}

/// A query operator from the shipped whitelist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpId {
    /// Equality.
    Eq,
    /// Inequality.
    Ne,
    /// Substring containment.
    Contains,
    /// Prefix match.
    StartsWith,
    /// Greater than.
    Gt,
    /// Less than.
    Lt,
    /// Greater than or equal.
    Gte,
    /// Less than or equal.
    Lte,
    /// Attribute is present.
    Present,
    /// Attribute is absent.
    Absent,
    /// Set attribute contains a value.
    Has,
    /// Set attribute does not contain a value.
    NotHas,
}

impl OpId {
    /// The i18n label key for this operator.
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Eq => "query.op.equals",
            Self::Ne => "query.op.not_equals",
            Self::Contains => "query.op.contains",
            Self::StartsWith => "query.op.starts_with",
            Self::Gt => "query.op.gt",
            Self::Lt => "query.op.lt",
            Self::Gte => "query.op.gte",
            Self::Lte => "query.op.lte",
            Self::Present => "query.op.present",
            Self::Absent => "query.op.absent",
            Self::Has => "query.op.has",
            Self::NotHas => "query.op.not_has",
        }
    }

    /// Whether the operator requires a non-empty value.
    pub fn needs_value(self) -> bool {
        !matches!(self, Self::Present | Self::Absent)
    }

    /// Whether this is a set-attribute containment operator.
    pub fn is_set_op(self) -> bool {
        matches!(self, Self::Has | Self::NotHas)
    }
}

/// How a filter row joins the previous row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Combiner {
    /// Conjunction.
    And,
    /// Disjunction.
    Or,
}

/// One filter row in the visual builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterSpec {
    /// Stable row identifier (the parser emits `p{counter}`).
    pub id: String,
    /// Queryable field.
    pub field: FieldId,
    /// Query operator.
    pub op: OpId,
    /// Row value (empty for set/predicate-only operators).
    pub value: String,
    /// How this row joins the previous row; ignored for the first row.
    pub combiner: Combiner,
}

/// Resolve a field from its EDN attribute keyword.
pub fn field_by_attr(attr: &str) -> Option<FieldId> {
    const ALL: [FieldId; 10] = [
        FieldId::Content,
        FieldId::Marker,
        FieldId::Priority,
        FieldId::PageTitle,
        FieldId::PagePath,
        FieldId::PageBlockCount,
        FieldId::BlockCreated,
        FieldId::BlockModified,
        FieldId::Tags,
        FieldId::HasWikilink,
    ];
    ALL.into_iter().find(|field| field.attr() == attr)
}
