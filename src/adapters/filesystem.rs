use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

use fs2::FileExt;
use sha2::{Digest, Sha256};
use tempfile::{Builder, NamedTempFile};

use crate::core::{
    document::{
        Document, DocumentSummary, Kind, MAX_DOCUMENT_BYTES, NewDocument, Status, parse_document,
        parse_id, parse_metadata_prefix, validate_document,
    },
    error::{MakespecError, Result},
    service::{ProjectFile, ProjectStore},
};

const MAX_STATE_BYTES: u64 = 1024;
const MAX_FRONTMATTER_BYTES: u64 = 64 * 1024;
const STAGING_PREFIX: &str = ".makespec-";
const STAGING_SUFFIX: &str = ".tmp";

pub(crate) struct FilesystemStore {
    root: PathBuf,
}

#[derive(Default)]
struct Counters {
    adr: u32,
    rfc: u32,
}

impl FilesystemStore {
    pub(crate) fn new(path: &Path) -> Result<Self> {
        fs::create_dir_all(path)?;
        let root = fs::canonicalize(path)?;

        if !root.is_dir() {
            return Err(MakespecError::Invalid(format!(
                "project path is not a directory: {}",
                root.display()
            )));
        }

        Ok(Self { root })
    }

    fn with_lock<T>(&self, action: impl FnOnce(&Path) -> Result<T>) -> Result<T> {
        let (lock_path, state_path) = self.lock_paths()?;

        if fs::symlink_metadata(&lock_path).is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            return Err(MakespecError::Invalid(format!(
                "refusing symlinked lock file {}",
                lock_path.display()
            )));
        }

        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)?;

        lock.lock_exclusive()?;

        let result = action(&state_path);
        let unlock_result = FileExt::unlock(&lock);

        match (result, unlock_result) {
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(MakespecError::Io(error)),
            (Ok(value), Ok(())) => Ok(value),
        }
    }

    fn lock_paths(&self) -> Result<(PathBuf, PathBuf)> {
        let configured_base = cache_directory()?;
        fs::create_dir_all(&configured_base)?;
        let base = fs::canonicalize(configured_base)?;
        let app_directory = base.join("makespec");
        fs::create_dir_all(&app_directory)?;
        let app_metadata = fs::symlink_metadata(&app_directory)?;

        if app_metadata.file_type().is_symlink() || !app_metadata.is_dir() {
            return Err(MakespecError::Invalid(format!(
                "unsafe Makespec cache directory {}",
                app_directory.display()
            )));
        }

        let directory = app_directory.join("locks");
        fs::create_dir_all(&directory)?;
        let metadata = fs::symlink_metadata(&directory)?;

        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(MakespecError::Invalid(format!(
                "unsafe lock directory {}",
                directory.display()
            )));
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
        }

        let identity = self
            .root
            .to_str()
            .ok_or_else(|| MakespecError::Invalid("project path must be valid Unicode".into()))?;
        let digest = Sha256::digest(identity.as_bytes());
        let name = format!("{digest:x}");

        Ok((
            directory.join(format!("{name}.lock")),
            directory.join(format!("{name}.state")),
        ))
    }

    fn document_paths(&self, require_directories: bool) -> Result<Vec<PathBuf>> {
        let docs_root = self.root.join("docs");

        if let Ok(metadata) = fs::symlink_metadata(&docs_root)
            && (metadata.file_type().is_symlink() || !metadata.is_dir())
        {
            return Err(MakespecError::Invalid(format!(
                "unsafe document root {}",
                docs_root.display()
            )));
        }

        let mut paths = Vec::new();

        for kind in [Kind::Adr, Kind::Rfc] {
            let directory = self.root.join("docs").join(kind.directory());

            match fs::symlink_metadata(&directory) {
                Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                    return Err(MakespecError::Invalid(format!(
                        "unsafe document directory {}",
                        directory.display()
                    )));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    if require_directories {
                        return Err(MakespecError::NotInitialized);
                    }

                    continue;
                }
                Err(error) => return Err(error.into()),
            }

            let mut entries = fs::read_dir(&directory)?.collect::<std::io::Result<Vec<_>>>()?;
            entries.sort_by_key(|entry| entry.file_name());

            for entry in entries {
                let path = entry.path();
                let file_type = entry.file_type()?;

                if file_type.is_symlink() || !file_type.is_file() {
                    return Err(MakespecError::Invalid(format!(
                        "unexpected non-regular file in document directory: {}",
                        path.display()
                    )));
                }

                if entry.file_name().to_str().is_some_and(|name| {
                    name.starts_with(STAGING_PREFIX) && name.ends_with(STAGING_SUFFIX)
                }) {
                    continue;
                }

                if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
                    return Err(MakespecError::Invalid(format!(
                        "unexpected file in document directory: {}",
                        path.display()
                    )));
                }

                paths.push(path);
            }
        }

        Ok(paths)
    }

    fn relative_path(&self, path: &Path) -> Result<String> {
        let relative = path.strip_prefix(&self.root).map_err(|_| {
            MakespecError::Invalid(format!("document path escapes project: {}", path.display()))
        })?;

        Ok(relative.to_string_lossy().replace('\\', "/"))
    }

    fn documents_in(&self, require_directories: bool) -> Result<Vec<Document>> {
        let mut documents = Vec::new();

        for path in self.document_paths(require_directories)? {
            let content = read_bounded(&path, MAX_DOCUMENT_BYTES)?;
            let relative = self.relative_path(&path)?;

            documents.push(parse_document(&content, relative)?);
        }

        documents.sort_by_key(|document| {
            let (prefix, number) = parse_id(&document.metadata.id).unwrap_or(("", 0));

            (prefix.to_string(), number)
        });

        Ok(documents)
    }

    fn ensure_safe_parent(&self, relative: &str, create: bool) -> Result<PathBuf> {
        let relative_path = safe_relative_path(relative)?;
        let mut parent = self.root.clone();
        let components: Vec<_> = relative_path.components().collect();

        for component in components.iter().take(components.len().saturating_sub(1)) {
            let Component::Normal(name) = component else {
                return Err(MakespecError::Invalid(format!(
                    "invalid project-relative path `{relative}`"
                )));
            };

            parent.push(name);
            match fs::symlink_metadata(&parent) {
                Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                    return Err(MakespecError::Invalid(format!(
                        "unsafe managed directory {}",
                        parent.display()
                    )));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound && create => {
                    fs::create_dir(&parent)?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }

        Ok(self.root.join(relative_path))
    }

    fn read_counters(&self, state_path: &Path) -> Result<Counters> {
        let content = match read_bounded(state_path, MAX_STATE_BYTES) {
            Ok(content) => content,
            Err(MakespecError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Counters::default());
            }
            Err(error) => return Err(error),
        };

        let mut counters = Counters::default();
        let mut seen = std::collections::BTreeSet::new();

        for line in content.lines() {
            let (kind, value) = line.split_once('=').ok_or_else(|| {
                MakespecError::Invalid("corrupt Makespec ID counter state".into())
            })?;
            let value = value
                .parse::<u32>()
                .map_err(|_| MakespecError::Invalid("corrupt Makespec ID counter state".into()))?;

            if !seen.insert(kind) {
                return Err(MakespecError::Invalid(
                    "duplicate entry in Makespec ID counter state".into(),
                ));
            }

            match kind {
                "adr" => counters.adr = value,
                "rfc" => counters.rfc = value,
                _ => {
                    return Err(MakespecError::Invalid(
                        "unknown entry in Makespec ID counter state".into(),
                    ));
                }
            }
        }

        if !content.is_empty() && seen.len() != 2 {
            return Err(MakespecError::Invalid(
                "incomplete Makespec ID counter state".into(),
            ));
        }

        Ok(counters)
    }

    fn write_counters(&self, state_path: &Path, counters: &Counters) -> Result<()> {
        let content = format!("adr={}\nrfc={}\n", counters.adr, counters.rfc);
        let temp = stage_file(state_path, &content)?;

        temp.persist(state_path)
            .map_err(|error| MakespecError::Io(error.error))?;

        Ok(())
    }

    fn raise_counters(&self, state_path: &Path, documents: &[Document]) -> Result<()> {
        let mut counters = self.read_counters(state_path)?;

        for document in documents {
            let (prefix, number) = parse_id(&document.metadata.id)?;
            let counter = if prefix == "ADR" {
                &mut counters.adr
            } else {
                &mut counters.rfc
            };

            *counter = (*counter).max(number);
        }

        self.write_counters(state_path, &counters)
    }
}

