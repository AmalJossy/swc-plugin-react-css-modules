use std::{
    collections::HashMap,
    convert::Infallible, fs, path::PathBuf, rc::Rc,
};

use lightningcss::{
    selector::{Component, Selector},
    stylesheet::{ParserOptions, StyleSheet},
    visitor::{Visit, VisitTypes, Visitor},
};

use crate::generic_names::Generator;

pub fn generate_style_name_map(name_gen: Rc<Generator>, virtual_path: &PathBuf, file_path: &PathBuf) -> HashMap<String, String> {
    let contents = fs::read_to_string(virtual_path.clone()).unwrap();
    let mut stylesheet = StyleSheet::parse(
        &contents,
        ParserOptions {
            filename: file_path.clone().into_os_string().into_string().unwrap(),
            ..ParserOptions::default()
        },
    )
    .unwrap();

    let mut class_name_acc = ClassNameAccumulator::new(file_path.clone(), name_gen);
    stylesheet.visit(&mut class_name_acc).unwrap();

    class_name_acc.style_name_map
}

/// Accumulates class names from a stylesheet into a set.
/// use generate_obj_props to generate the object properties from the set of class names.
struct ClassNameAccumulator {
    file_path: PathBuf,
    style_name_map: HashMap<String, String>,
    generator: Rc<Generator>,
}

impl ClassNameAccumulator {
    fn new(file_path: PathBuf, generator: Rc<Generator>) -> Self {
        ClassNameAccumulator {
            file_path,
            style_name_map: HashMap::new(),
            generator
        }
    }

    fn insert_class_name(&mut self, class_name: String) {
        self.style_name_map.insert(class_name.clone(), self.generator.generate(&class_name, self.file_path.clone()));
    }
}

impl<'i> Visitor<'i> for ClassNameAccumulator {
    type Error = Infallible;

    fn visit_types(&self) -> VisitTypes {
        VisitTypes::SELECTORS
    }

    fn visit_selector(&mut self, selector: &mut Selector<'i>) -> Result<(), Self::Error> {
        for c in selector.iter_mut_raw_match_order() {
            match c {
                // .class-name
                Component::Class(c) => {
                    self.insert_class_name(c.to_string());
                }
                // do nothing for any other selector
                _ => {}
            }
        }

        Ok(())
    }
}
