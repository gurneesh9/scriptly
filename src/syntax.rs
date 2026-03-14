use std::collections::HashMap;
use std::path::Path;
use std::ops::Range;
use std::ffi::OsStr;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style, Color};
use syntect::easy::HighlightLines;
use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileType {
    Plain,
    Python,
    Rust,
    JavaScript,
    TypeScript,
    HTML,
    CSS,
    Dockerfile,
    YAML,
    JSON,
    Markdown,
    Shell,
    Go,
    Java,
    C,
    Cpp,
    CSharp,
    Ruby,
    PHP,
    SQL,
    TOML,
    XML,
    Makefile,
    Swift,
    Scala,
    Lua,
    PowerShell,
    Perl,
    Less,
    Diff,
    Kotlin,
}

pub struct HighlightedText<'a> {
    pub text: &'a str,
    pub styles: Vec<(Style, Range<usize>)>,
}

pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    current_theme: String,
    highlight_cache: HashMap<(String, usize), Vec<(Style, String)>>,
}

impl SyntaxHighlighter {
    const MAX_CACHE_SIZE: usize = 1000;

    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();

        Self {
            syntax_set,
            theme_set,
            current_theme: "base16-mocha.dark".to_string(),
            highlight_cache: HashMap::new(),
        }
    }

    pub fn set_ghibli_theme(&mut self) -> bool {
        let ghibli_themes = vec![
            "base16-mocha.dark",
            "base16-eighties.dark",
            "base16-tomorrow.dark",
            "base16-atelier-forest.dark",
            "base16-atelier-heath.dark",
            "Monokai",
            "Solarized (dark)",
        ];

        for theme_name in ghibli_themes {
            if self.set_theme(theme_name) {
                println!("🌳 Applied Studio Ghibli theme: {}", theme_name);
                return true;
            }
        }

        println!("Available themes:");
        for theme in self.available_themes() {
            println!("  - {}", theme);
        }

        false
    }

    pub fn detect_file_type(filename: &str) -> FileType {
        let basename = Path::new(filename)
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or(filename);

        // Special case: exact filename or prefix matches
        match basename {
            "Dockerfile" | "dockerfile" => return FileType::Dockerfile,
            "Makefile" | "makefile" | "GNUmakefile" | "GNUMakefile" | "BSDmakefile" => return FileType::Makefile,
            "Rakefile" | "Gemfile" | "Vagrantfile" | "Capfile" | "Guardfile" | "Brewfile" => return FileType::Ruby,
            _ => {}
        }

        if basename.starts_with("Dockerfile.") {
            return FileType::Dockerfile;
        }

        // Common YAML filenames without extensions
        match filename.to_lowercase().as_str() {
            ".travis.yml" | "docker-compose.yml" | "docker-compose.yaml" |
            ".gitlab-ci.yml" | "appveyor.yml" | "circle.yml" | "wercker.yml" |
            "ansible.yml" | "playbook.yml" | "site.yml" => return FileType::YAML,
            _ => {}
        }

        if filename.starts_with(".github/") && (filename.ends_with(".yml") || filename.ends_with(".yaml")) {
            return FileType::YAML;
        }

        match Path::new(filename).extension().and_then(OsStr::to_str) {
            Some("py") | Some("py3") | Some("pyw") | Some("pyi") => FileType::Python,
            Some("rs") => FileType::Rust,
            Some("js") | Some("mjs") | Some("cjs") => FileType::JavaScript,
            Some("jsx") => FileType::JavaScript,
            Some("ts") => FileType::TypeScript,
            Some("tsx") => FileType::TypeScript,
            Some("html") | Some("htm") | Some("xhtml") => FileType::HTML,
            Some("css") => FileType::CSS,
            Some("yaml") | Some("yml") => FileType::YAML,
            Some("json") | Some("jsonc") | Some("json5") => FileType::JSON,
            Some("md") | Some("markdown") | Some("mdown") | Some("markdn") => FileType::Markdown,
            Some("sh") | Some("bash") | Some("zsh") | Some("ksh") | Some("fish") => FileType::Shell,
            Some("go") => FileType::Go,
            Some("java") => FileType::Java,
            Some("c") | Some("h") => FileType::C,
            Some("cpp") | Some("cc") | Some("cxx") | Some("c++") |
            Some("hpp") | Some("hh") | Some("hxx") | Some("h++") => FileType::Cpp,
            Some("cs") | Some("csx") => FileType::CSharp,
            Some("rb") | Some("rbx") | Some("rjs") | Some("rake") |
            Some("gemspec") | Some("ru") => FileType::Ruby,
            Some("php") | Some("php3") | Some("php4") | Some("php5") |
            Some("php7") | Some("phtml") => FileType::PHP,
            Some("sql") => FileType::SQL,
            Some("toml") => FileType::TOML,
            Some("xml") | Some("xsd") | Some("xsl") | Some("xslt") |
            Some("svg") | Some("plist") => FileType::XML,
            Some("mk") | Some("mak") => FileType::Makefile,
            Some("swift") => FileType::Swift,
            Some("scala") | Some("sbt") | Some("sc") => FileType::Scala,
            Some("lua") => FileType::Lua,
            Some("ps1") | Some("psm1") | Some("psd1") | Some("pssc") => FileType::PowerShell,
            Some("pl") | Some("pm") | Some("pod") | Some("t") => FileType::Perl,
            Some("less") => FileType::Less,
            Some("diff") | Some("patch") => FileType::Diff,
            Some("kt") | Some("kts") => FileType::Kotlin,
            _ => FileType::Plain,
        }
    }

    /// Resolve the syntect SyntaxReference for a given FileType.
    fn resolve_syntax<'a>(syntax_set: &'a SyntaxSet, file_type: FileType) -> &'a syntect::parsing::SyntaxReference {
        let plain = || syntax_set.find_syntax_plain_text();
        match file_type {
            FileType::Plain => plain(),
            FileType::Python => syntax_set.find_syntax_by_extension("py").unwrap_or_else(plain),
            FileType::Rust => syntax_set.find_syntax_by_extension("rs").unwrap_or_else(plain),
            FileType::JavaScript => syntax_set.find_syntax_by_extension("js").unwrap_or_else(plain),
            FileType::TypeScript => {
                syntax_set.find_syntax_by_extension("ts")
                    .or_else(|| syntax_set.find_syntax_by_name("TypeScript"))
                    .or_else(|| syntax_set.find_syntax_by_name("TypeScript (JavaScript)"))
                    .or_else(|| syntax_set.find_syntax_by_extension("js"))
                    .unwrap_or_else(plain)
            },
            FileType::HTML => syntax_set.find_syntax_by_extension("html").unwrap_or_else(plain),
            FileType::CSS => syntax_set.find_syntax_by_extension("css").unwrap_or_else(plain),
            FileType::Dockerfile => {
                syntax_set.find_syntax_by_name("Dockerfile")
                    .or_else(|| syntax_set.find_syntax_by_name("Docker"))
                    .unwrap_or_else(plain)
            },
            FileType::YAML => {
                syntax_set.find_syntax_by_extension("yaml")
                    .or_else(|| syntax_set.find_syntax_by_extension("yml"))
                    .or_else(|| syntax_set.find_syntax_by_name("YAML"))
                    .or_else(|| syntax_set.find_syntax_by_extension("json"))
                    .unwrap_or_else(plain)
            },
            FileType::JSON => {
                syntax_set.find_syntax_by_extension("json")
                    .or_else(|| syntax_set.find_syntax_by_name("JSON"))
                    .unwrap_or_else(plain)
            },
            FileType::Markdown => {
                syntax_set.find_syntax_by_extension("md")
                    .or_else(|| syntax_set.find_syntax_by_name("Markdown"))
                    .or_else(|| syntax_set.find_syntax_by_name("Markdown GFM"))
                    .unwrap_or_else(plain)
            },
            FileType::Shell => {
                syntax_set.find_syntax_by_extension("sh")
                    .or_else(|| syntax_set.find_syntax_by_name("Shell Script"))
                    .or_else(|| syntax_set.find_syntax_by_name("Bash"))
                    .or_else(|| syntax_set.find_syntax_by_name("Shell"))
                    .unwrap_or_else(plain)
            },
            FileType::Go => {
                syntax_set.find_syntax_by_extension("go")
                    .or_else(|| syntax_set.find_syntax_by_name("Go"))
                    .unwrap_or_else(plain)
            },
            FileType::Java => {
                syntax_set.find_syntax_by_extension("java")
                    .or_else(|| syntax_set.find_syntax_by_name("Java"))
                    .unwrap_or_else(plain)
            },
            FileType::C => {
                syntax_set.find_syntax_by_extension("c")
                    .or_else(|| syntax_set.find_syntax_by_name("C"))
                    .unwrap_or_else(plain)
            },
            FileType::Cpp => {
                syntax_set.find_syntax_by_extension("cpp")
                    .or_else(|| syntax_set.find_syntax_by_extension("cc"))
                    .or_else(|| syntax_set.find_syntax_by_name("C++"))
                    .unwrap_or_else(plain)
            },
            FileType::CSharp => {
                syntax_set.find_syntax_by_extension("cs")
                    .or_else(|| syntax_set.find_syntax_by_name("C#"))
                    .unwrap_or_else(plain)
            },
            FileType::Ruby => {
                syntax_set.find_syntax_by_extension("rb")
                    .or_else(|| syntax_set.find_syntax_by_name("Ruby"))
                    .unwrap_or_else(plain)
            },
            FileType::PHP => {
                syntax_set.find_syntax_by_extension("php")
                    .or_else(|| syntax_set.find_syntax_by_name("PHP"))
                    .unwrap_or_else(plain)
            },
            FileType::SQL => {
                syntax_set.find_syntax_by_extension("sql")
                    .or_else(|| syntax_set.find_syntax_by_name("SQL"))
                    .unwrap_or_else(plain)
            },
            FileType::TOML => {
                syntax_set.find_syntax_by_extension("toml")
                    .or_else(|| syntax_set.find_syntax_by_name("TOML"))
                    .unwrap_or_else(plain)
            },
            FileType::XML => {
                syntax_set.find_syntax_by_extension("xml")
                    .or_else(|| syntax_set.find_syntax_by_name("XML"))
                    .unwrap_or_else(plain)
            },
            FileType::Makefile => {
                syntax_set.find_syntax_by_name("Makefile")
                    .or_else(|| syntax_set.find_syntax_by_name("Make")
                    .or_else(|| syntax_set.find_syntax_by_extension("mk")))
                    .unwrap_or_else(plain)
            },
            FileType::Swift => {
                syntax_set.find_syntax_by_extension("swift")
                    .or_else(|| syntax_set.find_syntax_by_name("Swift"))
                    .unwrap_or_else(plain)
            },
            FileType::Scala => {
                syntax_set.find_syntax_by_extension("scala")
                    .or_else(|| syntax_set.find_syntax_by_name("Scala"))
                    .unwrap_or_else(plain)
            },
            FileType::Lua => {
                syntax_set.find_syntax_by_extension("lua")
                    .or_else(|| syntax_set.find_syntax_by_name("Lua"))
                    .unwrap_or_else(plain)
            },
            FileType::PowerShell => {
                syntax_set.find_syntax_by_extension("ps1")
                    .or_else(|| syntax_set.find_syntax_by_name("PowerShell"))
                    .or_else(|| syntax_set.find_syntax_by_name("Powershell"))
                    .unwrap_or_else(plain)
            },
            FileType::Perl => {
                syntax_set.find_syntax_by_extension("pl")
                    .or_else(|| syntax_set.find_syntax_by_name("Perl"))
                    .unwrap_or_else(plain)
            },
            FileType::Less => {
                syntax_set.find_syntax_by_extension("less")
                    .or_else(|| syntax_set.find_syntax_by_name("Less"))
                    // Fall back to CSS if Less isn't available
                    .or_else(|| syntax_set.find_syntax_by_extension("css"))
                    .unwrap_or_else(plain)
            },
            FileType::Diff => {
                syntax_set.find_syntax_by_extension("diff")
                    .or_else(|| syntax_set.find_syntax_by_name("Diff"))
                    .unwrap_or_else(plain)
            },
            FileType::Kotlin => {
                syntax_set.find_syntax_by_extension("kt")
                    .or_else(|| syntax_set.find_syntax_by_name("Kotlin"))
                    // Fall back to Java (similar syntax structure)
                    .or_else(|| syntax_set.find_syntax_by_extension("java"))
                    .unwrap_or_else(plain)
            },
        }
    }

    pub fn highlight_line<'a>(
        &'a mut self,
        line: &'a str,
        file_type: FileType,
        line_number: usize
    ) -> Vec<(Style, &'a str)> {
        // For YAML files, use custom highlighting if native isn't available
        if file_type == FileType::YAML {
            if let Some(custom_highlight) = self.custom_yaml_highlight(line) {
                return custom_highlight;
            }
        }

        let cache_key = (line.to_string(), line_number);

        let syntax = Self::resolve_syntax(&self.syntax_set, file_type);

        let mut highlighter = HighlightLines::new(
            syntax,
            &self.theme_set.themes[&self.current_theme]
        );

        let highlighted = highlighter.highlight_line(line, &self.syntax_set)
            .unwrap_or_else(|_| vec![(Style::default(), line)]);

        // Cache owns strings; we return references into the original `line`
        let cached: Vec<(Style, String)> = highlighted.iter()
            .map(|(style, text)| (*style, text.to_string()))
            .collect();

        if self.highlight_cache.len() >= Self::MAX_CACHE_SIZE {
            self.prune_cache_if_needed();
        }
        self.highlight_cache.insert(cache_key, cached);

        highlighted
    }

    fn prune_cache_if_needed(&mut self) {
        if self.highlight_cache.len() > Self::MAX_CACHE_SIZE {
            let to_remove = self.highlight_cache.len() - Self::MAX_CACHE_SIZE;
            let keys: Vec<_> = self.highlight_cache.keys()
                .take(to_remove)
                .cloned()
                .collect();
            for key in keys {
                self.highlight_cache.remove(&key);
            }
        }
    }

    pub fn clear_cache(&mut self) {
        self.highlight_cache.clear();
    }

    pub fn set_theme(&mut self, theme_name: &str) -> bool {
        if self.theme_set.themes.contains_key(theme_name) {
            self.current_theme = theme_name.to_string();
            self.clear_cache();
            true
        } else {
            false
        }
    }

    pub fn load_custom_theme(&mut self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let theme = ThemeSet::get_theme(path)?;
        let theme_name = path.file_stem()
            .and_then(OsStr::to_str)
            .unwrap_or("custom")
            .to_string();

        self.theme_set.themes.insert(theme_name.clone(), theme);
        self.current_theme = theme_name;
        self.clear_cache();
        Ok(())
    }

    pub fn available_themes(&self) -> Vec<String> {
        self.theme_set.themes.keys()
            .map(|k| k.to_string())
            .collect()
    }

    pub fn list_available_syntaxes(&self) -> Vec<String> {
        self.syntax_set.syntaxes().iter()
            .map(|syntax| format!("{} ({})", syntax.name, syntax.file_extensions.join(", ")))
            .collect()
    }

    pub fn has_syntax_for_extension(&self, extension: &str) -> bool {
        self.syntax_set.find_syntax_by_extension(extension).is_some()
    }

    pub fn debug_syntax_for_filetype(&self, file_type: FileType) -> String {
        let syntax = Self::resolve_syntax(&self.syntax_set, file_type);
        format!("FileType: {:?} -> Syntax: {}", file_type, syntax.name)
    }

    /// Custom YAML highlighting when syntect doesn't have YAML support
    fn custom_yaml_highlight<'a>(&self, line: &'a str) -> Option<Vec<(Style, &'a str)>> {
        // Use native highlighting if available
        if self.syntax_set.find_syntax_by_extension("yaml").is_some() ||
           self.syntax_set.find_syntax_by_extension("yml").is_some() {
            return None;
        }

        let mut result = Vec::new();

        let comment_style = Style {
            foreground: Color { r: 156, g: 142, b: 124, a: 255 },
            ..Style::default()
        };

        let key_style = Style {
            foreground: Color { r: 76, g: 119, b: 79, a: 255 },
            ..Style::default()
        };

        let special_style = Style {
            foreground: Color { r: 147, g: 112, b: 179, a: 255 },
            ..Style::default()
        };

        if line.trim_start().starts_with('#') {
            result.push((comment_style, line));
        } else if line.contains(':') && !line.trim_start().starts_with('-') {
            if let Some(colon_pos) = line.find(':') {
                result.push((key_style, &line[..colon_pos + 1]));
                if colon_pos + 1 < line.len() {
                    result.push((Style::default(), &line[colon_pos + 1..]));
                }
            } else {
                result.push((Style::default(), line));
            }
        } else if line.trim_start().starts_with('-') {
            if let Some(dash_pos) = line.find('-') {
                result.push((Style::default(), &line[..dash_pos]));
                result.push((special_style, "-"));
                if dash_pos + 1 < line.len() {
                    result.push((Style::default(), &line[dash_pos + 1..]));
                }
            } else {
                result.push((Style::default(), line));
            }
        } else {
            result.push((Style::default(), line));
        }

        Some(result)
    }
}

