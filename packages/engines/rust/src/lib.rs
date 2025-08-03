#![deny(clippy::all)]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::time::Instant;

#[napi(object)]
pub struct CompileOptions {
  pub development: Option<bool>,
  pub jsx: Option<bool>,
  pub jsx_runtime: Option<String>,
  pub jsx_import_source: Option<String>,
  pub pragma: Option<String>,
  pub pragma_frag: Option<String>,
  pub pragma_import_source: Option<String>,
}

#[napi(object)]
pub struct CompileResult {
  pub code: String,
  pub map: Option<String>,
  pub timing: f64,
  // Currently not implemented - mdxjs crate doesn't provide VFile data
}

#[napi]
pub fn compile_sync(content: String, options: Option<CompileOptions>) -> Result<CompileResult> {
  let start = Instant::now();
  
  let mut mdx_options = mdxjs::Options::default();
  
  if let Some(opts) = options {
    if let Some(development) = opts.development {
      mdx_options.development = development;
    }
    
    if let Some(jsx) = opts.jsx {
      mdx_options.jsx = jsx;
    }
    
    if let Some(jsx_runtime) = opts.jsx_runtime {
      match jsx_runtime.as_str() {
        "automatic" => mdx_options.jsx_runtime = Some(mdxjs::JsxRuntime::Automatic),
        "classic" => mdx_options.jsx_runtime = Some(mdxjs::JsxRuntime::Classic),
        _ => {}
      }
    }
    
    if let Some(jsx_import_source) = opts.jsx_import_source {
      mdx_options.jsx_import_source = Some(jsx_import_source);
    }
    
    if let Some(pragma) = opts.pragma {
      mdx_options.pragma = Some(pragma);
    }
    
    if let Some(pragma_frag) = opts.pragma_frag {
      mdx_options.pragma_frag = Some(pragma_frag);
    }
    
    if let Some(pragma_import_source) = opts.pragma_import_source {
      mdx_options.pragma_import_source = Some(pragma_import_source);
    }
  }
  
  match mdxjs::compile(&content, &mdx_options) {
    Ok(result) => {
      let timing = start.elapsed().as_secs_f64() * 1000.0;
      Ok(CompileResult {
        code: result,
        map: None,
        timing,
      })
    }
    Err(err) => Err(Error::new(
      Status::GenericFailure,
      format!("MDX compilation failed: {}", err),
    )),
  }
}

#[napi]
pub fn is_available() -> bool {
  true
}