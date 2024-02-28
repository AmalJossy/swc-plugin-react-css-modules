use std::{collections::HashMap, fs, path::PathBuf};

use lightningcss::{
    css_modules::{Config, CssModuleExport, CssModuleReference, Pattern},
    printer::PrinterOptions,
    stylesheet::{ParserOptions, StyleSheet, ToCssResult},
    targets::Targets,
};
use path_absolutize::Absolutize;

use crate::generic_names::{Generator, Options};

pub struct CssModuleParser {
    /// the same pattern passed to genericNames
    pattern: String,
    /// root of project
    context: PathBuf,
    hash_prefix: String,
    /// path to file that can actually be read
    /// works with virtualized fs
    fs_path: PathBuf,
    /// absolute path in the non-virtualized environment
    /// used to generate hash
    full_path: PathBuf,
}

impl CssModuleParser {
    pub fn new(
        pattern: String,
        context: PathBuf,
        hash_prefix: String,
        fs_path: PathBuf,
        full_path: PathBuf,
    ) -> Self {
        Self {
            pattern,
            context,
            hash_prefix,
            fs_path,
            full_path,
        }
    }

    pub fn generate_style_name_map(&self) -> Result<HashMap<String, String>, String> {
        let file = fs::read_to_string(self.fs_path.clone());
        let contents = match file {
            Ok(data) => data,
            Err(_) => return Err(format!("Could not read {:?}", self.full_path)),
        };
        let stylesheet = StyleSheet::parse(
            &contents,
            ParserOptions {
                filename: self
                    .full_path
                    .clone()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
                css_modules: Some(Config {
                    // not using lightning css hashing in favour of hashing via generic names
                    // lightning suggests that we do hashing ourselves https://github.com/parcel-bundler/lightningcss/issues/156#issuecomment-1131828962
                    pattern: Pattern::parse("[local]").unwrap(),
                    dashed_idents: false,
                }),
                ..ParserOptions::default()
            },
        )
        .unwrap();

        let css_result = stylesheet.to_css(PrinterOptions {
            minify: false,
            analyze_dependencies: None,
            pseudo_classes: None,
            source_map: None,
            targets: Targets::default(),
            project_root: Some(&self.context.clone().into_os_string().into_string().unwrap()),
        });
        match css_result {
            Ok(ToCssResult { exports, .. }) => {
                 match exports {
                    Some(exports) =>  {
                        let generator = Generator::new_with_options(
                            &self.pattern,
                            Options {
                                context: self.context.clone(),
                                hash_prefix: self.hash_prefix.clone(),
                            },
                        );
                        Ok(exports
                            .iter()
                            .map(|(k, v)| (k.clone(), self.css_module_exports_to_str(&v, &generator)))
                            .collect())
                    },
                    _ => Ok(HashMap::new())
                 }
            }
            Err(printer_err) => {
                Err(printer_err.to_string())
            }
        }
    }

    fn css_module_exports_to_str(&self, export: &CssModuleExport, generator: &Generator) -> String {
        format!(
            "{} {}",
            generator.generate(&export.name, self.full_path.clone()),
            export
                .composes
                .iter()
                .map(|reference| match reference {
                    CssModuleReference::Local { name } =>
                        generator.generate(name, self.full_path.clone()),
                    // global compose need not be transformed
                    CssModuleReference::Global { name } => name.clone(),
                    CssModuleReference::Dependency { name, specifier } => {
                        let path = self.full_path.clone().parent().unwrap().join(specifier);
                        generator.generate(name, path.absolutize().unwrap().to_path_buf())
                    }
                })
                .collect::<Vec<String>>()
                .join(" ")
        )
        .trim()
        .to_string()
    }
}
