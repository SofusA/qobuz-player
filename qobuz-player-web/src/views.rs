use std::path::Path;

use skabelon::Templates;

pub(crate) fn templates(root_dir: &Path) -> Templates {
    let dir = format!("{}/**/*.html", root_dir.to_str().unwrap());
    let mut templates = Templates::new();
    templates.load_glob(&dir);

    templates
}
