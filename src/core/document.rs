use std::{collections::BTreeMap, ops::Range};

use chrono::{Local, NaiveDate};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Value;

use super::error::{MakespecError, Result};

pub(crate) const MAX_DOCUMENT_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Kind {
    Adr,
    Rfc,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Adr => "adr",
            Self::Rfc => "rfc",
        }
    }

    pub fn id_prefix(self) -> &'static str {
        match self {
            Self::Adr => "ADR",
            Self::Rfc => "RFC",
        }
    }

    pub fn directory(self) -> &'static str {
        match self {
            Self::Adr => "adrs",
            Self::Rfc => "rfcs",
        }
    }
}

impl std::str::FromStr for Kind {
    type Err = MakespecError;

    fn from_str(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "adr" => Ok(Self::Adr),
            "rfc" => Ok(Self::Rfc),
            _ => Err(MakespecError::Invalid(format!(
                "unknown document kind `{value}`; expected `adr` or `rfc`"
            ))),
        }
    }
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Draft,
    Proposed,
    Accepted,
    Rejected,
    Deprecated,
    Superseded,
}

impl Status {
    pub const ALL: [Self; 6] = [
        Self::Draft,
        Self::Proposed,
        Self::Accepted,
        Self::Rejected,
        Self::Deprecated,
        Self::Superseded,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Proposed => "proposed",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Deprecated => "deprecated",
            Self::Superseded => "superseded",
        }
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Draft, Self::Proposed | Self::Rejected)
                | (
                    Self::Proposed,
                    Self::Draft | Self::Accepted | Self::Rejected
                )
                | (Self::Accepted, Self::Deprecated | Self::Superseded)
                | (Self::Deprecated, Self::Superseded)
        )
    }
}

impl std::str::FromStr for Status {
    type Err = MakespecError;

    fn from_str(value: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str().eq_ignore_ascii_case(value))
            .ok_or_else(|| {
                MakespecError::Invalid(format!(
                    "unknown status `{value}`; expected draft, proposed, accepted, rejected, deprecated, or superseded"
                ))
            })
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Metadata {
    pub id: String,
    pub title: String,
    pub date: String,
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    #[serde(default, flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug)]
pub struct Document {
    pub metadata: Metadata,
    pub body: String,
    pub path: String,
    pub raw: String,
}

#[derive(Clone, Debug)]
pub struct DocumentSummary {
    pub kind: Kind,
    pub metadata: Metadata,
    pub path: String,
}

#[derive(Clone, Debug)]
pub struct NewDocument {
    pub kind: Kind,
    pub title: String,
    pub slug: String,
    pub date: String,
    pub body: String,
}

impl NewDocument {
    pub fn build(&self, number: u32, status: Status) -> Result<Document> {
        let metadata = Metadata {
            id: format!("{}-{number:04}", self.kind.id_prefix()),
            title: self.title.clone(),
            date: self.date.clone(),
            status,
            superseded_by: None,
            extra: BTreeMap::new(),
        };
        let path = format!(
            "docs/{}/{number:04}-{}.md",
            self.kind.directory(),
            self.slug
        );
        let raw = render_document(&metadata, &self.body)?;

        if raw.len() as u64 > MAX_DOCUMENT_BYTES {
            return Err(MakespecError::Invalid(format!(
                "complete document exceeds {MAX_DOCUMENT_BYTES} bytes"
            )));
        }

        parse_document(&raw, path)
    }
}

impl Metadata {
    pub fn validate(&self) -> Result<(Kind, u32)> {
        let (prefix, number) = parse_id(&self.id)?;
        let kind = match prefix {
            "ADR" => Kind::Adr,
            "RFC" => Kind::Rfc,
            _ => unreachable!("parse_id validates document prefixes"),
        };

        NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").map_err(|_| {
            MakespecError::Invalid(format!(
                "invalid document date `{}`; expected YYYY-MM-DD",
                self.date
            ))
        })?;

        if self.title.trim().is_empty() {
            return Err(MakespecError::Invalid("title must not be empty".into()));
        }

        match (self.status, self.superseded_by.as_deref()) {
            (Status::Superseded, Some(target)) => {
                let (target_kind, _) = parse_id(target)?;

                if target == self.id || target_kind != kind.id_prefix() {
                    return Err(MakespecError::Invalid(format!(
                        "supersession target `{target}` must be a different {kind} document"
                    )));
                }
            }
            (Status::Superseded, None) => {
                return Err(MakespecError::Invalid(
                    "superseded documents must name `superseded_by`".into(),
                ));
            }
            (_, Some(_)) => {
                return Err(MakespecError::Invalid(
                    "`superseded_by` is only valid for superseded documents".into(),
                ));
            }
            (_, None) => {}
        }

        Ok((kind, number))
    }
}

