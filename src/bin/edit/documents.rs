// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use std::collections::LinkedList;
use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};

use edit::buffer::{RcTextBuffer, TextBuffer};
use edit::helpers::{CoordType, Point};
use edit::simd::memrchr2;
use edit::{apperr, path, sys};
use edit::syntax::{SyntaxHighlighter, FileType};

use crate::state::DisplayablePathBuf;

pub struct Document {
    pub buffer: RcTextBuffer,
    pub path: Option<PathBuf>,
    pub dir: Option<DisplayablePathBuf>,
    pub filename: String,
    pub file_id: Option<sys::FileId>,
    pub new_file_counter: usize,
    pub syntax_highlighter: Option<SyntaxHighlighter>,
    pub file_type: FileType,
}

impl Document {
    pub fn save(&mut self, new_path: Option<PathBuf>) -> apperr::Result<()> {
        let path = new_path.as_deref().unwrap_or_else(|| self.path.as_ref().unwrap().as_path());
        let mut file = DocumentManager::open_for_writing(path)?;

        {
            let mut tb = self.buffer.borrow_mut();
            tb.write_file(&mut file)?;
        }

        if let Ok(id) = sys::file_id(None, path) {
            self.file_id = Some(id);
        }

        if let Some(path) = new_path {
            self.set_path(path);
        }

        Ok(())
    }

    pub fn reread(&mut self, encoding: Option<&'static str>) -> apperr::Result<()> {
        let path = self.path.as_ref().unwrap().as_path();
        let mut file = DocumentManager::open_for_reading(path)?;

        {
            let mut tb = self.buffer.borrow_mut();
            tb.read_file_with_path(&mut file, path, encoding)?;
        }

        if let Ok(id) = sys::file_id(None, path) {
            self.file_id = Some(id);
        }

        Ok(())
    }

    fn set_path(&mut self, path: PathBuf) {
        let filename = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let dir = path.parent().map(ToOwned::to_owned).unwrap_or_default();
        self.filename = filename.clone();
        self.dir = Some(DisplayablePathBuf::from_path(dir));
        self.path = Some(path.clone());
        
        // Detect file type and initialize syntax highlighting
        self.file_type = SyntaxHighlighter::detect_file_type(
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
        );
        
        // Set the file type in the buffer for smart indentation
        {
            let mut buffer = self.buffer.borrow_mut();
            buffer.set_file_type(self.file_type);
        }
        
        // Create syntax highlighter for all file types
        let mut highlighter = SyntaxHighlighter::new();
        // Try to set Studio Ghibli theme
        highlighter.set_ghibli_theme();
        self.syntax_highlighter = Some(highlighter);
        
        self.update_file_mode();
    }

    fn update_file_mode(&mut self) {
        let mut tb = self.buffer.borrow_mut();
        tb.set_ruler(if self.filename == "COMMIT_EDITMSG" { 72 } else { 0 });
    }
}

#[derive(Default)]
pub struct DocumentManager {
    list: LinkedList<Document>,
}

impl DocumentManager {
    #[inline]
    pub fn len(&self) -> usize {
        self.list.len()
    }

    #[inline]
    pub fn active(&self) -> Option<&Document> {
        self.list.front()
    }

    #[inline]
    pub fn active_mut(&mut self) -> Option<&mut Document> {
        self.list.front_mut()
    }

    #[inline]
    pub fn update_active<F: FnMut(&Document) -> bool>(&mut self, mut func: F) -> bool {
        let mut cursor = self.list.cursor_front_mut();
        while let Some(doc) = cursor.current() {
            if func(doc) {
                let list = cursor.remove_current_as_list().unwrap();
                self.list.cursor_front_mut().splice_before(list);
                return true;
            }
            cursor.move_next();
        }
        false
    }

    pub fn remove_active(&mut self) {
        self.list.pop_front();
    }

    /// Get all documents as a vector (for tab display)
    pub fn all_documents(&self) -> Vec<&Document> {
        self.list.iter().collect()
    }

