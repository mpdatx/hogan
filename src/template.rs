use config::Environment;
use failure::Error;
use find_file_paths;
use handlebars::Handlebars;
use regex::Regex;
use zip::write::{FileOptions, ZipWriter};
use zip::CompressionMethod::Stored;

use std::clone::Clone;
use std::fs;
use std::io::{Cursor, Write};
use std::path::PathBuf;

pub struct TemplateDir {
    directory: PathBuf,
}

impl TemplateDir {
    pub fn new(path: PathBuf) -> Result<TemplateDir, Error> {
        if !path.is_dir() {
            bail!(
                "{:?} either does not exist or is not a directory. It needs to be both",
                path
            )
        } else {
            Ok(TemplateDir { directory: path })
        }
    }

    pub fn find(&self, filter: Regex) -> Vec<Template> {
        find_file_paths(&self.directory, filter)
            .filter_map(|path| Template::from_path_buf(path).ok())
            .collect()
    }
}

pub struct Template {
    pub path: PathBuf,
    pub contents: String,
}

impl Template {
    fn from_path_buf(path: PathBuf) -> Result<Template, Error> {
        Ok(Template {
            path: path.clone(),
            contents: fs::read_to_string(path)?,
        })
    }
}

impl Template {
    pub fn render(
        &self,
        handlebars: &Handlebars,
        environment: &Environment,
    ) -> Result<Rendered, Error> {
        let mut buf = Cursor::new(Vec::new());
        handlebars.render_template_to_write(&self.contents, &environment.config_data, &mut buf)?;

        Ok(Rendered {
            path: self.path.clone().with_file_name(
                self.path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .replace("template", &environment.environment),
            ),
            contents: buf.into_inner(),
        })
    }

    pub fn render_to_zip(
        &self,
        handlebars: &Handlebars,
        environments: &Vec<Environment>,
    ) -> Result<Vec<u8>, Error> {
        let options = FileOptions::default().compression_method(Stored);
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));

        for environment in environments {
            let rendered = self.render(&handlebars, &environment)?;
            zip.start_file(
                rendered.path.file_name().unwrap().to_string_lossy(),
                options,
            )?;
            zip.write_all(&rendered.contents)?;
        }

        Ok(zip.finish()?.into_inner())
    }
}

pub struct Rendered {
    pub path: PathBuf,
    pub contents: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::RegexBuilder;

    #[test]
    fn test_find_all_templates() {
        let template_dir =
            TemplateDir::new(PathBuf::from("tests/fixtures/projects/templates")).unwrap();
        let templates = template_dir.find(
            RegexBuilder::new("(.*\\.)?template(\\.Release|\\-liquibase|\\-quartz)?\\.([Cc]onfig|yaml|properties)$")
                .case_insensitive(true)
                .build()
                .unwrap(),
        );
        assert_eq!(templates.len(), 3)
    }
}
