//! Code fence detection utilities to guard import hoisting.

/// Fence parsing phases tracked across lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FencePhase {
    /// Not currently inside a fence.
    Outside,
    /// Within fence contents.
    InsideFence,
}

impl Default for FencePhase {
    fn default() -> Self {
        FencePhase::Outside
    }
}

/// Current fence state (phase, marker, and indent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FenceState {
    /// Current fence phase.
    pub phase: FencePhase,
    /// Fence marker character (``` or ~~~).
    pub marker: Option<char>,
    /// Leading whitespace count captured at opening.
    pub indent: usize,
}

impl Default for FenceState {
    fn default() -> Self {
        FenceState {
            phase: FencePhase::Outside,
            marker: None,
            indent: 0,
        }
    }
}

/// Outcome of processing a single line for fence state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineParseOutcome {
    /// State to carry into the next line.
    pub next_state: FenceState,
    /// Whether import hoisting should be skipped for this line.
    pub skip_imports: bool,
}

/// Advance fence state based on a single line of text.
pub fn advance_fence_state(line: &str, state: FenceState) -> LineParseOutcome {
    let indent = leading_whitespace_len(line);
    let after_indent = &line[indent..];

    let mut next_state = state;
    let mut skip_imports = matches!(state.phase, FencePhase::InsideFence);

    // Opening
    if matches!(state.phase, FencePhase::Outside)
        && let Some(marker) = detect_fence_marker(after_indent)
    {
        next_state = FenceState {
            phase: FencePhase::InsideFence,
            marker: Some(marker),
            indent,
        };
        skip_imports = true;
    } else if matches!(state.phase, FencePhase::InsideFence)
        && indent <= state.indent
        && let Some(marker) = detect_fence_marker(after_indent)
        && Some(marker) == state.marker
    {
        // Closing
        next_state = FenceState {
            phase: FencePhase::Outside,
            marker: None,
            indent: 0,
        };
        skip_imports = true;
    }

    LineParseOutcome {
        next_state,
        skip_imports,
    }
}

/// Collect root-level import/export statements while preserving the remaining lines.
pub fn collect_root_imports(body: &str) -> (Vec<String>, Vec<String>) {
    let mut fence_state = FenceState::default();
    let mut hoisted = Vec::new();
    let mut body_lines = Vec::new();
    let mut buffer = String::new();
    let mut depth = 0isize;
    let mut collecting = false;

    let mut lines_iter = body.lines().peekable();
    while let Some(line) = lines_iter.next() {
        let outcome = advance_fence_state(line, fence_state);
        fence_state = outcome.next_state;

        if outcome.skip_imports {
            if collecting {
                buffer.push_str(line);
                buffer.push('\n');
            } else {
                body_lines.push(line.to_string());
            }
            continue;
        }

        let trimmed = line.trim_start();
        if !collecting && (is_import_start(trimmed) || is_export_start(trimmed)) {
            collecting = true;
            depth = 0;
            buffer.push_str(line);
            buffer.push('\n');
            depth += paren_delta(line);
            let next_line = lines_iter.peek().copied();
            if ends_statement(line, depth, next_line) {
                hoisted.push(buffer.trim_end().to_string());
                buffer.clear();
                collecting = false;
                depth = 0;
            }
            continue;
        }

        if collecting {
            buffer.push_str(line);
            buffer.push('\n');
            depth += paren_delta(line);
            let next_line = lines_iter.peek().copied();
            if ends_statement(line, depth, next_line) {
                hoisted.push(buffer.trim_end().to_string());
                buffer.clear();
                collecting = false;
                depth = 0;
            }
        } else {
            body_lines.push(line.to_string());
        }
    }

    if collecting && !buffer.is_empty() {
        hoisted.push(buffer.trim_end().to_string());
    }

    (hoisted, body_lines)
}

fn is_import_start(trimmed: &str) -> bool {
    trimmed.starts_with("import ")
}

fn is_export_start(trimmed: &str) -> bool {
    if let Some(rest) = trimmed.strip_prefix("export") {
        match rest.chars().next() {
            Some(' ' | '{' | '*') => true,
            _ => false,
        }
    } else {
        false
    }
}

