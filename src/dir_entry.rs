use std::ffi::OsStr;
use std::{
    fs::{FileType, Metadata},
    path::{Path, PathBuf},
};
use std::borrow::Cow;

use once_cell::unsync::OnceCell;
use regex::bytes::Regex;

use crate::filesystem;

enum DirEntryInner {
    Normal(ignore::DirEntry),
    BrokenSymlink(PathBuf),
}

pub struct DirEntry {
    inner: DirEntryInner,
    metadata: OnceCell<Option<Metadata>>,
    matches: Vec<String>,
}

impl DirEntry {
    #[inline]
    pub fn normal(e: ignore::DirEntry) -> Self {
        Self {
            inner: DirEntryInner::Normal(e),
            metadata: OnceCell::new(),
            matches: Vec::new(),
        }
    }

    pub fn broken_symlink(path: PathBuf) -> Self {
        Self {
            inner: DirEntryInner::BrokenSymlink(path),
            metadata: OnceCell::new(),
            matches: Vec::new(),
        }
    }

    pub fn path(&self) -> &Path {
        match &self.inner {
            DirEntryInner::Normal(e) => e.path(),
            DirEntryInner::BrokenSymlink(pathbuf) => pathbuf.as_path(),
        }
    }

    pub fn matches(&self) -> &Vec<String> {
        &&self.matches
    }

    pub fn into_path(self) -> PathBuf {
        match self.inner {
            DirEntryInner::Normal(e) => e.into_path(),
            DirEntryInner::BrokenSymlink(p) => p,
        }
    }

    pub fn file_type(&self) -> Option<FileType> {
        match &self.inner {
            DirEntryInner::Normal(e) => e.file_type(),
            DirEntryInner::BrokenSymlink(_) => self.metadata().map(|m| m.file_type()),
        }
    }

    pub fn metadata(&self) -> Option<&Metadata> {
        self.metadata
            .get_or_init(|| match &self.inner {
                DirEntryInner::Normal(e) => e.metadata().ok(),
                DirEntryInner::BrokenSymlink(path) => path.symlink_metadata().ok(),
            })
            .as_ref()
    }

    pub fn depth(&self) -> Option<usize> {
        match &self.inner {
            DirEntryInner::Normal(e) => Some(e.depth()),
            DirEntryInner::BrokenSymlink(_) => None,
        }
    }

    pub fn is_match(&mut self, pattern: &Regex, search_full_path: bool) -> bool {
        let search_str = self.get_search_str(search_full_path);
        let search_res = filesystem::osstr_to_bytes(search_str.as_ref());

        let mut parts:Vec<String> = Vec::new();
        for matched in pattern.captures_iter(&search_res) {
            for (j, group) in matched.iter().enumerate() {
                if j > 0 {
                    if let Some(value) = group {
                        let cap = value.as_bytes();
                        let text = std::str::from_utf8(cap).unwrap();
                        let part = text.to_string();
                        parts.push(part);
                    }
                }
            }
        }
        self.matches = parts;
        self.matches.len() > 0
    }

    fn get_search_str(&self, search_full_path: bool) -> Cow<OsStr> {
        let entry_path = self.path();

        let search_str: Cow<OsStr> = if search_full_path {
            let path_abs_buf = filesystem::path_absolute_form(entry_path)
                .expect("Retrieving absolute path succeeds");
            Cow::Owned(path_abs_buf.as_os_str().to_os_string())
        } else {
            match entry_path.file_name() {
                Some(filename) => Cow::Borrowed(filename),
                None => unreachable!(
                    "Encountered file system entry without a file name. This should only \
                        happen for paths like 'foo/bar/..' or '/' which are not supposed to \
                        appear in a file system traversal."
                ),
            }
        };
        search_str
    }
}

impl PartialEq for DirEntry {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.path() == other.path()
    }
}
impl Eq for DirEntry {}

impl PartialOrd for DirEntry {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.path().partial_cmp(other.path())
    }
}

impl Ord for DirEntry {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.path().cmp(other.path())
    }
}