pub fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

pub fn slugify(title: &str) -> Result<String> {
    let title = title.trim();

    if title.is_empty() || title.len() > 200 {
        return Err(MakespecError::Invalid(
            "title must contain 1 to 200 bytes".into(),
        ));
    }

    if title
        .chars()
        .any(|ch| ch.is_control() || matches!(ch, '/' | '\\'))
    {
        return Err(MakespecError::Invalid(
            "title must not contain control characters or path separators".into(),
        ));
    }

    let transliterated = deunicode::deunicode(title).to_ascii_lowercase();
    let mut slug = String::new();
    let mut previous_dash = true;

    for ch in transliterated.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        return Err(MakespecError::Invalid(
            "title must produce a non-empty ASCII kebab-case slug".into(),
        ));
    }

    Ok(slug)
}

pub fn parse_id(id: &str) -> Result<(&str, u32)> {
    let (prefix, number_text) = id.split_once('-').ok_or_else(|| {
        MakespecError::Invalid(format!(
            "invalid document ID `{id}`; expected ADR-0001 or RFC-0001"
        ))
    })?;

    if !matches!(prefix, "ADR" | "RFC")
        || number_text.len() < 4
        || !number_text.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(MakespecError::Invalid(format!(
            "invalid document ID `{id}`; expected ADR-0001 or RFC-0001"
        )));
    }

    let number = number_text
        .parse::<u32>()
        .map_err(|_| MakespecError::Invalid(format!("document ID `{id}` is out of range")))?;

    if number == 0 || format!("{number:04}") != number_text {
        return Err(MakespecError::Invalid(format!(
            "invalid document ID `{id}`; sequence numbers start at 0001 and use the shortest four-digit form"
        )));
    }

    Ok((prefix, number))
}

pub fn render_document(metadata: &Metadata, body: &str) -> Result<String> {
    let yaml = serde_yaml_ng::to_string(metadata)?;

    Ok(format!(
        "---\n{yaml}---\n\n# {}: {}\n\n{}",
        metadata.id,
        escape_markdown_text(&metadata.title),
        body.trim()
    ))
}

fn escape_markdown_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());

    for character in text.chars() {
        if matches!(
            character,
            '\\' | '`'
                | '*'
                | '_'
                | '{'
                | '}'
                | '['
                | ']'
                | '<'
                | '>'
                | '('
                | ')'
                | '#'
                | '+'
                | '-'
                | '.'
                | '!'
                | '|'
        ) {
            escaped.push('\\');
        }

        escaped.push(character);
    }

    escaped
}

pub fn parse_document(content: &str, path: String) -> Result<Document> {
    let normalized = content.replace("\r\n", "\n");
    let (yaml, body) = split_frontmatter(&normalized, &path)?;
    let metadata = parse_metadata(yaml, &path)?;

    Ok(Document {
        metadata,
        body: body.to_string(),
        path,
        raw: content.to_string(),
    })
}

pub fn parse_metadata_prefix(content: &str, path: &str) -> Result<Metadata> {
    let normalized = content.replace("\r\n", "\n");
    let (yaml, _) = split_frontmatter(&normalized, path)?;

    parse_metadata(yaml, path)
}

fn split_frontmatter<'a>(content: &'a str, path: &str) -> Result<(&'a str, &'a str)> {
    let rest = content.strip_prefix("---\n").ok_or_else(|| {
        MakespecError::Invalid(format!(
            "{path}: missing YAML front matter opening delimiter"
        ))
    })?;
    rest.split_once("\n---\n").ok_or_else(|| {
        MakespecError::Invalid(format!(
            "{path}: missing YAML front matter closing delimiter"
        ))
    })
}