    /// Get the index of the currently active document
    pub fn active_index(&self) -> Option<usize> {
        if self.list.is_empty() {
            None
        } else {
            Some(0) // Active document is always at the front
        }
    }

    /// Switch to document at the given index
    pub fn switch_to_index(&mut self, index: usize) -> bool {
        if index >= self.list.len() {
            return false;
        }

        if index == 0 {
            return true; // Already at the front
        }

        // Move the document at `index` to the front
        let mut cursor = self.list.cursor_front_mut();
        for _ in 0..index {
            cursor.move_next();
        }

        if let Some(doc_list) = cursor.remove_current_as_list() {
            self.list.cursor_front_mut().splice_before(doc_list);
            true
        } else {
            false
        }
    }

    /// Switch to the next document (cycle forward)
    pub fn switch_to_next(&mut self) -> bool {
        if self.list.len() <= 1 {
            return false;
        }

        // Move the front document to the back
        if let Some(doc) = self.list.pop_front() {
            self.list.push_back(doc);
            true
        } else {
            false
        }
    }

    /// Switch to the previous document (cycle backward)
    pub fn switch_to_previous(&mut self) -> bool {
        if self.list.len() <= 1 {
            return false;
        }

        // Move the back document to the front
        if let Some(doc) = self.list.pop_back() {
            self.list.push_front(doc);
            true
        } else {
            false
        }
    }

    pub fn add_untitled(&mut self) -> apperr::Result<&mut Document> {
        let buffer = Self::create_buffer()?;
        let mut doc = Document {
            buffer,
            path: None,
            dir: Default::default(),
            filename: Default::default(),
            file_id: None,
            new_file_counter: 0,
            syntax_highlighter: None,
            file_type: FileType::Plain,
        };
        self.gen_untitled_name(&mut doc);

        self.list.push_front(doc);
        Ok(self.list.front_mut().unwrap())
    }

    pub fn gen_untitled_name(&self, doc: &mut Document) {
        let mut new_file_counter = 0;
        for doc in &self.list {
            new_file_counter = new_file_counter.max(doc.new_file_counter);
        }
        new_file_counter += 1;

        doc.filename = format!("Untitled-{new_file_counter}.txt");
        doc.new_file_counter = new_file_counter;
    }

    pub fn add_file_path(&mut self, path: &Path) -> apperr::Result<&mut Document> {
        let (path, goto) = Self::parse_filename_goto(path);
        let path = path::normalize(path);

        let mut file = match Self::open_for_reading(&path) {
            Ok(file) => Some(file),
            Err(err) if sys::apperr_is_not_found(err) => None,
            Err(err) => return Err(err),
        };

        let file_id = if file.is_some() { Some(sys::file_id(file.as_ref(), &path)?) } else { None };

        // Check if the file is already open.
        if file_id.is_some() && self.update_active(|doc| doc.file_id == file_id) {
            let doc = self.active_mut().unwrap();
            if let Some(goto) = goto {
                doc.buffer.borrow_mut().cursor_move_to_logical(goto);
            }
            return Ok(doc);
        }

        let buffer = Self::create_buffer()?;
        {
            if let Some(file) = &mut file {
                let mut tb = buffer.borrow_mut();
                tb.read_file_with_path(file, &path, None)?;

                if let Some(goto) = goto
                    && goto != Default::default()
                {
                    tb.cursor_move_to_logical(goto);
                }
            }
        }

        let mut doc = Document {
            buffer,
            path: None,
            dir: None,
            filename: Default::default(),
            file_id,
            new_file_counter: 0,
            syntax_highlighter: None,
            file_type: FileType::Plain,
        };
        doc.set_path(path);

        if let Some(active) = self.active()
            && active.path.is_none()
            && active.file_id.is_none()
            && !active.buffer.borrow().is_dirty()
        {
            // If the current document is a pristine Untitled document with no
            // name and no ID, replace it with the new document.
            self.remove_active();
        }

        self.list.push_front(doc);
        Ok(self.list.front_mut().unwrap())
    }

    pub fn open_for_reading(path: &Path) -> apperr::Result<File> {
        File::open(path).map_err(apperr::Error::from)
    }