/// Smart indentation rule for a language
#[derive(Debug)]
pub struct IndentRule {
    /// Patterns that increase indent on next line
    pub increase_patterns: Vec<Regex>,
    /// Patterns that decrease current line indent
    pub decrease_patterns: Vec<Regex>,
    /// Patterns that both decrease current line and keep same indent for next
    pub decrease_increase_patterns: Vec<Regex>,
}

impl IndentRule {
    pub fn python() -> Self {
        Self {
            increase_patterns: vec![
                Regex::new(r":\s*(?:#.*)?$").unwrap(),
                Regex::new(r"^\s*@\w+").unwrap(),
            ],
            decrease_patterns: vec![
                Regex::new(r"^\s*(elif|else|except|finally|break|continue|pass|return)\b").unwrap(),
            ],
            decrease_increase_patterns: vec![
                Regex::new(r"^\s*(elif|else|except|finally).*:\s*(?:#.*)?$").unwrap(),
            ],
        }
    }

    pub fn rust() -> Self {
        Self {
            increase_patterns: vec![
                Regex::new(r"\{\s*(?://.*)?$").unwrap(),
                Regex::new(r"=>\s*(?://.*)?$").unwrap(),
            ],
            decrease_patterns: vec![
                Regex::new(r"^\s*\}").unwrap(),
            ],
            decrease_increase_patterns: vec![
                Regex::new(r"^\s*\}\s*else\s*\{").unwrap(),
            ],
        }
    }