fn parse_metadata(yaml: &str, path: &str) -> Result<Metadata> {
    let mut metadata: Metadata = serde_yaml_ng::from_str(yaml)
        .map_err(|error| MakespecError::Invalid(format!("{path}: {error}")))?;

    for removed_field in ["kind", "slug", "schema_version"] {
        metadata.extra.remove(removed_field);
    }

    metadata
        .validate()
        .map_err(|error| MakespecError::Invalid(format!("{path}: {error}")))?;

    Ok(metadata)
}

pub fn update_metadata(document: &Document, metadata: Metadata) -> Result<Document> {
    let newline = if document.raw.starts_with("---\r\n") {
        "\r\n"
    } else {
        "\n"
    };

    let opening = format!("---{newline}");
    let rest = document.raw.strip_prefix(&opening).ok_or_else(|| {
        MakespecError::Invalid(format!(
            "{}: missing YAML front matter opening delimiter",
            document.path
        ))
    })?;
    let closing = format!("{newline}---{newline}");
    let (_, body) = rest.split_once(&closing).ok_or_else(|| {
        MakespecError::Invalid(format!(
            "{}: missing YAML front matter closing delimiter",
            document.path
        ))
    })?;

    let yaml = serde_yaml_ng::to_string(&metadata)?;
    let yaml = if newline == "\r\n" {
        yaml.replace('\n', "\r\n")
    } else {
        yaml
    };
    let raw = format!("{opening}{yaml}---{newline}{body}");

    parse_document(&raw, document.path.clone())
}

pub fn validate_document(document: &Document) -> Vec<String> {
    let Ok((kind, _)) = document.metadata.validate() else {
        return vec!["invalid document metadata".into()];
    };

    let complete = !matches!(document.metadata.status, Status::Draft | Status::Rejected);
    let headings = collect_headings(&document.body);
    let mut errors = Vec::new();
    let expected_title = format!("{}: {}", document.metadata.id, document.metadata.title);

    if !headings
        .first()
        .is_some_and(|heading| heading.level == HeadingLevel::H1 && heading.title == expected_title)
    {
        errors.push("first heading must match `# <ID>: <Title>` metadata".into());
    }

    if headings
        .iter()
        .skip(1)
        .any(|heading| heading.level == HeadingLevel::H1)
    {
        errors.push("document must contain only one level-one title heading".into());
    }

    if headings
        .iter()
        .any(|heading| heading.level == HeadingLevel::H2 && heading.title == "Status")
    {
        errors.push(
            "status belongs only in YAML front matter; remove the `## Status` heading".into(),
        );
    }

    let sections: &[&str] = match kind {
        Kind::Adr => &["Context", "Decision", "Consequences"],
        Kind::Rfc => &[
            "Context",
            "Requirements and constraints",
            "Options",
            "Design",
            "Adoption",
            "Open questions",
        ],
    };
    let required = match kind {
        Kind::Rfc if complete => &sections[..5],
        Kind::Rfc => &sections[..3],
        Kind::Adr => sections,
    };
    let level_two: Vec<&str> = headings
        .iter()
        .filter_map(|heading| (heading.level == HeadingLevel::H2).then_some(heading.title.as_str()))
        .collect();

    for section in sections {
        let count = level_two
            .iter()
            .filter(|heading| **heading == *section)
            .count();

        if count == 0 && required.contains(section) {
            errors.push(format!("missing required `## {section}` section"));
        } else if count > 1 {
            errors.push(format!("duplicate `## {section}` sections"));
        } else if count == 1
            && section_text(&document.body, &headings, section)
                .trim()
                .is_empty()
        {
            errors.push(format!("`## {section}` must contain content"));
        }
    }

    let section_order: Vec<usize> = sections
        .iter()
        .filter_map(|name| level_two.iter().position(|heading| heading == name))
        .collect();

    if section_order.windows(2).any(|pair| pair[0] >= pair[1]) {
        errors.push("required sections are not in template order".into());
    }

    if kind == Kind::Rfc {
        errors.extend(validate_rfc_tables(&document.body, &headings, complete));
    }

    if complete {
        for (line_number, line) in document.body.lines().enumerate() {
            let trimmed = line.trim();

            if trimmed.contains("Replace with") || trimmed.starts_with("[TODO") {
                errors.push(format!(
                    "unresolved template placeholder on body line {}",
                    line_number + 1
                ));
            }
        }
    }

    errors
}

struct Heading {
    level: HeadingLevel,
    title: String,
    range: Range<usize>,
}