impl ProjectStore for FilesystemStore {
    fn initialize(&self, files: &[ProjectFile]) -> Result<()> {
        self.with_lock(|state_path| {
            let mut seen = std::collections::BTreeSet::new();
            let mut pending = Vec::new();

            for file in files {
                if !seen.insert(file.relative_path.as_str()) {
                    return Err(MakespecError::Invalid(format!(
                        "duplicate initialization path `{}`",
                        file.relative_path
                    )));
                }

                let destination = self.ensure_safe_parent(&file.relative_path, false)?;

                match fs::symlink_metadata(&destination) {
                    Ok(metadata) => {
                        if metadata.file_type().is_symlink() || !metadata.is_file() {
                            return Err(MakespecError::Invalid(format!(
                                "unsafe initialization destination {}",
                                destination.display()
                            )));
                        }

                        let existing = read_bounded(&destination, MAX_DOCUMENT_BYTES)?;

                        if existing == file.content {
                            continue;
                        }

                        if let Some(seed) = &file.seed {
                            let parsed = parse_document(&existing, file.relative_path.clone())?;

                            if parsed.metadata.id == seed.id
                                && parsed.metadata.title == seed.title
                                && parsed.metadata.status == Status::Accepted
                                && validate_document(&parsed).is_empty()
                            {
                                continue;
                            }
                        }

                        return Err(MakespecError::Invalid(format!(
                            "initialization conflict at {}",
                            destination.display()
                        )));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }

                pending.push(file);
            }

            let mut staged = Vec::new();

            for file in pending {
                let destination = self.ensure_safe_parent(&file.relative_path, true)?;
                let temp = stage_file(&destination, &file.content)?;

                staged.push((temp, destination, file.content.as_str()));
            }

            let mut published = Vec::new();

            for (temp, destination, expected) in staged {
                if let Err(error) = temp.persist_noclobber(&destination) {
                    let rollback_errors = rollback(&published);
                    let mut message = format!(
                        "cannot publish initialization file {}: {}",
                        destination.display(),
                        error.error
                    );

                    if !rollback_errors.is_empty() {
                        message.push_str("; rollback failed for ");
                        message.push_str(&rollback_errors.join(", "));
                    }

                    return Err(MakespecError::Invalid(message));
                }

                published.push((destination, expected.to_string()));
            }

            let documents = self.documents_in(false)?;

            self.raise_counters(state_path, &documents)
        })
    }

    fn existing_documents(&self) -> Result<Vec<Document>> {
        self.documents_in(false)
    }

    fn documents(&self) -> Result<Vec<Document>> {
        self.documents_in(true)
    }

    fn catalog(&self) -> Result<Vec<DocumentSummary>> {
        let mut summaries = Vec::new();

        for path in self.document_paths(true)? {
            let content = read_prefix(&path, MAX_FRONTMATTER_BYTES)?;
            let relative = self.relative_path(&path)?;
            let metadata = parse_metadata_prefix(&content, &relative)?;
            let (kind, _, _) = metadata.validate()?;

            summaries.push(DocumentSummary {
                kind,
                metadata,
                path: relative,
            });
        }

        summaries.sort_by_key(|summary| {
            let (prefix, number) = parse_id(&summary.metadata.id).unwrap_or(("", 0));

            (prefix.to_string(), number)
        });

        Ok(summaries)
    }

    fn document(&self, id: &str) -> Result<Document> {
        let summary = self
            .catalog()?
            .into_iter()
            .find(|summary| summary.metadata.id == id)
            .ok_or_else(|| MakespecError::NotFound(id.to_string()))?;
        let path = self.ensure_safe_parent(&summary.path, false)?;
        let content = read_bounded(&path, MAX_DOCUMENT_BYTES)?;

        parse_document(&content, summary.path)
    }

    fn create_draft(&self, new_document: NewDocument) -> Result<Document> {
        self.with_lock(|state_path| {
            let documents = self.catalog()?;
            let mut counters = self.read_counters(state_path)?;
            let max_number = documents
                .iter()
                .filter_map(|document| {
                    let (prefix, number) = parse_id(&document.metadata.id).ok()?;
                    (prefix == new_document.kind.id_prefix()).then_some(number)
                })
                .max()
                .unwrap_or(0);
            let current = if new_document.kind == Kind::Adr {
                &mut counters.adr
            } else {
                &mut counters.rfc
            };

            *current = (*current).max(max_number);
            let number = current.checked_add(1).ok_or_else(|| {
                MakespecError::Invalid("document ID sequence is exhausted".into())
            })?;

            *current = number;
            self.write_counters(state_path, &counters)?;

            let document = new_document.build(number, Status::Draft)?;
            let destination = self.ensure_safe_parent(&document.path, false)?;
            let temp = stage_file(&destination, &document.raw)?;

            temp.persist_noclobber(&destination).map_err(|error| {
                if error.error.kind() == std::io::ErrorKind::AlreadyExists {
                    MakespecError::AlreadyExists(destination.clone())
                } else {
                    MakespecError::Io(error.error)
                }
            })?;

            Ok(document)
        })
    }

    fn replace(&self, previous: &Document, replacement: &Document) -> Result<Document> {
        if previous.path != replacement.path
            || previous.metadata.id != replacement.metadata.id
            || previous.metadata.date != replacement.metadata.date
        {
            return Err(MakespecError::Invalid(
                "document updates must preserve path, ID, and date".into(),
            ));
        }

        self.with_lock(|_| {
            let path = self.ensure_safe_parent(&previous.path, false)?;
            let current = read_bounded(&path, MAX_DOCUMENT_BYTES)?;

            if current != previous.raw {
                return Err(MakespecError::ConcurrentModification(path));
            }

            let temp = stage_file(&path, &replacement.raw)?;

            if read_bounded(&path, MAX_DOCUMENT_BYTES)? != previous.raw {
                return Err(MakespecError::ConcurrentModification(path));
            }

            fn stage_file(destination: &Path, content: &str) -> Result<NamedTempFile> {
                let parent = destination
                    .parent()
                    .ok_or_else(|| MakespecError::Invalid("managed file has no parent".into()))?;
                let mut temp = Builder::new()
                    .prefix(STAGING_PREFIX)
                    .suffix(STAGING_SUFFIX)
                    .tempfile_in(parent)?;

                temp.write_all(content.as_bytes())?;
                temp.as_file().sync_all()?;

                Ok(temp)
            }

            temp.persist(&path)
                .map_err(|error| MakespecError::Io(error.error))?;

            parse_document(&replacement.raw, replacement.path.clone())
        })
    }
}

fn cache_directory() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        if let Some(path) = std::env::var_os("LOCALAPPDATA") {
            return Ok(PathBuf::from(path));
        }
    }

    if let Some(path) = std::env::var_os("XDG_CACHE_HOME") {
        return Ok(PathBuf::from(path));
    }

    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".cache"))
        .ok_or_else(|| MakespecError::Invalid("could not locate the user cache directory".into()))
}