    /// Shared brace-based rule for JS, TS, Java, C, C++, C#, Go, Swift, Scala, Kotlin, PHP
    pub fn brace_based() -> Self {
        Self {
            increase_patterns: vec![
                Regex::new(r"\{\s*(?://.*)?$").unwrap(),
                Regex::new(r"=>\s*(?://.*)?$").unwrap(),
            ],
            decrease_patterns: vec![
                Regex::new(r"^\s*\}").unwrap(),
            ],
            decrease_increase_patterns: vec![
                Regex::new(r"^\s*\}\s*else\s*(if\s*\()?\{?").unwrap(),
                Regex::new(r"^\s*\}\s*catch\s*\(").unwrap(),
                Regex::new(r"^\s*\}\s*finally\s*\{").unwrap(),
            ],
        }
    }

    pub fn javascript() -> Self {
        Self::brace_based()
    }

    pub fn html() -> Self {
        Self {
            increase_patterns: vec![
                Regex::new(r"<[a-zA-Z][^/>]*>$").unwrap(),
                Regex::new(r"<(div|p|ul|ol|li|table|tr|td|th|head|body|html|section|article|nav|aside|header|footer|main)[^>]*>").unwrap(),
            ],
            decrease_patterns: vec![
                Regex::new(r"^\s*</").unwrap(),
            ],
            decrease_increase_patterns: vec![],
        }
    }