    pub fn open_for_writing(path: &Path) -> apperr::Result<File> {
        File::create(path).map_err(apperr::Error::from)
    }

    fn create_buffer() -> apperr::Result<RcTextBuffer> {
        let buffer = TextBuffer::new_rc(false)?;
        {
            let mut tb = buffer.borrow_mut();
            tb.set_insert_final_newline(!cfg!(windows)); // As mandated by POSIX.
            tb.set_margin_enabled(true);
            tb.set_line_highlight_enabled(true);
        }
        Ok(buffer)
    }

    // Parse a filename in the form of "filename:line:char".
    // Returns the position of the first colon and the line/char coordinates.
    fn parse_filename_goto(path: &Path) -> (&Path, Option<Point>) {
        fn parse(s: &[u8]) -> Option<CoordType> {
            if s.is_empty() {
                return None;
            }

            let mut num: CoordType = 0;
            for &b in s {
                if !b.is_ascii_digit() {
                    return None;
                }
                let digit = (b - b'0') as CoordType;
                num = num.checked_mul(10)?.checked_add(digit)?;
            }
            Some(num)
        }

        let bytes = path.as_os_str().as_encoded_bytes();
        let colend = match memrchr2(b':', b':', bytes, bytes.len()) {
            // Reject filenames that would result in an empty filename after stripping off the :line:char suffix.
            // For instance, a filename like ":123:456" will not be processed by this function.
            Some(colend) if colend > 0 => colend,
            _ => return (path, None),
        };

        let last = match parse(&bytes[colend + 1..]) {
            Some(last) => last,
            None => return (path, None),
        };
        let last = (last - 1).max(0);
        let mut len = colend;
        let mut goto = Point { x: 0, y: last };

        if let Some(colbeg) = memrchr2(b':', b':', bytes, colend) {
            // Same here: Don't allow empty filenames.
            if colbeg != 0
                && let Some(first) = parse(&bytes[colbeg + 1..colend])
            {
                let first = (first - 1).max(0);
                len = colbeg;
                goto = Point { x: last, y: first };
            }
        }

        // Strip off the :line:char suffix.
        let path = &bytes[..len];
        let path = unsafe { OsStr::from_encoded_bytes_unchecked(path) };
        let path = Path::new(path);
        (path, Some(goto))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_last_numbers() {
        fn parse(s: &str) -> (&str, Option<Point>) {
            let (p, g) = DocumentManager::parse_filename_goto(Path::new(s));
            (p.to_str().unwrap(), g)
        }

        assert_eq!(parse("123"), ("123", None));
        assert_eq!(parse("abc"), ("abc", None));
        assert_eq!(parse(":123"), (":123", None));
        assert_eq!(parse("abc:123"), ("abc", Some(Point { x: 0, y: 122 })));
        assert_eq!(parse("45:123"), ("45", Some(Point { x: 0, y: 122 })));
        assert_eq!(parse(":45:123"), (":45", Some(Point { x: 0, y: 122 })));
        assert_eq!(parse("abc:45:123"), ("abc", Some(Point { x: 122, y: 44 })));
        assert_eq!(parse("abc:def:123"), ("abc:def", Some(Point { x: 0, y: 122 })));
        assert_eq!(parse("1:2:3"), ("1", Some(Point { x: 2, y: 1 })));
        assert_eq!(parse("::3"), (":", Some(Point { x: 0, y: 2 })));
        assert_eq!(parse("1::3"), ("1:", Some(Point { x: 0, y: 2 })));
        assert_eq!(parse(""), ("", None));
        assert_eq!(parse(":"), (":", None));
        assert_eq!(parse("::"), ("::", None));
        assert_eq!(parse("a:1"), ("a", Some(Point { x: 0, y: 0 })));
        assert_eq!(parse("1:a"), ("1:a", None));
        assert_eq!(parse("file.txt:10"), ("file.txt", Some(Point { x: 0, y: 9 })));
        assert_eq!(parse("file.txt:10:5"), ("file.txt", Some(Point { x: 4, y: 9 })));
    }
}