fn paren_delta(line: &str) -> isize {
    let mut depth: isize = 0;
    let mut chars = line.chars().peekable();

    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_template = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while let Some(ch) = chars.next() {
        if in_line_comment {
            // Everything after '//' on this line is a comment.
            break;
        }

        if in_block_comment {
            if ch == '*' {
                if let Some('/') = chars.peek() {
                    // End of block comment: '*/'
                    chars.next();
                    in_block_comment = false;
                }
            }
            continue;
        }

        if in_single_quote {
            if ch == '\\' {
                // Skip escaped character inside single-quoted string.
                chars.next();
            } else if ch == '\'' {
                in_single_quote = false;
            }
            continue;
        }

        if in_double_quote {
            if ch == '\\' {
                // Skip escaped character inside double-quoted string.
                chars.next();
            } else if ch == '"' {
                in_double_quote = false;
            }
            continue;
        }

        if in_template {
            if ch == '\\' {
                // Skip escaped character inside template literal.
                chars.next();
            } else if ch == '`' {
                in_template = false;
            }
            continue;
        }

        // Not currently in a string or comment: check for comment starts, strings, and brackets.
        match ch {
            '/' => {
                if let Some(next) = chars.peek() {
                    if *next == '/' {
                        // Line comment: '//' to end of line.
                        chars.next();
                        in_line_comment = true;
                        continue;
                    } else if *next == '*' {
                        // Block comment: '/* ... */'
                        chars.next();
                        in_block_comment = true;
                        continue;
                    }
                }
            }
            '\'' => {
                in_single_quote = true;
                continue;
            }
            '"' => {
                in_double_quote = true;
                continue;
            }
            '`' => {
                in_template = true;
                continue;
            }
            '(' | '{' | '[' => {
                depth += 1;
            }
            ')' | '}' | ']' => {
                depth -= 1;
            }
            _ => {}
        }
    }

    depth
}

fn ends_statement(line: &str, depth: isize, next_line: Option<&str>) -> bool {
    if depth != 0 {
        return false;
    }

    let trimmed = line.trim_end();

    // explicit terminator
    if trimmed.ends_with(';') {
        return true;
    }

    // explicit continuations
    if trimmed.ends_with('\\')
        || trimmed.ends_with(',')
        || trimmed.ends_with('{')
        || trimmed.ends_with('(')
    {
        return false;
    }

    // closing brace/paren/bracket: peek next line for continuation
    if trimmed.ends_with('}') || trimmed.ends_with(')') || trimmed.ends_with(']') {
        if let Some(next) = next_line {
            let nt = next.trim_start();
            if nt.starts_with(',')
                || nt.starts_with('.')
                || nt.starts_with('{')
                || nt.starts_with('(')
            {
                return false;
            }
        }
        return true;
    }

    // default: treat as ended unless the next line is a continuation
    if let Some(next) = next_line {
        let nt = next.trim_start();
        if nt.starts_with(',') || nt.starts_with('{') || nt.starts_with('(') || nt.starts_with('.')
        {
            return false;
        }
        return true;
    }

    true
}

fn leading_whitespace_len(line: &str) -> usize {
    line.bytes()
        .take_while(|b| matches!(*b, b' ' | b'\t'))
        .count()
}