    pub fn css() -> Self {
        Self {
            increase_patterns: vec![
                Regex::new(r"\{\s*(?:/\*.*\*/\s*)?$").unwrap(),
            ],
            decrease_patterns: vec![
                Regex::new(r"^\s*\}").unwrap(),
            ],
            decrease_increase_patterns: vec![],
        }
    }

    pub fn yaml() -> Self {
        Self {
            increase_patterns: vec![
                Regex::new(r":\s*$").unwrap(),
                Regex::new(r":\s*\|").unwrap(),
                Regex::new(r":\s*>").unwrap(),
                Regex::new(r"^\s*-\s*$").unwrap(),
                Regex::new(r"^\s*-\s+\w+:\s*$").unwrap(),
            ],
            decrease_patterns: vec![],
            decrease_increase_patterns: vec![],
        }
    }

    pub fn json() -> Self {
        Self {
            increase_patterns: vec![
                Regex::new(r"[\[{]\s*$").unwrap(),
            ],
            decrease_patterns: vec![
                Regex::new(r"^\s*[\]}]").unwrap(),
            ],
            decrease_increase_patterns: vec![],
        }
    }

    pub fn shell() -> Self {
        Self {
            increase_patterns: vec![
                Regex::new(r"\bdo\s*$").unwrap(),
                Regex::new(r"\bthen\s*$").unwrap(),
                Regex::new(r"\{\s*$").unwrap(),
                Regex::new(r"\(\s*$").unwrap(),
            ],
            decrease_patterns: vec![
                Regex::new(r"^\s*(done|fi|esac)\b").unwrap(),
                Regex::new(r"^\s*\}\s*$").unwrap(),
                Regex::new(r"^\s*\)\s*$").unwrap(),
            ],
            decrease_increase_patterns: vec![
                Regex::new(r"^\s*(elif|else)\b").unwrap(),
            ],
        }
    }