fn safe_relative_path(path: &str) -> Result<PathBuf> {
    let path = Path::new(path);

    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.to_string_lossy().contains('\\')
        || path.to_string_lossy().contains(':')
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(MakespecError::Invalid(format!(
            "unsafe project-relative path `{}`",
            path.display()
        )));
    }

    Ok(path.to_path_buf())
}

fn regular_file_metadata(path: &Path) -> Result<fs::Metadata> {
    let metadata = fs::symlink_metadata(path)?;

    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(MakespecError::Invalid(format!(
            "refusing to read non-regular file {}",
            path.display()
        )));
    }

    Ok(metadata)
}

fn read_bounded(path: &Path, limit: u64) -> Result<String> {
    let metadata = regular_file_metadata(path)?;

    if metadata.len() > limit {
        return Err(MakespecError::Invalid(format!(
            "file exceeds {limit} bytes: {}",
            path.display()
        )));
    }

    let file = File::open(path)?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    Read::take(file, limit + 1).read_to_end(&mut bytes)?;

    if bytes.len() as u64 > limit {
        return Err(MakespecError::Invalid(format!(
            "file exceeds {limit} bytes: {}",
            path.display()
        )));
    }

    String::from_utf8(bytes)
        .map_err(|_| MakespecError::Invalid(format!("file is not UTF-8: {}", path.display())))
}

