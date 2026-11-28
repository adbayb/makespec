#[derive(Clone, Copy, Debug)]
pub(crate) struct SkillFile {
    /// Path relative to the installed skill directory.
    pub relative_path: &'static str,
    pub content: &'static str,
}

/// Driven port for obtaining the agent skill installed by initialization.
pub(crate) trait ForObtainingSkillFiles {
    fn get_skill_files(&self) -> &[SkillFile];
}