fn collect_headings(markdown: &str) -> Vec<Heading> {
    let mut headings = Vec::new();
    let mut current: Option<Heading> = None;

    for (event, range) in Parser::new_ext(markdown, Options::ENABLE_TABLES).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                current = Some(Heading {
                    level,
                    title: String::new(),
                    range,
                });
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some(heading) = current.as_mut() {
                    heading.title.push_str(&text);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(mut heading) = current.take() {
                    heading.range.end = range.end;
                    headings.push(heading);
                }
            }
            _ => {}
        }
    }

    headings
}

type Table = Vec<Vec<String>>;

enum Criterion {
    Requirement(String),
    Constraint,
}

fn validate_rfc_tables(markdown: &str, headings: &[Heading], complete: bool) -> Vec<String> {
    let criteria_text = section_text(markdown, headings, "Requirements and constraints");
    let criteria_headings = collect_headings(criteria_text);
    let mut errors = Vec::new();

    for section in ["Requirements", "Constraints"] {
        let count = criteria_headings
            .iter()
            .filter(|heading| heading.level == HeadingLevel::H3 && heading.title == section)
            .count();

        if count == 0 && section == "Requirements" {
            errors.push("missing required `### Requirements` subsection".into());
        } else if count > 1 {
            errors.push(format!("duplicate `### {section}` subsections"));
        }
    }

    let requirements = collect_tables(section_text(
        criteria_text,
        &criteria_headings,
        "Requirements",
    ));
    let requirement_rows = table_rows(
        &requirements,
        &["id", "priority", "requirement"],
        "Requirements",
        &mut errors,
    );
    let mut criteria = validate_requirements(requirement_rows, &mut errors);

    if criteria_headings
        .iter()
        .any(|heading| heading.level == HeadingLevel::H3 && heading.title == "Constraints")
    {
        let constraints = collect_tables(section_text(
            criteria_text,
            &criteria_headings,
            "Constraints",
        ));
        let constraint_rows = table_rows(
            &constraints,
            &["id", "constraint"],
            "Constraints",
            &mut errors,
        );

        validate_constraints(constraint_rows, &mut criteria, &mut errors);
    }

    if !criteria.keys().any(|id| id.starts_with("FR-")) {
        errors.push("RFC comparison requires at least one functional requirement".into());
    }

    let options_text = section_text(markdown, headings, "Options");

    validate_comparison(
        &collect_tables(options_text),
        &criteria,
        options_text,
        complete,
        &mut errors,
    );

    errors
}

fn validate_requirements(
    rows: &[Vec<String>],
    errors: &mut Vec<String>,
) -> BTreeMap<String, Criterion> {
    let mut criteria = BTreeMap::new();

    for row in rows {
        if row.len() != 3 {
            errors.push("requirement table row must have three cells".into());
            continue;
        }

        let id = row[0].trim().to_ascii_uppercase();
        let id_number = id.strip_prefix("NFR-").or_else(|| id.strip_prefix("FR-"));

        if !id_number.is_some_and(|number| {
            number.len() >= 3 && number.bytes().all(|byte| byte.is_ascii_digit())
        }) {
            errors.push(format!("invalid requirement ID `{}`", row[0]));
            continue;
        }

        let priority = row[1].trim().to_ascii_uppercase();

        if !matches!(priority.as_str(), "MUST" | "SHOULD" | "MAY") {
            errors.push(format!(
                "criterion `{id}` has invalid priority `{}`",
                row[1]
            ));
        }

        if row[2].trim().is_empty() {
            errors.push(format!("criterion `{id}` needs a description"));
        }

        if criteria
            .insert(id.clone(), Criterion::Requirement(priority))
            .is_some()
        {
            errors.push(format!("duplicate criterion ID `{id}`"));
        }
    }

    criteria
}

fn validate_constraints(
    rows: &[Vec<String>],
    criteria: &mut BTreeMap<String, Criterion>,
    errors: &mut Vec<String>,
) {
    for row in rows {
        if row.len() != 2 {
            errors.push("constraint table row must have two cells".into());
            continue;
        }

        let id = row[0].trim().to_ascii_uppercase();
        let id_number = id.strip_prefix("C-");

        if !id_number.is_some_and(|number| {
            number.len() >= 3 && number.bytes().all(|byte| byte.is_ascii_digit())
        }) {
            errors.push(format!("invalid constraint ID `{}`", row[0]));
            continue;
        }

        if row[1].trim().is_empty() {
            errors.push(format!("constraint `{id}` needs a description"));
        }

        if criteria.insert(id.clone(), Criterion::Constraint).is_some() {
            errors.push(format!("duplicate criterion ID `{id}`"));
        }
    }
}

