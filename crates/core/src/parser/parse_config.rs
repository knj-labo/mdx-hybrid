/// Fine-grained parser feature flags used to switch between `.md` and `.mdx` modes.
#[derive(Clone, Copy, Debug)]
pub struct ParseConstructs {
    /// Enables JSX tags inside documents.
    pub jsx: bool,
    /// Enables `import`/`export` statements at the document root.
    pub esm: bool,
    /// Enables `{expression}` nodes inside the flow.
    pub expression: bool,
    /// Enables directive syntax such as `:::note`.
    pub directives: bool,
}

impl ParseConstructs {
    /// Construct a parser flag set with all features disabled.
    pub const fn disabled() -> Self {
        Self {
            jsx: false,
            esm: false,
            expression: false,
            directives: false,
        }
    }
}

/// Parser configuration applied while walking markdown-rs events.
#[derive(Clone, Copy, Debug)]
pub struct ParseConfig {
    /// Optional syntax extensions toggled on/off for the current file.
    pub constructs: ParseConstructs,
    /// Converts HTML nodes to sanitized JSX output (used for `.md` mode).
    pub html_to_jsx: bool,
}

impl ParseConfig {
    /// Returns the configuration used for `.md` files.
    pub const fn markdown() -> Self {
        Self {
            constructs: ParseConstructs {
                jsx: false,
                esm: false,
                expression: false,
                directives: true,
            },
            html_to_jsx: true,
        }
    }

    /// Returns the configuration used for `.mdx` files.
    pub const fn mdx() -> Self {
        Self {
            constructs: ParseConstructs {
                jsx: true,
                esm: true,
                expression: true,
                directives: true,
            },
            html_to_jsx: false,
        }
    }
}

impl Default for ParseConfig {
    fn default() -> Self {
        Self::mdx()
    }
}