    pub fn ruby() -> Self {
        Self {
            increase_patterns: vec![
                Regex::new(r"\bdo\s*(?:\|[^|]*\|)?\s*$").unwrap(),
                Regex::new(r"\{\s*(?:#.*)?$").unwrap(),
                Regex::new(r"^\s*(class|module|def|if|unless|while|until|for|case|begin)\b").unwrap(),
            ],
            decrease_patterns: vec![
                Regex::new(r"^\s*end\b").unwrap(),
                Regex::new(r"^\s*\}").unwrap(),
            ],
            decrease_increase_patterns: vec![
                Regex::new(r"^\s*(elsif|else|rescue|ensure|when)\b").unwrap(),
            ],
        }
    }

    pub fn lua() -> Self {
        Self {
            increase_patterns: vec![
                Regex::new(r"\bdo\s*$").unwrap(),
                Regex::new(r"\bthen\s*$").unwrap(),
                Regex::new(r"\bfunction\b.*\)\s*$").unwrap(),
                Regex::new(r"^\s*(if|for|while|repeat)\b").unwrap(),
            ],
            decrease_patterns: vec![
                Regex::new(r"^\s*(end|until)\b").unwrap(),
            ],
            decrease_increase_patterns: vec![
                Regex::new(r"^\s*(elseif|else)\b").unwrap(),
            ],
        }
    }
}