fn read_prefix(path: &Path, limit: u64) -> Result<String> {
    let metadata = regular_file_metadata(path)?;
    let file = File::open(path)?;
    let mut bytes = Vec::with_capacity(metadata.len().min(limit) as usize);
    Read::take(file, limit).read_to_end(&mut bytes)?;
    let delimiter = if bytes.starts_with(b"---\r\n") {
        b"\r\n---\r\n".as_slice()
    } else {
        b"\n---\n".as_slice()
    };
    let Some(end) = bytes
        .windows(delimiter.len())
        .position(|window| window == delimiter)
        .map(|index| index + delimiter.len())
    else {
        return Err(MakespecError::Invalid(format!(
            "missing or oversized YAML front matter (limit: {limit} bytes): {}",
            path.display()
        )));
    };
    bytes.truncate(end);

    String::from_utf8(bytes)
        .map_err(|_| MakespecError::Invalid(format!("file is not UTF-8: {}", path.display())))
}

fn rollback(published: &[(PathBuf, String)]) -> Vec<String> {
    published
        .iter()
        .filter_map(|(path, expected)| match fs::read_to_string(path) {
            Ok(content) if content == *expected => fs::remove_file(path)
                .err()
                .map(|error| format!("{} ({error})", path.display())),
            Ok(_) => Some(format!("{} (content changed)", path.display())),
            Err(error) => Some(format!("{} ({error})", path.display())),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{adapters::embedded, core::service};

    const ADR_BODY: &str = "## Context\n\nA focused decision is needed.\n\n## Decision\n\nUse the proposed approach.\n\n## Consequences\n\nThe choice is documented.";

    fn initialized_store() -> (tempfile::TempDir, FilesystemStore) {
        let directory = tempfile::tempdir().unwrap();
        let store = FilesystemStore::new(directory.path()).unwrap();
        service::initialize(&store, "2026-11-09", embedded::SKILL_FILES).unwrap();

        (directory, store)
    }

    #[test]
    fn initialization_is_idempotent_and_seed_documents_check() {
        let (_directory, store) = initialized_store();
        service::initialize(&store, "2026-11-09", embedded::SKILL_FILES).unwrap();

        assert!(service::check(&store).unwrap().is_empty());
        assert_eq!(service::list(&store).unwrap().len(), 2);
    }

    #[test]
    fn creates_unique_documents_under_concurrent_calls() {
        let (_directory, store) = initialized_store();
        let store = Arc::new(store);
        let workers: Vec<_> = (0..8)
            .map(|index| {
                let store = Arc::clone(&store);

                std::thread::spawn(move || {
                    service::create(
                        store.as_ref(),
                        Kind::Adr,
                        format!("Concurrent decision {index}"),
                        ADR_BODY.to_string(),
                        "2026-11-09",
                    )
                    .unwrap()
                    .metadata
                    .id
                })
            })
            .collect();
        let ids: std::collections::BTreeSet<_> = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect();

        assert_eq!(ids.len(), 8);
        assert!(ids.contains("ADR-0002"));
        assert!(ids.contains("ADR-0009"));
    }

    #[test]
    fn creation_conflicts_preserve_existing_files_and_reserved_ids() {
        let (directory, store) = initialized_store();
        let destination = directory.path().join("docs/adrs/0002-collision.md");
        let existing = store.document("ADR-0001").unwrap().raw;

        fs::write(&destination, &existing).unwrap();

        assert!(
            service::check(&store)
                .unwrap()
                .iter()
                .any(|error| error.contains("duplicate"))
        );
        assert!(matches!(
            service::create(
                &store,
                Kind::Adr,
                "Collision".into(),
                ADR_BODY.into(),
                "2026-11-09"
            ),
            Err(MakespecError::AlreadyExists(_))
        ));
        assert_eq!(fs::read_to_string(&destination).unwrap(), existing);
        assert_eq!(
            fs::read_dir(destination.parent().unwrap()).unwrap().count(),
            2
        );

        fs::remove_file(destination).unwrap();

        let created = service::create(
            &store,
            Kind::Adr,
            "After collision".into(),
            ADR_BODY.into(),
            "2026-11-09",
        )
        .unwrap();

        assert_eq!(created.metadata.id, "ADR-0003");
    }

    #[test]
    fn superseding_requires_an_accepted_same_kind_replacement() {
        let (_directory, store) = initialized_store();
        let replacement = service::create(
            &store,
            Kind::Adr,
            "A replacement decision".into(),
            ADR_BODY.into(),
            "2026-11-09",
        )
        .unwrap();

        service::change_status(&store, &replacement.metadata.id, Status::Proposed, None).unwrap();
        service::change_status(&store, &replacement.metadata.id, Status::Accepted, None).unwrap();

        let superseded = service::change_status(
            &store,
            "ADR-0001",
            Status::Superseded,
            Some(&replacement.metadata.id),
        )
        .unwrap();

        assert_eq!(
            superseded.metadata.superseded_by.as_deref(),
            Some(replacement.metadata.id.as_str())
        );
        assert!(service::check(&store).unwrap().is_empty());
    }

    #[test]
    fn catalog_lists_metadata_without_loading_large_bodies() {
        let (directory, store) = initialized_store();
        let path = directory.path().join("docs/adrs/0002-large-document.md");
        let content = format!(
            "---\nid: ADR-0002\ntitle: Large document\ndate: 2026-11-09\nstatus: proposed\n---\n\n{}",
            "x".repeat(MAX_DOCUMENT_BYTES as usize + 1)
        );

        fs::write(path, content).unwrap();

        assert_eq!(service::list(&store).unwrap().len(), 3);
        assert!(store.document("ADR-0002").is_err());
    }

    #[test]
    fn discovery_requires_both_directories_except_during_initialization() {
        let directory = tempfile::tempdir().unwrap();
        let store = FilesystemStore::new(directory.path()).unwrap();

        fs::create_dir_all(directory.path().join("docs/adrs")).unwrap();

        assert!(store.existing_documents().unwrap().is_empty());
        assert!(matches!(
            store.documents(),
            Err(MakespecError::NotInitialized)
        ));
        assert!(matches!(
            store.catalog(),
            Err(MakespecError::NotInitialized)
        ));

        fs::create_dir(directory.path().join("docs/rfcs")).unwrap();

        assert!(store.documents().unwrap().is_empty());
        assert!(store.catalog().unwrap().is_empty());
    }

    #[test]
    fn discovery_ignores_staging_files_but_rejects_other_entries() {
        let (directory, store) = initialized_store();
        let unexpected = directory.path().join("docs/adrs/unexpected.txt");
        let _staged = stage_file(&unexpected, "unfinished content").unwrap();

        assert_eq!(store.documents().unwrap().len(), 2);
        assert_eq!(store.catalog().unwrap().len(), 2);

        fs::write(&unexpected, "not a document").unwrap();

        assert!(store.documents().is_err());
        assert!(store.catalog().is_err());

        fs::remove_file(&unexpected).unwrap();
        fs::create_dir(unexpected.with_extension("md")).unwrap();

        assert!(store.documents().is_err());
        assert!(store.catalog().is_err());
    }

    #[cfg(unix)]
    #[test]
    fn initialization_rejects_symlinked_document_directory() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();

        fs::create_dir(directory.path().join("docs")).unwrap();
        symlink(outside.path(), directory.path().join("docs").join("adrs")).unwrap();

        let store = FilesystemStore::new(directory.path()).unwrap();
        let files = [ProjectFile {
            relative_path: "docs/adrs/0001-example.md".into(),
            content: "document".into(),
            seed: None,
        }];

        assert!(store.initialize(&files).is_err());
        assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    }
}
