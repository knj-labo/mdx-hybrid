//! Generates Astro-compatible module code from the compilation IR.

use crate::types::CompileIrResult;
use napi::bindgen_prelude::{Error, Result};
use std::fmt::Write as FmtWrite;

#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_module_code_from_ir(
    ir: &CompileIrResult,
    hoisted_imports: &[String],
    headings_json: &str,
) -> Result<String> {
    let mut code = String::new();
    writeln!(
        code,
        "import {{ Fragment, jsx as __jsx }} from 'astro/jsx-runtime';"
    )
    .map_err(|err| Error::from_reason(err.to_string()))?;
    writeln!(code, "const _Fragment = Fragment;")
        .map_err(|err| Error::from_reason(err.to_string()))?;
    writeln!(
        code,
        "const _jsx = (type, props, ...children) => {{\n  const resolved = props ?? {{}};\n  if (children.length > 0) {{\n    resolved.children = children.length === 1 ? children[0] : children;\n  }}\n  return __jsx(type, resolved, resolved.key);\n}};"
    )
    .map_err(|err| Error::from_reason(err.to_string()))?;
    writeln!(
        code,
        "import {{ createComponent, renderJSX }} from 'astro/runtime/server/index.js';"
    )
    .map_err(|err| Error::from_reason(err.to_string()))?;
    if let Some(layout) = ir.layout_import.as_deref() {
        writeln!(code, "import Layout from {};", js_string_literal(layout))
            .map_err(|err| Error::from_reason(err.to_string()))?;
    }

    for import in hoisted_imports {
        writeln!(code, "{}", import).map_err(|err| Error::from_reason(err.to_string()))?;
    }

    writeln!(code, "export const frontmatter = {};", ir.frontmatter_json)
        .map_err(|err| Error::from_reason(err.to_string()))?;

    writeln!(
        code,
        "export const file = {};",
        js_string_literal(&ir.file_path)
    )
    .map_err(|err| Error::from_reason(err.to_string()))?;

    let url_literal = ir
        .url
        .as_deref()
        .map(js_string_literal)
        .unwrap_or_else(|| "undefined".to_string());
    writeln!(code, "export const url = {};", url_literal)
        .map_err(|err| Error::from_reason(err.to_string()))?;

    writeln!(code, "export const headings = {};", headings_json)
        .map_err(|err| Error::from_reason(err.to_string()))?;
    writeln!(code, "export function getHeadings() {{")
        .map_err(|err| Error::from_reason(err.to_string()))?;
    writeln!(code, "  return {};", headings_json)
        .map_err(|err| Error::from_reason(err.to_string()))?;
    writeln!(code, "}}").map_err(|err| Error::from_reason(err.to_string()))?;

    writeln!(code, "// function MarkflowContent")
        .map_err(|err| Error::from_reason(err.to_string()))?;
    writeln!(
        code,
        "const MarkflowContent = createComponent((result, props) => {{"
    )
    .map_err(|err| Error::from_reason(err.to_string()))?;
    writeln!(code, "  return renderJSX(result, (")
        .map_err(|err| Error::from_reason(err.to_string()))?;
    writeln!(code, "    <>").map_err(|err| Error::from_reason(err.to_string()))?;
    code.push_str(&ir.html);
    if !ir.html.ends_with('\n') {
        code.push('\n');
    }
    writeln!(code, "    </>").map_err(|err| Error::from_reason(err.to_string()))?;
    writeln!(code, "  ));").map_err(|err| Error::from_reason(err.to_string()))?;
    writeln!(code, "}}, file);").map_err(|err| Error::from_reason(err.to_string()))?;

    writeln!(code, "export const Content = MarkflowContent;")
        .map_err(|err| Error::from_reason(err.to_string()))?;

    // Only generate export default if user didn't provide their own
    if !ir.has_user_default_export {
        if ir.layout_import.is_some() {
            writeln!(
                code,
                "export default createComponent((result, props) => renderJSX(result, _jsx(Layout, {{...props, frontmatter: frontmatter, children: _jsx(MarkflowContent, {{...props}})}})), file);"
            )
            .map_err(|err| Error::from_reason(err.to_string()))?;
        } else {
            writeln!(code, "export default MarkflowContent;")
                .map_err(|err| Error::from_reason(err.to_string()))?;
        }
    }

    Ok(code)
}

fn js_string_literal(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}