/// Smart indentation engine
pub struct SmartIndenter {
    rules: HashMap<FileType, IndentRule>,
}

impl SmartIndenter {
    pub fn new() -> Self {
        let mut rules = HashMap::new();
        rules.insert(FileType::Python, IndentRule::python());
        rules.insert(FileType::Rust, IndentRule::rust());
        // Brace-based languages
        rules.insert(FileType::JavaScript, IndentRule::brace_based());
        rules.insert(FileType::TypeScript, IndentRule::brace_based());
        rules.insert(FileType::Go, IndentRule::brace_based());
        rules.insert(FileType::Java, IndentRule::brace_based());
        rules.insert(FileType::C, IndentRule::brace_based());
        rules.insert(FileType::Cpp, IndentRule::brace_based());
        rules.insert(FileType::CSharp, IndentRule::brace_based());
        rules.insert(FileType::PHP, IndentRule::brace_based());
        rules.insert(FileType::Swift, IndentRule::brace_based());
        rules.insert(FileType::Scala, IndentRule::brace_based());
        rules.insert(FileType::Kotlin, IndentRule::brace_based());
        rules.insert(FileType::PowerShell, IndentRule::brace_based());
        // Tag-based
        rules.insert(FileType::HTML, IndentRule::html());
        rules.insert(FileType::XML, IndentRule::html());
        // Style-based
        rules.insert(FileType::CSS, IndentRule::css());
        rules.insert(FileType::Less, IndentRule::css());
        // Others
        rules.insert(FileType::YAML, IndentRule::yaml());
        rules.insert(FileType::JSON, IndentRule::json());
        rules.insert(FileType::Shell, IndentRule::shell());
        rules.insert(FileType::Ruby, IndentRule::ruby());
        rules.insert(FileType::Lua, IndentRule::lua());

        Self { rules }
    }