fn detect_fence_marker(after_indent: &str) -> Option<char> {
    let mut chars = after_indent.chars();
    let first = chars.next()?;
    if first != '`' && first != '~' {
        return None;
    }
    let run_len = 1 + chars.take_while(|c| *c == first).count();
    if run_len >= 3 { Some(first) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_and_closes_backtick_fence() {
        let start = advance_fence_state("```js", FenceState::default());
        assert!(start.skip_imports);
        assert!(matches!(start.next_state.phase, FencePhase::InsideFence));
        assert_eq!(start.next_state.marker, Some('`'));
        assert_eq!(start.next_state.indent, 0);

        let inner = advance_fence_state("console.log('hi');", start.next_state);
        assert!(inner.skip_imports);
        assert!(matches!(inner.next_state.phase, FencePhase::InsideFence));

        let end = advance_fence_state("```", inner.next_state);
        assert!(end.skip_imports);
        assert!(matches!(end.next_state.phase, FencePhase::Outside));
        assert_eq!(end.next_state.marker, None);
    }

    #[test]
    fn does_not_close_with_greater_indent() {
        let start = advance_fence_state("    ```", FenceState::default());
        let inner = advance_fence_state("      code", start.next_state);
        let not_closed = advance_fence_state("      ```", inner.next_state);

        assert!(not_closed.skip_imports);
        assert!(matches!(
            not_closed.next_state.phase,
            FencePhase::InsideFence
        ));
    }

    #[test]
    fn ignores_mismatched_marker() {
        let start = advance_fence_state("~~~ts", FenceState::default());
        let still_inside = advance_fence_state("```", start.next_state);

        assert!(still_inside.skip_imports);
        assert!(matches!(
            still_inside.next_state.phase,
            FencePhase::InsideFence
        ));
        assert_eq!(still_inside.next_state.marker, Some('~'));
    }

    #[test]
    fn requires_three_markers_to_open() {
        let outcome = advance_fence_state("``", FenceState::default());
        assert!(!outcome.skip_imports);
        assert!(matches!(outcome.next_state.phase, FencePhase::Outside));
    }

    #[test]
    fn collects_single_line_import() {
        let body = "import A from './a'\nconst x = 1;";
        let (imports, rest) = collect_root_imports(body);
        assert_eq!(imports, vec!["import A from './a'"]);
        assert_eq!(rest, vec!["const x = 1;"]);
    }

    #[test]
    fn collects_multiline_import() {
        let body = "import {\n  A,\n  B,\n} from './a';\nconsole.log(A);";
        let (imports, rest) = collect_root_imports(body);
        assert_eq!(imports, vec!["import {\n  A,\n  B,\n} from './a';"]);
        assert_eq!(rest, vec!["console.log(A);"]);
    }

    #[test]
    fn collects_semicolonless_export_const() {
        let body = "export const foo = () => {\n  return 1\n}\nconsole.log(foo());";
        let (imports, rest) = collect_root_imports(body);
        assert_eq!(imports, vec!["export const foo = () => {\n  return 1\n}"]);
        assert_eq!(rest, vec!["console.log(foo());"]);
    }

    #[test]
    fn collects_export_block() {
        let body = "export {\n  foo,\n  bar,\n}\nconst x = 1;";
        let (imports, rest) = collect_root_imports(body);
        assert_eq!(imports, vec!["export {\n  foo,\n  bar,\n}"]);
        assert_eq!(rest, vec!["const x = 1;"]);
    }

    #[test]
    fn collects_export_default_function_multiline() {
        let body = "export default function test()\n{\n  return 1;\n}\n# Title";
        let (imports, rest) = collect_root_imports(body);
        assert_eq!(
            imports,
            vec!["export default function test()\n{\n  return 1;\n}"]
        );
        assert_eq!(rest, vec!["# Title"]);
    }

    #[test]
    fn collects_export_default_async_arrow() {
        let body = "export default async () => {\n  return 1\n}\nContent";
        let (imports, rest) = collect_root_imports(body);
        assert_eq!(imports, vec!["export default async () => {\n  return 1\n}"]);
        assert_eq!(rest, vec!["Content"]);
    }

    #[test]
    fn collects_export_all() {
        let body = "export * from './mod';\nText";
        let (imports, rest) = collect_root_imports(body);
        assert_eq!(imports, vec!["export * from './mod';"]);
        assert_eq!(rest, vec!["Text"]);
    }

    #[test]
    fn collects_export_with_inline_comment() {
        let body = "export const foo = 1 // inline\nNext";
        let (imports, rest) = collect_root_imports(body);
        assert_eq!(imports, vec!["export const foo = 1 // inline"]);
        assert_eq!(rest, vec!["Next"]);
    }

    #[test]
    fn ignores_export_inside_fence() {
        let body = "```\nexport const skip = true\n```\nexport const ok = false;";
        let (imports, rest) = collect_root_imports(body);
        assert_eq!(imports, vec!["export const ok = false;"]);
        assert_eq!(rest, vec!["```", "export const skip = true", "```"]);
    }

    #[test]
    fn ignores_import_inside_fence() {
        let body = "```\nimport bad from './nope'\n```\nimport good from './ok';";
        let (imports, rest) = collect_root_imports(body);
        assert_eq!(imports, vec!["import good from './ok';"]);
        assert_eq!(rest, vec!["```", "import bad from './nope'", "```"]);
    }

    #[test]
    fn detects_after_fence_with_indent() {
        let body = "    ```\ncode\n    ```\nexport const x = 1;";
        let (imports, rest) = collect_root_imports(body);
        assert_eq!(imports, vec!["export const x = 1;"]);
        assert_eq!(rest, vec!["    ```", "code", "    ```"]);
    }
}
