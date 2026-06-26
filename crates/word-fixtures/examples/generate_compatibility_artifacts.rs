use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/compatibility"));
    fs::create_dir_all(&output_dir)?;

    let document = word_fixtures::compatibility_sample();
    let base = output_dir.join("900word-compatibility-sample");

    fs::write(
        base.with_extension("odt"),
        word_odf::write_odt_bytes(&document)?,
    )?;
    fs::write(
        base.with_extension("docx"),
        word_docx::write_docx_bytes(&document)?,
    )?;
    fs::write(
        base.with_extension("txt"),
        word_export::export_txt(&document)?,
    )?;
    fs::write(
        base.with_extension("html"),
        word_export::export_html(&document)?,
    )?;
    fs::write(
        output_dir.join("900word-compatibility-sample.print.html"),
        word_export::export_print_html(&document)?,
    )?;
    fs::write(
        base.with_extension("pdf"),
        word_export::export_basic_pdf(&document)?,
    )?;

    fs::write(output_dir.join("README.md"), compatibility_readme())?;

    println!("Generated compatibility artifacts.");
    Ok(())
}

fn compatibility_readme() -> &'static str {
    "# 900Word Compatibility Artifacts\n\n\
Generated placeholder files for manual compatibility testing.\n\n\
Do not commit these files. Do not edit them to include real user, customer, school, NGO, legal, medical, financial, or personal content.\n\n\
Use docs/COMPATIBILITY_TESTING.md for the manual matrix and evidence template.\n"
}