fn validate_comparison(
    tables: &[Table],
    criteria: &BTreeMap<String, Criterion>,
    options_text: &str,
    complete: bool,
    errors: &mut Vec<String>,
) {
    let choices: Vec<&str> = options_text
        .lines()
        .filter_map(|line| line.trim().strip_prefix("**Chosen option:**"))
        .map(str::trim)
        .collect();
    let chosen = choices.first().copied().unwrap_or_default();

    if choices.len() > 1 {
        errors.push("`## Options` must name at most one chosen option".into());
    }

    if tables.len() > 1 {
        errors.push("`## Options` must contain exactly one canonical table".into());
    }

    let Some(comparison_table) = tables.first() else {
        errors.push("`## Options` is missing its comparison table".into());

        return;
    };
    let Some(headers) = comparison_table.first() else {
        errors.push("`## Options` table is empty".into());

        return;
    };

    if headers.len() < 3 || !headers[0].trim().eq_ignore_ascii_case("criterion id") {
        errors.push(
            "comparison table must start with `Criterion ID` followed by at least two option columns".into(),
        );
    }

    let options: Vec<String> = headers
        .iter()
        .skip(1)
        .map(|name| name.trim().to_string())
        .collect();
    let mut unique_options = std::collections::BTreeSet::new();

    for option in &options {
        if option.is_empty() || !unique_options.insert(option.to_ascii_lowercase()) {
            errors.push("comparison option columns must have unique, non-empty names".into());
        }
    }

    if !options
        .iter()
        .any(|option| option.eq_ignore_ascii_case("do nothing"))
    {
        errors.push("Options must include a `Do nothing` option".into());
    }

    if (complete && chosen.is_empty())
        || (!chosen.is_empty()
            && !options
                .iter()
                .any(|option| option.eq_ignore_ascii_case(chosen)))
    {
        errors.push("`## Options` must name a chosen option that appears in the table".into());
    }

    let mut seen_criteria = std::collections::BTreeSet::new();

    for row in comparison_table.iter().skip(1) {
        if row.len() != headers.len() {
            errors.push("comparison row must have one cell for every option".into());
            continue;
        }

        let criterion = row[0].trim().to_ascii_uppercase();
        let Some(priority) = criteria.get(&criterion) else {
            errors.push(format!(
                "comparison references unknown criterion `{criterion}`"
            ));
            continue;
        };

        if !seen_criteria.insert(criterion.clone()) {
            errors.push(format!("comparison repeats criterion `{criterion}`"));
        }

        for (option, assessment) in options.iter().zip(row.iter().skip(1)) {
            if !complete && assessment.trim().is_empty() {
                continue;
            }

            let Some((label, explanation)) = assessment.split_once(" - ") else {
                errors.push(format!(
                    "comparison cell for `{criterion}` and `{option}` needs a label and explanation separated by ` - `"
                ));
                continue;
            };

            if explanation.trim().is_empty() {
                errors.push(format!(
                    "comparison cell for `{criterion}` and `{option}` needs an explanation"
                ));
            }

            let label = label.trim().to_ascii_lowercase();

            match priority {
                Criterion::Requirement(priority) => {
                    if !matches!(
                        label.as_str(),
                        "does not meet" | "partially meets" | "fully meets"
                    ) {
                        errors.push(format!(
                            "invalid requirement label `{label}` for `{criterion}`"
                        ));
                    }

                    if complete
                        && priority == "MUST"
                        && label != "fully meets"
                        && option.eq_ignore_ascii_case(chosen)
                    {
                        errors.push("chosen option does not fully meet a MUST requirement".into());
                    }
                }
                Criterion::Constraint => {
                    if !matches!(label.as_str(), "pass" | "fail") {
                        errors.push(format!(
                            "constraint `{criterion}` assessment must be PASS or FAIL"
                        ));
                    }

                    if complete && label == "fail" && option.eq_ignore_ascii_case(chosen) {
                        errors.push("chosen option fails a constraint".into());
                    }
                }
            }
        }
    }

    for criterion in criteria.keys() {
        if complete && !seen_criteria.contains(criterion) {
            errors.push(format!("comparison has no row for `{criterion}`"));
        }
    }
}

