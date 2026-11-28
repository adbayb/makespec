use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

use chrono::NaiveDate;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Value;

use super::error::{MakespecError, Result};

pub(crate) const MAX_DOCUMENT_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Kind {
    Decision,
    Specification,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Decision => "decision",
            Self::Specification => "specification",
        }
    }

    pub fn id_prefix(self) -> &'static str {
        match self {
            Self::Decision => "ADR",
            Self::Specification => "RFC",
        }
    }

    pub fn directory(self) -> &'static str {
        match self {
            Self::Decision => "decisions",
            Self::Specification => "specifications",
        }
    }

    fn from_id_prefix(prefix: &str) -> Option<Self> {
        match prefix {
            "ADR" => Some(Self::Decision),
            "RFC" => Some(Self::Specification),
            _ => None,
        }
    }
}

impl std::str::FromStr for Kind {
    type Err = MakespecError;

    fn from_str(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "decision" => Ok(Self::Decision),
            "specification" => Ok(Self::Specification),
            _ => Err(MakespecError::Invalid(format!(
                "unknown document kind `{value}`; expected `decision` or `specification`"
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
    Proposed,
    Accepted,
    Rejected,
    Deprecated,
    Superseded,
}

impl Status {
    pub const ALL: [Self; 5] = [
        Self::Proposed,
        Self::Accepted,
        Self::Rejected,
        Self::Deprecated,
        Self::Superseded,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
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
            (Self::Proposed, Self::Accepted | Self::Rejected)
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
                    "unknown status `{value}`; expected proposed, accepted, rejected, deprecated, or superseded"
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
        let (kind, number) = parse_id(&self.id)?;

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

                if target == self.id || target_kind != kind {
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

pub fn parse_id(id: &str) -> Result<(Kind, u32)> {
    let invalid = || {
        MakespecError::Invalid(format!(
            "invalid document ID `{id}`; expected ADR-0001 or RFC-0001"
        ))
    };
    let (prefix, number_text) = id.split_once('-').ok_or_else(invalid)?;
    let kind = Kind::from_id_prefix(prefix).ok_or_else(invalid)?;

    if number_text.len() < 4 || !number_text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid());
    }

    let number = number_text
        .parse::<u32>()
        .map_err(|_| MakespecError::Invalid(format!("document ID `{id}` is out of range")))?;

    if number == 0 || format!("{number:04}") != number_text {
        return Err(MakespecError::Invalid(format!(
            "invalid document ID `{id}`; sequence numbers start at 0001 and use the shortest four-digit form"
        )));
    }

    Ok((kind, number))
}

fn render_document(metadata: &Metadata, body: &str) -> Result<String> {
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
    let metadata: Metadata = serde_yaml_ng::from_str(yaml)
        .map_err(|error| MakespecError::Invalid(format!("{path}: {error}")))?;

    if let Some(field) = ["kind", "slug", "schema_version"]
        .into_iter()
        .find(|field| metadata.extra.contains_key(*field))
    {
        return Err(MakespecError::Invalid(format!(
            "{path}: unsupported front matter field `{field}`"
        )));
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
    validate(document, document.metadata.status != Status::Rejected)
}

pub(crate) fn validate_document_structure(document: &Document) -> Vec<String> {
    validate(document, false)
}

/// Fails with every validation error of `document`, prefixed by `context`.
pub(crate) fn ensure_valid(document: &Document, context: &str) -> Result<()> {
    let errors = validate_document(document);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(validation_error(context, &errors))
    }
}

pub(crate) fn validation_error(context: &str, errors: &[String]) -> MakespecError {
    MakespecError::Invalid(format!("{context}: {}", errors.join("; ")))
}

fn validate(document: &Document, complete: bool) -> Vec<String> {
    let Ok((kind, _)) = document.metadata.validate() else {
        return vec!["invalid document metadata".into()];
    };

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
        Kind::Decision => &["Context", "Decision", "Consequences", "References"],
        Kind::Specification => &[
            "Context",
            "Requirements and constraints",
            "Options",
            "Design",
            "Adoption",
            "Open questions",
            "References",
        ],
    };
    let required = match kind {
        Kind::Specification if complete => &sections[..5],
        _ => &sections[..3],
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
        errors.push("sections are not in template order".into());
    }

    if kind == Kind::Specification {
        errors.extend(validate_specification_tables(
            &document.body,
            &headings,
            complete,
        ));
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
    Requirement { must: bool },
    Constraint,
}

fn validate_specification_tables(
    markdown: &str,
    headings: &[Heading],
    complete: bool,
) -> Vec<String> {
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
        errors.push("specification comparison requires at least one functional requirement".into());
    }

    let options_text = section_text(markdown, headings, "Options");

    validate_options(options_text, &criteria, complete, &mut errors);

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

        if !is_criterion_id(&id, &["FR-", "NFR-"]) {
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
            .insert(
                id.clone(),
                Criterion::Requirement {
                    must: priority == "MUST",
                },
            )
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

        if !is_criterion_id(&id, &["C-"]) {
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

fn is_criterion_id(id: &str, prefixes: &[&str]) -> bool {
    prefixes
        .iter()
        .find_map(|prefix| id.strip_prefix(prefix))
        .is_some_and(|number| number.len() >= 3 && number.bytes().all(|byte| byte.is_ascii_digit()))
}

fn validate_options(
    options_text: &str,
    criteria: &BTreeMap<String, Criterion>,
    complete: bool,
    errors: &mut Vec<String>,
) {
    let themes: Vec<Heading> = collect_headings(options_text)
        .into_iter()
        .filter(|heading| heading.level == HeadingLevel::H3)
        .collect();
    let mut seen_criteria = BTreeSet::new();

    if themes.is_empty() {
        seen_criteria.extend(validate_comparison(
            &collect_tables(options_text),
            criteria,
            options_text,
            complete,
            errors,
        ));
    } else {
        let preamble = &options_text[..themes[0].range.start];

        if !collect_tables(preamble).is_empty()
            || preamble
                .lines()
                .any(|line| line.trim().starts_with("**Chosen option:**"))
        {
            errors.push(
                "`## Options` comparisons and choices must be inside a named theme when using themes"
                    .into(),
            );
        }

        let mut names = BTreeSet::new();

        for (index, theme) in themes.iter().enumerate() {
            if theme.title.trim().is_empty()
                || !names.insert(theme.title.trim().to_ascii_lowercase())
            {
                errors.push("option themes must have unique, non-empty names".into());
            }

            let end = themes
                .get(index + 1)
                .map_or(options_text.len(), |heading| heading.range.start);
            let text = &options_text[theme.range.end..end];
            let mut theme_errors = Vec::new();

            seen_criteria.extend(validate_comparison(
                &collect_tables(text),
                criteria,
                text,
                complete,
                &mut theme_errors,
            ));
            errors.extend(
                theme_errors
                    .into_iter()
                    .map(|error| format!("option theme `{}`: {error}", theme.title)),
            );
        }
    }

    for criterion in criteria.keys() {
        if complete && !seen_criteria.contains(criterion) {
            errors.push(format!("comparison has no row for `{criterion}`"));
        }
    }
}

fn validate_comparison(
    tables: &[Table],
    criteria: &BTreeMap<String, Criterion>,
    options_text: &str,
    complete: bool,
    errors: &mut Vec<String>,
) -> BTreeSet<String> {
    let choices: Vec<&str> = options_text
        .lines()
        .filter_map(|line| line.trim().strip_prefix("**Chosen option:**"))
        .map(str::trim)
        .collect();
    let chosen = choices.first().copied().unwrap_or_default();
    let mut seen_criteria = BTreeSet::new();

    if choices.len() > 1 {
        errors.push("`## Options` must name at most one chosen option".into());
    }

    if tables.len() > 1 {
        errors.push("`## Options` must contain exactly one canonical table".into());
    }

    let Some(comparison_table) = tables.first() else {
        errors.push("`## Options` is missing its comparison table".into());

        return seen_criteria;
    };
    let Some(headers) = comparison_table.first() else {
        errors.push("`## Options` table is empty".into());

        return seen_criteria;
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
    let mut unique_options = BTreeSet::new();

    for option in &options {
        if option.is_empty() || !unique_options.insert(option.to_ascii_lowercase()) {
            errors.push("comparison option columns must have unique, non-empty names".into());
        }
    }

    if (complete && chosen.is_empty())
        || (!chosen.is_empty()
            && !options
                .iter()
                .any(|option| option.eq_ignore_ascii_case(chosen)))
    {
        errors.push("`## Options` must name a chosen option that appears in the table".into());
    }

    for row in comparison_table.iter().skip(1) {
        if row.len() != headers.len() {
            errors.push("comparison row must have one cell for every option".into());
            continue;
        }

        let criterion = row[0].trim().to_ascii_uppercase();
        let Some(criterion_kind) = criteria.get(&criterion) else {
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

            match criterion_kind {
                Criterion::Requirement { must } => {
                    if !matches!(
                        label.as_str(),
                        "does not meet" | "partially meets" | "fully meets"
                    ) {
                        errors.push(format!(
                            "invalid requirement label `{label}` for `{criterion}`"
                        ));
                    }

                    if complete
                        && *must
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

    if complete && seen_criteria.is_empty() {
        errors.push("comparison table must assess at least one criterion".into());
    }

    seen_criteria
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
        assert!("unknown".parse::<Status>().is_err());
        assert!(Status::Proposed.can_transition_to(Status::Rejected));
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

    const GROUPED_SPECIFICATION: &str = r#"## Requirements and constraints

### Requirements

| ID | Priority | Requirement |
| --- | --- | --- |
| FR-001 | MUST | Users can publish offers. |
| NFR-001 | MUST | Clients support the required platforms. |

### Constraints

| ID | Constraint |
| --- | --- |
| C-001 | Use no AI providers. |

## Options

### User experience

| Criterion ID | Moderated offers | Direct offers |
| --- | --- | --- |
| FR-001 | Fully meets - Reviewed offers are published. | Fully meets - Users publish immediately. |
| C-001 | PASS - Review is manual. | PASS - Publication is deterministic. |

**Chosen option:** Moderated offers

### Technical stack

| Criterion ID | Flutter | React Native | Native clients |
| --- | --- | --- | --- |
| NFR-001 | Fully meets - Targets all required platforms. | Fully meets - Includes a web client. | Fully meets - Separate clients cover each platform. |
| C-001 | PASS - No AI dependency. | PASS - No AI dependency. | PASS - No AI dependency. |

**Chosen option:** Flutter
"#;

    #[test]
    fn specification_themes_validate_local_choices_and_collective_coverage() {
        let headings = collect_headings(GROUPED_SPECIFICATION);
        let errors = validate_specification_tables(GROUPED_SPECIFICATION, &headings, true);

        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn specification_themes_reject_incomplete_or_ineligible_choices() {
        for (from, to, expected) in [
            (
                "**Chosen option:** Flutter",
                "",
                "must name a chosen option",
            ),
            (
                "**Chosen option:** Flutter",
                "**Chosen option:** Moderated offers",
                "must name a chosen option",
            ),
            (
                "Fully meets - Targets all required platforms.",
                "Partially meets - Missing a required platform.",
                "does not fully meet a MUST",
            ),
            (
                "PASS - No AI dependency.",
                "FAIL - Requires an AI provider.",
                "fails a constraint",
            ),
            (
                "| NFR-001 | Fully meets - Targets all required platforms. | Fully meets - Includes a web client. | Fully meets - Separate clients cover each platform. |",
                "",
                "comparison has no row for `NFR-001`",
            ),
            (
                "### Technical stack",
                "### User experience",
                "unique, non-empty",
            ),
            (
                "**Chosen option:** Flutter",
                "**Chosen option:** Flutter\n\n**Chosen option:** Native clients",
                "at most one chosen option",
            ),
            ("### Technical stack", "", "exactly one canonical table"),
            ("### User experience", "", "inside a named theme"),
            (
                "## Options",
                "## Options\n\n### Infrastructure\n\n| Criterion ID | Hosted | Self-hosted |\n| --- | --- | --- |\n\n**Chosen option:** Hosted",
                "must assess at least one criterion",
            ),
            (
                "| NFR-001 | Fully meets",
                "| NFR-099 | Fully meets",
                "unknown criterion `NFR-099`",
            ),
        ] {
            let body = GROUPED_SPECIFICATION.replace(from, to);
            let errors = validate_specification_tables(&body, &collect_headings(&body), true);

            assert!(
                errors.iter().any(|error| error.contains(expected)),
                "expected {expected:?}, got {errors:?}"
            );
        }
    }

    #[test]
    fn comparison_option_count_has_a_minimum_but_no_fixed_target() {
        let criteria = BTreeMap::from([("FR-001".into(), Criterion::Requirement { must: true })]);

        for count in 1..=5 {
            let mut headers = vec!["Criterion ID".into()];
            let mut row = vec!["FR-001".into()];

            for number in 1..=count {
                headers.push(format!("Approach {number}"));
                row.push("Fully meets - Supports publication.".into());
            }

            let mut errors = Vec::new();

            validate_comparison(
                &[vec![headers, row]],
                &criteria,
                "**Chosen option:** Approach 1",
                true,
                &mut errors,
            );

            assert_eq!(errors.is_empty(), count >= 2, "{errors:?}");
        }
    }

    #[test]
    fn document_ids_use_canonical_positive_sequences() {
        assert_eq!(parse_id("ADR-0001").unwrap(), (Kind::Decision, 1));
        assert_eq!(
            parse_id("RFC-10000").unwrap(),
            (Kind::Specification, 10_000)
        );
        assert!(parse_id("XYZ-0001").is_err());
        assert!(parse_id("ADR-0000").is_err());
        assert!(parse_id("ADR-00001").is_err());
    }

    #[test]
    fn unrelated_front_matter_fields_are_preserved_when_documents_are_updated() {
        let content = "---\nid: ADR-0001\ntitle: A focused decision\ndate: 2026-11-09\nstatus: proposed\nowner: platform\n---\n\n# ADR-0001: A focused decision\n\n## Context\n\nIssue.\n\n## Decision\n\nChoice.\n\n## Consequences\n\nTrade-off.";
        let document =
            parse_document(content, "docs/decisions/0001-focused-decision.md".into()).unwrap();
        let mut metadata = document.metadata.clone();
        metadata.status = Status::Accepted;

        let updated = update_metadata(&document, metadata).unwrap();

        assert!(updated.raw.contains("owner: platform"));
    }

    #[test]
    fn unsupported_front_matter_fields_are_rejected() {
        for field in [
            "kind: decision",
            "slug: focused-decision",
            "schema_version: 1",
        ] {
            let content = format!(
                "---\nid: ADR-0001\ntitle: A focused decision\ndate: 2026-11-09\nstatus: proposed\n{field}\n---\n\n# ADR-0001: A focused decision\n\n## Context\n\nIssue.\n\n## Decision\n\nChoice.\n\n## Consequences\n\nTrade-off."
            );
            let error = parse_document(content.as_str(), "record.md".into()).unwrap_err();
            let field_name = field.split_once(':').unwrap().0;

            assert!(
                error
                    .to_string()
                    .contains(&format!("unsupported front matter field `{field_name}`"))
            );
        }
    }

    #[test]
    fn decision_requires_nygard_sections_except_body_status() {
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
            path: "docs/decisions/0001-use-a-focused-record.md".into(),
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
            kind: Kind::Decision,
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
            kind: Kind::Decision,
            title: "Bounded record".into(),
            slug: "bounded-record".into(),
            date: "2026-11-09".into(),
            body: String::new(),
        };
        let overhead = document.build(9999, Status::Proposed).unwrap().raw.len();

        document.body = "x".repeat(MAX_DOCUMENT_BYTES as usize - overhead);

        assert_eq!(
            document.build(9999, Status::Proposed).unwrap().raw.len(),
            MAX_DOCUMENT_BYTES as usize
        );
        assert!(document.build(10_000, Status::Proposed).is_err());
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
        let document = parse_document(&raw, "docs/decisions/0001-title.md".into()).unwrap();
        let mut accepted_metadata = metadata;
        accepted_metadata.status = Status::Accepted;

        let updated = update_metadata(&document, accepted_metadata).unwrap();

        assert!(updated.raw.ends_with("## Consequences\r\n\r\nTrade-off."));
    }

    fn complete_specification_body() -> &'static str {
        r#"## Context

Define a consistent specification format for functionality. Specifications use the request for comments (RFC) format and include product requirements documents (PRDs) as well as technical change proposals. Authors need a reviewable record of the problem, evaluation criteria, options, chosen design, and adoption approach without a hosted service.

## Requirements and constraints

### Requirements

| ID | Priority | Requirement |
| --- | --- | --- |
| FR-001 | MUST | A specification describes functionality requirements, constraints, and qualitative alternatives. |
| NFR-001 | SHOULD | A specification remains readable and reviewable as plain Markdown in Git. |

### Constraints

| ID | Constraint |
| --- | --- |
| C-001 | The format must work with repository files and require no service or database. |

## Options

- No change: leave proposal formats to individual authors.
- Free-form Markdown: record proposals without required sections or comparison rules.
- Decision-only records: reuse the focused decision format for functionality specifications.
- Makespec format: use shared criteria, an options comparison, and a design for the chosen option.

| Criterion ID | No change | Free-form Markdown | Decision-only records | Makespec format |
| --- | --- | --- | --- | --- |
| FR-001 | Does not meet - Proposals need not capture the required areas. | Does not meet - No structure requires all required areas. | Does not meet - Decisions do not specify functionality. | Fully meets - The format requires functionality criteria, constraints, and comparisons. |
| NFR-001 | Partially meets - Markdown stays readable, but authors lack a shared structure. | Fully meets - Free-form Markdown remains easy to read and diff. | Fully meets - Decisions remain plain Markdown in Git. | Fully meets - The format uses reviewable Markdown tables. |
| C-001 | PASS - It requires no service or database. | PASS - It uses repository files only. | PASS - It uses repository files only. | PASS - The CLI reads and writes local Markdown files. |

**Chosen option:** Makespec format

The chosen format fully meets FR-001 and satisfies the hard constraint. It keeps records portable while making comparison criteria explicit.

## Design

Every specification has YAML front matter containing `id`, `title`, `date`, and `status`. The CLI supplies this metadata and the title heading. The ID prefix determines the document kind: `ADR-` for decisions and `RFC-` for specifications. The filename retains its creation-time slug if the title later changes.

Specification bodies use Context, Requirements and constraints, Options, Design, and Adoption in that order. Context starts with a short proposal summary and explains the problem and scope. Put functional requirements in a `### Requirements` table, adding non-functional requirements only when relevant. Add a separate `### Constraints` table only for genuine hard boundaries. An optional Open questions section may follow Adoption.

An optional References section comes last. Link related decisions and specifications by ID using relative Markdown paths, explain each relationship, and include external sources when relevant. A specification may link to broader or focused specifications instead of duplicating requirements.

Functional requirements use unique `FR-001`-style IDs, and non-functional requirements use `NFR-001`-style IDs. Both use priority (MUST, SHOULD, or MAY). Constraints use unique `C-001` IDs. Options includes candidate descriptions and one comparison table with criteria in rows and options in columns. Include at least two options. Include No change only when it represents a meaningful current-state alternative; omit it when it would add no useful comparison, such as for a greenfield project with no existing capability. When omitting No change, compare at least two distinct approaches that address the same need. Requirement cells use `Does not meet`, `Partially meets`, or `Fully meets`, followed by ` - ` and evidence. Constraint cells use `PASS` or `FAIL`, followed by ` - ` and evidence. The chosen option must fully meet each MUST requirement and pass every constraint. Explain the choice using criterion IDs rather than an aggregate score.

New records start as proposed. Creation checks the initial structure, including at least one functional requirement and a comparison table. Checking a proposal requires all five sections, an assessment of every criterion for every option, an eligible chosen option, and no template placeholders. Incomplete proposals fail checking and cannot be accepted. Proposed records may become accepted or rejected. Accepted records may become deprecated or superseded.

Design describes only the chosen option, including behavior, examples, and failure cases. Add a Risks and mitigations subsection when material risks need explanation. Adoption covers rollout, compatibility, and verification, with migration or rollback when relevant.

### Risks and mitigations

Comparison labels require supporting evidence; the CLI checks structure and eligibility but cannot judge whether an assessment is true. Reviewers must examine the evidence before accepting a proposal.

## Adoption

Create proposals from the bundled template and run `makespec check` after edits. Resolve missing information and validation errors, then request acceptance after review. Verify adoption by exercising creation, checking an incomplete proposal, completing it, and accepting it."#
    }

    const INCOMPLETE_SPECIFICATION_BODY: &str = "## Context\n\nLet users search local records.\n\n## Requirements and constraints\n\n### Requirements\n\n| ID | Priority | Requirement |\n| --- | --- | --- |\n| FR-001 | MUST | Search records by title. |\n\n## Options\n\n| Criterion ID | No change | Local search |\n| --- | --- | --- |\n| FR-001 | | |\n\n**Chosen option:**";

    #[test]
    fn incomplete_proposals_pass_creation_structure_but_fail_readiness_checks() {
        let proposal = specification(INCOMPLETE_SPECIFICATION_BODY, Status::Proposed);

        assert!(validate_document_structure(&proposal).is_empty());
        assert!(
            validate_document(&specification(
                INCOMPLETE_SPECIFICATION_BODY,
                Status::Rejected
            ))
            .is_empty()
        );

        let errors = validate_document(&proposal);

        for expected in [
            "`## Design`",
            "`## Adoption`",
            "chosen option",
            "label and explanation",
        ] {
            assert!(
                errors.iter().any(|error| error.contains(expected)),
                "{errors:?}"
            );
        }

        let body = INCOMPLETE_SPECIFICATION_BODY
            .replace(
                "| FR-001 | | |",
                "| FR-001 | Does not meet - No search exists. | Fully meets - Search matches titles. |",
            )
            .replace("**Chosen option:**", "**Chosen option:** Local search")
            + "\n\n## Design\n\nMatch a query against record titles.\n\n## Adoption\n\nShip the search command and verify title matches.";
        let errors = validate_document(&specification(&body, Status::Proposed));

        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn creation_rejects_inconsistent_comparison_structure() {
        for (original, replacement, expected) in [
            (
                "| Criterion ID | No change | Local search |\n| --- | --- | --- |\n| FR-001 | | |",
                "| Criterion ID | Local search |\n| --- | --- |\n| FR-001 | |",
                "at least two option columns",
            ),
            (
                "| FR-001 | MUST | Search records by title. |",
                "",
                "at least one functional requirement",
            ),
            ("| FR-001 | | |", "| FR-002 | | |", "unknown criterion"),
            (
                "| FR-001 | | |",
                "| FR-001 | | |\n| FR-001 | | |",
                "repeats criterion",
            ),
            (
                "| FR-001 | | |",
                "| FR-001 | PASS - Confused labels. | |",
                "invalid requirement label",
            ),
            (
                "**Chosen option:**",
                "**Chosen option:** Missing",
                "chosen option",
            ),
            (
                "### Requirements",
                "### Requirements\n\n### Requirements",
                "duplicate `### Requirements`",
            ),
            (
                "## Options",
                "## Open questions\n\nPending.\n\n## Options",
                "template order",
            ),
        ] {
            let body = INCOMPLETE_SPECIFICATION_BODY.replace(original, replacement);
            let errors = validate_document_structure(&specification(&body, Status::Proposed));

            assert!(
                errors.iter().any(|error| error.contains(expected)),
                "{errors:?}"
            );
        }
    }

    #[test]
    fn complete_specifications_can_compare_two_approaches_without_no_change() {
        let body = complete_specification_body()
            .replace("| --- | --- | --- | --- | --- |", "| --- | --- | --- |")
            .replace(
                "| Criterion ID | No change | Free-form Markdown | Decision-only records | Makespec format |",
                "| Criterion ID | Free-form Markdown | Makespec format |",
            )
            .replace(
                "| FR-001 | Does not meet - Proposals need not capture the required areas. | Does not meet - No structure requires all required areas. | Does not meet - Decisions do not specify functionality. | Fully meets - The format requires functionality criteria, constraints, and comparisons. |",
                "| FR-001 | Does not meet - No structure requires all required areas. | Fully meets - The format requires functionality criteria, constraints, and comparisons. |",
            )
            .replace(
                "| NFR-001 | Partially meets - Markdown stays readable, but authors lack a shared structure. | Fully meets - Free-form Markdown remains easy to read and diff. | Fully meets - Decisions remain plain Markdown in Git. | Fully meets - The format uses reviewable Markdown tables. |",
                "| NFR-001 | Fully meets - Free-form Markdown remains easy to read and diff. | Fully meets - The format uses reviewable Markdown tables. |",
            )
            .replace(
                "| C-001 | PASS - It requires no service or database. | PASS - It uses repository files only. | PASS - It uses repository files only. | PASS - The CLI reads and writes local Markdown files. |",
                "| C-001 | PASS - It uses repository files only. | PASS - The CLI reads and writes local Markdown files. |",
            );
        let errors = validate_document(&specification(&body, Status::Proposed));

        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn templates_require_completion_and_specification_candidates() {
        for (kind, body) in [
            (
                Kind::Decision,
                include_str!("../../.agents/skills/makespec/references/decision-template.md"),
            ),
            (
                Kind::Specification,
                include_str!("../../.agents/skills/makespec/references/specification-template.md"),
            ),
        ] {
            let mut new_document = NewDocument {
                kind,
                title: "Template record".into(),
                slug: "template-record".into(),
                date: "2026-11-09".into(),
                body: body.into(),
            };
            if kind == Kind::Specification {
                let errors =
                    validate_document_structure(&new_document.build(1, Status::Proposed).unwrap());

                assert_eq!(errors, ["`## Options` is missing its comparison table"]);

                new_document.body = new_document.body.replace(
                    "## Options",
                    "## Options\n\n| Criterion ID | List view | Card view |\n| --- | --- | --- |\n| FR-001 | | |\n| NFR-001 | | |\n| C-001 | | |",
                ).replace("Selected candidate", "List view");
            }

            let errors =
                validate_document_structure(&new_document.build(1, Status::Proposed).unwrap());

            assert!(errors.is_empty(), "{errors:?}");
            assert!(
                validate_document(&new_document.build(1, Status::Proposed).unwrap())
                    .iter()
                    .any(|error| error.contains("unresolved template placeholder"))
            );
        }
    }

    #[test]
    fn references_are_optional_unique_nonempty_and_in_template_order() {
        let references = "## References\n\n- [ADR-0001: Use Makespec to manage decisions and specifications](../decisions/0001-use-makespec-to-manage-decisions-and-specifications.md) adopts the workflow.";

        for (kind, body) in [
            (
                Kind::Decision,
                "## Context\n\nContributors need a focused choice.\n\n## Decision\n\nUse the proposed approach.\n\n## Consequences\n\nKeep the decision in Git.".to_string(),
            ),
            (
                Kind::Specification,
                format!(
                    "{}\n\n## Open questions\n\nWhich follow-up metrics would help?",
                    complete_specification_body()
                ),
            ),
        ] {
            for (body, expected_error) in [
                (body.clone(), None),
                (format!("{body}\n\n{references}"), None),
                (
                    format!("{body}\n\n## References\n"),
                    Some("`## References` must contain content"),
                ),
                (
                    format!("{body}\n\n{references}\n\n{references}"),
                    Some("duplicate `## References`"),
                ),
                (format!("{references}\n\n{body}"), Some("template order")),
            ] {
                let document = record(kind, "Reference validation", &body, Status::Accepted);
                let errors = validate_document(&document);

                if let Some(expected) = expected_error {
                    assert!(
                        errors.iter().any(|error| error.contains(expected)),
                        "{kind}: {errors:?}"
                    );
                } else {
                    assert!(errors.is_empty(), "{kind}: {errors:?}");
                }
            }
        }

        let body = format!(
            "{}\n\n{references}\n\n## Open questions\n\nWhich follow-up metrics would help?",
            complete_specification_body()
        );
        let errors = validate_document(&specification(&body, Status::Proposed));

        assert!(
            errors.iter().any(|error| error.contains("template order")),
            "{errors:?}"
        );
    }

    #[test]
    fn specification_comparisons_validate_descriptions_labels_and_chosen_option_eligibility() {
        assert!(
            validate_document(&specification(
                complete_specification_body(),
                Status::Proposed
            ))
            .is_empty()
        );

        for (original, replacement, expected, valid_structure) in [
            (
                "Fully meets - The format requires functionality criteria, constraints, and comparisons.",
                "Does not meet - The format requires functionality criteria, constraints, and comparisons.",
                "chosen option does not fully meet a MUST requirement",
                true,
            ),
            (
                "PASS - The CLI reads and writes local Markdown files.",
                "FAIL - The CLI reads and writes local Markdown files.",
                "chosen option fails a constraint",
                true,
            ),
            (
                "Fully meets - The format requires functionality criteria, constraints, and comparisons.",
                "3 - The format requires functionality criteria, constraints, and comparisons.",
                "invalid requirement label `3` for `FR-001`",
                false,
            ),
            (
                "| FR-001 | MUST | A specification describes functionality requirements, constraints, and qualitative alternatives. |",
                "| FR-001 | MUST | |",
                "needs a description",
                false,
            ),
        ] {
            let body = complete_specification_body().replace(original, replacement);
            let errors = validate_document(&specification(&body, Status::Proposed));

            assert!(
                errors.iter().any(|error| error.contains(expected)),
                "{errors:?}"
            );
            assert_eq!(
                validate_document_structure(&specification(&body, Status::Proposed)).is_empty(),
                valid_structure
            );
        }
    }

    #[test]
    fn specification_reports_requirement_and_constraint_errors_together() {
        let body = complete_specification_body()
            .replace(
                "| FR-001 | MUST | A specification describes functionality requirements, constraints, and qualitative alternatives. |",
                "| FR-001 | MUST | |",
            )
            .replace(
                "PASS - The CLI reads and writes local Markdown files.",
                "FAIL - The CLI reads and writes local Markdown files.",
            );
        let errors = validate_document(&specification(&body, Status::Proposed));

        assert!(
            errors
                .iter()
                .any(|error| error == "criterion `FR-001` needs a description")
        );
        assert!(
            errors
                .iter()
                .any(|error| error == "chosen option fails a constraint")
        );
    }

    fn specification(body: &str, status: Status) -> Document {
        record(Kind::Specification, "Local search", body, status)
    }

    fn record(kind: Kind, title: &str, body: &str, status: Status) -> Document {
        NewDocument {
            kind,
            title: title.into(),
            slug: slugify(title).unwrap(),
            date: "2026-11-09".into(),
            body: body.into(),
        }
        .build(1, status)
        .unwrap()
    }
}
