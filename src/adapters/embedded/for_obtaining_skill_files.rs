use crate::hexagon::ports::driven::for_obtaining_skill_files::{ForObtainingSkillFiles, SkillFile};

const SKILL_FILES: &[SkillFile] = &[
    SkillFile {
        relative_path: "SKILL.md",
        content: include_str!("../../../.agents/skills/makespec/SKILL.md"),
    },
    SkillFile {
        relative_path: "references/decision-template.md",
        content: include_str!("../../../.agents/skills/makespec/references/decision-template.md"),
    },
    SkillFile {
        relative_path: "references/cli.md",
        content: include_str!("../../../.agents/skills/makespec/references/cli.md"),
    },
    SkillFile {
        relative_path: "references/specification-template.md",
        content: include_str!(
            "../../../.agents/skills/makespec/references/specification-template.md"
        ),
    },
    SkillFile {
        relative_path: "references/writing.md",
        content: include_str!("../../../.agents/skills/makespec/references/writing.md"),
    },
];

/// Supplies the agent skill compiled into the binary.
pub(crate) struct EmbeddedSkillFiles;

impl ForObtainingSkillFiles for EmbeddedSkillFiles {
    fn get_skill_files(&self) -> &[SkillFile] {
        SKILL_FILES
    }
}