fn table_rows<'a>(
    tables: &'a [Table],
    required_headers: &[&str],
    section: &str,
    errors: &mut Vec<String>,
) -> &'a [Vec<String>] {
    if tables.len() > 1 {
        errors.push(format!(
            "`### {section}` must contain exactly one canonical table"
        ));
    }

    let Some(table) = tables.first() else {
        errors.push(format!("`### {section}` is missing its table"));

        return &[];
    };
    let Some(headers) = table.first() else {
        errors.push(format!("`### {section}` table is empty"));

        return &[];
    };

    let normalized: Vec<String> = headers
        .iter()
        .map(|header| header.trim().to_ascii_lowercase())
        .collect();

    for required in required_headers {
        if !normalized.iter().any(|header| header == required) {
            errors.push(format!(
                "`### {section}` table is missing `{required}` column"
            ));
        }
    }

    if normalized.len() != required_headers.len()
        || !normalized
            .iter()
            .map(String::as_str)
            .eq(required_headers.iter().copied())
    {
        errors.push(format!(
            "`### {section}` table columns are not in canonical order"
        ));
    }

    &table[1..]
}

fn section_text<'a>(markdown: &'a str, headings: &[Heading], section: &str) -> &'a str {
    let Some(index) = headings.iter().position(|heading| heading.title == section) else {
        return "";
    };
    let selected = &headings[index];
    let end = headings[index + 1..]
        .iter()
        .find(|heading| heading.level <= selected.level)
        .map_or(markdown.len(), |heading| heading.range.start);

    &markdown[selected.range.end..end]
}

