use crate::hexagon::ports::driven::for_obtaining_skill_files::{ForObtainingSkillFiles, SkillFile};

/// Supplies a one-file skill so tests do not depend on the bundled skill content.
pub(crate) struct StubSkillFiles;

impl ForObtainingSkillFiles for StubSkillFiles {
    fn get_skill_files(&self) -> &[SkillFile] {
        &[SkillFile {
            relative_path: "SKILL.md",
            content: "# Stub skill\n",
        }]
    }
}
