use std::collections::BTreeMap;

pub const ADR_TEMPLATE: &str =
    include_str!("../../.agents/skills/makespec/references/adr-template.md");

pub const RFC_TEMPLATE: &str =
    include_str!("../../.agents/skills/makespec/references/rfc-template.md");

pub fn skill_files() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        (
            "SKILL.md",
            include_str!("../../.agents/skills/makespec/SKILL.md"),
        ),
        (
            "references/adr-template.md",
            include_str!("../../.agents/skills/makespec/references/adr-template.md"),
        ),
        (
            "references/cli.md",
            include_str!("../../.agents/skills/makespec/references/cli.md"),
        ),
        (
            "references/planning.md",
            include_str!("../../.agents/skills/makespec/references/planning.md"),
        ),
        (
            "references/rfc-template.md",
            include_str!("../../.agents/skills/makespec/references/rfc-template.md"),
        ),
    ])
}