fn collect_tables(markdown: &str) -> Vec<Table> {
    let parser = Parser::new_ext(markdown, Options::ENABLE_TABLES);
    let mut tables = Vec::new();
    let mut table = Vec::new();
    let mut row = Vec::new();
    let mut cell: Option<String> = None;

    for event in parser {
        match event {
            Event::Start(Tag::TableCell) => cell = Some(String::new()),
            Event::Text(text) | Event::Code(text) => {
                if let Some(cell) = cell.as_mut() {
                    cell.push_str(&text);
                }
            }
            Event::End(TagEnd::TableCell) => {
                if let Some(cell) = cell.take() {
                    row.push(cell.trim().to_string());
                }
            }
            Event::End(TagEnd::TableHead | TagEnd::TableRow) => {
                table.push(std::mem::take(&mut row));
            }
            Event::End(TagEnd::Table) => {
                tables.push(std::mem::take(&mut table));
            }
            _ => {}
        }
    }

    tables
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_transliterates_and_rejects_paths() {
        assert_eq!(slugify("Use naïve search").unwrap(), "use-naive-search");
        assert!(slugify("../escape").is_err());
        assert!(slugify("東京").unwrap().is_ascii());
    }

    #[test]
    fn status_transitions_follow_the_lifecycle() {
        assert!(Status::Draft.can_transition_to(Status::Proposed));
        assert!(Status::Draft.can_transition_to(Status::Rejected));
        assert!(!Status::Draft.can_transition_to(Status::Accepted));
        assert!(Status::Proposed.can_transition_to(Status::Draft));
        assert!(Status::Proposed.can_transition_to(Status::Accepted));
        assert!(Status::Accepted.can_transition_to(Status::Superseded));
        assert!(!Status::Accepted.can_transition_to(Status::Rejected));
        assert!(!Status::Rejected.can_transition_to(Status::Accepted));
    }

    #[test]
    fn section_extraction_respects_nested_headings_and_code_examples() {
        let markdown = "## Context\n\nExample:\n\n```md\n## Options\n```\n\nStill context.\n\n## Requirements and constraints\n\n### Requirements\n\nBehavior.\n\n### Constraints\n\nLimits.\n\n## Options\n\nChoices.";
        let headings = collect_headings(markdown);

        assert!(section_text(markdown, &headings, "Context").contains("Still context."));
        assert_eq!(
            section_text(markdown, &headings, "Requirements").trim(),
            "Behavior."
        );
        assert_eq!(
            section_text(markdown, &headings, "Constraints").trim(),
            "Limits."
        );
        assert_eq!(
            section_text(markdown, &headings, "Options").trim(),
            "Choices."
        );
    }

    #[test]
    fn document_ids_use_canonical_positive_sequences() {
        assert_eq!(parse_id("ADR-0001").unwrap(), ("ADR", 1));
        assert_eq!(parse_id("RFC-10000").unwrap(), ("RFC", 10_000));
        assert!(parse_id("ADR-0000").is_err());
        assert!(parse_id("ADR-00001").is_err());
    }

    #[test]
    fn removed_front_matter_fields_are_omitted_when_documents_are_updated() {
        let legacy = "---\nid: ADR-0001\nkind: rfc\ntitle: Old title\nslug: old-title\ndate: 2026-11-09\nstatus: proposed\nschema_version: 99\n---\n\n# ADR-0001: Old title\n\n## Context\n\nIssue.\n\n## Decision\n\nChoice.\n\n## Consequences\n\nTrade-off.";
        let document = parse_document(legacy, "docs/adrs/0001-old-title.md".into()).unwrap();
        let mut metadata = document.metadata.clone();
        metadata.status = Status::Accepted;

        let updated = update_metadata(&document, metadata).unwrap();

        for field in ["kind:", "slug:", "schema_version:"] {
            assert!(!updated.raw.contains(field));
        }
    }

    #[test]
    fn adr_requires_nygard_sections_except_body_status() {
        let doc = Document {
            metadata: Metadata {
                id: "ADR-0001".into(),
                title: "Use a focused record".into(),
                date: "2026-11-09".into(),
                status: Status::Proposed,
                superseded_by: None,
                extra: BTreeMap::new(),
            },
            body: "# ADR-0001: Use a focused record\n\n## Context\n\nIssue.\n\n## Decision\n\nChoice.\n\n## Consequences\n\nTrade-off.".into(),
            path: "docs/adrs/0001-use-a-focused-record.md".into(),
            raw: String::new(),
        };

        assert!(validate_document(&doc).is_empty());

        let mut with_status = doc.clone();
        with_status.body = with_status
            .body
            .replace("## Context", "## Status\n\nproposed\n\n## Context");

        assert!(
            validate_document(&with_status)
                .iter()
                .any(|error| error.contains("status belongs only"))
        );
    }

    #[test]
    fn rendered_heading_preserves_markdown_special_characters_in_title() {
        let title = "Choose [links](safely) and `code`";
        let document = NewDocument {
            kind: Kind::Adr,
            title: title.into(),
            slug: slugify(title).unwrap(),
            date: "2026-11-09".into(),
            body: "## Context\n\nIssue.\n\n## Decision\n\nChoice.\n\n## Consequences\n\nTrade-off."
                .into(),
        }
        .build(1, Status::Proposed)
        .unwrap();

        assert!(validate_document(&document).is_empty());
    }

    #[test]
    fn document_size_limit_includes_metadata_and_expanding_identifiers() {
        let mut document = NewDocument {
            kind: Kind::Adr,
            title: "Bounded record".into(),
            slug: "bounded-record".into(),
            date: "2026-11-09".into(),
            body: String::new(),
        };
        let overhead = document.build(9999, Status::Draft).unwrap().raw.len();

        document.body = "x".repeat(MAX_DOCUMENT_BYTES as usize - overhead);

        assert_eq!(
            document.build(9999, Status::Draft).unwrap().raw.len(),
            MAX_DOCUMENT_BYTES as usize
        );
        assert!(document.build(10_000, Status::Draft).is_err());
    }

    #[test]
    fn metadata_updates_preserve_crlf_body_bytes() {
        let metadata = Metadata {
            id: "ADR-0001".into(),
            title: "Keep body line endings".into(),
            date: "2026-11-09".into(),
            status: Status::Proposed,
            superseded_by: None,
            extra: BTreeMap::new(),
        };

        let raw = render_document(
            &metadata,
            "## Context\n\nIssue.\n\n## Decision\n\nChoice.\n\n## Consequences\n\nTrade-off.",
        )
        .unwrap()
        .replace('\n', "\r\n");
        let document = parse_document(&raw, "docs/adrs/0001-title.md".into()).unwrap();
        let mut accepted_metadata = metadata;
        accepted_metadata.status = Status::Accepted;

        let updated = update_metadata(&document, accepted_metadata).unwrap();

        assert!(updated.raw.ends_with("## Consequences\r\n\r\nTrade-off."));
    }
}
