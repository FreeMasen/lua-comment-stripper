use std::path::PathBuf;

use ::lua_comment_stripper::walk_dir;
use pyo3::prelude::*;

#[pyfunction]
pub fn strip_comments(
    input: &str,
    output: &str,
    diff_verbose: bool,
    diff_dir: Option<&str>,
) -> PyResult<()> {
    walk_dir(
        PathBuf::from(input),
        PathBuf::from(output),
        diff_dir.map(PathBuf::from),
        diff_verbose,
    );
    Ok(())
}

#[pymodule]
pub fn lcs_python_bindings(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(strip_comments, m)?)?;
    Ok(())
}