    /// Calculate the indent for a new line based on the previous lines
    pub fn calculate_indent(
        &self,
        lines: &[String],
        current_line_idx: usize,
        current_line_content: &str,
        file_type: FileType,
        tab_size: usize,
    ) -> usize {
        let rule = match self.rules.get(&file_type) {
            Some(rule) => rule,
            None => return self.get_previous_indent(lines, current_line_idx, tab_size),
        };

        if lines.is_empty() {
            return 0;
        }

        let prev_line_idx = if current_line_idx >= lines.len() {
            lines.len() - 1
        } else if current_line_idx == 0 {
            return 0;
        } else {
            current_line_idx - 1
        };

        let prev_line = &lines[prev_line_idx];
        let prev_indent = self.get_line_indent(prev_line, tab_size);

        if rule.decrease_patterns.iter().any(|pattern| pattern.is_match(current_line_content)) {
            return prev_indent.saturating_sub(tab_size);
        }

        if rule.decrease_increase_patterns.iter().any(|pattern| pattern.is_match(current_line_content)) {
            return prev_indent;
        }

        if file_type == FileType::Python && prev_indent == 0 && prev_line.trim().contains("__name__") && prev_line.trim().contains("__main__") {
            return 0;
        }

        if rule.increase_patterns.iter().any(|pattern| pattern.is_match(prev_line)) {
            return prev_indent + tab_size;
        }

        prev_indent
    }

    pub fn get_line_indent(&self, line: &str, tab_size: usize) -> usize {
        let mut count = 0;
        for ch in line.chars() {
            match ch {
                ' ' => count += 1,
                '\t' => count += tab_size,
                _ => break,
            }
        }
        count
    }

    fn get_previous_indent(&self, lines: &[String], current_line_idx: usize, tab_size: usize) -> usize {
        if current_line_idx == 0 {
            return 0;
        }

        let prev_line = &lines[current_line_idx - 1];
        self.get_line_indent(prev_line, tab_size)
    }
}

impl SyntaxHighlighter {
    pub fn create_smart_indenter() -> SmartIndenter {
        SmartIndenter::new()
    }
}

/// Returns the single-line comment prefix for a file type, or `None` if
/// the language uses only block comments or has no comment syntax.
pub fn comment_prefix_for_file_type(file_type: FileType) -> Option<&'static str> {
    match file_type {
        // Hash-style comments
        FileType::Python | FileType::Ruby | FileType::Shell |
        FileType::Perl | FileType::YAML | FileType::TOML |
        FileType::Dockerfile | FileType::Makefile | FileType::PowerShell => Some("#"),
        // Double-slash comments
        FileType::Rust | FileType::JavaScript | FileType::TypeScript |
        FileType::Go | FileType::Java | FileType::C | FileType::Cpp |
        FileType::CSharp | FileType::Swift | FileType::Scala |
        FileType::Kotlin | FileType::PHP | FileType::Less => Some("//"),
        // Double-dash comments
        FileType::SQL | FileType::Lua => Some("--"),
        // No simple single-line comment syntax
        FileType::HTML | FileType::XML | FileType::CSS |
        FileType::JSON | FileType::Markdown | FileType::Diff |
        FileType::Plain => None,
    }
}
